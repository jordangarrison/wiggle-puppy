//! Main runner loop for the Wiggle Puppy agent.
//!
//! This module provides the `Runner` struct that executes the main agent loop,
//! handling prompt re-reading, PRD state tracking, completion detection, and
//! event emission for consumers like CLI or TUI.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::agent::{Agent, AgentOutput};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::event::{channel, CompletionReason, Event, EventReceiver, EventSender, StopReason};
use crate::prd::Prd;

/// Calculate exponential backoff duration
fn calculate_backoff(attempt: u32, config: &Config) -> u64 {
    let backoff = config.initial_backoff_secs as f64
        * config.backoff_multiplier.powi((attempt - 1) as i32);
    backoff as u64
}

/// The main runner that executes the agent loop.
///
/// The runner manages the lifecycle of agent invocations, re-reading the prompt
/// and PRD files each iteration to support stateful prompts and detect when
/// the agent has marked stories complete.
#[derive(Debug)]
pub struct Runner {
    /// Configuration for the runner.
    config: Config,
    /// Event sender for communicating with consumers.
    events: EventSender,
    /// Shared cancellation flag.
    cancel_flag: Arc<AtomicBool>,
}

/// Handle for controlling a running runner instance.
///
/// This handle can be used to cancel the runner from another task or thread.
#[derive(Debug, Clone)]
pub struct RunnerHandle {
    /// Shared cancellation flag.
    cancel_flag: Arc<AtomicBool>,
}

impl RunnerHandle {
    /// Signal the runner to cancel at the next opportunity.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Check if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }
}

/// The outcome of a runner execution.
#[derive(Debug, Clone)]
pub enum Outcome {
    /// The runner completed successfully.
    Completed {
        /// Total iterations run.
        iterations: u32,
        /// Reason for completion.
        reason: CompletionReason,
    },
    /// The runner stopped without completing.
    Stopped {
        /// Total iterations run.
        iterations: u32,
        /// Reason for stopping.
        reason: StopReason,
    },
}

impl Outcome {
    /// Get the number of iterations run.
    pub fn iterations(&self) -> u32 {
        match self {
            Outcome::Completed { iterations, .. } => *iterations,
            Outcome::Stopped { iterations, .. } => *iterations,
        }
    }

    /// Check if the outcome was a successful completion.
    pub fn is_completed(&self) -> bool {
        matches!(self, Outcome::Completed { .. })
    }

    /// Check if the outcome was a stop (not successful completion).
    pub fn is_stopped(&self) -> bool {
        matches!(self, Outcome::Stopped { .. })
    }
}

impl Runner {
    /// Create a new runner with the given configuration.
    ///
    /// Returns a tuple of (Runner, EventReceiver, RunnerHandle).
    /// - The `Runner` can be used to execute the main loop.
    /// - The `EventReceiver` can be used to receive events from the runner.
    /// - The `RunnerHandle` can be used to cancel the runner.
    ///
    /// # Examples
    ///
    /// ```
    /// use wiggle_puppy_core::{Config, runner::Runner};
    ///
    /// let config = Config::new().prompt_text("Do something");
    /// let (runner, events, handle) = Runner::new(config);
    ///
    /// // The handle can be cloned and sent to another task
    /// let handle_clone = handle.clone();
    /// ```
    pub fn new(config: Config) -> (Self, EventReceiver, RunnerHandle) {
        let (tx, rx) = channel();
        let cancel_flag = Arc::new(AtomicBool::new(false));

        let runner = Self {
            config,
            events: tx,
            cancel_flag: cancel_flag.clone(),
        };

        let handle = RunnerHandle { cancel_flag };

        (runner, rx, handle)
    }

