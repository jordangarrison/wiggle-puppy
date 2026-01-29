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

/// Default agent execution timeout in seconds (15 minutes).
const DEFAULT_AGENT_TIMEOUT_SECS: u64 = 900;

/// Default maximum retry attempts after error/timeout.
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default initial backoff in seconds.
const DEFAULT_INITIAL_BACKOFF_SECS: u64 = 5;

/// Default backoff multiplier.
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;

/// Default circuit breaker threshold (stop after N consecutive failures).
const DEFAULT_CIRCUIT_BREAKER_THRESHOLD: u32 = 5;

/// Default error patterns that indicate Claude Code failure.
fn default_error_patterns() -> Vec<String> {
    vec![
        "Error: No messages returned".to_string(),
        "This error originated either by throwing inside of an async function".to_string(),
        "@anthropic-ai/claude-code".to_string(),
        "The promise rejected with the reason:".to_string(),
    ]
}

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

    /// Agent execution timeout in seconds.
    pub agent_timeout_secs: u64,

    /// Error patterns that indicate Claude Code failure.
    pub error_patterns: Vec<String>,

    /// Maximum retry attempts after error/timeout.
    pub max_retries: u32,

    /// Initial backoff in seconds.
    pub initial_backoff_secs: u64,

    /// Backoff multiplier.
    pub backoff_multiplier: f64,

    /// Circuit breaker threshold (stop after N consecutive failures, 0=disabled).
    pub circuit_breaker_threshold: u32,
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
            agent_timeout_secs: DEFAULT_AGENT_TIMEOUT_SECS,
            error_patterns: default_error_patterns(),
            max_retries: DEFAULT_MAX_RETRIES,
            initial_backoff_secs: DEFAULT_INITIAL_BACKOFF_SECS,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            circuit_breaker_threshold: DEFAULT_CIRCUIT_BREAKER_THRESHOLD,
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
        self.agent_args = args.into().split_whitespace().map(String::from).collect();
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

    /// Set the agent execution timeout in seconds.
    pub fn agent_timeout_secs(mut self, secs: u64) -> Self {
        self.agent_timeout_secs = secs;
        self
    }

    /// Set the error patterns that indicate Claude Code failure.
    pub fn error_patterns(mut self, patterns: Vec<String>) -> Self {
        self.error_patterns = patterns;
        self
    }

    /// Add an error pattern to the list.
    pub fn add_error_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.error_patterns.push(pattern.into());
        self
    }

    /// Clear all error patterns.
    pub fn no_error_patterns(mut self) -> Self {
        self.error_patterns.clear();
        self
    }

    /// Set the maximum retry attempts after error/timeout.
    pub fn max_retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }

    /// Set the initial backoff in seconds.
    pub fn initial_backoff_secs(mut self, secs: u64) -> Self {
        self.initial_backoff_secs = secs;
        self
    }

    /// Set the backoff multiplier.
    pub fn backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Set the circuit breaker threshold (0 to disable).
    pub fn circuit_breaker_threshold(mut self, threshold: u32) -> Self {
        self.circuit_breaker_threshold = threshold;
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

    #[test]
    fn test_default_retry_and_timeout_values() {
        let config = Config::default();
        assert_eq!(config.agent_timeout_secs, 900);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_secs, 5);
        assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
        assert_eq!(config.circuit_breaker_threshold, 5);
    }

    #[test]
    fn test_default_error_patterns() {
        let config = Config::default();
        assert!(!config.error_patterns.is_empty());
        assert!(config
            .error_patterns
            .iter()
            .any(|p| p.contains("No messages returned")));
        assert!(config
            .error_patterns
            .iter()
            .any(|p| p.contains("async function")));
        assert!(config
            .error_patterns
            .iter()
            .any(|p| p.contains("@anthropic-ai/claude-code")));
        assert!(config
            .error_patterns
            .iter()
            .any(|p| p.contains("promise rejected")));
    }

    #[test]
    fn test_agent_timeout_secs_builder() {
        let config = Config::new().agent_timeout_secs(1800);
        assert_eq!(config.agent_timeout_secs, 1800);
    }

    #[test]
    fn test_error_patterns_builder() {
        let config = Config::new().error_patterns(vec!["custom error".to_string()]);
        assert_eq!(config.error_patterns, vec!["custom error"]);
    }

    #[test]
    fn test_add_error_pattern_builder() {
        let config = Config::new().add_error_pattern("additional error");
        assert!(config
            .error_patterns
            .contains(&"additional error".to_string()));
        // Should still have default patterns plus the new one
        assert!(config.error_patterns.len() > 1);
    }

    #[test]
    fn test_no_error_patterns_builder() {
        let config = Config::new().no_error_patterns();
        assert!(config.error_patterns.is_empty());
    }

    #[test]
    fn test_max_retries_builder() {
        let config = Config::new().max_retries(5);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_initial_backoff_secs_builder() {
        let config = Config::new().initial_backoff_secs(10);
        assert_eq!(config.initial_backoff_secs, 10);
    }

    #[test]
    fn test_backoff_multiplier_builder() {
        let config = Config::new().backoff_multiplier(3.0);
        assert!((config.backoff_multiplier - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_circuit_breaker_threshold_builder() {
        let config = Config::new().circuit_breaker_threshold(10);
        assert_eq!(config.circuit_breaker_threshold, 10);

        // Test disabling with 0
        let config = Config::new().circuit_breaker_threshold(0);
        assert_eq!(config.circuit_breaker_threshold, 0);
    }

    #[test]
    fn test_retry_config_builder_chain() {
        let config = Config::new()
            .agent_timeout_secs(600)
            .max_retries(5)
            .initial_backoff_secs(10)
            .backoff_multiplier(1.5)
            .circuit_breaker_threshold(3)
            .no_error_patterns()
            .add_error_pattern("my error");

        assert_eq!(config.agent_timeout_secs, 600);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_backoff_secs, 10);
        assert!((config.backoff_multiplier - 1.5).abs() < f64::EPSILON);
        assert_eq!(config.circuit_breaker_threshold, 3);
        assert_eq!(config.error_patterns, vec!["my error"]);
    }
}
