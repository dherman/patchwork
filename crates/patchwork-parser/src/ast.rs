/// Abstract Syntax Tree types for patchwork
///
/// These types represent the parsed structure of patchwork programs.
/// All types carry a lifetime 'input for zero-copy string slices.

use std::marker::PhantomData;

/// A complete patchwork program
#[derive(Debug, Clone, PartialEq)]
pub struct Program<'input> {
    pub items: Vec<Item<'input>>,
}

/// Top-level item (import, skill, task, or function declaration)
#[derive(Debug, Clone, PartialEq)]
pub enum Item<'input> {
    Import(ImportDecl<'input>),
    Skill(SkillDecl<'input>),
    Task(TaskDecl<'input>),
    Function(FunctionDecl<'input>),
}

/// Import declaration: `import std.log` or `import ./{analyst, narrator}`
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDecl<'input> {
    pub path: ImportPath<'input>,
}

/// Import path - either simple dotted path or relative multi-import
#[derive(Debug, Clone, PartialEq)]
pub enum ImportPath<'input> {
    /// Simple path: `std.log` or `./foo`
    Simple(Vec<&'input str>),
    /// Relative multi-import: `./{analyst, narrator, scribe}`
    RelativeMulti(Vec<&'input str>),
}

/// Skill declaration: `skill name(params) { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct SkillDecl<'input> {
    pub name: &'input str,
    pub params: Vec<Param<'input>>,
    pub body: Block<'input>,
}

/// Task declaration: `task name(params) { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct TaskDecl<'input> {
    pub name: &'input str,
    pub params: Vec<Param<'input>>,
    pub body: Block<'input>,
}

/// Function declaration: `fun name(params) { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl<'input> {
    pub name: &'input str,
    pub params: Vec<Param<'input>>,
    pub body: Block<'input>,
}

/// Function/task/skill parameter
#[derive(Debug, Clone, PartialEq)]
pub struct Param<'input> {
    pub name: &'input str,
    // Type annotations will be added in Milestone 8
}

/// Block of statements: `{ stmt1; stmt2; ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct Block<'input> {
    pub statements: Vec<Statement<'input>>,
}

/// Statement in a block
#[derive(Debug, Clone, PartialEq)]
pub enum Statement<'input> {
    /// Variable declaration: `var x` or `var x: type = expr`
    VarDecl {
        name: &'input str,
        type_ann: Option<TypeExpr<'input>>,
        init: Option<Expr<'input>>,
    },
    /// Expression statement (expression used as statement)
    Expr(Expr<'input>),
    /// If statement: `if expr { ... } else { ... }`
    If {
        condition: Expr<'input>,
        then_block: Block<'input>,
        else_block: Option<Block<'input>>,
    },
    /// For loop: `for var x in expr { ... }`
    For {
        var: &'input str,
        iter: Expr<'input>,
        body: Block<'input>,
    },
    /// While loop: `while (expr) { ... }`
    While {
        condition: Expr<'input>,
        body: Block<'input>,
    },
    /// Return statement: `return` or `return expr`
    Return(Option<Expr<'input>>),
    /// Succeed statement (for tasks): `succeed`
    Succeed,
    /// Fail statement (for tasks): `fail`
    Fail,
    /// Break statement (for loops): `break`
    Break,
}

/// Type expression (Milestone 3: minimal placeholder, full implementation in Milestone 8)
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr<'input> {
    /// Simple type name: `string`, `int`, etc.
    Name(&'input str),
}

/// Binary operator
#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    // Arithmetic
    Add,      // +
    Sub,      // -
    Mul,      // *
    Div,      // /
    // Comparison
    Eq,       // ==
    NotEq,    // !=
    Lt,       // <
    Gt,       // >
    // Logical
    And,      // &&
    Or,       // ||
    // Other
    Pipe,     // |
    Range,    // ...
    Assign,   // =
}

/// Unary operator
#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Not,      // !
    Neg,      // -
}

/// String literal (Milestone 6: with interpolation support)
#[derive(Debug, Clone, PartialEq)]
pub struct StringLiteral<'input> {
    /// Parts of the string - mixture of text and interpolated expressions
    pub parts: Vec<StringPart<'input>>,
}

/// Part of a string literal - either text or an interpolated expression
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart<'input> {
    /// Plain text: `"hello"` or text between interpolations
    Text(&'input str),
    /// Interpolated expression: `${expr}`, `$(cmd)`, or `$id`
    Interpolation(Box<Expr<'input>>),
}

/// Expression (Milestone 3: minimal set for statement support, expanded in Milestone 4)
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'input> {
    /// Identifier reference: `foo`
    Identifier(&'input str),
    /// Number literal: `42`, `3.14`
    Number(&'input str),
    /// String literal: `"hello"`
    String(StringLiteral<'input>),
    /// Boolean literal: `true`
    True,
    /// Boolean literal: `false`
    False,
    /// Binary operation: `a + b`, `x == y`
    Binary {
        op: BinOp,
        left: Box<Expr<'input>>,
        right: Box<Expr<'input>>,
    },
    /// Unary operation: `!x`, `-5`
    Unary {
        op: UnOp,
        operand: Box<Expr<'input>>,
    },
    /// Function call: `foo(a, b, c)`
    Call {
        callee: Box<Expr<'input>>,
        args: Vec<Expr<'input>>,
    },
    /// Member access: `obj.field`
    Member {
        object: Box<Expr<'input>>,
        field: &'input str,
    },
    /// Index access: `arr[i]`
    Index {
        object: Box<Expr<'input>>,
        index: Box<Expr<'input>>,
    },
    /// Parenthesized expression: `(expr)`
    Paren(Box<Expr<'input>>),
    /// Think expression: `think { ... }`
    Think(PromptBlock<'input>),
    /// Ask expression: `ask { ... }`
    Ask(PromptBlock<'input>),
    /// Do expression: `do { ... }`
    Do(Block<'input>),
    /// Placeholder for unparsed expressions (temporary for incremental implementation)
    Placeholder(PhantomData<&'input ()>),
}

/// Prompt block content - mixture of text and embedded code
#[derive(Debug, Clone, PartialEq)]
pub struct PromptBlock<'input> {
    pub items: Vec<PromptItem<'input>>,
}

/// Item within a prompt block
#[derive(Debug, Clone, PartialEq)]
pub enum PromptItem<'input> {
    /// Raw prompt text
    Text(&'input str),
    /// Embedded code block: `do { ... }`
    Code(Block<'input>),
}
