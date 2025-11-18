/// Patchwork compiler CLI

use std::path::PathBuf;
use std::process;
use std::fs;
use clap::Parser;
use patchwork_compiler::{Compiler, CompileOptions};

#[derive(Parser, Debug)]
#[command(name = "patchworkc")]
#[command(about = "Patchwork compiler - transforms Patchwork source into executable agent systems")]
#[command(version)]
struct Args {
    /// Input Patchwork source file
    #[arg(value_name = "FILE")]
    input: PathBuf,

    /// Output directory for generated files
    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Print the AST and exit (debug mode)
    #[arg(long)]
    dump_ast: bool,
}

fn main() {
    let args = Args::parse();

    // Build compiler options
    let mut options = CompileOptions::new(args.input);

    let output_dir = if let Some(ref output) = args.output {
        options = options.output_dir(output.clone());
        Some(output.clone())
    } else {
        None
    };

    options = options.verbose(args.verbose);

    // Create and run compiler
    let compiler = Compiler::new(options);

    match compiler.compile() {
        Ok(output) => {
            if args.dump_ast {
                // we skip AST dump (no easy way to re-parse)
                // In future we could store the AST in the output
                eprintln!("AST dump not available ");
            } else if let Some(ref dir) = output_dir {
                // Write files to disk
                if let Err(e) = write_output_files(&output, dir, args.verbose) {
                    eprintln!("Failed to write output files: {}", e);
                    process::exit(1);
                }
            } else {
                // Write generated code to stdout (original behavior)
                if args.verbose {
                    println!("Compilation successful!");
                    println!("  Source: {}", output.source_file.display());
                    println!("  Generated {} bytes of JavaScript", output.javascript.len());
                    println!("  Runtime: {} bytes", output.runtime.len());
                    println!("  Prompts: {} templates", output.prompts.len());
                    println!("  Plugin manifest: {} files", output.manifest_files.len());
                    println!("\nGenerated code:");
                }
                println!("{}", output.javascript);

                if args.verbose {
                    println!("\n=== Runtime (patchwork-runtime.js) ===");
                    println!("{}", output.runtime);

                    if !output.prompts.is_empty() {
                        println!("\n=== Prompt Templates ===");
                        for (id, markdown) in &output.prompts {
                            println!("\n--- {} ---", id);
                            println!("{}", markdown);
                        }
                    }

                    if !output.manifest_files.is_empty() {
                        println!("\n=== Plugin Manifest Files ===");
                        for (path, content) in &output.manifest_files {
                            println!("\n--- {} ---", path);
                            println!("{}", content);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Compilation failed: {}", e);
            process::exit(1);
        }
    }
}

fn write_output_files(output: &patchwork_compiler::CompileOutput, output_dir: &PathBuf, verbose: bool) -> std::io::Result<()> {
    // Create output directory
    fs::create_dir_all(output_dir)?;

    // Write main JavaScript module
    let main_js = output_dir.join("index.js");
    fs::write(&main_js, &output.javascript)?;
    if verbose {
        println!("Wrote: {}", main_js.display());
    }

    // Write runtime
    let runtime_js = output_dir.join("patchwork-runtime.js");
    fs::write(&runtime_js, &output.runtime)?;
    if verbose {
        println!("Wrote: {}", runtime_js.display());
    }

    // Write code process init script
    let code_init_js = output_dir.join("code-process-init.js");
    fs::write(&code_init_js, &output.code_process_init)?;
    if verbose {
        println!("Wrote: {}", code_init_js.display());
    }

    // Write prompt templates
    // These are think/ask blocks compiled to skill documents
    // Keys are already full paths like "skills/{name}/SKILL.md"
    for (path, markdown) in &output.prompts {
        let full_path = output_dir.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, markdown)?;
        if verbose {
            println!("Wrote: {}", full_path.display());
        }
    }

    // Write plugin manifest files (includes skills, agents, etc.)
    for (path, content) in &output.manifest_files {
        let full_path = output_dir.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&full_path, content)?;
        if verbose {
            println!("Wrote: {}", full_path.display());
        }
    }

    if verbose {
        println!("\nCompilation successful! Output written to: {}", output_dir.display());
        println!("  Main code: index.js");
        println!("  Runtime: patchwork-runtime.js");
        println!("  Init script: code-process-init.js");
        println!("  Prompts: {} skill documents", output.prompts.len());
        println!("  Manifest: {} files", output.manifest_files.len());
    } else {
        println!("Compilation successful! Output written to: {}", output_dir.display());
    }

    Ok(())
}
