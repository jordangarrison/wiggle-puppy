//! Error types for the Wiggle Puppy agent loop.
//!
//! This module provides a unified error type for all operations in the
//! wiggle-puppy-core library, including PRD parsing, agent execution,
//! configuration, and prompt handling.

use std::path::PathBuf;
use thiserror::Error;

/// The main error type for wiggle-puppy-core operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to read the PRD file from disk.
    #[error("failed to read PRD file '{path}': {source}")]
    PrdReadError {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse the PRD JSON content.
    #[error("failed to parse PRD JSON from '{path}': {source}")]
    PrdParseError {
        /// The path containing invalid JSON.
        path: PathBuf,
        /// The underlying JSON parse error.
        #[source]
        source: serde_json::Error,
    },

    /// Failed to write the PRD file to disk.
    #[error("failed to write PRD file '{path}': {source}")]
    PrdWriteError {
        /// The path that could not be written.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to read the prompt file from disk.
    #[error("failed to read prompt file '{path}': {source}")]
    PromptReadError {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The agent process encountered an error during execution.
    #[error("agent execution failed: {message}")]
    AgentError {
        /// Description of what went wrong.
        message: String,
    },

    /// The configured agent command was not found.
    #[error("agent command not found: '{command}'")]
    AgentNotFound {
        /// The command that was not found.
        command: String,
    },

    /// No prompt was provided (neither file path nor inline text).
    #[error("no prompt provided: specify either a prompt file or inline prompt text")]
    NoPrompt,

    /// Configuration error.
    #[error("configuration error: {message}")]
    ConfigError {
        /// Description of the configuration problem.
        message: String,
    },

    /// The operation was cancelled.
    #[error("operation cancelled")]
    Cancelled,

    /// An error that doesn't fit other categories.
    #[error("{message}")]
    Other {
        /// Description of the error.
        message: String,
    },
}

impl Error {
    /// Create a new `AgentError` with the given message.
    pub fn agent_error(message: impl Into<String>) -> Self {
        Self::AgentError {
            message: message.into(),
        }
    }

    /// Create a new `AgentNotFound` error for the given command.
    pub fn agent_not_found(command: impl Into<String>) -> Self {
        Self::AgentNotFound {
            command: command.into(),
        }
    }

    /// Create a new `ConfigError` with the given message.
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }

    /// Create a new `Other` error with the given message.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
}

/// A specialized `Result` type for wiggle-puppy-core operations.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_messages() {
        let err = Error::NoPrompt;
        assert!(err.to_string().contains("no prompt provided"));

        let err = Error::Cancelled;
        assert_eq!(err.to_string(), "operation cancelled");

        let err = Error::agent_error("process exited with code 1");
        assert!(err.to_string().contains("process exited with code 1"));

        let err = Error::agent_not_found("nonexistent-agent");
        assert!(err.to_string().contains("nonexistent-agent"));

        let err = Error::config_error("invalid max_iterations");
        assert!(err.to_string().contains("invalid max_iterations"));

        let err = Error::other("something unexpected");
        assert!(err.to_string().contains("something unexpected"));
    }
}
