use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;

use dbgmcp::{CLIDebugSession, CLIDebugger, generate_session_id};

use rmcp::{
    ServerHandler, ServiceExt,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool,
};

#[derive(Clone)]
pub(crate) struct GdbServer {
    sessions: Arc<Mutex<HashMap<String, CLIDebugSession>>>,
}

#[tool(tool_box)]
impl ServerHandler for GdbServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("GNU Debugger".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tool(tool_box)]
impl GdbServer {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tool(
        description = "Start a new GDB debugging session. When done using it, terminate the session"
    )]
    async fn gdb_start(&self) -> Result<String, String> {
        let session_id = format!("gdb-{}", generate_session_id());

        let mut session = CLIDebugger::new("gdb")
            .args(["--interpreter=mi"])
            .prompt("(gdb)")
            .spawn()
            .map_err(|err| format!("Failed to start GDB session. [Error]: {}", err))?;
        let response = session
            .read_response()
            .await
            .map_err(|err| format!("Failed to read from GDB session. [Error]: {}", err))?;

        self.sessions
            .lock()
            .await
            .insert(session_id.clone(), session);
        Ok(format!(
            "GDB session started with ID {}. [GDB output]: {}",
            session_id, response
        ))
    }

    #[tool(description = "Load a program into existing GDB session")]
    async fn gdb_load(
        &self,
        #[tool(param)]
        #[schemars(description = "GDB session ID")]
        session_id: String,
        #[tool(param)]
        #[schemars(description = "Absolute path to the program to debug")]
        program: String,
        #[tool(param)]
        #[schemars(description = "Arguments to pass to the program")]
        arguments: Option<Vec<String>>,
    ) -> Result<String, String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(&session_id).ok_or(format!(
            "Session with ID {} not found. Start a new session",
            session_id
        ))?;

        let run_commands = async || -> Result<String, std::io::Error> {
            let mut response = session
                .execute_command(&format!("file {}", program))
                .await?;
            if let Some(args) = arguments {
                let args_response = session
                    .execute_command(&format!("set args {}", args.join(" ")))
                    .await?;
                response.push_str(&args_response);
            }
            Ok(response)
        };
        let response = run_commands()
            .await
            .map_err(|err| format!("Failed to load program. [Error]: {}", err))?;

        Ok(format!(
            "Program loaded into GDB.\n [GDB output]: {}",
            response
        ))
    }

    #[tool(description = "Execute a GDB command")]
    async fn gdb_command(
        &self,
        #[tool(param)]
        #[schemars(description = "GDB session ID")]
        session_id: String,
        #[tool(param)]
        #[schemars(description = "GDB command to execute")]
        command: String,
    ) -> Result<String, String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(&session_id).ok_or(format!(
            "Session with ID {} not found. Start a new session",
            session_id
        ))?;

        let response = session
            .execute_command(&command)
            .await
            .map_err(|err| format!("Failed to execute GDB command. [Error]: {}", err))?;

        Ok(format!("Command executed.\n[GDB output]: {}", response))
    }

    #[tool(description = "Wait for GDB debugee to hit a breakpoint or stop running")]
    async fn gdb_wait(
        &self,
        #[tool(param)]
        #[schemars(description = "GDB session ID")]
        session_id: String,
        #[tool(param)]
        #[schemars(description = "Timeout in seconds")]
        timeout: Option<u64>,
    ) -> Result<String, String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(&session_id).ok_or(format!(
            "Session with ID {} not found. Start a new session",
            session_id
        ))?;
        let timeout = Duration::from_secs(timeout.unwrap_or(10));

        let response = session
            .read_response_until(Some("*stopped"), timeout)
            .await
            .map_err(|err| format!("Failed to read from GDB session. [Error]: {}", err))?;

        Ok(format!("GDB debugee stopped.\n[GDB output]: {}", response))
    }

    #[tool(description = "Terminate a GDB session")]
    async fn gdb_terminate(
        &self,
        #[tool(param)]
        #[schemars(description = "GDB session ID")]
        session_id: String,
    ) -> Result<String, String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(&session_id).ok_or(format!(
            "Session with ID {} not found. Start a new session",
            session_id
        ))?;

        session
            .terminate()
            .await
            .map_err(|err| format!("Failed to terminate GDB session. [Error]: {}", err))?;
        sessions.remove(&session_id);
        Ok("GDB session terminated".to_string())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let gdb_service = GdbServer::new()
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;
    gdb_service.waiting().await?;

    Ok(())
}
