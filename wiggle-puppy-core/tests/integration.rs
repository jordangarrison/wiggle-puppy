//! Integration tests for the Wiggle Puppy runner.
//!
//! These tests verify the full agent loop works correctly by using
//! a mock agent script that tracks invocation count and outputs the
//! completion phrase on a specific iteration.

use std::fs;
use std::time::Duration;
use wiggle_puppy_core::{Config, Outcome, Runner};

/// Creates a mock agent script that tracks call count via a file and
/// outputs the completion phrase on the specified call number.
///
/// The script:
/// 1. Reads the current count from a counter file (or 0 if not exists)
/// 2. Increments the count and writes it back
/// 3. Echoes the input prompt
/// 4. If count equals target, outputs the completion phrase
fn create_mock_agent_script(counter_file: &str, complete_on_call: u32) -> String {
    format!(
        r#"#!/bin/bash
# Mock agent script for integration testing
# Tracks call count and outputs completion phrase on specified call

COUNTER_FILE="{counter_file}"
COMPLETE_ON={complete_on_call}
COMPLETION_PHRASE="<promise>COMPLETE</promise>"

# Read current count (default to 0)
if [ -f "$COUNTER_FILE" ]; then
    COUNT=$(cat "$COUNTER_FILE")
else
    COUNT=0
fi

# Increment count
COUNT=$((COUNT + 1))
echo "$COUNT" > "$COUNTER_FILE"

# Echo the prompt (first argument after flags)
echo "Mock agent call #$COUNT"
echo "Received prompt: $1"

# Output completion phrase on target call
if [ "$COUNT" -eq "$COMPLETE_ON" ]; then
    echo "$COMPLETION_PHRASE"
fi
"#
    )
}

