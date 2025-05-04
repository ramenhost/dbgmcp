use std::ffi::OsStr;
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    time::{self, Duration},
};

pub struct CLIDebugSession {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::io::BufReader<tokio::process::ChildStdout>,
    stderr: tokio::io::BufReader<tokio::process::ChildStderr>,
    prompt: String,
    quit_command: String,
}

pub struct CLIDebugger {
    command: tokio::process::Command,
    prompt: Option<String>,
    quit_command: Option<String>,
}

impl CLIDebugger {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        let command = tokio::process::Command::new(program);
        Self {
            command,
            prompt: None,
            quit_command: None,
        }
    }

    #[allow(unused)]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    #[allow(unused)]
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    #[allow(unused)]
    pub fn quit_command<S: Into<String>>(mut self, quit_command: S) -> Self {
        self.quit_command = Some(quit_command.into());
        self
    }

    pub fn spawn(mut self) -> Result<CLIDebugSession, std::io::Error> {
        let mut child = self
            .command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        Ok(CLIDebugSession {
            stdin: child.stdin.take().unwrap(),
            stdout: tokio::io::BufReader::new(child.stdout.take().unwrap()),
            stderr: tokio::io::BufReader::new(child.stderr.take().unwrap()),
            child,
            prompt: self.prompt.unwrap_or(String::from("$")),
            quit_command: self.quit_command.unwrap_or(String::from("quit")),
        })
    }
}

const CHILD_READ_TIMEOUT: Duration = Duration::from_secs(10);
impl CLIDebugSession {
    pub async fn send_command(&mut self, command: &str) -> Result<(), std::io::Error> {
        self.stdin.write_all(command.as_bytes()).await?;
        self.stdin.write_u8(b'\n').await?;
        self.stdin.flush().await?;
        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<String, std::io::Error> {
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
                    output.push_str("[command timed out]");
                    break Ok(output);
                }
            }
            stdout_buffer.clear();
            stderr_buffer.clear();

            // Check if we got next input prompt
            if output.contains(&self.prompt) {
                break Ok(output);
            }
        }
    }

    pub async fn execute_command<S: AsRef<str>>(
        &mut self,
        command: S,
    ) -> Result<String, std::io::Error> {
        self.send_command(command.as_ref()).await?;
        self.read_response().await
    }

    pub async fn terminate(&mut self) -> Result<(), std::io::Error> {
        self.send_command(self.quit_command.clone().as_str())
            .await?;
        self.child.wait().await?;
        Ok(())
    }
}

pub fn generate_session_id() -> u32 {
    static SESSION_ID: AtomicUsize = AtomicUsize::new(0);
    SESSION_ID.fetch_add(1, Ordering::Relaxed) as u32
}
