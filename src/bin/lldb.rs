use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;

use dbgmcp::{CLIDebugSession, CLIDebugger, generate_session_id};

use rmcp::{
    ServerHandler, ServiceExt,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool,
};

#[derive(Clone)]
pub(crate) struct LldbServer {
    sessions: Arc<Mutex<HashMap<String, CLIDebugSession>>>,
}

#[tool(tool_box)]
impl ServerHandler for LldbServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("LLVM Debugger".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tool(tool_box)]
impl LldbServer {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tool(
        description = "Start a new LLDB debugging session. When done using it, terminate the session"
    )]
    async fn lldb_start(&self) -> Result<String, String> {
        let session_id = format!("lldb-{}", generate_session_id());

        let session = CLIDebugger::new("lldb")
            .args(["--no-use-colors", "--source-quietly"])
            .prompt("(lldb)")
            .spawn()
            .map_err(|err| format!("Failed to start LLDB session. [Error]: {}", err))?;

        self.sessions
            .lock()
            .await
            .insert(session_id.clone(), session);
        Ok(format!("LLDB session started with ID {}.", session_id))
    }

    #[tool(description = "Load a program into existing LLDB session")]
    async fn lldb_load(
        &self,
        #[tool(param)]
        #[schemars(description = "LLDB session ID")]
        session_id: String,
        #[tool(param)]
        #[schemars(description = "Path to the program to debug")]
        program: String,
    ) -> Result<String, String> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(&session_id).ok_or(format!(
            "Session with ID {} not found. Start a new session",
            session_id
        ))?;

        let response = session
            .execute_command(&format!("file {}", program))
            .await
            .map_err(|err| format!("Failed to execute LLDB command. [Error]: {}", err))?;

        Ok(format!(
            "Program loaded into LLDB.\n [LLDB output]: {}",
            response
        ))
    }

    #[tool(description = "Execute a LLDB command")]
    async fn lldb_command(
        &self,
        #[tool(param)]
        #[schemars(description = "LLDB session ID")]
        session_id: String,
        #[tool(param)]
        #[schemars(description = "LLDB command to execute")]
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
            .map_err(|err| format!("Failed to execute LLDB command. [Error]: {}", err))?;

        Ok(format!("Command executed.\n[LLDB output]: {}", response))
    }

    #[tool(description = "Wait for LLDB debugee to hit a breakpoint or stop running")]
    async fn lldb_wait(
        &self,
        #[tool(param)]
        #[schemars(description = "LLDB session ID")]
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
            .read_response_until(Some("stop reason"), timeout)
            .await
            .map_err(|err| format!("Failed to read from GDB session. [Error]: {}", err))?;

        Ok(format!(
            "LLDB debugee stopped.\n[LLDB output]: {}",
            response
        ))
    }

    #[tool(description = "Terminate a LLDB session")]
    async fn lldb_terminate(
        &self,
        #[tool(param)]
        #[schemars(description = "LLDB session ID")]
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
            .map_err(|err| format!("Failed to terminate LLDB session, [Error]: {}", err))?;
        sessions.remove(&session_id);
        Ok("LLDB session terminated".to_string())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lldb_service = LldbServer::new()
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;
    lldb_service.waiting().await?;

    Ok(())
}
