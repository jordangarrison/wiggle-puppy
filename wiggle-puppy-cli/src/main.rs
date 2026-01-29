//! Wiggle Puppy CLI - An autonomous AI agent loop runner.

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;
use wiggle_puppy_core::{
    CompletionReason, Event, EventReceiver, Outcome, Prd, Runner, StopReason,
};

/// Wiggle Puppy - Run autonomous AI agent loops with completion detection.
///
/// Wiggle Puppy repeatedly runs an AI agent with a prompt until it detects
/// a completion phrase in the output or reaches the maximum iteration limit.
/// It can optionally track progress via a PRD (Product Requirements Document)
/// JSON file.
#[derive(Parser, Debug)]
#[command(name = "wiggle-puppy")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the prompt file to use.
    ///
    /// The prompt will be re-read each iteration, allowing for stateful prompts.
    /// Conflicts with --prompt.
    #[arg(value_name = "PROMPT_FILE")]
    pub prompt_file: Option<PathBuf>,

    /// Inline prompt text to use instead of a file.
    ///
    /// Conflicts with the prompt file positional argument.
    #[arg(short = 'p', long = "prompt", conflicts_with = "prompt_file")]
    pub prompt: Option<String>,

    /// Agent command to run.
    ///
    /// The agent will receive the prompt content via stdin or as an argument
    /// depending on the agent-args configuration.
    #[arg(
        short = 'a',
        long = "agent",
        default_value = "claude",
        env = "WIGGLE_PUPPY_AGENT"
    )]
    pub agent: String,

    /// Arguments to pass to the agent command.
    ///
    /// Space-separated arguments that will be passed to the agent.
    /// The prompt will typically be passed after these arguments.
    #[arg(long = "agent-args", default_value = "-p")]
    pub agent_args: String,

    /// Maximum number of iterations before stopping.
    ///
    /// The loop will stop after this many iterations even if completion
    /// is not detected.
    #[arg(short = 'm', long = "max-iterations", default_value = "20")]
    pub max_iterations: u32,

    /// Path to the PRD (Product Requirements Document) JSON file.
    ///
    /// If provided, the runner will check if all stories pass after each
    /// iteration and can detect completion via PRD state.
    #[arg(short = 's', long = "state")]
    pub state: Option<PathBuf>,

    /// Completion phrase to detect in agent output.
    ///
    /// When this phrase is detected in the agent's output, the loop completes.
    #[arg(short = 'c', long = "completion", default_value = "<promise>COMPLETE</promise>")]
    pub completion: String,

    /// Delay in seconds between iterations.
    ///
    /// A short delay between iterations can help prevent rate limiting
    /// and allows the system to stabilize between runs.
    #[arg(short = 'd', long = "delay", default_value = "2")]
    pub delay: u64,

    /// Enable verbose output.
    ///
    /// When enabled, all agent output is printed as it streams.
    /// When disabled, only a summary is shown.
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Disable automatic completion instruction.
    ///
    /// By default, an instruction telling the agent to output the completion
    /// phrase is appended to the prompt. Use this flag to disable that behavior.
    #[arg(long = "no-auto-instruction")]
    pub no_auto_instruction: bool,
}

impl Cli {
    /// Convert CLI arguments to a Config.
    pub fn to_config(&self) -> wiggle_puppy_core::Config {
        let mut config = wiggle_puppy_core::Config::new()
            .agent_command(&self.agent)
            .agent_args_str(&self.agent_args)
            .max_iterations(self.max_iterations)
            .delay_secs(self.delay)
            .completion_phrase(&self.completion)
            .auto_completion_instruction(!self.no_auto_instruction);

        if let Some(ref path) = self.prompt_file {
            config = config.prompt_path(path);
        }

        if let Some(ref text) = self.prompt {
            config = config.prompt_text(text);
        }

        if let Some(ref path) = self.state {
            config = config.prd_path(path);
        }

        config
    }
}

/// Print the startup header with configuration info.
fn print_header(cli: &Cli) {
    println!("Wiggle Puppy - Autonomous Agent Runner");
    println!("======================================");
    println!("Agent: {} {}", cli.agent, cli.agent_args);
    println!("Max iterations: {}", cli.max_iterations);

    if let Some(ref state_path) = cli.state {
        println!("State file: {}", state_path.display());
    }

    println!();
}

/// Print PRD progress summary if a state file is configured.
fn print_prd_summary(cli: &Cli) {
    if let Some(ref state_path) = cli.state {
        match Prd::load(state_path) {
            Ok(prd) => {
                let completed = prd.stories.iter().filter(|s| s.passes).count();
                let total = prd.stories.len();
                println!("PRD: {} ({}/{})", prd.name, completed, total);

                if let Some(next) = prd.next_story() {
                    println!("Next story: {} - {}", next.id, next.title);
                }

                println!();
            }
            Err(e) => {
                eprintln!("Warning: Could not load PRD: {}", e);
                println!();
            }
        }
    }
}

