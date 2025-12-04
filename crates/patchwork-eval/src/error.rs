//! Error types for the Patchwork interpreter.

use std::fmt;

use crate::value::Value;

/// Errors that can occur during interpretation.
#[derive(Debug, Clone)]
pub enum Error {
    /// A parse error occurred.
    Parse(String),
    /// A runtime error occurred.
    Runtime(String),
    /// A Patchwork exception was thrown (via `throw` keyword).
    /// This propagates up the call stack using Rust's `?` operator.
    Exception(Value),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
            Error::Runtime(msg) => write!(f, "Runtime error: {}", msg),
            Error::Exception(value) => write!(f, "Exception: {}", value.to_string_value()),
        }
    }
}

impl std::error::Error for Error {}
