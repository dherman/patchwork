//! JavaScript runtime for Patchwork
//!
//! This module contains the JavaScript runtime code that gets bundled
//! with compiled Patchwork programs.

/// Get the complete JavaScript runtime code
pub fn get_runtime_code() -> &'static str {
    include_str!("runtime.js")
}

/// Get the code process initialization script
pub fn get_code_process_init() -> &'static str {
    include_str!("code-process-init.js")
}

/// Get the runtime module name that generated code should import from
pub fn get_runtime_module_name() -> &'static str {
    "./patchwork-runtime.js"
}
