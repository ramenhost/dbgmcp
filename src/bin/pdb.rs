use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

use dbgmcp::{CLIDebugSession, CLIDebugger, generate_session_id};

use rmcp::{
    ServerHandler, ServiceExt,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool,
};

#[derive(Clone)]
pub(crate) struct PdbServer {
    sessions: Arc<Mutex<HashMap<String, CLIDebugSession>>>,
}

#[tool(tool_box)]
impl ServerHandler for PdbServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("Python Debugger".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

#[tool(tool_box)]
impl PdbServer {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tool(
        description = "Start a new PDB debugging session. When done using it, terminate the session"
    )]
    async fn pdb_start(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the python script to debug")]
        program: String,
        #[tool(param)]
        #[schemars(description = "Arguments to pass to the python script")]
        arguments: Option<Vec<String>>,
    ) -> Result<String, String> {
        let session_id = format!("pdb-{}", generate_session_id());
        let mut pdb_args = vec!["-m".to_owned(), "pdb".to_owned(), program];
        if let Some(arg) = arguments {
            pdb_args.extend(arg);
        }
        let mut session = CLIDebugger::new("python3")
            .args(pdb_args)
            .prompt("(Pdb)")
            .spawn()
            .map_err(|err| format!("Failed to start PDB session. [Error]: {}", err))?;
        let response = session
            .read_response()
            .await
            .map_err(|err| format!("Failed to read from PDB session. [Error]: {}", err))?;

        self.sessions
            .lock()
            .await
            .insert(session_id.clone(), session);
        Ok(format!(
            "PDB session started with ID {}. [PDB output]: {}",
            session_id, response
        ))
    }

    #[tool(description = "Execute a PDB command")]
    async fn pdb_command(
        &self,
        #[tool(param)]
        #[schemars(description = "PDB session ID")]
        session_id: String,
        #[tool(param)]
        #[schemars(description = "PDB command to execute")]
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
            .map_err(|err| format!("Failed to execute PDB command. [Error]: {}", err))?;

        Ok(format!("Command executed.\n[PDB output]: {}", response))
    }

    #[tool(description = "Terminate a PDB session")]
    async fn pdb_terminate(
        &self,
        #[tool(param)]
        #[schemars(description = "PDB session ID")]
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
            .map_err(|err| format!("Failed to terminate PDB session. [Error]: {}", err))?;
        sessions.remove(&session_id);
        Ok("PDB session terminated".to_string())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdb_service = PdbServer::new()
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await?;
    pdb_service.waiting().await?;

    Ok(())
}
