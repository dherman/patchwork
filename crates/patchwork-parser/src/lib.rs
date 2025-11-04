pub mod token;
pub mod adapter;
pub mod ast;

// Include generated parser code from lalrpop
#[allow(clippy::all)]
mod patchwork {
    include!(concat!(env!("OUT_DIR"), "/patchwork.rs"));
}

pub use adapter::{LexerAdapter, ParseError};
pub use token::ParserToken;
pub use ast::*;

use patchwork_lexer::lex_str;

/// Parse a patchwork program from a string
pub fn parse(input: &str) -> Result<Program<'_>, ParseError> {
    // Create lexer
    let lexer = lex_str(input).map_err(|e| ParseError::LexerError(e.to_string()))?;

    // Create adapter
    let adapter = LexerAdapter::new(input, lexer);

    // Parse using generated parser
    patchwork::ProgramParser::new()
        .parse(input, adapter)
        .map_err(|e| ParseError::UnexpectedToken(format!("{:?}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        // Empty input should parse successfully (empty program)
        let result = parse("");
        assert!(result.is_ok(), "Failed to parse empty input: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 0, "Expected empty program");
    }

    #[test]
    fn test_parse_simple_import() {
        let input = "import foo";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse simple import: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Import(decl) => {
                match &decl.path {
                    ImportPath::Simple(parts) => {
                        assert_eq!(parts.len(), 1);
                        assert_eq!(parts[0], "foo");
                    }
                    _ => panic!("Expected Simple import path"),
                }
            }
            _ => panic!("Expected Import item"),
        }
    }

    #[test]
    fn test_parse_relative_multi_import() {
        let input = "import ./{analyst, narrator, scribe}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse relative multi-import: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Import(decl) => {
                match &decl.path {
                    ImportPath::RelativeMulti(names) => {
                        assert_eq!(names.len(), 3);
                        assert_eq!(names[0], "analyst");
                        assert_eq!(names[1], "narrator");
                        assert_eq!(names[2], "scribe");
                    }
                    _ => panic!("Expected RelativeMulti import path"),
                }
            }
            _ => panic!("Expected Import item"),
        }
    }

    #[test]
    fn test_parse_skill_empty() {
        let input = "skill foo() {}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse empty skill: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Skill(decl) => {
                assert_eq!(decl.name, "foo");
                assert_eq!(decl.params.len(), 0);
                assert_eq!(decl.body.statements.len(), 0);
            }
            _ => panic!("Expected Skill item"),
        }
    }

    #[test]
    fn test_parse_skill_with_params() {
        let input = "skill rewriting_git_branch(changeset_description) {}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse skill with params: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Skill(decl) => {
                assert_eq!(decl.name, "rewriting_git_branch");
                assert_eq!(decl.params.len(), 1);
                assert_eq!(decl.params[0].name, "changeset_description");
            }
            _ => panic!("Expected Skill item"),
        }
    }

    #[test]
    fn test_parse_task() {
        let input = "task analyst(session_id, work_dir, changeset) {}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse task: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Task(decl) => {
                assert_eq!(decl.name, "analyst");
                assert_eq!(decl.params.len(), 3);
                assert_eq!(decl.params[0].name, "session_id");
                assert_eq!(decl.params[1].name, "work_dir");
                assert_eq!(decl.params[2].name, "changeset");
            }
            _ => panic!("Expected Task item"),
        }
    }

    #[test]
    fn test_parse_function() {
        let input = "fun helper(x, y) {}";
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse function: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Function(decl) => {
                assert_eq!(decl.name, "helper");
                assert_eq!(decl.params.len(), 2);
                assert_eq!(decl.params[0].name, "x");
                assert_eq!(decl.params[1].name, "y");
            }
            _ => panic!("Expected Function item"),
        }
    }

    #[test]
    fn test_parse_multiple_items() {
        let input = r#"
            import ./{analyst, narrator, scribe}

            skill rewriting_git_branch(changeset_description) {}

            task analyst(session_id) {}
            task narrator(session_id) {}

            fun helper() {}
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse multiple items: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 5);

        // Check item types
        assert!(matches!(program.items[0], Item::Import(_)));
        assert!(matches!(program.items[1], Item::Skill(_)));
        assert!(matches!(program.items[2], Item::Task(_)));
        assert!(matches!(program.items[3], Item::Task(_)));
        assert!(matches!(program.items[4], Item::Function(_)));
    }

    #[test]
    fn test_parse_historian_main_structure() {
        // Parse just the structure (import + skill declaration) from historian main.pw
        // Can't parse the body yet (Milestone 3+), but structure should work
        let input = r#"
            import ./{analyst, narrator, scribe}

            skill rewriting_git_branch(changeset_description) {}
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse historian main structure: {:?}", result);

        let program = result.unwrap();
        assert_eq!(program.items.len(), 2);

        // Verify import
        match &program.items[0] {
            Item::Import(decl) => {
                match &decl.path {
                    ImportPath::RelativeMulti(names) => {
                        assert_eq!(names.len(), 3);
                        assert!(names.contains(&"analyst"));
                        assert!(names.contains(&"narrator"));
                        assert!(names.contains(&"scribe"));
                    }
                    _ => panic!("Expected RelativeMulti import"),
                }
            }
            _ => panic!("Expected Import item"),
        }

        // Verify skill
        match &program.items[1] {
            Item::Skill(decl) => {
                assert_eq!(decl.name, "rewriting_git_branch");
                assert_eq!(decl.params.len(), 1);
                assert_eq!(decl.params[0].name, "changeset_description");
            }
            _ => panic!("Expected Skill item"),
        }
    }

    // ==================== Variable Declarations ====================

    #[test]
    fn test_var_decl_no_init() {
        let input = r#"
            fun test() {
                var x
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse var x: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 1);
        match &func.body.statements[0] {
            Statement::VarDecl { name, type_ann, init } => {
                assert_eq!(*name, "x");
                assert!(type_ann.is_none());
                assert!(init.is_none());
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_var_decl_with_init() {
        let input = r#"
            fun test() {
                var x = foo
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse var x = foo: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 1);
        match &func.body.statements[0] {
            Statement::VarDecl { name, type_ann, init } => {
                assert_eq!(*name, "x");
                assert!(type_ann.is_none());
                assert!(init.is_some());
                match init.as_ref().unwrap() {
                    Expr::Identifier(id) => assert_eq!(*id, "foo"),
                    _ => panic!("Expected identifier expression"),
                }
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_var_decl_with_type() {
        let input = r#"
            fun test() {
                var x: string
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse var x: string: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 1);
        match &func.body.statements[0] {
            Statement::VarDecl { name, type_ann, init } => {
                assert_eq!(*name, "x");
                assert!(type_ann.is_some());
                match type_ann.as_ref().unwrap() {
                    TypeExpr::Name(t) => assert_eq!(*t, "string"),
                }
                assert!(init.is_none());
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    #[test]
    fn test_var_decl_with_type_and_init() {
        let input = r#"
            fun test() {
                var x: int = 42
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse var x: int = 42: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 1);
        match &func.body.statements[0] {
            Statement::VarDecl { name, type_ann, init } => {
                assert_eq!(*name, "x");
                assert!(type_ann.is_some());
                assert!(init.is_some());
            }
            _ => panic!("Expected VarDecl"),
        }
    }

    // ==================== Control Flow ====================

    #[test]
    fn test_if_statement() {
        let input = r#"
            fun test() {
                if condition {
                    var x = 1
                }
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse if statement: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 1);
        match &func.body.statements[0] {
            Statement::If { condition, then_block, else_block } => {
                match condition {
                    Expr::Identifier(id) => assert_eq!(*id, "condition"),
                    _ => panic!("Expected identifier"),
                }
                assert_eq!(then_block.statements.len(), 1);
                assert!(else_block.is_none());
            }
            _ => panic!("Expected If statement"),
        }
    }

    #[test]
    fn test_if_else_statement() {
        let input = r#"
            fun test() {
                if x {
                    var a = 1
                } else {
                    var b = 2
                }
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse if-else: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::If { condition: _, then_block, else_block } => {
                assert_eq!(then_block.statements.len(), 1);
                assert!(else_block.is_some());
                assert_eq!(else_block.as_ref().unwrap().statements.len(), 1);
            }
            _ => panic!("Expected If statement"),
        }
    }

    #[test]
    fn test_for_loop() {
        let input = r#"
            fun test() {
                for var item in items {
                    var x = item
                }
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse for loop: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::For { var, iter, body } => {
                assert_eq!(*var, "item");
                match iter {
                    Expr::Identifier(id) => assert_eq!(*id, "items"),
                    _ => panic!("Expected identifier"),
                }
                assert_eq!(body.statements.len(), 1);
            }
            _ => panic!("Expected For statement"),
        }
    }

    #[test]
    fn test_while_loop() {
        let input = r#"
            fun test() {
                while (condition) {
                    var x = 1
                }
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse while loop: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::While { condition, body } => {
                match condition {
                    Expr::Identifier(id) => assert_eq!(*id, "condition"),
                    _ => panic!("Expected identifier"),
                }
                assert_eq!(body.statements.len(), 1);
            }
            _ => panic!("Expected While statement"),
        }
    }

    // ==================== Flow Control Keywords ====================

    #[test]
    fn test_return_no_value() {
        let input = r#"
            fun test() {
                return
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse return: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::Return(expr) => {
                assert!(expr.is_none(), "Expected return with no value");
            }
            _ => panic!("Expected Return statement"),
        }
    }

    #[test]
    fn test_return_with_value() {
        let input = r#"
            fun test() {
                return value
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse return value: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::Return(expr) => {
                assert!(expr.is_some(), "Expected return with value");
                match expr.as_ref().unwrap() {
                    Expr::Identifier(id) => assert_eq!(*id, "value"),
                    _ => panic!("Expected identifier"),
                }
            }
            _ => panic!("Expected Return statement"),
        }
    }

    #[test]
    fn test_succeed_fail_break() {
        let input = r#"
            task test() {
                succeed
                fail
                break
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse succeed/fail/break: {:?}", result);

        let program = result.unwrap();
        let task = match &program.items[0] {
            Item::Task(t) => t,
            _ => panic!("Expected task"),
        };

        assert_eq!(task.body.statements.len(), 3);
        assert!(matches!(task.body.statements[0], Statement::Succeed));
        assert!(matches!(task.body.statements[1], Statement::Fail));
        assert!(matches!(task.body.statements[2], Statement::Break));
    }

    // ==================== Statement Separation ====================

    #[test]
    fn test_return_newline_separation() {
        // Key test: newlines SEPARATE statements (Swift-style)
        // return\nx means: return nothing, then x as next statement
        let input = r#"
            fun test() {
                return
                x
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse return with newline: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        // Should have TWO statements: return (no value) and x (expression statement)
        assert_eq!(func.body.statements.len(), 2, "Expected 2 statements");

        match &func.body.statements[0] {
            Statement::Return(expr) => {
                assert!(expr.is_none(), "return should have no value (separated by newline)");
            }
            _ => panic!("Expected Return statement"),
        }

        match &func.body.statements[1] {
            Statement::Expr(Expr::Identifier(id)) => {
                assert_eq!(*id, "x");
            }
            _ => panic!("Expected expression statement"),
        }
    }

    #[test]
    fn test_semicolon_separator() {
        let input = r#"
            fun test() {
                var x = 1; var y = 2; var z = 3
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse semicolon-separated statements: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        // Should have 3 statements on one line
        assert_eq!(func.body.statements.len(), 3);
    }

    #[test]
    fn test_multiple_statements_newline_separated() {
        let input = r#"
            fun test() {
                var x = 1
                var y = 2
                if x {
                    return y
                }
                var z = 3
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse multiple statements: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 4);
    }

    #[test]
    fn test_expression_statement() {
        let input = r#"
            fun test() {
                foo
                42
                true
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse expression statements: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 3);
        assert!(matches!(func.body.statements[0], Statement::Expr(Expr::Identifier(_))));
        assert!(matches!(func.body.statements[1], Statement::Expr(Expr::Number(_))));
        assert!(matches!(func.body.statements[2], Statement::Expr(Expr::True)));
    }

    // ==================== Milestone 4: Basic Expressions ====================

    #[test]
    fn test_literals() {
        let input = r#"
            fun test() {
                42
                "hello"
                true
                false
                foo
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse literals: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 5);
        assert!(matches!(func.body.statements[0], Statement::Expr(Expr::Number("42"))));
        assert!(matches!(func.body.statements[1], Statement::Expr(Expr::String(_))));
        assert!(matches!(func.body.statements[2], Statement::Expr(Expr::True)));
        assert!(matches!(func.body.statements[3], Statement::Expr(Expr::False)));
        assert!(matches!(func.body.statements[4], Statement::Expr(Expr::Identifier("foo"))));
    }

    #[test]
    fn test_string_literal() {
        let input = r#"
            fun test() {
                var x = "hello"
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse string literal: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::VarDecl { name, init, .. } => {
                assert_eq!(*name, "x");
                match init.as_ref().unwrap() {
                    Expr::String(s) => assert_eq!(s.text, "hello"),
                    _ => panic!("Expected string literal"),
                }
            }
            _ => panic!("Expected var decl"),
        }
    }

    #[test]
    fn test_binary_arithmetic() {
        let input = r#"
            fun test() {
                1 + 2
                x - y
                a * b
                c / d
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse binary arithmetic: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 4);

        // Check first binary op: 1 + 2
        match &func.body.statements[0] {
            Statement::Expr(Expr::Binary { op, .. }) => {
                assert!(matches!(op, BinOp::Add));
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn test_operator_precedence() {
        // Test that 1 + 2 * 3 parses as 1 + (2 * 3), not (1 + 2) * 3
        let input = r#"
            fun test() {
                var x = 1 + 2 * 3
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse precedence: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::VarDecl { init, .. } => {
                match init.as_ref().unwrap() {
                    // Should be: Add(1, Mul(2, 3))
                    Expr::Binary { op: BinOp::Add, left, right } => {
                        // Left should be 1
                        assert!(matches!(**left, Expr::Number("1")));
                        // Right should be 2 * 3
                        match &**right {
                            Expr::Binary { op: BinOp::Mul, .. } => {},
                            _ => panic!("Expected multiplication on right side"),
                        }
                    }
                    _ => panic!("Expected Add binary expression"),
                }
            }
            _ => panic!("Expected var decl"),
        }
    }

    #[test]
    fn test_comparison_operators() {
        let input = r#"
            fun test() {
                x == y
                a != b
                c < d
                e > f
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse comparisons: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 4);

        let ops = vec![BinOp::Eq, BinOp::NotEq, BinOp::Lt, BinOp::Gt];
        for (i, expected_op) in ops.iter().enumerate() {
            match &func.body.statements[i] {
                Statement::Expr(Expr::Binary { op, .. }) => {
                    assert_eq!(op, expected_op);
                }
                _ => panic!("Expected binary expression"),
            }
        }
    }

    #[test]
    fn test_logical_operators() {
        let input = r#"
            fun test() {
                a && b
                x || y
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse logical ops: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 2);

        match &func.body.statements[0] {
            Statement::Expr(Expr::Binary { op: BinOp::And, .. }) => {},
            _ => panic!("Expected && expression"),
        }

        match &func.body.statements[1] {
            Statement::Expr(Expr::Binary { op: BinOp::Or, .. }) => {},
            _ => panic!("Expected || expression"),
        }
    }

    #[test]
    fn test_unary_operators() {
        let input = r#"
            fun test() {
                !x
                -5
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse unary ops: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 2);

        match &func.body.statements[0] {
            Statement::Expr(Expr::Unary { op: UnOp::Not, .. }) => {},
            _ => panic!("Expected ! expression"),
        }

        match &func.body.statements[1] {
            Statement::Expr(Expr::Unary { op: UnOp::Neg, .. }) => {},
            _ => panic!("Expected - expression"),
        }
    }

    #[test]
    fn test_function_call() {
        let input = r#"
            fun test() {
                log(a, b, c)
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse function call: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::Expr(Expr::Call { callee, args }) => {
                match &**callee {
                    Expr::Identifier(name) => assert_eq!(*name, "log"),
                    _ => panic!("Expected identifier as callee"),
                }
                assert_eq!(args.len(), 3);
            }
            _ => panic!("Expected function call"),
        }
    }

    #[test]
    fn test_member_access() {
        let input = r#"
            fun test() {
                commit.num
                plan.length
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse member access: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 2);

        match &func.body.statements[0] {
            Statement::Expr(Expr::Member { object, field }) => {
                match &**object {
                    Expr::Identifier(name) => assert_eq!(*name, "commit"),
                    _ => panic!("Expected identifier as object"),
                }
                assert_eq!(*field, "num");
            }
            _ => panic!("Expected member access"),
        }
    }

    #[test]
    fn test_method_call() {
        let input = r#"
            fun test() {
                self.receive(timeout)
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse method call: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::Expr(Expr::Call { callee, args }) => {
                // Callee should be self.receive
                match &**callee {
                    Expr::Member { object, field } => {
                        match &**object {
                            Expr::Identifier(name) => assert_eq!(*name, "self"),
                            _ => panic!("Expected self as object"),
                        }
                        assert_eq!(*field, "receive");
                    }
                    _ => panic!("Expected member access as callee"),
                }
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected call expression"),
        }
    }

    #[test]
    fn test_index_access() {
        let input = r#"
            fun test() {
                arr[i]
                data[0]
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse index access: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        assert_eq!(func.body.statements.len(), 2);

        match &func.body.statements[0] {
            Statement::Expr(Expr::Index { object, index }) => {
                match &**object {
                    Expr::Identifier(name) => assert_eq!(*name, "arr"),
                    _ => panic!("Expected identifier as object"),
                }
                match &**index {
                    Expr::Identifier(name) => assert_eq!(*name, "i"),
                    _ => panic!("Expected identifier as index"),
                }
            }
            _ => panic!("Expected index access"),
        }
    }

    #[test]
    fn test_range_operator() {
        let input = r#"
            fun test() {
                1...3
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse range: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        match &func.body.statements[0] {
            Statement::Expr(Expr::Binary { op: BinOp::Range, left, right }) => {
                assert!(matches!(**left, Expr::Number("1")));
                assert!(matches!(**right, Expr::Number("3")));
            }
            _ => panic!("Expected range expression"),
        }
    }

    #[test]
    fn test_parenthesized_expr() {
        let input = r#"
            fun test() {
                (x + y) * z
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse parenthesized expr: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        // Should parse as Mul(Paren(Add(x, y)), z)
        match &func.body.statements[0] {
            Statement::Expr(Expr::Binary { op: BinOp::Mul, left, right }) => {
                match &**left {
                    Expr::Paren(inner) => {
                        match &**inner {
                            Expr::Binary { op: BinOp::Add, .. } => {},
                            _ => panic!("Expected Add inside parens"),
                        }
                    }
                    _ => panic!("Expected parenthesized expression"),
                }
                assert!(matches!(**right, Expr::Identifier("z")));
            }
            _ => panic!("Expected multiplication"),
        }
    }

    #[test]
    fn test_complex_nested_expression() {
        let input = r#"
            fun test() {
                var x = self.receive(timeout).status == "success"
            }
        "#;
        let result = parse(input);
        assert!(result.is_ok(), "Failed to parse complex expression: {:?}", result);

        let program = result.unwrap();
        let func = match &program.items[0] {
            Item::Function(f) => f,
            _ => panic!("Expected function"),
        };

        // Should parse successfully - verify it's a var decl with a complex init
        match &func.body.statements[0] {
            Statement::VarDecl { init, .. } => {
                assert!(init.is_some(), "Expected init expression");
                // It should be an Eq comparison
                match init.as_ref().unwrap() {
                    Expr::Binary { op: BinOp::Eq, .. } => {},
                    _ => panic!("Expected == comparison at top level"),
                }
            }
            _ => panic!("Expected var decl"),
        }
    }

    // ===== Milestone 5: Prompt Expressions =====

    #[test]
    fn test_simple_think_block() {
        let input = r#"
            task test() {
                var x = think {
                    What is the answer?
                }
            }
        "#;
        let program = parse(input).expect("Should parse");
        assert_eq!(program.items.len(), 1);

        // Verify it's a task with a var decl containing a Think expression
        match &program.items[0] {
            Item::Task(task) => {
                assert_eq!(task.body.statements.len(), 1);
                match &task.body.statements[0] {
                    Statement::VarDecl { name, init, .. } => {
                        assert_eq!(*name, "x");
                        assert!(init.is_some());
                        match init.as_ref().unwrap() {
                            Expr::Think(_) => {}, // Success!
                            _ => panic!("Expected Think expression"),
                        }
                    }
                    _ => panic!("Expected var decl"),
                }
            }
            _ => panic!("Expected task"),
        }
    }

    #[test]
    fn test_simple_ask_block() {
        let input = r#"
            task test() {
                var approval = ask {
                    Do you approve?
                }
            }
        "#;
        let program = parse(input).expect("Should parse");
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Task(task) => {
                assert_eq!(task.body.statements.len(), 1);
                match &task.body.statements[0] {
                    Statement::VarDecl { init, .. } => {
                        match init.as_ref().unwrap() {
                            Expr::Ask(_) => {}, // Success!
                            _ => panic!("Expected Ask expression"),
                        }
                    }
                    _ => panic!("Expected var decl"),
                }
            }
            _ => panic!("Expected task"),
        }
    }

    #[test]
    fn test_think_with_fallback() {
        let input = r#"
            task test() {
                var cmd = think {
                    Figure it out
                } || ask {
                    What command?
                }
            }
        "#;
        let program = parse(input).expect("Should parse");
        assert_eq!(program.items.len(), 1);

        // The || creates a Binary expr with Think on left and Ask on right
        match &program.items[0] {
            Item::Task(task) => {
                match &task.body.statements[0] {
                    Statement::VarDecl { init, .. } => {
                        match init.as_ref().unwrap() {
                            Expr::Binary { op: BinOp::Or, left, right } => {
                                // Left should be Think, right should be Ask
                                assert!(matches!(**left, Expr::Think(_)));
                                assert!(matches!(**right, Expr::Ask(_)));
                            }
                            _ => panic!("Expected Binary Or expression"),
                        }
                    }
                    _ => panic!("Expected var decl"),
                }
            }
            _ => panic!("Expected task"),
        }
    }

    #[test]
    fn test_prompt_with_embedded_do() {
        let input = r#"
            task test() {
                var result = think {
                    First analyze the problem.
                    do {
                        var x = read_file()
                    }
                    Then explain the solution.
                }
            }
        "#;
        let program = parse(input).expect("Should parse");
        assert_eq!(program.items.len(), 1);

        // PromptBlock should have multiple items: text words, then code block, then more text words
        // Note: lexer splits prompt text into individual words
        match &program.items[0] {
            Item::Task(task) => {
                match &task.body.statements[0] {
                    Statement::VarDecl { init, .. } => {
                        match init.as_ref().unwrap() {
                            Expr::Think(prompt_block) => {
                                // Should have at least some items
                                assert!(prompt_block.items.len() > 0);

                                // Find the Code item
                                let has_code = prompt_block.items.iter()
                                    .any(|item| matches!(item, PromptItem::Code(_)));
                                assert!(has_code, "Expected to find a Code item in prompt block");

                                // Should have some text items too
                                let has_text = prompt_block.items.iter()
                                    .any(|item| matches!(item, PromptItem::Text(_)));
                                assert!(has_text, "Expected to find Text items in prompt block");
                            }
                            _ => panic!("Expected Think expression"),
                        }
                    }
                    _ => panic!("Expected var decl"),
                }
            }
            _ => panic!("Expected task"),
        }
    }

    // Note: do { } is NOT a standalone expression in patchwork
    // It's only used inside think/ask prompt blocks
    // So we don't have a test for standalone do expressions

    #[test]
    fn test_multiline_think_block() {
        let input = r#"
            task test() {
                var build_command = think {
                    Figure out how to run a lightweight build for this project:

                    **Common patterns:**
                    - Rust: cargo check
                    - TypeScript: tsc --noEmit

                    **Check for:**
                    1. Build files
                    2. Build scripts
                }
            }
        "#;
        let program = parse(input).expect("Should parse");
        assert_eq!(program.items.len(), 1);
    }

    #[test]
    fn test_nested_prompts_in_binary_expr() {
        // think { } || ask { } is a binary OR expression
        let input = r#"
            task foo() {
                var x = think { analyze } || ask { what should I do? }
            }
        "#;
        let program = parse(input).expect("Should parse");
        assert_eq!(program.items.len(), 1);
    }
}
