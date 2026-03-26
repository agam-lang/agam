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

            let out_path = output
                .unwrap_or_else(|| {
                    let stem = files[0].file_stem().unwrap().to_str().unwrap();
                    PathBuf::from(format!("{}.exe", stem))
                });

            match build_file(&files[0], &out_path, opt_level, cli.verbose) {
                Ok(()) => {
                    eprintln!("\x1b[1;32m✓\x1b[0m Built: {}", out_path.display());
                }
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        }

        Command::Run { file, args } => {
            if cli.verbose {
                eprintln!("[agamc] Running {}...", file.display());
                if !args.is_empty() {
                    eprintln!("[agamc] Args: {:?}", args);
                }
            }

            let stem = file.file_stem().unwrap().to_str().unwrap();
            let exe_path = PathBuf::from(format!("{}.exe", stem));

            match build_file(&file, &exe_path, 2, cli.verbose) {
                Ok(()) => {
                    // Execute the built binary
                    let status = std::process::Command::new(&exe_path)
                        .args(&args)
                        .status();

                    match status {
                        Ok(s) => {
                            if !s.success() {
                                process::exit(s.code().unwrap_or(1));
                            }
                        }
                        Err(e) => {
                            eprintln!("\x1b[1;31merror\x1b[0m: failed to run {}: {}", exe_path.display(), e);
                            process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
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

/// Full compilation pipeline: Lex → Parse → HIR → MIR → C → gcc → native binary
fn build_file(path: &PathBuf, output: &PathBuf, opt_level: u8, verbose: bool) -> Result<(), String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read `{}`: {}", path.display(), e))?;

    if verbose {
        eprintln!("[agamc] Read {} ({} bytes)", path.display(), source.len());
    }

    // === Phase 1: Lexing ===
    let tokens = agam_lexer::tokenize(&source, SourceId(0));
    if verbose {
        eprintln!("[agamc] Lexed {} tokens", tokens.len());
    }

    // === Phase 2: Parsing ===
    let module = agam_parser::parse(tokens, SourceId(0))
        .map_err(|errors| {
            for err in &errors {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", err.message);
            }
            format!("{} parse error(s)", errors.len())
        })?;

    if verbose {
        eprintln!("[agamc] Parsed {} declarations", module.declarations.len());
    }

    // === Phase 3: HIR Lowering ===
    let mut hir_lowering = agam_hir::lower::HirLowering::new();
    let hir = hir_lowering.lower_module(&module);

    if verbose {
        eprintln!("[agamc] Lowered to HIR: {} functions", hir.functions.len());
    }

    // === Phase 4: MIR Lowering ===
    let mut mir_lowering = agam_mir::lower::MirLowering::new();
    let mut mir = mir_lowering.lower_module(&hir);

    let optimized = agam_mir::opt::optimize_module(&mut mir);

    if verbose {
        eprintln!("[agamc] Lowered to MIR: {} functions", mir.functions.len());
        if optimized {
            eprintln!("[agamc] Applied MIR optimization passes");
        }
    }

    // === Phase 5: C Code Generation ===
    let c_code = agam_codegen::c_emitter::emit_c(&mir);

    let c_path = output.with_extension("c");
    std::fs::write(&c_path, &c_code)
        .map_err(|e| format!("failed to write C file: {}", e))?;

    if verbose {
        eprintln!("[agamc] Generated C code: {} ({} bytes)", c_path.display(), c_code.len());
    }

    // === Phase 6: Native Compilation via gcc/clang ===
    let opt_flag = format!("-O{}", opt_level);
    let compiler = if cfg!(windows) { "gcc" } else { "cc" };

    let result = std::process::Command::new(compiler)
        .args(&[
            c_path.to_str().unwrap(),
            "-o", output.to_str().unwrap(),
            &opt_flag,
            "-lm",
        ])
        .output();

    match result {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                // If gcc is not found, fall back to just producing the C file
                if stderr.contains("not recognized") || stderr.contains("not found") {
                    eprintln!("\x1b[1;33mwarning\x1b[0m: C compiler not found, generated C file: {}", c_path.display());
                    eprintln!("\x1b[1;32minfo\x1b[0m: compile manually with: gcc {} -o {} -O2 -lm",
                        c_path.display(), output.display());
                    return Ok(());
                }
                return Err(format!("C compilation failed:\n{}", stderr));
            }
            // Clean up .c file
            let _ = std::fs::remove_file(&c_path);
            Ok(())
        }
        Err(_) => {
            eprintln!("\x1b[1;33mwarning\x1b[0m: C compiler not found, generated C file: {}", c_path.display());
            eprintln!("\x1b[1;32minfo\x1b[0m: compile manually with: gcc {} -o {} -O2 -lm",
                c_path.display(), output.display());
            Ok(())
        }
    }
}
