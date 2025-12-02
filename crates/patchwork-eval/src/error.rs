//! Error types for the Patchwork interpreter.

use std::fmt;

/// Errors that can occur during interpretation.
#[derive(Debug, Clone)]
pub enum Error {
    /// A parse error occurred.
    Parse(String),
    /// A runtime error occurred.
    Runtime(String),
    /// Resume called when not in Yield state.
    InvalidResume,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
            Error::Runtime(msg) => write!(f, "Runtime error: {}", msg),
            Error::InvalidResume => write!(f, "Cannot resume: interpreter is not in Yield state"),
        }
    }
}

impl std::error::Error for Error {}
