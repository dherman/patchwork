//! The Patchwork interpreter with suspend/resume capability.

use std::collections::HashMap;

use crate::error::Error;
use crate::value::Value;

/// Variable bindings passed to an LLM operation.
pub type Bindings = HashMap<String, Value>;

/// The type of LLM operation being requested.
#[derive(Debug, Clone, PartialEq)]
pub enum LlmOp {
    /// A `think { ... }` block - LLM processes and returns a value.
    Think,
    /// An `ask { ... }` block - interactive prompt (future).
    Ask,
}

/// The control state of the interpreter.
///
/// This enum represents the current execution state, inspired by
/// generator/coroutine semantics where execution can suspend and resume.
#[derive(Debug, Clone)]
pub enum ControlState {
    /// The interpreter is ready to evaluate or currently evaluating.
    Eval,

    /// The interpreter has suspended, waiting for an LLM response.
    Yield {
        /// The type of LLM operation.
        op: LlmOp,
        /// The interpolated prompt text to send to the LLM.
        prompt: String,
        /// Variable bindings available in the prompt context.
        bindings: Bindings,
        /// Description of the expected response type.
        expect: String,
    },

    /// The interpreter has completed successfully with a value.
    Return(Value),

    /// The interpreter has thrown an exception.
    Throw(Value),
}

impl ControlState {
    /// Check if the interpreter is in a terminal state (Return or Throw).
    pub fn is_terminal(&self) -> bool {
        matches!(self, ControlState::Return(_) | ControlState::Throw(_))
    }

    /// Check if the interpreter is yielding, waiting for LLM input.
    pub fn is_yield(&self) -> bool {
        matches!(self, ControlState::Yield { .. })
    }
}

/// The Patchwork interpreter.
///
/// Executes Patchwork code with the ability to suspend at `think` blocks,
/// allowing external systems to provide LLM responses before resuming.
pub struct Interpreter {
    /// Current control state.
    state: ControlState,
}

impl Interpreter {
    /// Create a new interpreter in the Eval state.
    pub fn new() -> Self {
        Self {
            state: ControlState::Eval,
        }
    }

    /// Get the current control state.
    pub fn state(&self) -> &ControlState {
        &self.state
    }

    /// Evaluate Patchwork code.
    ///
    /// Parses and executes the code, returning the resulting control state.
    /// If the code contains `think` blocks, the interpreter may yield,
    /// requiring a call to `resume()` with the LLM's response.
    pub fn eval(&mut self, code: &str) -> crate::Result<&ControlState> {
        // Parse the code using patchwork-parser
        match patchwork_parser::parse(code) {
            Ok(ast) => {
                // For Phase 1, just log the AST and return success
                eprintln!("[patchwork-eval] Parsed AST: {:?}", ast);
                self.state = ControlState::Return(Value::Null);
                Ok(&self.state)
            }
            Err(e) => {
                let msg = format!("{:?}", e);
                self.state = ControlState::Throw(Value::String(msg.clone()));
                Err(Error::Parse(msg))
            }
        }
    }

    /// Resume execution after an LLM response.
    ///
    /// This should only be called when the interpreter is in the `Yield` state.
    /// The provided value is the LLM's response, which becomes the result of
    /// the `think` block that caused the yield.
    pub fn resume(&mut self, _value: Value) -> crate::Result<&ControlState> {
        if !self.state.is_yield() {
            return Err(Error::InvalidResume);
        }

        // Phase 1 stub - not yet implemented
        Err(Error::Runtime("resume not yet implemented".to_string()))
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_interpreter() {
        let interp = Interpreter::new();
        assert!(matches!(interp.state(), ControlState::Eval));
    }

    #[test]
    fn test_eval_empty_program() {
        let mut interp = Interpreter::new();
        // Empty program is valid Patchwork
        let result = interp.eval("");
        assert!(result.is_ok());
        assert!(matches!(interp.state(), ControlState::Return(_)));
    }

    #[test]
    fn test_eval_simple_function() {
        let mut interp = Interpreter::new();
        // A simple function definition
        let result = interp.eval("fun hello() {}");
        assert!(result.is_ok());
        assert!(matches!(interp.state(), ControlState::Return(_)));
    }

    #[test]
    fn test_resume_without_yield() {
        let mut interp = Interpreter::new();
        let result = interp.resume(Value::Null);
        assert!(matches!(result, Err(Error::InvalidResume)));
    }
}
