//! Patchwork interpreter with suspend/resume for LLM integration.
//!
//! This crate provides an interpreter for Patchwork code that can suspend
//! execution when encountering `think` blocks, allowing an external system
//! (like an ACP proxy) to send the prompt to an LLM and resume with the response.

mod value;
mod interpreter;
mod error;

pub use value::Value;
pub use interpreter::{Interpreter, ControlState, LlmOp, Bindings};
pub use error::Error;

/// Result type for interpreter operations.
pub type Result<T> = std::result::Result<T, Error>;
