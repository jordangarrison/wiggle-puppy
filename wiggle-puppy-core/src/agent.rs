//! Agent execution for the Wiggle Puppy runner.
//!
//! This module provides the `Agent` struct for spawning and managing
//! external AI agent processes (like Claude, Aider, etc.), streaming
//! their output through the event system, and capturing results.

use crate::error::{Error, Result};
use crate::event::{Event, EventSender};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

/// An agent that can be spawned to execute tasks.
///
/// The agent wraps an external command (like `claude` or `aider`) and provides
/// methods to run it with a prompt, streaming output through the event system.
#[derive(Debug, Clone)]
pub struct Agent {
    /// The command to run.
    command: String,
    /// Arguments to pass to the command.
    args: Vec<String>,
    /// Patterns to detect in output that indicate an error.
    error_patterns: Vec<String>,
    /// Timeout in seconds for the agent process.
    timeout_secs: u64,
}

impl Agent {
    /// Create a new agent with the given command, arguments, error patterns, and timeout.
    ///
    /// # Arguments
    ///
    /// * `command` - The command to run (e.g., "claude", "aider").
    /// * `args` - Arguments to pass to the command.
    /// * `error_patterns` - Patterns to detect in output that indicate an error.
    /// * `timeout_secs` - Timeout in seconds for the agent process.
    ///
    /// # Examples
    ///
    /// ```
    /// use wiggle_puppy_core::Agent;
    ///
    /// let agent = Agent::new(
    ///     "claude",
    ///     vec!["-p".to_string()],
    ///     vec!["FATAL ERROR".to_string()],
    ///     300,
    /// );
    /// ```
    pub fn new(
        command: impl Into<String>,
        args: Vec<String>,
        error_patterns: Vec<String>,
        timeout_secs: u64,
    ) -> Self {
        Self {
            command: command.into(),
            args,
            error_patterns,
            timeout_secs,
        }
    }

    /// Get the command this agent will run.
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Get the arguments this agent will pass.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Run the agent with the given prompt.
    ///
    /// Spawns the agent process, passes the prompt as the final argument,
    /// and streams stdout/stderr through the provided event sender.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt to pass to the agent.
    /// * `events` - Channel sender for streaming output events.
    ///
    /// # Returns
    ///
    /// Returns `AgentOutput` containing the captured output and exit information.
    ///
    /// # Errors
    ///
    /// Returns `Error::AgentNotFound` if the command cannot be found.
    /// Returns `Error::AgentError` if the process fails to spawn or run.
    pub async fn run(&self, prompt: &str, events: &EventSender) -> Result<AgentOutput> {
        let start = Instant::now();

        let mut cmd = Command::new(&self.command);
        cmd.args(&self.args);
        cmd.arg(prompt);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::AgentNotFound {
                    command: self.command.clone(),
                }
            } else {
                Error::AgentError {
                    message: format!("failed to spawn agent process: {}", e),
                }
            }
        })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::agent_error("failed to capture stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| Error::agent_error("failed to capture stderr"))?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();
        let mut combined_lines = Vec::new();

        // Track error patterns detected during streaming
        let mut detected_error: Option<String> = None;

        // Read stdout and stderr concurrently
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(text)) => {
                            stdout_lines.push(text.clone());
                            combined_lines.push(text.clone());
                            let _ = events.send(Event::agent_output(&text)).await;
                            // Check for error patterns
                            for pattern in &self.error_patterns {
                                if text.contains(pattern) {
                                    detected_error = Some(pattern.clone());
                                }
                            }
                        }
                        Ok(None) => {
                            // stdout closed, but stderr might still have data
                            // Continue reading stderr only
                            while let Ok(Some(text)) = stderr_reader.next_line().await {
                                stderr_lines.push(text.clone());
                                combined_lines.push(text.clone());
                                let _ = events.send(Event::agent_stderr(&text)).await;
                                // Check for error patterns in stderr
                                for pattern in &self.error_patterns {
                                    if text.contains(pattern) {
                                        detected_error = Some(pattern.clone());
                                    }
                                }
                            }
                            break;
                        }
                        Err(e) => {
                            let _ = events.send(Event::error(format!("error reading stdout: {}", e))).await;
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(text)) => {
                            stderr_lines.push(text.clone());
                            combined_lines.push(text.clone());
                            let _ = events.send(Event::agent_stderr(&text)).await;
                            // Check for error patterns
                            for pattern in &self.error_patterns {
                                if text.contains(pattern) {
                                    detected_error = Some(pattern.clone());
                                }
                            }
                        }
                        Ok(None) => {
                            // stderr closed, continue with stdout only
                        }
                        Err(e) => {
                            let _ = events.send(Event::error(format!("error reading stderr: {}", e))).await;
                        }
                    }
                }
            }
        }

        // Handle detected error pattern
        if let Some(pattern) = detected_error {
            let _ = child.kill().await;
            let _ = events
                .send(Event::AgentErrorDetected {
                    pattern: pattern.clone(),
                })
                .await;
            return Err(Error::agent_error_detected(pattern));
        }

        // Wait for process with timeout
        let status = match timeout(Duration::from_secs(self.timeout_secs), child.wait()).await {
            Ok(Ok(status)) => status,
            Ok(Err(e)) => {
                return Err(Error::AgentError {
                    message: format!("wait failed: {}", e),
                })
            }
            Err(_) => {
                let _ = child.kill().await;
                let _ = events
                    .send(Event::AgentTimeout {
                        timeout_secs: self.timeout_secs,
                    })
                    .await;
                return Err(Error::agent_timeout(self.timeout_secs));
            }
        };

        let duration_secs = start.elapsed().as_secs_f64();
        let exit_code = status.code();

        let _ = events
            .send(Event::AgentFinished {
                exit_code,
                duration_secs,
            })
            .await;

        Ok(AgentOutput {
            stdout: stdout_lines.join("\n"),
            stderr: stderr_lines.join("\n"),
            combined: combined_lines.join("\n"),
            exit_code,
            duration_secs,
        })
    }
}

