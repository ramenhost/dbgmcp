//! Wrapper over tokio::process::Command to interact with a CLI debugger process.
use std::ffi::OsStr;
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    time::{self, Duration},
};

/// A debugging session that wraps a running CLI debugger process. It abstracts interaction with the inner debugger process.
/// Use [`CLIDebugger::spawn`] to create a new CLIDebugSession instance.
pub struct CLIDebugSession {
    child: tokio::process::Child,
    stdin: tokio::process::ChildStdin,
    stdout: tokio::io::BufReader<tokio::process::ChildStdout>,
    stderr: tokio::io::BufReader<tokio::process::ChildStderr>,
    prompt: String,
    quit_command: String,
}

/// A CLI debugger program that when spawned, creates a new [`CLIDebugSession`] instance.
pub struct CLIDebugger {
    command: tokio::process::Command,
    prompt: Option<String>,
    quit_command: Option<String>,
}

impl CLIDebugger {
    /// Creates a new [`CLIDebugger`] instance from the given program.
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        let command = tokio::process::Command::new(program);
        Self {
            command,
            prompt: None,
            quit_command: None,
        }
    }

    /// Adds arguments to the debugger program.
    #[allow(unused)]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    /// Sets the prompt used by the debugger program.
    /// The prompt (example: "(gdb)") is used to keep the interaction in sync with the debugger.
    /// The default is ">".
    #[allow(unused)]
    pub fn prompt<S: Into<String>>(mut self, prompt: S) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Sets the command used to quit the debugger program. The default is "quit".
    #[allow(unused)]
    pub fn quit_command<S: Into<String>>(mut self, quit_command: S) -> Self {
        self.quit_command = Some(quit_command.into());
        self
    }

    /// Start a new debugger session. The Ok value returned is a [`CLIDebugSession`] instance that corresponds to the spawned debugger process.
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
            prompt: self.prompt.unwrap_or(String::from(">")),
            quit_command: self.quit_command.unwrap_or(String::from("quit")),
        })
    }
}

const CHILD_READ_TIMEOUT: Duration = Duration::from_secs(10);
impl CLIDebugSession {
    /// Send a command to the inner debugger process followed by a newline.
    pub async fn send_command(&mut self, command: &str) -> Result<(), std::io::Error> {
        self.stdin.write_all(command.as_bytes()).await?;
        self.stdin.write_u8(b'\n').await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// Read the response from the inner debugger process until a pattern is matched or a timeout occurs.
    pub async fn read_response_until<S: AsRef<str>>(
        &mut self,
        pattern: Option<S>,
        timeout: Duration,
    ) -> Result<String, std::io::Error> {
        let mut stdout_buffer = String::new();
        let mut stderr_buffer = String::new();
        let mut output = String::new();

        let sleep = time::sleep(timeout);
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
                    // Timeout occurred, stop reading
                    if output.is_empty() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "Timeout while waiting for response",
                        ));
                    }
                    break Ok(output);
                }
            }
            stdout_buffer.clear();
            stderr_buffer.clear();

            // Check if we got next input prompt and the expected pattern if any
            if let Some(pattern) = &pattern {
                if output.contains(pattern.as_ref()) && output.contains(&self.prompt) {
                    break Ok(output);
                }
            } else if output.contains(&self.prompt) {
                break Ok(output);
            }
        }
    }

    /// Read the response from the inner debugger process until next prompt appears.
    /// This function can timeout if no prompt is received within the default timeout (10s).
    pub async fn read_response(&mut self) -> Result<String, std::io::Error> {
        self.read_response_until::<&str>(None, CHILD_READ_TIMEOUT)
            .await
    }

    /// Send a command to the inner debugger process and read the response until next prompt appears.
    /// This function can timeout if no prompt is received within the default timeout (10s).
    pub async fn execute_command<S: AsRef<str>>(
        &mut self,
        command: S,
    ) -> Result<String, std::io::Error> {
        let one_msecond = Duration::from_millis(1);
        let mut response = self
            .read_response_until::<&str>(None, one_msecond)
            .await
            .unwrap_or_default();
        self.send_command(command.as_ref()).await?;
        response.push_str(&self.read_response().await?);
        response.push_str(
            &self
                .read_response_until::<&str>(None, one_msecond)
                .await
                .unwrap_or_default(),
        );
        Ok(response)
    }

    /// Gracefully terminate the inner debugger process.
    pub async fn terminate(&mut self) -> Result<(), std::io::Error> {
        self.send_command(self.quit_command.clone().as_str())
            .await?;
        self.child.wait().await?;
        Ok(())
    }
}

/// Get a unique number for debugger session identifier.
pub fn generate_session_id() -> u32 {
    static SESSION_ID: AtomicUsize = AtomicUsize::new(0);
    SESSION_ID.fetch_add(1, Ordering::Relaxed) as u32
}