#[tokio::test]
async fn test_runner_completes_on_third_iteration() {
    // Create a temporary directory for test artifacts
    let temp_dir = std::env::temp_dir().join(format!("wiggle_puppy_test_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let script_path = temp_dir.join("mock_agent.sh");
    let counter_path = temp_dir.join("call_count.txt");

    // Create mock agent script that completes on 3rd call
    let script_content = create_mock_agent_script(counter_path.to_str().unwrap(), 3);
    fs::write(&script_path, script_content).expect("failed to write script");

    // Make script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)
            .expect("failed to get metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("failed to set permissions");
    }

    // Configure runner to use our mock agent
    // Use bash to run the script, passing the script path as the first arg
    let config = Config::new()
        .agent_command("bash")
        .agent_args(vec![script_path.to_str().unwrap().to_string()])
        .prompt_text("Test prompt for integration test")
        .completion_phrase("<promise>COMPLETE</promise>")
        .max_iterations(10)
        .delay(Duration::ZERO)
        .auto_completion_instruction(false);

    let (runner, mut events, _handle) = Runner::new(config);

    // Run the loop
    let outcome = runner.run().await.expect("runner should succeed");

    // Verify we got Outcome::Completed
    assert!(
        outcome.is_completed(),
        "expected Outcome::Completed, got {:?}",
        outcome
    );

    // Verify exactly 3 iterations
    assert_eq!(
        outcome.iterations(),
        3,
        "expected 3 iterations, got {}",
        outcome.iterations()
    );

    // Verify completion reason is phrase detection
    match &outcome {
        Outcome::Completed { reason, .. } => {
            assert_eq!(
                *reason,
                wiggle_puppy_core::CompletionReason::CompletionPhraseDetected,
                "expected CompletionPhraseDetected reason"
            );
        }
        _ => panic!("expected Completed outcome"),
    }

    // Drain events and verify we got expected lifecycle events
    drop(runner);
    let mut started = false;
    let mut iterations_started = vec![];
    let mut completed = false;

    while let Some(event) = events.recv().await {
        match event {
            wiggle_puppy_core::Event::Started { .. } => started = true,
            wiggle_puppy_core::Event::IterationStarted { iteration, .. } => {
                iterations_started.push(iteration);
            }
            wiggle_puppy_core::Event::Completed { .. } => completed = true,
            _ => {}
        }
    }

    assert!(started, "should have received Started event");
    assert_eq!(
        iterations_started,
        vec![1, 2, 3],
        "should have started iterations 1, 2, and 3"
    );
    assert!(completed, "should have received Completed event");

    // Verify counter file shows 3 calls
    let final_count: u32 = fs::read_to_string(&counter_path)
        .expect("failed to read counter")
        .trim()
        .parse()
        .expect("failed to parse counter");
    assert_eq!(final_count, 3, "mock agent should have been called 3 times");

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_runner_stops_at_max_iterations() {
    // Create a temporary directory for test artifacts
    let temp_dir =
        std::env::temp_dir().join(format!("wiggle_puppy_test_max_iter_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let script_path = temp_dir.join("mock_agent_never_complete.sh");
    let counter_path = temp_dir.join("call_count.txt");

    // Create mock agent script that never outputs completion phrase
    let script_content = create_mock_agent_script(
        counter_path.to_str().unwrap(),
        999, // Never reached
    );
    fs::write(&script_path, script_content).expect("failed to write script");

    // Make script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)
            .expect("failed to get metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("failed to set permissions");
    }

    // Configure runner with low max iterations
    // Use bash to run the script, passing the script path as the first arg
    let config = Config::new()
        .agent_command("bash")
        .agent_args(vec![script_path.to_str().unwrap().to_string()])
        .prompt_text("Test prompt")
        .max_iterations(5)
        .delay(Duration::ZERO)
        .auto_completion_instruction(false);

    let (runner, _events, _handle) = Runner::new(config);

    // Run the loop
    let outcome = runner.run().await.expect("runner should succeed");

    // Verify we got Outcome::Stopped
    assert!(
        outcome.is_stopped(),
        "expected Outcome::Stopped, got {:?}",
        outcome
    );

    // Verify stopped at max iterations (5)
    assert_eq!(
        outcome.iterations(),
        5,
        "expected 5 iterations, got {}",
        outcome.iterations()
    );

    // Verify stop reason is MaxIterations
    match &outcome {
        Outcome::Stopped { reason, .. } => {
            assert_eq!(
                *reason,
                wiggle_puppy_core::StopReason::MaxIterations,
                "expected MaxIterations reason"
            );
        }
        _ => panic!("expected Stopped outcome"),
    }

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}

#[tokio::test]
async fn test_mock_agent_echoes_input() {
    // Create a temporary directory for test artifacts
    let temp_dir =
        std::env::temp_dir().join(format!("wiggle_puppy_test_echo_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

    let script_path = temp_dir.join("mock_agent_echo.sh");
    let counter_path = temp_dir.join("call_count.txt");

    // Create mock agent script that completes on 1st call
    let script_content = create_mock_agent_script(counter_path.to_str().unwrap(), 1);
    fs::write(&script_path, script_content).expect("failed to write script");

    // Make script executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)
            .expect("failed to get metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("failed to set permissions");
    }

    let test_prompt = "Hello from integration test!";

    // Configure runner
    // Use bash to run the script, passing the script path as the first arg
    let config = Config::new()
        .agent_command("bash")
        .agent_args(vec![script_path.to_str().unwrap().to_string()])
        .prompt_text(test_prompt)
        .completion_phrase("<promise>COMPLETE</promise>")
        .max_iterations(5)
        .delay(Duration::ZERO)
        .auto_completion_instruction(false);

    let (runner, mut events, _handle) = Runner::new(config);

    // Run the loop
    let outcome = runner.run().await.expect("runner should succeed");
    assert!(outcome.is_completed());

    // Collect agent output events
    drop(runner);
    let mut output_lines = vec![];
    while let Some(event) = events.recv().await {
        if let wiggle_puppy_core::Event::AgentOutput { text, .. } = event {
            output_lines.push(text);
        }
    }

    // Verify the agent echoed the prompt
    let combined_output = output_lines.join("\n");
    assert!(
        combined_output.contains(test_prompt),
        "agent output should contain the prompt: {}",
        combined_output
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).ok();
}
