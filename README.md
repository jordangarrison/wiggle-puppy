# Wiggle Puppy

An autonomous AI agent loop runner written in Rust. Wiggle Puppy repeatedly runs an AI agent with a prompt until it detects a completion phrase in the output or reaches the maximum iteration limit.

## Features

- **Stateful prompts**: Prompt files are re-read each iteration, allowing dynamic prompt content
- **PRD tracking**: Optionally track progress via a Product Requirements Document (PRD) JSON file
- **Completion detection**: Automatically detect when the agent signals completion
- **Event-driven architecture**: All state changes emit events for easy integration
- **Cancellation support**: Gracefully stop the loop at any time

## Installation

### From source

```bash
# Clone the repository
git clone https://github.com/jordangarrison/wiggle-puppy.git
cd wiggle-puppy

# Install to your Cargo bin directory
cargo install --path wiggle-puppy-cli
```

### Development build

```bash
# Build both crates
cargo build --release

# The binary will be at target/release/wiggle-puppy
```

### Nix flake

```bash
# Run without installing
nix run github:jordangarrison/wiggle-puppy

# Install to your profile
nix profile install github:jordangarrison/wiggle-puppy

# Build locally
nix build github:jordangarrison/wiggle-puppy
```

#### Using in your own flake

Add wiggle-puppy as an input in your `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    wiggle-puppy = {
      url = "github:jordangarrison/wiggle-puppy";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, wiggle-puppy, ... }: {
    # Your outputs here
  };
}
```

Then use the package in your configuration:

**NixOS configuration:**

```nix
{ pkgs, wiggle-puppy, ... }:
{
  environment.systemPackages = [
    wiggle-puppy.packages.${pkgs.system}.default
  ];
}
```

**home-manager:**

```nix
{ pkgs, wiggle-puppy, ... }:
{
  home.packages = [
    wiggle-puppy.packages.${pkgs.system}.default
  ];
}
```

Make sure to pass `wiggle-puppy` to your modules via `specialArgs` or `extraSpecialArgs`:

```nix
# NixOS
nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
  system = "x86_64-linux";
  specialArgs = { inherit wiggle-puppy; };
  modules = [ ./configuration.nix ];
};

# home-manager
homeConfigurations.myuser = home-manager.lib.homeManagerConfiguration {
  pkgs = nixpkgs.legacyPackages.x86_64-linux;
  extraSpecialArgs = { inherit wiggle-puppy; };
  modules = [ ./home.nix ];
};
```

### Devbox (Jetify)

Add wiggle-puppy to your `devbox.json` using the flake reference:

```json
{
  "packages": [
    "github:jordangarrison/wiggle-puppy"
  ]
}
```

Or add it via the CLI:

```bash
devbox add github:jordangarrison/wiggle-puppy
```

Then enter the devbox shell:

```bash
devbox shell
wiggle-puppy --help
```

## Usage

### Basic usage with a prompt file

```bash
wiggle-puppy PROMPT.md
```

This runs the default agent (`claude -p`) with the prompt file, re-reading it each iteration until completion is detected or max iterations (20) is reached.

### Inline prompt

```bash
wiggle-puppy -p "Write a hello world program in Rust"
```

### With a PRD state file

```bash
wiggle-puppy PROMPT.md -s prd.json
```

When a PRD file is provided, Wiggle Puppy will:
- Display progress (completed/total stories)
- Check if all stories pass after each iteration
- Detect completion when all stories are marked complete

### Custom agent

```bash
# Use a different AI CLI
wiggle-puppy PROMPT.md -a ollama --agent-args "run llama2"

# Set via environment variable
export WIGGLE_PUPPY_AGENT=aider
wiggle-puppy PROMPT.md --agent-args ""
```

### Full options

```bash
wiggle-puppy [OPTIONS] [PROMPT_FILE]

Arguments:
  [PROMPT_FILE]  Path to the prompt file to use

Options:
  -p, --prompt <PROMPT>              Inline prompt text (conflicts with PROMPT_FILE)
  -a, --agent <AGENT>                Agent command [default: claude] [env: WIGGLE_PUPPY_AGENT]
      --agent-args <AGENT_ARGS>      Arguments to pass to the agent [default: -p]
  -m, --max-iterations <N>           Maximum iterations [default: 20]
  -s, --state <PATH>                 Path to PRD JSON file
  -c, --completion <PHRASE>          Completion phrase [default: <promise>COMPLETE</promise>]
  -d, --delay <SECONDS>              Delay between iterations [default: 2]
  -v, --verbose                      Print all agent output
      --no-auto-instruction          Don't append completion instruction to prompt
  -h, --help                         Print help
  -V, --version                      Print version
```

## Architecture

Wiggle Puppy uses a workspace structure with two crates:

```
wiggle-puppy/
├── Cargo.toml              # Workspace root with shared dependencies
├── wiggle-puppy-core/      # Library crate
│   └── src/
│       ├── lib.rs          # Public API re-exports
│       ├── error.rs        # Error types (thiserror)
│       ├── prd.rs          # PRD parsing and story management
│       ├── event.rs        # Event system for TUI/CLI
│       ├── config.rs       # Configuration and builder
│       ├── agent.rs        # Agent process execution
│       └── runner.rs       # Main loop logic
└── wiggle-puppy-cli/       # Binary crate
    └── src/
        └── main.rs         # CLI entry point (clap)
```

### Design principles

1. **Separation of concerns**: Core logic lives in `wiggle-puppy-core`, presentation in the CLI (or future TUI)
2. **Event-driven**: The `Runner` emits events through a channel, allowing any consumer to display or log them
3. **Stateful iteration**: Prompt and PRD files are re-read each iteration, enabling dynamic workflows
4. **Graceful cancellation**: `RunnerHandle` allows external cancellation of the loop

### Key types

- `Runner`: Executes the main agent loop
- `Config`: Builder for configuring the runner
- `Agent`: Spawns and streams output from the AI CLI
- `Prd`: Parses and manages PRD JSON files
- `Event`: Enum of all events emitted during execution

## PRD Format

The PRD (Product Requirements Document) JSON format tracks stories and their completion status:

```json
{
  "name": "My Project",
  "branchName": "feature/my-feature",
  "description": "A description of the project",
  "stories": [
    {
      "id": "1",
      "title": "First story",
      "description": "What this story accomplishes",
      "priority": 1,
      "passes": false,
      "acceptance_criteria": [
        "Criterion 1",
        "Criterion 2"
      ],
      "depends_on": []
    },
    {
      "id": "2",
      "title": "Second story",
      "description": "Depends on the first story",
      "priority": 2,
      "passes": false,
      "acceptance_criteria": [
        "Another criterion"
      ],
      "depends_on": ["1"]
    }
  ]
}
```

Stories are processed in priority order. A story is only available when all its dependencies have `"passes": true`.

## Future Plans

- **TUI (Terminal UI)**: A rich terminal interface with real-time progress, scrollable output, and interactive controls using `ratatui`
- **Web interface**: HTTP API and web UI for remote monitoring
- **Parallel agents**: Run multiple agent instances in parallel
- **Plugin system**: Extensible hooks for custom completion detection and post-processing

## License

MIT
