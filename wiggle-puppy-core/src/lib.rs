//! Wiggle Puppy core library
//!
//! This crate provides the core functionality for the Wiggle Puppy autonomous
//! AI agent loop, including error handling, PRD parsing, event system,
//! configuration, agent execution, and the main runner loop.

pub mod error;

pub use error::{Error, Result};
