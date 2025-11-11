/// Code generation module
///
/// Transforms Patchwork AST into executable JavaScript.
/// Phase 2 focuses on simple worker codegen with only code mode features.

use patchwork_parser::ast::*;
use crate::error::{CompileError, Result};
use std::fmt::Write as _;

/// JavaScript code generator
pub struct CodeGenerator {
    /// Indentation level for pretty-printing
    indent: usize,
    /// Output buffer
    output: String,
}

impl CodeGenerator {
    /// Create a new code generator
    pub fn new() -> Self {
        Self {
            indent: 0,
            output: String::new(),
        }
    }

    /// Generate JavaScript code for a program
    pub fn generate(&mut self, program: &Program) -> Result<String> {
        // For Phase 2, we only support workers (no traits, skills, imports yet)
        for item in &program.items {
            match item {
                Item::Worker(worker) => {
                    self.generate_worker(worker)?;
                    self.output.push('\n');
                }
                Item::Function(func) => {
                    self.generate_function(func)?;
                    self.output.push('\n');
                }
                Item::Import(_) => {
                    // Phase 2: Skip imports (Phase 7)
                }
                Item::Skill(_) => {
                    // Phase 2: Skip skills (Phase 6)
                }
                Item::Trait(_) => {
                    // Phase 2: Skip traits (Phase 6)
                }
                Item::Type(_) => {
                    // Phase 2: Skip type declarations (Phase 8)
                }
            }
        }

        Ok(std::mem::take(&mut self.output))
    }

    /// Generate code for a worker declaration
    fn generate_worker(&mut self, worker: &WorkerDecl) -> Result<()> {
        // Generate: export function workerName(params) { ... }
        // Note: Workers become exported functions for the runtime to invoke

        write!(self.output, "export function {}", worker.name)?;
        self.generate_params(&worker.params)?;
        self.output.push_str(" {\n");

        self.indent += 1;
        self.generate_block(&worker.body)?;
        self.indent -= 1;

        self.output.push_str("}\n");
        Ok(())
    }

    /// Generate code for a function declaration
    fn generate_function(&mut self, func: &FunctionDecl) -> Result<()> {
        // Generate: export function funcName(params) { ... }
        let export = if func.is_exported { "export " } else { "" };

        write!(self.output, "{}function {}", export, func.name)?;
        self.generate_params(&func.params)?;
        self.output.push_str(" {\n");

        self.indent += 1;
        self.generate_block(&func.body)?;
        self.indent -= 1;

        self.output.push_str("}\n");
        Ok(())
    }