/// Output captured from an agent run.
#[derive(Debug, Clone)]
pub struct AgentOutput {
    /// All stdout output, joined with newlines.
    pub stdout: String,
    /// All stderr output, joined with newlines.
    pub stderr: String,
    /// Combined stdout and stderr in order received, joined with newlines.
    pub combined: String,
    /// Exit code of the process, if available.
    pub exit_code: Option<i32>,
    /// Duration of the run in seconds.
    pub duration_secs: f64,
}

impl AgentOutput {
    /// Create an empty AgentOutput for retry scenarios.
    ///
    /// # Examples
    ///
    /// ```
    /// use wiggle_puppy_core::AgentOutput;
    ///
    /// let output = AgentOutput::empty();
    /// assert!(output.stdout.is_empty());
    /// assert!(output.stderr.is_empty());
    /// assert!(output.combined.is_empty());
    /// assert_eq!(output.exit_code, None);
    /// assert_eq!(output.duration_secs, 0.0);
    /// ```
    pub fn empty() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            combined: String::new(),
            exit_code: None,
            duration_secs: 0.0,
        }
    }

    /// Check if the combined output contains the given phrase.
    ///
    /// # Examples
    ///
    /// ```
    /// use wiggle_puppy_core::AgentOutput;
    ///
    /// let output = AgentOutput {
    ///     stdout: "Task complete!".to_string(),
    ///     stderr: String::new(),
    ///     combined: "Task complete!".to_string(),
    ///     exit_code: Some(0),
    ///     duration_secs: 1.5,
    /// };
    ///
    /// assert!(output.contains("complete"));
    /// assert!(!output.contains("error"));
    /// ```
    pub fn contains(&self, phrase: &str) -> bool {
        self.combined.contains(phrase)
    }

    /// Get the last N lines from the combined output.
    ///
    /// # Examples
    ///
    /// ```
    /// use wiggle_puppy_core::AgentOutput;
    ///
    /// let output = AgentOutput {
    ///     stdout: "line 1\nline 2\nline 3\nline 4\nline 5".to_string(),
    ///     stderr: String::new(),
    ///     combined: "line 1\nline 2\nline 3\nline 4\nline 5".to_string(),
    ///     exit_code: Some(0),
    ///     duration_secs: 1.5,
    /// };
    ///
    /// assert_eq!(output.last_lines(2), vec!["line 4", "line 5"]);
    /// assert_eq!(output.last_lines(10), vec!["line 1", "line 2", "line 3", "line 4", "line 5"]);
    /// ```
    pub fn last_lines(&self, n: usize) -> Vec<&str> {
        let lines: Vec<&str> = self.combined.lines().collect();
        let start = lines.len().saturating_sub(n);
        lines[start..].to_vec()
    }

    /// Check if the agent exited successfully (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Get the total number of lines in the combined output.
    pub fn line_count(&self) -> usize {
        if self.combined.is_empty() {
            0
        } else {
            self.combined.lines().count()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::channel;

    #[test]
    fn test_agent_new() {
        let agent = Agent::new(
            "claude",
            vec!["-p".to_string()],
            vec!["FATAL ERROR".to_string()],
            300,
        );
        assert_eq!(agent.command(), "claude");
        assert_eq!(agent.args(), &["-p".to_string()]);
    }

    #[test]
    fn test_agent_output_empty() {
        let output = AgentOutput::empty();
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        assert!(output.combined.is_empty());
        assert_eq!(output.exit_code, None);
        assert_eq!(output.duration_secs, 0.0);
        assert!(!output.success());
        assert_eq!(output.line_count(), 0);
    }

    #[test]
    fn test_agent_output_contains() {
        let output = AgentOutput {
            stdout: "Hello world".to_string(),
            stderr: String::new(),
            combined: "Hello world\n<promise>COMPLETE</promise>".to_string(),
            exit_code: Some(0),
            duration_secs: 1.0,
        };

        assert!(output.contains("COMPLETE"));
        assert!(output.contains("<promise>COMPLETE</promise>"));
        assert!(output.contains("Hello"));
        assert!(!output.contains("goodbye"));
    }

    #[test]
    fn test_agent_output_last_lines() {
        let output = AgentOutput {
            stdout: String::new(),
            stderr: String::new(),
            combined: "one\ntwo\nthree\nfour\nfive".to_string(),
            exit_code: Some(0),
            duration_secs: 1.0,
        };

        assert_eq!(output.last_lines(3), vec!["three", "four", "five"]);
        assert_eq!(output.last_lines(1), vec!["five"]);
        assert_eq!(
            output.last_lines(10),
            vec!["one", "two", "three", "four", "five"]
        );
        assert_eq!(output.last_lines(0), Vec::<&str>::new());
    }

    #[test]
    fn test_agent_output_last_lines_empty() {
        let output = AgentOutput {
            stdout: String::new(),
            stderr: String::new(),
            combined: String::new(),
            exit_code: Some(0),
            duration_secs: 0.0,
        };

        assert_eq!(output.last_lines(3), Vec::<&str>::new());
    }

    #[test]
    fn test_agent_output_success() {
        let mut output = AgentOutput {
            stdout: String::new(),
            stderr: String::new(),
            combined: String::new(),
            exit_code: Some(0),
            duration_secs: 0.0,
        };
        assert!(output.success());

        output.exit_code = Some(1);
        assert!(!output.success());

        output.exit_code = None;
        assert!(!output.success());
    }

    #[test]
    fn test_agent_output_line_count() {
        let output = AgentOutput {
            stdout: String::new(),
            stderr: String::new(),
            combined: "one\ntwo\nthree".to_string(),
            exit_code: Some(0),
            duration_secs: 0.0,
        };
        assert_eq!(output.line_count(), 3);

        let empty_output = AgentOutput {
            stdout: String::new(),
            stderr: String::new(),
            combined: String::new(),
            exit_code: Some(0),
            duration_secs: 0.0,
        };
        assert_eq!(empty_output.line_count(), 0);
    }

    #[tokio::test]
    async fn test_agent_run_echo() {
        let agent = Agent::new("echo", vec![], vec![], 60);
        let (tx, mut rx) = channel();

        let result = agent.run("hello world", &tx).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("hello world"));
        assert!(output.success());

        // Drain events
        drop(tx);
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_agent_run_not_found() {
        let agent = Agent::new(
            "nonexistent-command-that-does-not-exist",
            vec![],
            vec![],
            60,
        );
        let (tx, _rx) = channel();

        let result = agent.run("test", &tx).await;
        assert!(result.is_err());

        match result {
            Err(Error::AgentNotFound { command }) => {
                assert_eq!(command, "nonexistent-command-that-does-not-exist");
            }
            _ => panic!("expected AgentNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_agent_run_with_stderr() {
        // Use sh to echo to stderr
        let agent = Agent::new("sh", vec!["-c".to_string()], vec![], 60);
        let (tx, mut rx) = channel();

        let result = agent
            .run("echo 'stdout line'; echo 'stderr line' >&2", &tx)
            .await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.stdout.contains("stdout line"));
        assert!(output.stderr.contains("stderr line"));
        assert!(output.combined.contains("stdout line"));
        assert!(output.combined.contains("stderr line"));

        // Drain events and check we got both stdout and stderr events
        drop(tx);
        let mut stdout_events = 0;
        let mut stderr_events = 0;
        while let Some(event) = rx.recv().await {
            if let Event::AgentOutput { is_stderr, .. } = event {
                if is_stderr {
                    stderr_events += 1;
                } else {
                    stdout_events += 1;
                }
            }
        }
        assert!(stdout_events > 0);
        assert!(stderr_events > 0);
    }
}
