//! Wiggle Puppy CLI - An autonomous AI agent loop runner.

use clap::Parser;
use std::path::PathBuf;

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

#[tokio::main]
async fn main() {
    let _cli = Cli::parse();

    // Event handling and output will be implemented in story 10
    println!("Wiggle Puppy CLI initialized");
}
