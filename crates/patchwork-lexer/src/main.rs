use patchwork_lexer::{lex_str, LexerContext};
use std::env;
use std::fs;
use std::io::{self, Read};
use try_next::TryNextWithContext;

fn main() {
    let args: Vec<String> = env::args().collect();

    let input = if args.len() > 1 {
        // Read from file
        let filename = &args[1];
        fs::read_to_string(filename)
            .unwrap_or_else(|e| {
                eprintln!("Error reading file '{}': {}", filename, e);
                std::process::exit(1);
            })
    } else {
        // Read from stdin
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .unwrap_or_else(|e| {
                eprintln!("Error reading stdin: {}", e);
                std::process::exit(1);
            });
        buffer
    };

    // Lex the input
    let mut lexer = match lex_str(&input) {
        Ok(lexer) => lexer,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            std::process::exit(1);
        }
    };

    let mut context = LexerContext::default();

    // Print tokens
    loop {
        match lexer.try_next_with_context(&mut context) {
            Ok(Some(token)) => {
                println!("{:?}", token.rule);
            }
            Ok(None) => break,
            Err(e) => {
                eprintln!("Error during tokenization: {}", e);
                std::process::exit(1);
            }
        }
    }
}
