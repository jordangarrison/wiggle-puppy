//! Event system for the Wiggle Puppy agent loop.
//!
//! This module provides an event-driven architecture for communicating
//! state changes from the runner to consumers (CLI, TUI). All lifecycle
//! events, agent output, and status updates are communicated through
//! this channel-based system.

use tokio::sync::mpsc;

/// Default channel buffer size.
const DEFAULT_CHANNEL_SIZE: usize = 100;

/// Events emitted by the runner during execution.
#[derive(Debug, Clone)]
pub enum Event {
    /// The runner has started.
    Started {
        /// Maximum number of iterations configured.
        max_iterations: u32,
    },

    /// A new iteration is starting.
    IterationStarted {
        /// The current iteration number (1-indexed).
        iteration: u32,
        /// Maximum iterations.
        max_iterations: u32,
    },

    /// Output from the agent (stdout or stderr).
    AgentOutput {
        /// The output text.
        text: String,
        /// Whether this is from stderr.
        is_stderr: bool,
    },

    /// The agent has finished running.
    AgentFinished {
        /// Exit code from the agent process.
        exit_code: Option<i32>,
        /// Duration in seconds.
        duration_secs: f64,
    },

    /// A story has been marked as complete.
    StoryCompleted {
        /// The story ID.
        story_id: String,
        /// The story title.
        story_title: String,
    },

    /// An iteration has finished.
    IterationFinished {
        /// The iteration number that finished.
        iteration: u32,
        /// Whether completion was detected this iteration.
        completion_detected: bool,
    },

    /// The PRD has been updated (e.g., a story marked complete).
    PrdUpdated {
        /// Number of completed stories.
        completed: usize,
        /// Total number of stories.
        total: usize,
    },

    /// General progress message.
    Progress {
        /// The progress message.
        message: String,
    },

    /// Warning message.
    Warning {
        /// The warning message.
        message: String,
    },

    /// Error message (non-fatal).
    Error {
        /// The error message.
        message: String,
    },

    /// The runner has completed successfully.
    Completed {
        /// Total iterations run.
        iterations: u32,
        /// Reason for completion.
        reason: CompletionReason,
    },

    /// The runner has stopped (not due to successful completion).
    Stopped {
        /// Total iterations run.
        iterations: u32,
        /// Reason for stopping.
        reason: StopReason,
    },
}

/// Reasons for successful completion of the runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionReason {
    /// All stories in the PRD are complete.
    AllStoriesComplete,
    /// The completion phrase was detected in agent output.
    CompletionPhraseDetected,
    /// Both conditions were met simultaneously.
    Both,
}

/// Reasons for the runner stopping without successful completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// Maximum iterations reached.
    MaxIterations,
    /// Externally cancelled.
    Cancelled,
    /// A fatal error occurred.
    FatalError {
        /// The error message.
        message: String,
    },
}

/// Sender for events.
pub type EventSender = mpsc::Sender<Event>;

/// Receiver for events.
pub type EventReceiver = mpsc::Receiver<Event>;

/// Create a new event channel with the default buffer size.
///
/// Returns a sender and receiver pair for event communication.
pub fn channel() -> (EventSender, EventReceiver) {
    mpsc::channel(DEFAULT_CHANNEL_SIZE)
}

/// Create a new event channel with a custom buffer size.
///
/// Returns a sender and receiver pair for event communication.
pub fn channel_with_size(size: usize) -> (EventSender, EventReceiver) {
    mpsc::channel(size)
}

impl Event {
    /// Create a progress event with the given message.
    pub fn progress(message: impl Into<String>) -> Self {
        Self::Progress {
            message: message.into(),
        }
    }

    /// Create a warning event with the given message.
    pub fn warning(message: impl Into<String>) -> Self {
        Self::Warning {
            message: message.into(),
        }
    }

    /// Create an error event with the given message.
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    /// Create an agent output event for stdout.
    pub fn agent_output(text: impl Into<String>) -> Self {
        Self::AgentOutput {
            text: text.into(),
            is_stderr: false,
        }
    }

    /// Create an agent output event for stderr.
    pub fn agent_stderr(text: impl Into<String>) -> Self {
        Self::AgentOutput {
            text: text.into(),
            is_stderr: true,
        }
    }
}

impl std::fmt::Display for CompletionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompletionReason::AllStoriesComplete => write!(f, "all stories complete"),
            CompletionReason::CompletionPhraseDetected => write!(f, "completion phrase detected"),
            CompletionReason::Both => write!(f, "all stories complete and completion phrase detected"),
        }
    }
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopReason::MaxIterations => write!(f, "maximum iterations reached"),
            StopReason::Cancelled => write!(f, "cancelled"),
            StopReason::FatalError { message } => write!(f, "fatal error: {}", message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let (tx, _rx) = channel();
        // Should be able to send without blocking
        tx.try_send(Event::progress("test")).unwrap();
    }

    #[test]
    fn test_event_constructors() {
        let progress = Event::progress("doing work");
        matches!(progress, Event::Progress { message } if message == "doing work");

        let warning = Event::warning("be careful");
        matches!(warning, Event::Warning { message } if message == "be careful");

        let error = Event::error("something failed");
        matches!(error, Event::Error { message } if message == "something failed");

        let output = Event::agent_output("hello world");
        matches!(output, Event::AgentOutput { text, is_stderr } if text == "hello world" && !is_stderr);

        let stderr = Event::agent_stderr("error output");
        matches!(stderr, Event::AgentOutput { text, is_stderr } if text == "error output" && is_stderr);
    }

    #[test]
    fn test_completion_reason_display() {
        assert_eq!(
            CompletionReason::AllStoriesComplete.to_string(),
            "all stories complete"
        );
        assert_eq!(
            CompletionReason::CompletionPhraseDetected.to_string(),
            "completion phrase detected"
        );
        assert_eq!(
            CompletionReason::Both.to_string(),
            "all stories complete and completion phrase detected"
        );
    }

    #[test]
    fn test_stop_reason_display() {
        assert_eq!(
            StopReason::MaxIterations.to_string(),
            "maximum iterations reached"
        );
        assert_eq!(StopReason::Cancelled.to_string(), "cancelled");
        assert_eq!(
            StopReason::FatalError {
                message: "disk full".to_string()
            }
            .to_string(),
            "fatal error: disk full"
        );
    }
}
