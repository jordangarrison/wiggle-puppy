# Wiggle Puppy Rust Implementation - Agent Instructions

You are an autonomous coding agent building a Rust implementation of Wiggle Puppy.

## Your Task

1. Read `prd.json` in this directory to see all stories and their status
2. Find the highest priority story where `"passes": false` and all dependencies are met
3. Implement that single story completely
4. Run quality checks: `cargo check`, `cargo test`, `cargo clippy -- -D warnings`
5. If all checks pass, update `prd.json` to set `"passes": true` for that story
6. Commit your changes with message: `feat(wiggle-puppy): <story title>`

## Project Structure

```
wiggle-puppy/
├── Cargo.toml              # Workspace root
├── prd.json                # This PRD (update passes field when done)
├── progress.txt            # Append learnings here
├── wiggle-puppy-core/      # Library crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Re-exports
│       ├── error.rs        # Error types
│       ├── prd.rs          # PRD parsing
│       ├── event.rs        # Event system for TUI
│       ├── config.rs       # Configuration
│       ├── agent.rs        # Agent execution
│       └── runner.rs       # Main loop
└── wiggle-puppy-cli/       # Binary crate
    ├── Cargo.toml
    └── src/
        └── main.rs         # CLI entry point
```

## Key Dependencies

Use these versions in workspace Cargo.toml:
- `anyhow = "1"` - Error handling
- `thiserror = "2"` - Custom error types
- `serde = { version = "1", features = ["derive"] }` - Serialization
- `serde_json = "1"` - JSON parsing
- `chrono = { version = "0.4", features = ["serde"] }` - Timestamps
- `tokio = { version = "1", features = ["full"] }` - Async runtime
- `tracing = "0.1"` - Logging
- `tracing-subscriber = { version = "0.3", features = ["env-filter"] }` - Log output
- `clap = { version = "4", features = ["derive", "env"] }` - CLI parsing

## Design Principles

1. **Stateful loops**: The prompt file is re-read each iteration. PRD is re-read to detect agent updates.
2. **Event-driven**: All state changes emit events for future TUI consumption.
3. **Separation of concerns**: Core logic in wiggle-puppy-core, presentation in wiggle-puppy-cli (and future wiggle-puppy-tui).
4. **Cancellation support**: RunnerHandle allows external cancellation of the loop.

## Quality Standards

- All public items must have doc comments
- No clippy warnings allowed
- Tests for non-trivial logic (especially PRD dependency resolution)
- Proper error handling with context (no unwrap in library code)

## Progress Tracking

After completing a story, append a brief note to `progress.txt`:
```
[YYYY-MM-DD HH:MM] Story <id>: <title> - COMPLETE
  Notes: <any learnings or gotchas>
```

## Completion Signal

When ALL stories in prd.json have `"passes": true`, output exactly:
<promise>COMPLETE</promise>

Do NOT output this phrase until every story is done.
