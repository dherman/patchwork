/// Patchwork compiler
///
/// Transforms Patchwork source code into executable agent systems.
/// For the MVP, targets Claude Code plugins.

pub mod driver;
pub mod error;
pub mod codegen;

pub use driver::{Compiler, CompileOptions, CompileOutput};
pub use error::{CompileError, Result};
pub use codegen::CodeGenerator;