/// Event handler that manages output display.
struct EventHandler {
    verbose: bool,
    line_count: usize,
    last_lines: Vec<String>,
}

impl EventHandler {
    fn new(verbose: bool) -> Self {
        Self {
            verbose,
            line_count: 0,
            last_lines: Vec::new(),
        }
    }

    /// Reset output tracking for a new iteration.
    fn reset(&mut self) {
        self.line_count = 0;
        self.last_lines.clear();
    }

    /// Handle an event and print appropriate output.
    fn handle(&mut self, event: Event) {
        match event {
            Event::Started { max_iterations } => {
                println!("Starting agent loop (max {} iterations)", max_iterations);
                println!();
            }

            Event::IterationStarted {
                iteration,
                max_iterations,
            } => {
                self.reset();
                println!("--- Iteration {}/{} ---", iteration, max_iterations);
            }

            Event::AgentOutput { text, is_stderr } => {
                // Track line count and last lines for summary
                for line in text.lines() {
                    self.line_count += 1;
                    self.last_lines.push(line.to_string());
                    // Keep only the last 3 lines
                    if self.last_lines.len() > 3 {
                        self.last_lines.remove(0);
                    }
                }

                if self.verbose {
                    if is_stderr {
                        eprintln!("{}", text);
                    } else {
                        println!("{}", text);
                    }
                }
            }

            Event::AgentFinished {
                exit_code,
                duration_secs,
            } => {
                if !self.verbose {
                    // Print summary in non-verbose mode
                    println!(
                        "  Output: {} lines ({:.1}s)",
                        self.line_count, duration_secs
                    );
                    if !self.last_lines.is_empty() {
                        println!("  Last output:");
                        for line in &self.last_lines {
                            // Truncate long lines for display
                            let display_line = if line.len() > 80 {
                                format!("{}...", &line[..77])
                            } else {
                                line.clone()
                            };
                            println!("    {}", display_line);
                        }
                    }
                } else {
                    println!();
                }

                if let Some(code) = exit_code {
                    if code != 0 {
                        println!("  Exit code: {}", code);
                    }
                }
            }

            Event::IterationFinished {
                iteration: _,
                completion_detected,
            } => {
                if completion_detected {
                    println!("  Completion detected!");
                }
                println!();
            }

            Event::PrdUpdated { completed, total } => {
                println!("  PRD progress: {}/{} stories complete", completed, total);
            }

            Event::StoryCompleted {
                story_id,
                story_title,
            } => {
                println!("  Story completed: {} - {}", story_id, story_title);
            }

            Event::Progress { message } => {
                println!("  {}", message);
            }

            Event::Warning { message } => {
                eprintln!("  Warning: {}", message);
            }

            Event::Error { message } => {
                eprintln!("  Error: {}", message);
            }

            Event::Completed { iterations, reason } => {
                println!("======================================");
                println!(
                    "Completed after {} iteration{}!",
                    iterations,
                    if iterations == 1 { "" } else { "s" }
                );
                println!("Reason: {}", format_completion_reason(&reason));
            }

            Event::Stopped { iterations, reason } => {
                println!("======================================");
                println!(
                    "Stopped after {} iteration{}",
                    iterations,
                    if iterations == 1 { "" } else { "s" }
                );
                println!("Reason: {}", format_stop_reason(&reason));
            }
        }
    }
}

/// Format a completion reason for display.
fn format_completion_reason(reason: &CompletionReason) -> &'static str {
    match reason {
        CompletionReason::AllStoriesComplete => "All stories in PRD are complete",
        CompletionReason::CompletionPhraseDetected => "Completion phrase detected in agent output",
        CompletionReason::Both => "All stories complete and completion phrase detected",
    }
}

/// Format a stop reason for display.
fn format_stop_reason(reason: &StopReason) -> String {
    match reason {
        StopReason::MaxIterations => "Maximum iterations reached".to_string(),
        StopReason::Cancelled => "Cancelled by user".to_string(),
        StopReason::FatalError { message } => format!("Fatal error: {}", message),
    }
}

/// Consume events from the receiver and handle them.
async fn handle_events(mut receiver: EventReceiver, verbose: bool) {
    let mut handler = EventHandler::new(verbose);

    while let Some(event) = receiver.recv().await {
        handler.handle(event);
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let verbose = cli.verbose;

    // Print header and PRD summary
    print_header(&cli);
    print_prd_summary(&cli);

    // Create runner
    let config = cli.to_config();
    let (runner, receiver, _handle) = Runner::new(config);

    // Spawn event handler task
    let event_task = tokio::spawn(handle_events(receiver, verbose));

    // Run the main loop
    let outcome = runner.run().await;

    // Wait for event handler to finish processing
    let _ = event_task.await;

    // Return appropriate exit code
    match outcome {
        Ok(Outcome::Completed { .. }) => ExitCode::SUCCESS,
        Ok(Outcome::Stopped { .. }) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("Fatal error: {}", e);
            ExitCode::FAILURE
        }
    }
}
