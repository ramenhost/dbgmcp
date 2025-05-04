use std::{collections::HashMap, process::Stdio, sync::Arc, time::SystemTime};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    sync::Mutex,
    time::{self, Duration},
};

use rmcp::{
    ServerHandler,
    model::{ServerCapabilities, ServerInfo},
    schemars, tool,
};

const CHILD_READ_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct Gdb {
    sessions: Arc<Mutex<HashMap<String, GdbSession>>>,
}

#[tool(tool_box)]
impl Gdb {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tool(
        description = "Start a new GDB debugging session. When done using it, terminate the session"
    )]
    async fn gdb_start(&self) -> Result<String, String> {
        let session_name = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
        let mut session =
            GdbSession::new().map_err(|err| format!("Failed to start GDB session: {}", err))?;
        let response = session
            .read_response()
            .await
            .map_err(|err| format!("Failed to start GDB session: {}", err))?;
        self.sessions
            .lock()
            .await
            .insert(session_name.clone(), session);
        Ok(format!(
            "GDB session started with ID {}. [GDB output]: {}",
            session_name, response
        ))
    }

    #[tool(description = "Load a program into a GDB session")]
    async fn gdb_load(
        &self,
        #[tool(param)]
        #[schemars(description = "GDB session ID")]
        session_id: String,
        #[tool(param)]
        #[schemars(description = "Path to the program to debug")]
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

        let mut response = session
            .execute_command(&format!("file {}", program))
            .await
            .map_err(|err| format!("Failed to execute GDB command: {}", err))?;

        if let Some(args) = arguments {
            let args_response = session
                .execute_command(&format!("set args {}", args.join(" ")))
                .await
                .map_err(|err| format!("Failed to execute GDB command: {}", err))?;
            response.push_str(&args_response);
        }
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
            .map_err(|err| format!("Failed to execute GDB command: {}", err))?;

        Ok(format!("Command executed.\n[GDB output]: {}", response))
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
            .map_err(|err| format!("Failed to terminate GDB session: {}", err))?;
        sessions.remove(&session_id);
        Ok("GDB session terminated".to_string())
    }
}

#[tool(tool_box)]
impl ServerHandler for Gdb {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("GNU Debugger".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

struct GdbSession {
    process: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::io::BufReader<tokio::process::ChildStdout>,
    stderr: tokio::io::BufReader<tokio::process::ChildStderr>,
}

impl GdbSession {
    fn new() -> Result<Self, std::io::Error> {
        let mut child = tokio::process::Command::new("gdb")
            .arg("--interpreter=mi")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        Ok(Self {
            stdin: child.stdin.take().unwrap(),
            stdout: tokio::io::BufReader::new(child.stdout.take().unwrap()),
            stderr: tokio::io::BufReader::new(child.stderr.take().unwrap()),
            process: child,
        })
    }

    async fn send_command(&mut self, command: &str) -> Result<(), std::io::Error> {
        self.stdin.write_all(command.as_bytes()).await?;
        self.stdin.write_u8(b'\n').await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_response(&mut self) -> Result<String, std::io::Error> {
        let mut stdout_buffer = String::new();
        let mut stderr_buffer = String::new();
        let mut output = String::new();

        let sleep = time::sleep(CHILD_READ_TIMEOUT);
        tokio::pin!(sleep);

        loop {
            tokio::select! {
                _ = self.stdout.read_line(&mut stdout_buffer) => {
                    output.push_str(&stdout_buffer);
                },
                _ = self.stderr.read_line(&mut stderr_buffer) => {
                    output.push_str("[stderr] ");
                    output.push_str(&stderr_buffer);
                }
                _ = &mut sleep => {
                    // Timeout occurred
                    output.push_str("[GDB timeout]");
                    break Ok(output);
                }
            }
            stdout_buffer.clear();
            stderr_buffer.clear();

            // Check if we got next gdb prompt
            if output.contains("(gdb)") {
                break Ok(output);
            }
        }
    }

    async fn execute_command(&mut self, command: &str) -> Result<String, std::io::Error> {
        self.send_command(command).await?;
        self.read_response().await
    }

    async fn terminate(&mut self) -> Result<(), std::io::Error> {
        self.send_command("quit").await?;
        self.process.wait().await?;
        Ok(())
    }
}