    /// Generate parameter list
    fn generate_params(&mut self, params: &[Param]) -> Result<()> {
        self.output.push('(');
        for (i, param) in params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(param.name);
            // Phase 2: Ignore type annotations
        }
        self.output.push(')');
        Ok(())
    }

    /// Generate code for a block
    fn generate_block(&mut self, block: &Block) -> Result<()> {
        for stmt in &block.statements {
            self.write_indent();
            self.generate_statement(stmt)?;
            self.output.push('\n');
        }
        Ok(())
    }

    /// Generate code for a statement
    fn generate_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::VarDecl { pattern, init } => {
                self.generate_var_decl(pattern, init)?;
            }
            Statement::Expr(expr) => {
                self.generate_expr(expr)?;
                self.output.push(';');
            }
            Statement::If { condition, then_block, else_block } => {
                self.generate_if(condition, then_block, else_block)?;
            }
            Statement::While { condition, body } => {
                self.generate_while(condition, body)?;
            }
            Statement::ForIn { var, iter, body } => {
                self.generate_for_in(var, iter, body)?;
            }
            Statement::Return(expr) => {
                self.output.push_str("return");
                if let Some(e) = expr {
                    self.output.push(' ');
                    self.generate_expr(e)?;
                }
                self.output.push(';');
            }
            Statement::Break => {
                self.output.push_str("break;");
            }
            Statement::Succeed => {
                // Phase 2: Skip succeed (task-specific, Phase 6)
                self.output.push_str("// succeed statement (not yet implemented)");
            }
            Statement::TypeDecl { .. } => {
                // Phase 2: Skip type declarations (Phase 8)
            }
        }
        Ok(())
    }

    /// Generate variable declaration
    fn generate_var_decl(&mut self, pattern: &Pattern, init: &Option<Expr>) -> Result<()> {
        match pattern {
            Pattern::Identifier { name, .. } => {
                // Simple case: var x = init
                write!(self.output, "let {}", name)?;
                if let Some(expr) = init {
                    self.output.push_str(" = ");
                    self.generate_expr(expr)?;
                }
                self.output.push(';');
            }
            Pattern::Ignore => {
                // var _ = expr → just evaluate expr
                if let Some(expr) = init {
                    self.generate_expr(expr)?;
                    self.output.push(';');
                }
            }
            Pattern::Object(_fields) => {
                // Phase 2: Skip destructuring (Milestone 7)
                return Err(CompileError::Unsupported("Object destructuring not yet supported in Phase 2".into()));
            }
            Pattern::Array(_items) => {
                // Phase 2: Skip destructuring (Milestone 7)
                return Err(CompileError::Unsupported("Array destructuring not yet supported in Phase 2".into()));
            }
        }
        Ok(())
    }

    /// Generate if statement
    fn generate_if(&mut self, condition: &Expr, then_block: &Block, else_block: &Option<Block>) -> Result<()> {
        self.output.push_str("if (");
        self.generate_expr(condition)?;
        self.output.push_str(") {\n");

        self.indent += 1;
        self.generate_block(then_block)?;
        self.indent -= 1;

        self.write_indent();
        self.output.push('}');

        if let Some(else_blk) = else_block {
            self.output.push_str(" else {\n");
            self.indent += 1;
            self.generate_block(else_blk)?;
            self.indent -= 1;
            self.write_indent();
            self.output.push('}');
        }

        Ok(())
    }

    /// Generate while loop
    fn generate_while(&mut self, condition: &Expr, body: &Block) -> Result<()> {
        self.output.push_str("while (");
        self.generate_expr(condition)?;
        self.output.push_str(") {\n");

        self.indent += 1;
        self.generate_block(body)?;
        self.indent -= 1;

        self.write_indent();
        self.output.push('}');
        Ok(())
    }

    /// Generate for-in loop
    fn generate_for_in(&mut self, var: &str, iter: &Expr, body: &Block) -> Result<()> {
        self.output.push_str("for (let ");
        self.output.push_str(var);
        self.output.push_str(" of ");
        self.generate_expr(iter)?;
        self.output.push_str(") {\n");

        self.indent += 1;
        self.generate_block(body)?;
        self.indent -= 1;

        self.write_indent();
        self.output.push('}');
        Ok(())
    }

    /// Generate code for an expression
    fn generate_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Identifier(name) => {
                self.output.push_str(name);
            }
            Expr::Number(n) => {
                self.output.push_str(n);
            }
            Expr::String(s) => {
                self.generate_string_literal(s)?;
            }
            Expr::True => {
                self.output.push_str("true");
            }
            Expr::False => {
                self.output.push_str("false");
            }
            Expr::Array(items) => {
                self.output.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.generate_expr(item)?;
                }
                self.output.push(']');
            }
            Expr::Object(fields) => {
                self.output.push_str("{ ");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(field.key);
                    if let Some(val) = &field.value {
                        self.output.push_str(": ");
                        self.generate_expr(val)?;
                    }
                    // If value is None, it's shorthand syntax {x} → {x: x}
                }
                self.output.push_str(" }");
            }
            Expr::Binary { op, left, right } => {
                self.generate_binary_op(op, left, right)?;
            }
            Expr::Unary { op, operand } => {
                self.generate_unary_op(op, operand)?;
            }
            Expr::Call { callee, args } => {
                self.generate_expr(callee)?;
                self.output.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.generate_expr(arg)?;
                }
                self.output.push(')');
            }
            Expr::Member { object, field } => {
                self.generate_expr(object)?;
                self.output.push('.');
                self.output.push_str(field);
            }
            Expr::Index { object, index } => {
                self.generate_expr(object)?;
                self.output.push('[');
                self.generate_expr(index)?;
                self.output.push(']');
            }
            Expr::Paren(inner) => {
                self.output.push('(');
                self.generate_expr(inner)?;
                self.output.push(')');
            }
            Expr::PostIncrement(operand) => {
                self.generate_expr(operand)?;
                self.output.push_str("++");
            }
            Expr::PostDecrement(operand) => {
                self.generate_expr(operand)?;
                self.output.push_str("--");
            }
            Expr::Await(inner) => {
                self.output.push_str("await ");
                self.generate_expr(inner)?;
            }
            Expr::BareCommand { name, args } => {
                // Shell command statement form
                self.generate_shell_command(name, args)?;
            }
            Expr::CommandSubst(cmd) => {
                // Shell command expression form: $(...)
                self.generate_command_subst(cmd)?;
            }
            Expr::ShellPipe { left, right } => {
                self.generate_shell_pipe(left, right)?;
            }
            Expr::ShellAnd { left, right } => {
                self.generate_shell_and(left, right)?;
            }
            Expr::ShellOr { left, right } => {
                self.generate_shell_or(left, right)?;
            }
            Expr::ShellRedirect { command, op, target } => {
                self.generate_shell_redirect(command, op, target)?;
            }
            Expr::Think(_) | Expr::Ask(_) => {
                // Phase 2: Skip prompt blocks (Phase 4)
                return Err(CompileError::Unsupported("Prompt blocks not yet supported in Phase 2".into()));
            }
            Expr::Do(_) => {
                // Phase 2: Skip do expressions
                return Err(CompileError::Unsupported("Do expressions not yet supported in Phase 2".into()));
            }
        }
        Ok(())
    }

    /// Generate string literal with interpolation support
    fn generate_string_literal(&mut self, s: &StringLiteral) -> Result<()> {
        if s.parts.len() == 1 {
            if let StringPart::Text(text) = &s.parts[0] {
                // Simple string with no interpolation
                write!(self.output, "\"{}\"", escape_string(text))?;
                return Ok(());
            }
        }

        // String with interpolation → use template literal
        self.output.push('`');
        for part in &s.parts {
            match part {
                StringPart::Text(text) => {
                    self.output.push_str(&escape_template_literal(text));
                }
                StringPart::Interpolation(expr) => {
                    self.output.push_str("${");
                    self.generate_expr(expr)?;
                    self.output.push('}');
                }
            }
        }
        self.output.push('`');
        Ok(())
    }

    /// Generate binary operation
    fn generate_binary_op(&mut self, op: &BinOp, left: &Expr, right: &Expr) -> Result<()> {
        self.generate_expr(left)?;
        let op_str = match op {
            BinOp::Add => " + ",
            BinOp::Sub => " - ",
            BinOp::Mul => " * ",
            BinOp::Div => " / ",
            BinOp::Eq => " === ",
            BinOp::NotEq => " !== ",
            BinOp::Lt => " < ",
            BinOp::Gt => " > ",
            BinOp::And => " && ",
            BinOp::Or => " || ",
            BinOp::Assign => " = ",
            BinOp::Pipe => {
                // Pipe operator for shell - handled specially
                return Err(CompileError::Unsupported("Use ShellPipe expression for shell pipes".into()));
            }
            BinOp::Range => {
                // Range operator - not yet implemented
                return Err(CompileError::Unsupported("Range operator not yet supported".into()));
            }
        };
        self.output.push_str(op_str);
        self.generate_expr(right)?;
        Ok(())
    }

    /// Generate unary operation
    fn generate_unary_op(&mut self, op: &UnOp, operand: &Expr) -> Result<()> {
        match op {
            UnOp::Not => self.output.push('!'),
            UnOp::Neg => self.output.push('-'),
            UnOp::Throw => {
                // throw expr → throw new Error(String(expr))
                self.output.push_str("throw new Error(String(");
                self.generate_expr(operand)?;
                self.output.push_str("))");
                return Ok(());
            }
        }
        self.generate_expr(operand)?;
        Ok(())
    }

    /// Generate shell command execution (statement form)
    fn generate_shell_command(&mut self, name: &str, args: &[CommandArg]) -> Result<()> {
        // For now, generate a runtime function call
        // TODO: Import runtime library in Phase 3
        self.output.push_str("await $shell(");

        // Build command string
        self.output.push('`');
        self.output.push_str(name);
        for arg in args {
            self.output.push(' ');
            match arg {
                CommandArg::Literal(lit) => {
                    self.output.push_str(lit);
                }
                CommandArg::String(s) => {
                    // Embedded string in command
                    for part in &s.parts {
                        match part {
                            StringPart::Text(text) => {
                                self.output.push_str(&escape_template_literal(text));
                            }
                            StringPart::Interpolation(expr) => {
                                self.output.push_str("${");
                                self.generate_expr(expr)?;
                                self.output.push('}');
                            }
                        }
                    }
                }
            }
        }
        self.output.push('`');
        self.output.push(')');
        Ok(())
    }

    /// Generate command substitution (expression form)
    fn generate_command_subst(&mut self, cmd: &Expr) -> Result<()> {
        // $(cmd) → await $shell(cmd, {capture: true})
        self.output.push_str("await $shell(");

        // If cmd is a bare command, extract its parts
        if let Expr::BareCommand { name, args } = cmd {
            self.output.push('`');
            self.output.push_str(name);
            for arg in args {
                self.output.push(' ');
                match arg {
                    CommandArg::Literal(lit) => {
                        self.output.push_str(lit);
                    }
                    CommandArg::String(s) => {
                        for part in &s.parts {
                            match part {
                                StringPart::Text(text) => {
                                    self.output.push_str(&escape_template_literal(text));
                                }
                                StringPart::Interpolation(expr) => {
                                    self.output.push_str("${");
                                    self.generate_expr(expr)?;
                                    self.output.push('}');
                                }
                            }
                        }
                    }
                }
            }
            self.output.push('`');
        } else {
            // Complex expression - just generate it
            self.generate_expr(cmd)?;
        }

        self.output.push_str(", {capture: true})");
        Ok(())
    }

    /// Generate shell pipe
    fn generate_shell_pipe(&mut self, left: &Expr, right: &Expr) -> Result<()> {
        // cmd1 | cmd2 → await $shellPipe([cmd1, cmd2])
        self.output.push_str("await $shellPipe([");
        self.generate_shell_expr_for_pipe(left)?;
        self.output.push_str(", ");
        self.generate_shell_expr_for_pipe(right)?;
        self.output.push_str("])");
        Ok(())
    }

    /// Generate shell && operator
    fn generate_shell_and(&mut self, left: &Expr, right: &Expr) -> Result<()> {
        // cmd1 && cmd2 → await $shellAnd([cmd1, cmd2])
        self.output.push_str("await $shellAnd([");
        self.generate_shell_expr_for_pipe(left)?;
        self.output.push_str(", ");
        self.generate_shell_expr_for_pipe(right)?;
        self.output.push_str("])");
        Ok(())
    }

    /// Generate shell || operator
    fn generate_shell_or(&mut self, left: &Expr, right: &Expr) -> Result<()> {
        // cmd1 || cmd2 → await $shellOr([cmd1, cmd2])
        self.output.push_str("await $shellOr([");
        self.generate_shell_expr_for_pipe(left)?;
        self.output.push_str(", ");
        self.generate_shell_expr_for_pipe(right)?;
        self.output.push_str("])");
        Ok(())
    }

    /// Generate shell redirect
    fn generate_shell_redirect(&mut self, command: &Expr, op: &RedirectOp, target: &Expr) -> Result<()> {
        // cmd > file → await $shellRedirect(cmd, '>', file)
        self.output.push_str("await $shellRedirect(");
        self.generate_shell_expr_for_pipe(command)?;
        self.output.push_str(", '");
        let op_str = match op {
            RedirectOp::Out => ">",
            RedirectOp::Append => ">>",
            RedirectOp::In => "<",
            RedirectOp::ErrOut => "2>",
            RedirectOp::ErrToOut => "2>&1",
        };
        self.output.push_str(op_str);
        self.output.push_str("', ");
        self.generate_expr(target)?;
        self.output.push(')');
        Ok(())
    }

    /// Helper to generate shell expressions for piping
    fn generate_shell_expr_for_pipe(&mut self, expr: &Expr) -> Result<()> {
        if let Expr::BareCommand { name, args } = expr {
            self.output.push('`');
            self.output.push_str(name);
            for arg in args {
                self.output.push(' ');
                match arg {
                    CommandArg::Literal(lit) => {
                        self.output.push_str(lit);
                    }
                    CommandArg::String(s) => {
                        for part in &s.parts {
                            match part {
                                StringPart::Text(text) => {
                                    self.output.push_str(&escape_template_literal(text));
                                }
                                StringPart::Interpolation(expr) => {
                                    self.output.push_str("${");
                                    self.generate_expr(expr)?;
                                    self.output.push('}');
                                }
                            }
                        }
                    }
                }
            }
            self.output.push('`');
        } else {
            self.generate_expr(expr)?;
        }
        Ok(())
    }

    /// Write current indentation
    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }
}

/// Escape a string for double-quoted JavaScript string literal
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Escape a string for JavaScript template literal
fn escape_template_literal(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_string("say \"hi\""), "say \\\"hi\\\"");
    }

    #[test]
    fn test_escape_template_literal() {
        assert_eq!(escape_template_literal("hello"), "hello");
        assert_eq!(escape_template_literal("use `backticks`"), "use \\`backticks\\`");
        assert_eq!(escape_template_literal("embed ${x}"), "embed \\${x}");
    }
}
