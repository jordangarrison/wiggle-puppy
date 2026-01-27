//! Configuration for the Wiggle Puppy agent loop.
//!
//! This module provides the `Config` struct with a builder pattern
//! for configuring the agent command, iteration limits, delays,
//! completion detection, and prompt handling.

use crate::error::{Error, Result};
use std::path::PathBuf;
use std::time::Duration;

/// Default agent command.
const DEFAULT_AGENT_COMMAND: &str = "claude";

/// Default agent arguments.
const DEFAULT_AGENT_ARGS: &str = "-p";

/// Default maximum iterations.
const DEFAULT_MAX_ITERATIONS: u32 = 20;

/// Default delay between iterations in seconds.
const DEFAULT_DELAY_SECS: u64 = 2;

/// Default completion phrase to detect.
const DEFAULT_COMPLETION_PHRASE: &str = "<promise>COMPLETE</promise>";

/// Default auto-completion instruction appended to prompts.
const DEFAULT_COMPLETION_INSTRUCTION: &str = "\n\nIMPORTANT: When you have completed ALL tasks in this prompt and there is nothing left to do, output exactly: <promise>COMPLETE</promise>\nDo NOT output this phrase until every single task is fully complete. Only output it once at the very end.";

/// Configuration for the Wiggle Puppy runner.
#[derive(Debug, Clone)]
pub struct Config {
    /// The agent command to run (e.g., "claude", "aider").
    pub agent_command: String,

    /// Arguments to pass to the agent command.
    pub agent_args: Vec<String>,

    /// Maximum number of iterations before stopping.
    pub max_iterations: u32,

    /// Delay between iterations.
    pub delay: Duration,

    /// Phrase that signals completion when detected in output.
    pub completion_phrase: String,

    /// Path to the PRD JSON file (optional).
    pub prd_path: Option<PathBuf>,

    /// Path to the prompt file (optional if prompt_text is set).
    pub prompt_path: Option<PathBuf>,

    /// Inline prompt text (optional if prompt_path is set).
    pub prompt_text: Option<String>,

    /// Path to the progress log file (optional).
    pub progress_path: Option<PathBuf>,

    /// Whether to append the auto-completion instruction to prompts.
    pub auto_completion_instruction: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent_command: DEFAULT_AGENT_COMMAND.to_string(),
            agent_args: DEFAULT_AGENT_ARGS
                .split_whitespace()
                .map(String::from)
                .collect(),
            max_iterations: DEFAULT_MAX_ITERATIONS,
            delay: Duration::from_secs(DEFAULT_DELAY_SECS),
            completion_phrase: DEFAULT_COMPLETION_PHRASE.to_string(),
            prd_path: None,
            prompt_path: None,
            prompt_text: None,
            progress_path: None,
            auto_completion_instruction: true,
        }
    }
}

impl Config {
    /// Create a new Config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the agent command.
    pub fn agent_command(mut self, command: impl Into<String>) -> Self {
        self.agent_command = command.into();
        self
    }

    /// Set the agent arguments.
    pub fn agent_args(mut self, args: Vec<String>) -> Self {
        self.agent_args = args;
        self
    }

    /// Set the agent arguments from a string (space-separated).
    pub fn agent_args_str(mut self, args: impl Into<String>) -> Self {
        self.agent_args = args
            .into()
            .split_whitespace()
            .map(String::from)
            .collect();
        self
    }

