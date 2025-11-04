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

/// Statement (placeholder for now - will be expanded in Milestone 3+)
#[derive(Debug, Clone, PartialEq)]
pub enum Statement<'input> {
    /// Placeholder - actual statements will be added in Milestone 3
    Placeholder(PhantomData<&'input ()>),
}

/// Expression (placeholder for now - will be expanded in Milestone 4+)
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'input> {
    /// Placeholder - actual expressions will be added in Milestone 4
    Placeholder(PhantomData<&'input ()>),
}
