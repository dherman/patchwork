//! Patchwork interpreter with synchronous blocking execution.
//!
//! This crate provides an interpreter for Patchwork code. In Phase 5,
//! `think` blocks will block on channel operations waiting for LLM responses.
//! Exceptions are modeled as `Error::Exception(Value)` and propagate using
//! Rust's `?` operator.

mod value;
mod interpreter;
mod runtime;
mod eval;
mod error;

pub use value::Value;
pub use interpreter::Interpreter;
pub use runtime::Runtime;
pub use eval::{eval_block, eval_expr, eval_statement};
pub use error::Error;

/// Result type for interpreter operations.
pub type Result<T> = std::result::Result<T, Error>;