    /// Set the maximum number of iterations.
    pub fn max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }

    /// Set the delay between iterations.
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Set the delay between iterations in seconds.
    pub fn delay_secs(mut self, secs: u64) -> Self {
        self.delay = Duration::from_secs(secs);
        self
    }

    /// Set the completion phrase.
    pub fn completion_phrase(mut self, phrase: impl Into<String>) -> Self {
        self.completion_phrase = phrase.into();
        self
    }

    /// Set the PRD file path.
    pub fn prd_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.prd_path = Some(path.into());
        self
    }

    /// Set the prompt file path.
    pub fn prompt_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.prompt_path = Some(path.into());
        self
    }

    /// Set the inline prompt text.
    pub fn prompt_text(mut self, text: impl Into<String>) -> Self {
        self.prompt_text = Some(text.into());
        self
    }

    /// Set the progress log file path.
    pub fn progress_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.progress_path = Some(path.into());
        self
    }

    /// Enable or disable the auto-completion instruction.
    pub fn auto_completion_instruction(mut self, enabled: bool) -> Self {
        self.auto_completion_instruction = enabled;
        self
    }

    /// Get a formatted display string for the agent command.
    ///
    /// Returns the command and arguments as they would appear on the command line.
    pub fn agent_display(&self) -> String {
        if self.agent_args.is_empty() {
            self.agent_command.clone()
        } else {
            format!("{} {}", self.agent_command, self.agent_args.join(" "))
        }
    }

    /// Get the prompt content.
    ///
    /// If `prompt_path` is set, reads from the file. Otherwise, returns `prompt_text`.
    /// If `auto_completion_instruction` is enabled, appends the completion instruction.
    ///
    /// # Errors
    ///
    /// Returns `Error::NoPrompt` if neither prompt_path nor prompt_text is set.
    /// Returns `Error::PromptReadError` if the prompt file cannot be read.
    pub fn get_prompt(&self) -> Result<String> {
        let base_prompt = if let Some(path) = &self.prompt_path {
            std::fs::read_to_string(path).map_err(|source| Error::PromptReadError {
                path: path.clone(),
                source,
            })?
        } else if let Some(text) = &self.prompt_text {
            text.clone()
        } else {
            return Err(Error::NoPrompt);
        };

        if self.auto_completion_instruction {
            Ok(format!("{}{}", base_prompt, DEFAULT_COMPLETION_INSTRUCTION))
        } else {
            Ok(base_prompt)
        }
    }

    /// Check if this config has a prompt configured.
    pub fn has_prompt(&self) -> bool {
        self.prompt_path.is_some() || self.prompt_text.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_values() {
        let config = Config::default();
        assert_eq!(config.agent_command, "claude");
        assert_eq!(config.agent_args, vec!["-p"]);
        assert_eq!(config.max_iterations, 20);
        assert_eq!(config.delay, Duration::from_secs(2));
        assert!(config.completion_phrase.contains("COMPLETE"));
        assert!(config.auto_completion_instruction);
    }

    #[test]
    fn test_builder_pattern() {
        let config = Config::new()
            .agent_command("aider")
            .agent_args_str("--yes --no-auto-commits")
            .max_iterations(10)
            .delay_secs(5)
            .completion_phrase("DONE")
            .auto_completion_instruction(false);

        assert_eq!(config.agent_command, "aider");
        assert_eq!(config.agent_args, vec!["--yes", "--no-auto-commits"]);
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.delay, Duration::from_secs(5));
        assert_eq!(config.completion_phrase, "DONE");
        assert!(!config.auto_completion_instruction);
    }

    #[test]
    fn test_agent_display() {
        let config = Config::default();
        assert_eq!(config.agent_display(), "claude -p");

        let config = Config::new().agent_command("echo").agent_args(vec![]);
        assert_eq!(config.agent_display(), "echo");

        let config = Config::new()
            .agent_command("custom-agent")
            .agent_args_str("-a -b --flag");
        assert_eq!(config.agent_display(), "custom-agent -a -b --flag");
    }

    #[test]
    fn test_get_prompt_from_text() {
        let config = Config::new()
            .prompt_text("Hello world")
            .auto_completion_instruction(false);

        let prompt = config.get_prompt().unwrap();
        assert_eq!(prompt, "Hello world");
    }

    #[test]
    fn test_get_prompt_with_instruction() {
        let config = Config::new()
            .prompt_text("Do something")
            .auto_completion_instruction(true);

        let prompt = config.get_prompt().unwrap();
        assert!(prompt.starts_with("Do something"));
        assert!(prompt.contains("<promise>COMPLETE</promise>"));
    }

    #[test]
    fn test_get_prompt_no_prompt_error() {
        let config = Config::new();
        let result = config.get_prompt();
        assert!(matches!(result, Err(Error::NoPrompt)));
    }

    #[test]
    fn test_get_prompt_from_file() {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join("test_prompt.txt");
        std::fs::write(&temp_path, "File prompt content").unwrap();

        let config = Config::new()
            .prompt_path(&temp_path)
            .auto_completion_instruction(false);

        let prompt = config.get_prompt().unwrap();
        assert_eq!(prompt, "File prompt content");

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_has_prompt() {
        let config = Config::new();
        assert!(!config.has_prompt());

        let config = Config::new().prompt_text("test");
        assert!(config.has_prompt());

        let config = Config::new().prompt_path("/some/path");
        assert!(config.has_prompt());
    }
}
