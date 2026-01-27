# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-01-27

### Added

- Initial release of Wiggle Puppy
- Core library (`wiggle-puppy-core`) with event-driven architecture
- CLI application (`wiggle-puppy-cli`) with full command-line interface
- Support for prompt files and inline prompts
- PRD (Product Requirements Document) tracking with JSON format
- Configurable completion phrase detection
- Customizable agent command and arguments
- Stateful prompts that re-read each iteration
- Graceful cancellation support via `RunnerHandle`
- Event system for integration with TUI/CLI consumers
- Nix flake for easy installation and development
- Devbox support for alternative development environment
- Comprehensive documentation and examples

[Unreleased]: https://github.com/jordangarrison/wiggle-puppy/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/jordangarrison/wiggle-puppy/releases/tag/v0.1.0
