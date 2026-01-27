# Contributing to Wiggle Puppy

Thank you for your interest in contributing to Wiggle Puppy! This document provides guidelines and information for contributors.

## Getting Started

### Development Environment

The easiest way to set up a development environment is using the Nix flake:

```bash
# Enter development shell with all dependencies
nix develop

# Or use direnv for automatic shell activation
direnv allow
```

Alternatively, you can use Devbox:

```bash
devbox shell
```

Or install Rust manually via [rustup](https://rustup.rs/).

### Building

```bash
# Build all crates
cargo build

# Build in release mode
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

## Code Style

### Formatting

All code must be formatted with `cargo fmt`:

```bash
cargo fmt
```

### Linting

Code must pass `clippy` with no warnings:

```bash
cargo clippy -- -D warnings
```

### Pre-commit Checklist

Before submitting a PR, ensure:

1. `cargo fmt` - Code is formatted
2. `cargo clippy -- -D warnings` - No clippy warnings
3. `cargo test` - All tests pass
4. `cargo check` - Code compiles

Or run all checks at once:

```bash
cargo check && cargo test && cargo clippy -- -D warnings
```

## Submitting Changes

### Reporting Issues

- Search existing issues before creating a new one
- Include relevant details: OS, Rust version, steps to reproduce
- For bugs, include the error message and minimal reproduction case

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run the pre-commit checklist
5. Commit with a descriptive message
6. Push to your fork
7. Open a Pull Request

### Commit Messages

Follow conventional commit format:

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `refactor:` Code refactoring
- `test:` Test additions or changes
- `chore:` Maintenance tasks

Example: `feat(core): add support for custom completion phrases`

## Project Structure

```
wiggle-puppy/
├── wiggle-puppy-core/    # Library crate (core logic)
└── wiggle-puppy-cli/     # Binary crate (CLI interface)
```

- **Core changes**: Modify `wiggle-puppy-core/`
- **CLI changes**: Modify `wiggle-puppy-cli/`
- **New features**: Usually require changes to both crates

## Questions?

Feel free to open an issue for questions or discussion about potential contributions.
