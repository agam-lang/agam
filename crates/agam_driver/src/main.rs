//! # agamc — The Agam Compiler
//!
//! Entry point for the Agam programming language toolchain.
//!
//! ## Subcommands
//!
//! - `build` — Compile source files to a native binary
//! - `run`   — Build and immediately execute
//! - `check` — Type-check without generating code (fast)
//! - `repl`  — Interactive REPL
//! - `fmt`   — Format source files
//! - `test`  — Run tests

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;

use agam_errors::{DiagnosticEmitter, SourceFile, SourceId};

/// The Agam programming language compiler.
#[derive(Parser, Debug)]
#[command(
    name = "agamc",
    version,
    about = "The Agam programming language compiler",
    long_about = "Agam — A natively compiled omni-language unifying Python's simplicity\nwith C++'s raw hardware control and Rust's memory safety."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compile source files to a native binary
    Build {
        /// Source file(s) to compile
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Target triple (e.g., x86_64-linux-gnu, wasm32-wasi)
        #[arg(long)]
        target: Option<String>,

        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "0")]
        opt_level: u8,
    },

    /// Build and immediately execute
    Run {
        /// Source file to run
        #[arg(required = true)]
        file: PathBuf,

        /// Arguments passed to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Type-check without generating code (fast feedback)
    Check {
        /// Source file(s) to check
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },

    /// Start the interactive REPL
    Repl,

    /// Format source files
    Fmt {
        /// Source file(s) to format (defaults to current directory)
        files: Vec<PathBuf>,

        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
    },

    /// Run tests
    Test {
        /// Source file(s) containing tests
        files: Vec<PathBuf>,

        /// Enable code coverage
        #[arg(long)]
        coverage: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Build {
            files,
            output,
            target,
            opt_level,
        } => {
            if cli.verbose {
                eprintln!("[agamc] Building {} file(s)...", files.len());
                if let Some(ref t) = target {
                    eprintln!("[agamc] Target: {}", t);
                }
                eprintln!("[agamc] Optimization level: O{}", opt_level);
            }

            for file in &files {
                if let Err(e) = compile_file(file, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }

            let out = output
                .unwrap_or_else(|| {
                    let stem = files[0].file_stem().unwrap().to_str().unwrap();
                    PathBuf::from(stem)
                });

            if cli.verbose {
                eprintln!("[agamc] Output: {}", out.display());
            }

            eprintln!(
                "\x1b[1;32minfo\x1b[0m: compilation pipeline not yet implemented (Phase 43+)"
            );
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: lexer and parser are functional — use `agamc check` to validate syntax"
            );
        }

        Command::Run { file, args } => {
            if cli.verbose {
                eprintln!("[agamc] Running {}...", file.display());
                if !args.is_empty() {
                    eprintln!("[agamc] Args: {:?}", args);
                }
            }
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: `run` subcommand not yet implemented (requires codegen, Phase 43+)"
            );
        }

        Command::Check { files } => {
            if cli.verbose {
                eprintln!("[agamc] Checking {} file(s)...", files.len());
            }

            let mut had_errors = false;
            for file in &files {
                match compile_file(file, cli.verbose) {
                    Ok(()) => {
                        if cli.verbose {
                            eprintln!("[agamc] {} — OK", file.display());
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        had_errors = true;
                    }
                }
            }

            if had_errors {
                process::exit(1);
            } else {
                eprintln!("\x1b[1;32m✓\x1b[0m All checks passed.");
            }
        }

        Command::Repl => {
            println!("Agam REPL v0.1.0");
            println!("Type :help for help, :quit to exit.");
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: REPL not yet implemented (requires JIT, Phase 73+)"
            );
        }

        Command::Fmt { files, check } => {
            let action = if check { "Checking" } else { "Formatting" };
            eprintln!("[agamc] {} {} file(s)...", action, files.len());
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: formatter not yet implemented (Phase 81)"
            );
        }

        Command::Test { files, coverage } => {
            eprintln!("[agamc] Running tests in {} file(s)...", files.len());
            if coverage {
                eprintln!("[agamc] Code coverage enabled.");
            }
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: test runner not yet implemented (Phase 89)"
            );
        }
    }
}

/// Read, lex, and parse a source file. Returns Ok(()) if no errors.
fn compile_file(path: &PathBuf, verbose: bool) -> Result<(), String> {
    // Read the source file
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read `{}`: {}", path.display(), e))?;

    if verbose {
        eprintln!(
            "[agamc] Read {} ({} bytes)",
            path.display(),
            source.len()
        );
    }

    // Create source file for diagnostics
    let source_file = SourceFile::new(
        SourceId(0),
        path.to_string_lossy().to_string(),
        source.clone(),
    );

    let mut emitter = DiagnosticEmitter::new();
    emitter.add_source(source_file);

    // === Lexing Phase ===
    let tokens = agam_lexer::tokenize(&source, SourceId(0));

    if verbose {
        eprintln!("[agamc] Lexed {} tokens", tokens.len());
    }

    // === Parsing Phase ===
    match agam_parser::parse(tokens, SourceId(0)) {
        Ok(module) => {
            if verbose {
                eprintln!("[agamc] Parsed {} top-level declarations", module.declarations.len());
            }
        }
        Err(errors) => {
            for err in &errors {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", err.message);
            }
            return Err(format!("{} parse error(s)", errors.len()));
        }
    }

    if emitter.has_errors() {
        emitter.print_summary();
        Err(format!("{} error(s) found", emitter.error_count()))
    } else {
        Ok(())
    }
}