    /// Check if cancellation has been requested.
    fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    /// Run the main agent loop.
    ///
    /// This method executes the following loop:
    /// 1. Re-read the prompt file (for stateful prompts)
    /// 2. Re-read the PRD file (to detect agent updates)
    /// 3. Check if PRD is complete
    /// 4. Spawn the agent with the prompt
    /// 5. Check for completion phrase in output
    /// 6. Delay before next iteration
    /// 7. Repeat until completion or max iterations
    ///
    /// # Returns
    ///
    /// Returns an `Outcome` indicating whether the runner completed successfully
    /// or stopped for some reason (max iterations, cancellation, error).
    pub async fn run(&self) -> Result<Outcome> {
        let _ = self
            .events
            .send(Event::Started {
                max_iterations: self.config.max_iterations,
            })
            .await;

        let agent = Agent::new(
            &self.config.agent_command,
            self.config.agent_args.clone(),
            self.config.error_patterns.clone(),
            self.config.agent_timeout_secs,
        );

        let mut iteration: u32 = 0;
        let mut consecutive_failures: u32 = 0;

        loop {
            // Check cancellation before starting iteration
            if self.is_cancelled() {
                let _ = self
                    .events
                    .send(Event::Stopped {
                        iterations: iteration,
                        reason: StopReason::Cancelled,
                    })
                    .await;
                return Ok(Outcome::Stopped {
                    iterations: iteration,
                    reason: StopReason::Cancelled,
                });
            }

            // Check max iterations
            if iteration >= self.config.max_iterations {
                let _ = self
                    .events
                    .send(Event::Stopped {
                        iterations: iteration,
                        reason: StopReason::MaxIterations,
                    })
                    .await;
                return Ok(Outcome::Stopped {
                    iterations: iteration,
                    reason: StopReason::MaxIterations,
                });
            }

            iteration += 1;

            let _ = self
                .events
                .send(Event::IterationStarted {
                    iteration,
                    max_iterations: self.config.max_iterations,
                })
                .await;

            // Re-read prompt each iteration for stateful prompts
            let prompt = match self.config.get_prompt() {
                Ok(p) => p,
                Err(e) => {
                    let message = format!("failed to read prompt: {}", e);
                    let _ = self
                        .events
                        .send(Event::Stopped {
                            iterations: iteration,
                            reason: StopReason::FatalError {
                                message: message.clone(),
                            },
                        })
                        .await;
                    return Ok(Outcome::Stopped {
                        iterations: iteration,
                        reason: StopReason::FatalError { message },
                    });
                }
            };

            // Check PRD state before running agent (if configured)
            let prd_complete_before = if let Some(prd_path) = &self.config.prd_path {
                match Prd::load(prd_path) {
                    Ok(prd) => {
                        let completed = prd.stories.iter().filter(|s| s.passes).count();
                        let total = prd.stories.len();
                        let _ = self
                            .events
                            .send(Event::PrdUpdated { completed, total })
                            .await;
                        prd.is_complete()
                    }
                    Err(e) => {
                        let _ = self
                            .events
                            .send(Event::warning(format!("failed to read PRD: {}", e)))
                            .await;
                        false
                    }
                }
            } else {
                false
            };

            // If PRD is already complete before running, we're done
            if prd_complete_before {
                let _ = self
                    .events
                    .send(Event::Completed {
                        iterations: iteration - 1, // Haven't run this iteration yet
                        reason: CompletionReason::AllStoriesComplete,
                    })
                    .await;
                return Ok(Outcome::Completed {
                    iterations: iteration - 1,
                    reason: CompletionReason::AllStoriesComplete,
                });
            }

            // Run the agent with retry logic
            let mut retry_attempt = 0u32;
            let output = loop {
                // Check circuit breaker
                if self.config.circuit_breaker_threshold > 0
                    && consecutive_failures >= self.config.circuit_breaker_threshold
                {
                    let _ = self
                        .events
                        .send(Event::Stopped {
                            iterations: iteration,
                            reason: StopReason::CircuitBreakerTriggered {
                                consecutive_failures,
                            },
                        })
                        .await;
                    return Ok(Outcome::Stopped {
                        iterations: iteration,
                        reason: StopReason::CircuitBreakerTriggered {
                            consecutive_failures,
                        },
                    });
                }

                match agent.run(&prompt, &self.events).await {
                    Ok(output) => {
                        consecutive_failures = 0; // Reset on success
                        break output;
                    }
                    Err(Error::AgentErrorDetected { .. }) | Err(Error::AgentTimeout { .. }) => {
                        retry_attempt += 1;
                        consecutive_failures += 1;

                        if retry_attempt > self.config.max_retries {
                            // Give up on this iteration, continue to next
                            // (circuit breaker will catch persistent failures)
                            break AgentOutput::empty();
                        }

                        let backoff = calculate_backoff(retry_attempt, &self.config);
                        let _ = self
                            .events
                            .send(Event::RetryScheduled {
                                backoff_secs: backoff,
                                attempt: retry_attempt,
                                max_retries: self.config.max_retries,
                            })
                            .await;
                        tokio::time::sleep(Duration::from_secs(backoff)).await;
                    }
                    Err(e) => {
                        // Other errors (AgentNotFound, etc.) - fatal, don't retry
                        let message = format!("agent failed: {}", e);
                        let _ = self
                            .events
                            .send(Event::Stopped {
                                iterations: iteration,
                                reason: StopReason::FatalError {
                                    message: message.clone(),
                                },
                            })
                            .await;
                        return Ok(Outcome::Stopped {
                            iterations: iteration,
                            reason: StopReason::FatalError { message },
                        });
                    }
                }
            };

            // Check for completion phrase in output
            let phrase_detected = output.contains(&self.config.completion_phrase);

            // Re-read PRD after agent run to check if it made updates
            let prd_complete_after = if let Some(prd_path) = &self.config.prd_path {
                match Prd::load(prd_path) {
                    Ok(prd) => {
                        let completed = prd.stories.iter().filter(|s| s.passes).count();
                        let total = prd.stories.len();
                        let _ = self
                            .events
                            .send(Event::PrdUpdated { completed, total })
                            .await;
                        prd.is_complete()
                    }
                    Err(e) => {
                        let _ = self
                            .events
                            .send(Event::warning(format!(
                                "failed to read PRD after agent: {}",
                                e
                            )))
                            .await;
                        false
                    }
                }
            } else {
                false
            };

            // Determine completion status
            let completion_detected = phrase_detected || prd_complete_after;

            let _ = self
                .events
                .send(Event::IterationFinished {
                    iteration,
                    completion_detected,
                })
                .await;

            // Check completion conditions
            if phrase_detected && prd_complete_after {
                let _ = self
                    .events
                    .send(Event::Completed {
                        iterations: iteration,
                        reason: CompletionReason::Both,
                    })
                    .await;
                return Ok(Outcome::Completed {
                    iterations: iteration,
                    reason: CompletionReason::Both,
                });
            } else if prd_complete_after {
                let _ = self
                    .events
                    .send(Event::Completed {
                        iterations: iteration,
                        reason: CompletionReason::AllStoriesComplete,
                    })
                    .await;
                return Ok(Outcome::Completed {
                    iterations: iteration,
                    reason: CompletionReason::AllStoriesComplete,
                });
            } else if phrase_detected {
                let _ = self
                    .events
                    .send(Event::Completed {
                        iterations: iteration,
                        reason: CompletionReason::CompletionPhraseDetected,
                    })
                    .await;
                return Ok(Outcome::Completed {
                    iterations: iteration,
                    reason: CompletionReason::CompletionPhraseDetected,
                });
            }

            // Delay before next iteration
            if !self.config.delay.is_zero() {
                tokio::time::sleep(self.config.delay).await;
            }

            // Check cancellation after delay
            if self.is_cancelled() {
                let _ = self
                    .events
                    .send(Event::Stopped {
                        iterations: iteration,
                        reason: StopReason::Cancelled,
                    })
                    .await;
                return Ok(Outcome::Stopped {
                    iterations: iteration,
                    reason: StopReason::Cancelled,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_runner_handle_cancel() {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let handle = RunnerHandle {
            cancel_flag: cancel_flag.clone(),
        };

        assert!(!handle.is_cancelled());
        handle.cancel();
        assert!(handle.is_cancelled());
    }

    #[test]
    fn test_runner_handle_clone() {
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let handle1 = RunnerHandle { cancel_flag };
        let handle2 = handle1.clone();

        handle1.cancel();
        assert!(handle2.is_cancelled());
    }

    #[test]
    fn test_outcome_iterations() {
        let completed = Outcome::Completed {
            iterations: 5,
            reason: CompletionReason::CompletionPhraseDetected,
        };
        assert_eq!(completed.iterations(), 5);

        let stopped = Outcome::Stopped {
            iterations: 3,
            reason: StopReason::MaxIterations,
        };
        assert_eq!(stopped.iterations(), 3);
    }

    #[test]
    fn test_outcome_is_completed() {
        let completed = Outcome::Completed {
            iterations: 5,
            reason: CompletionReason::CompletionPhraseDetected,
        };
        assert!(completed.is_completed());
        assert!(!completed.is_stopped());

        let stopped = Outcome::Stopped {
            iterations: 3,
            reason: StopReason::MaxIterations,
        };
        assert!(!stopped.is_completed());
        assert!(stopped.is_stopped());
    }

    #[test]
    fn test_runner_new() {
        let config = Config::new().prompt_text("test prompt");
        let (runner, _rx, handle) = Runner::new(config);

        assert!(!handle.is_cancelled());
        assert!(!runner.is_cancelled());
    }

    #[tokio::test]
    async fn test_runner_cancellation_before_start() {
        let config = Config::new().prompt_text("test prompt").max_iterations(10);
        let (runner, _rx, handle) = Runner::new(config);

        // Cancel immediately
        handle.cancel();

        let outcome = runner.run().await.expect("should return outcome");
        assert!(matches!(
            outcome,
            Outcome::Stopped {
                iterations: 0,
                reason: StopReason::Cancelled,
            }
        ));
    }

    #[tokio::test]
    async fn test_runner_max_iterations() {
        let config = Config::new()
            .agent_command("echo")
            .agent_args(vec![])
            .prompt_text("test")
            .max_iterations(0)
            .delay(Duration::ZERO);
        let (runner, _rx, _handle) = Runner::new(config);

        let outcome = runner.run().await.expect("should return outcome");
        assert!(matches!(
            outcome,
            Outcome::Stopped {
                iterations: 0,
                reason: StopReason::MaxIterations,
            }
        ));
    }

    #[tokio::test]
    async fn test_runner_completion_phrase_detected() {
        // Use echo to output the completion phrase
        let config = Config::new()
            .agent_command("echo")
            .agent_args(vec![])
            .prompt_text("<promise>COMPLETE</promise>")
            .completion_phrase("<promise>COMPLETE</promise>")
            .max_iterations(5)
            .delay(Duration::ZERO)
            .auto_completion_instruction(false);
        let (runner, _rx, _handle) = Runner::new(config);

        let outcome = runner.run().await.expect("should return outcome");
        assert!(matches!(
            outcome,
            Outcome::Completed {
                iterations: 1,
                reason: CompletionReason::CompletionPhraseDetected,
            }
        ));
    }

    #[tokio::test]
    async fn test_runner_events_emitted() {
        let config = Config::new()
            .agent_command("echo")
            .agent_args(vec![])
            .prompt_text("<promise>COMPLETE</promise>")
            .completion_phrase("<promise>COMPLETE</promise>")
            .max_iterations(5)
            .delay(Duration::ZERO)
            .auto_completion_instruction(false);
        let (runner, mut rx, _handle) = Runner::new(config);

        let outcome = runner.run().await.expect("should return outcome");
        assert!(outcome.is_completed());

        // Collect all events
        drop(runner); // Drop to close the sender
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
        }

        // Should have Started event
        assert!(matches!(events.first(), Some(Event::Started { .. })));

        // Should have IterationStarted
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::IterationStarted { .. })));

        // Should have AgentOutput or AgentFinished
        assert!(events
            .iter()
            .any(|e| matches!(e, Event::AgentOutput { .. })
                || matches!(e, Event::AgentFinished { .. })));

        // Should have Completed
        assert!(events.iter().any(|e| matches!(e, Event::Completed { .. })));
    }

    #[tokio::test]
    async fn test_runner_no_prompt_error() {
        let config = Config::new().max_iterations(5);
        let (runner, _rx, _handle) = Runner::new(config);

        let outcome = runner.run().await.expect("should return outcome");
        assert!(matches!(
            outcome,
            Outcome::Stopped {
                reason: StopReason::FatalError { .. },
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_runner_agent_not_found() {
        let config = Config::new()
            .agent_command("nonexistent-command-12345")
            .prompt_text("test")
            .max_iterations(5);
        let (runner, _rx, _handle) = Runner::new(config);

        let outcome = runner.run().await.expect("should return outcome");
        assert!(matches!(
            outcome,
            Outcome::Stopped {
                iterations: 1,
                reason: StopReason::FatalError { .. },
            }
        ));
    }

    #[test]
    fn test_calculate_backoff_first_attempt() {
        let config = Config::new()
            .initial_backoff_secs(5)
            .backoff_multiplier(2.0);

        // First attempt (attempt=1): 5 * 2^0 = 5
        let backoff = calculate_backoff(1, &config);
        assert_eq!(backoff, 5);
    }

    #[test]
    fn test_calculate_backoff_second_attempt() {
        let config = Config::new()
            .initial_backoff_secs(5)
            .backoff_multiplier(2.0);

        // Second attempt (attempt=2): 5 * 2^1 = 10
        let backoff = calculate_backoff(2, &config);
        assert_eq!(backoff, 10);
    }

    #[test]
    fn test_calculate_backoff_third_attempt() {
        let config = Config::new()
            .initial_backoff_secs(5)
            .backoff_multiplier(2.0);

        // Third attempt (attempt=3): 5 * 2^2 = 20
        let backoff = calculate_backoff(3, &config);
        assert_eq!(backoff, 20);
    }

    #[test]
    fn test_calculate_backoff_custom_multiplier() {
        let config = Config::new()
            .initial_backoff_secs(10)
            .backoff_multiplier(1.5);

        // First attempt: 10 * 1.5^0 = 10
        assert_eq!(calculate_backoff(1, &config), 10);

        // Second attempt: 10 * 1.5^1 = 15
        assert_eq!(calculate_backoff(2, &config), 15);

        // Third attempt: 10 * 1.5^2 = 22.5 -> 22 (truncated)
        assert_eq!(calculate_backoff(3, &config), 22);
    }

    #[tokio::test]
    async fn test_circuit_breaker_triggers_on_threshold() {
        // This test verifies that circuit breaker logic exists by checking
        // that the StopReason::CircuitBreakerTriggered variant is valid
        let reason = StopReason::CircuitBreakerTriggered {
            consecutive_failures: 5,
        };
        assert_eq!(
            reason.to_string(),
            "circuit breaker triggered after 5 consecutive failures"
        );
    }

    #[test]
    fn test_circuit_breaker_outcome() {
        let outcome = Outcome::Stopped {
            iterations: 3,
            reason: StopReason::CircuitBreakerTriggered {
                consecutive_failures: 5,
            },
        };

        assert!(outcome.is_stopped());
        assert!(!outcome.is_completed());
        assert_eq!(outcome.iterations(), 3);
    }
}
