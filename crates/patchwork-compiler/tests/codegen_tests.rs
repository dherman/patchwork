/// Integration tests for code generation

use patchwork_compiler::{Compiler, CompileOptions};

/// Helper to compile a Patchwork source string
fn compile_source(source: &str) -> Result<String, String> {
    // Write source to a temp file
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join(format!("test_{}.pw", rand::random::<u32>()));
    std::fs::write(&test_file, source).map_err(|e| e.to_string())?;

    // Compile it
    let options = CompileOptions::new(&test_file);
    let compiler = Compiler::new(options);
    let output = compiler.compile().map_err(|e| e.to_string())?;

    // Clean up
    let _ = std::fs::remove_file(&test_file);

    Ok(output.javascript)
}

#[test]
fn test_simple_worker() {
    let source = r#"
worker example() {
    var x = 5
    return x
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("export function example()"));
    assert!(js.contains("let x = 5"));
    assert!(js.contains("return x"));
}

#[test]
fn test_worker_with_params() {
    let source = r#"
worker process(a, b) {
    var sum = a + b
    return sum
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("export function process(a, b)"));
    assert!(js.contains("let sum = a + b"));
}

#[test]
fn test_if_statement() {
    let source = r#"
worker check(x) {
    if x > 10 {
        return true
    } else {
        return false
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("if (x > 10)"));
    assert!(js.contains("} else {"));
}

#[test]
fn test_while_loop() {
    let source = r#"
worker loop_test() {
    var i = 0
    while (i < 10) {
        var temp = i
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("while (i < 10)"));
}

#[test]
fn test_for_loop() {
    let source = r#"
worker iterate(items) {
    for var item in items {
        var x = item
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("for (let item of items)"));
}

#[test]
fn test_string_interpolation() {
    let source = r#"
worker greet(name) {
    var msg = "Hello, ${name}!"
    return msg
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("`Hello, ${name}!`"));
}

#[test]
fn test_shell_command_statement() {
    let source = r#"
worker run_cmd() {
    $ echo "hello"
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await $shell(`echo hello`)"));
}

#[test]
fn test_shell_command_substitution() {
    let source = r#"
worker get_output() {
    var result = $(ls)
    return result
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await $shell(`ls`, {capture: true})"));
}

#[test]
fn test_shell_pipe() {
    let source = r#"
worker pipe_test() {
    $ echo "test" | grep test
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await $shellPipe"));
}

#[test]
fn test_shell_and() {
    let source = r#"
worker and_test() {
    $ touch file.txt && cat file.txt
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("await $shellAnd"));
}

#[test]
fn test_array_literal() {
    let source = r#"
worker arrays() {
    var nums = [1, 2, 3]
    return nums
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("[1, 2, 3]"));
}

#[test]
fn test_object_literal() {
    let source = r#"
worker objects() {
    var obj = {x: 1, y: 2}
    return obj
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("{ x: 1, y: 2 }"));
}

#[test]
fn test_member_access() {
    let source = r#"
worker member() {
    var obj = {x: 1}
    var val = obj.x
    return val
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("obj.x"));
}

#[test]
fn test_function_call() {
    let source = r#"
worker caller() {
    var result = foo(1, 2)
    return result
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("foo(1, 2)"));
}

#[test]
fn test_binary_operators() {
    let source = r#"
worker math() {
    var a = 5 + 3
    var b = 10 - 2
    var c = 4 * 2
    var d = 8 / 2
    return d
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("5 + 3"));
    assert!(js.contains("10 - 2"));
    assert!(js.contains("4 * 2"));
    assert!(js.contains("8 / 2"));
}

#[test]
fn test_comparison_operators() {
    let source = r#"
worker compare(x, y) {
    if x == y {
        return true
    }
    if x != y {
        return false
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("x === y"));
    assert!(js.contains("x !== y"));
}

#[test]
fn test_logical_operators() {
    let source = r#"
worker logic(a, b) {
    if a && b {
        return true
    }
    if a || b {
        return false
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("a && b"));
    assert!(js.contains("a || b"));
}

#[test]
fn test_unary_operators() {
    let source = r#"
worker unary(x) {
    var neg = -x
    var not = !x
    return neg
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("-x"));
    assert!(js.contains("!x"));
}

#[test]
fn test_throw_expression() {
    let source = r#"
worker error_test() {
    throw "Something went wrong"
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("throw new Error"));
}

#[test]
fn test_function_declaration() {
    let source = r#"
fun helper(x) {
    return x + 1
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("function helper(x)"));
    assert!(!js.contains("export function helper")); // Not exported
}

#[test]
fn test_exported_function() {
    let source = r#"
export fun helper(x) {
    return x + 1
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("export function helper(x)"));
}

#[test]
fn test_break_statement() {
    let source = r#"
worker break_test(x) {
    while (x > 0) {
        break
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");
    assert!(js.contains("break;"));
}

#[test]
fn test_complex_example() {
    let source = r#"
worker example() {
    var x = 5
    var y = $(echo "hello")
    if x > 3 {
        $ echo "x is big"
    }
}
"#;

    let js = compile_source(source).expect("compilation failed");

    // Verify all expected components
    assert!(js.contains("export function example()"));
    assert!(js.contains("let x = 5"));
    assert!(js.contains("await $shell(`echo hello`, {capture: true})"));
    assert!(js.contains("if (x > 3)"));
    assert!(js.contains("await $shell(`echo x is big`)"));
}
