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

use clap::{Parser, Subcommand, ValueEnum};
use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;
use std::process::{self, Stdio};

use agam_ast::decl::DeclKind;
use agam_errors::{Diagnostic, DiagnosticEmitter, Label, SourceFile, SourceId, Span};
use agam_lexer::{Token, TokenKind};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum Backend {
    Auto,
    C,
    Llvm,
    Jit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum LtoMode {
    Thin,
    Full,
}

#[derive(Clone, Copy, Debug, Default)]
struct FeatureFlags {
    call_cache: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct SourceFeatureFlags {
    call_cache: CallCacheSelection,
    experimental_usages: Vec<ExperimentalFeatureUsage>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CallCacheSelection {
    enable_all: bool,
    optimize_all: bool,
    include_functions: BTreeSet<String>,
    optimize_functions: BTreeSet<String>,
    exclude_functions: BTreeSet<String>,
}

impl CallCacheSelection {
    fn is_enabled(&self) -> bool {
        self.enable_all
            || self.optimize_all
            || !self.include_functions.is_empty()
            || !self.optimize_functions.is_empty()
    }

    fn merge_cli(&self, cli_enabled: bool) -> Self {
        let mut merged = self.clone();
        if cli_enabled {
            merged.enable_all = true;
        }
        merged
    }

    fn included_functions(&self) -> Vec<String> {
        self.include_functions
            .union(&self.optimize_functions)
            .cloned()
            .collect()
    }

    fn excluded_functions(&self) -> Vec<String> {
        self.exclude_functions.iter().cloned().collect()
    }

    fn optimized_functions(&self) -> Vec<String> {
        self.optimize_functions.iter().cloned().collect()
    }

    fn is_optimized_enabled(&self) -> bool {
        self.optimize_all || !self.optimize_functions.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ExperimentalFeature {
    CallCacheOptimize,
}

#[derive(Clone, Copy, Debug)]
struct ExperimentalFeatureSpec {
    code: &'static str,
    annotation: &'static str,
    warning: &'static str,
    help: &'static str,
}

impl ExperimentalFeature {
    fn spec(self) -> ExperimentalFeatureSpec {
        match self {
            ExperimentalFeature::CallCacheOptimize => ExperimentalFeatureSpec {
                code: "W2001",
                annotation: "@experimental.call_cache.optimize",
                warning: "call-cache optimize mode is experimental",
                help: "keep this opt-in local to hot paths; admission and eviction heuristics may change",
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExperimentalFeatureUsage {
    feature: ExperimentalFeature,
    span: Span,
}

#[derive(Clone)]
struct ParsedSource {
    module: agam_ast::Module,
    source_features: SourceFeatureFlags,
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

        /// Use the fastest current native path (equivalent to `-O 3` and auto backend selection)
        #[arg(long)]
        fast: bool,

        /// Code generation backend
        #[arg(long, value_enum, default_value_t = Backend::Auto)]
        backend: Backend,

        /// Enable LLVM link-time optimization
        #[arg(long, value_enum)]
        lto: Option<LtoMode>,

        /// Build an instrumented LLVM binary for profile generation
        #[arg(long, value_name = "DIR")]
        pgo_generate: Option<PathBuf>,

        /// Rebuild with previously collected LLVM profile data
        #[arg(long, value_name = "PROFDATA")]
        pgo_use: Option<PathBuf>,

        /// Enable scalar call-result caching on supported backends
        #[arg(long = "call-cache", alias = "experimental-call-cache", alias = "experimental-jit-call-cache")]
        call_cache: bool,
    },

    /// Build and immediately execute
    Run {
        /// Source file to run
        #[arg(required = true)]
        file: PathBuf,

        /// Code generation backend
        #[arg(long, value_enum, default_value_t = Backend::Auto)]
        backend: Backend,

        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "2")]
        opt_level: u8,

        /// Use the fastest current native path (equivalent to `-O 3` and auto backend selection)
        #[arg(long)]
        fast: bool,

        /// Enable LLVM link-time optimization
        #[arg(long, value_enum)]
        lto: Option<LtoMode>,

        /// Build an instrumented LLVM binary for profile generation
        #[arg(long, value_name = "DIR")]
        pgo_generate: Option<PathBuf>,

        /// Rebuild with previously collected LLVM profile data
        #[arg(long, value_name = "PROFDATA")]
        pgo_use: Option<PathBuf>,

        /// Enable scalar call-result caching on supported backends
        #[arg(long = "call-cache", alias = "experimental-call-cache", alias = "experimental-jit-call-cache")]
        call_cache: bool,

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
            fast,
            backend,
            lto,
            pgo_generate,
            pgo_use,
            call_cache,
        } => {
            let opt_level = effective_opt_level(opt_level, fast);
            let backend = resolve_backend(backend, false);
            let tuning = ReleaseTuning {
                native_cpu: fast,
                lto,
                pgo_generate,
                pgo_use,
            };
            let features = FeatureFlags {
                call_cache,
            };
            if let Err(e) = validate_release_tuning(backend, &tuning) {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
            if cli.verbose {
                eprintln!("[agamc] Building {} file(s)...", files.len());
                if let Some(ref t) = target {
                    eprintln!("[agamc] Target: {}", t);
                }
                eprintln!("[agamc] Optimization level: O{}", opt_level);
                if fast {
                    eprintln!("[agamc] Fast mode enabled (native CPU tuning requested)");
                }
                eprintln!("[agamc] Backend: {:?}", backend);
                if let Some(lto) = tuning.lto {
                    eprintln!("[agamc] LTO: {:?}", lto);
                }
                if let Some(dir) = &tuning.pgo_generate {
                    eprintln!("[agamc] PGO generate: {}", dir.display());
                }
                if let Some(profile) = &tuning.pgo_use {
                    eprintln!("[agamc] PGO use: {}", profile.display());
                }
                if features.call_cache {
                    eprintln!("[agamc] Call cache enabled");
                }
            }

            let out_path = output.unwrap_or_else(|| {
                let stem = files[0].file_stem().unwrap().to_str().unwrap();
                PathBuf::from(format!("{}.exe", stem))
            });

            match build_file(
                &files[0],
                &out_path,
                opt_level,
                backend,
                &tuning,
                features,
                cli.verbose,
            ) {
                Ok(outcome) => {
                    if outcome.native_binary {
                        eprintln!("\x1b[1;32m✓\x1b[0m Built: {}", out_path.display());
                        if outcome.generated_path != out_path {
                            eprintln!(
                                "\x1b[1;32minfo\x1b[0m: Generated IR: {}",
                                outcome.generated_path.display()
                            );
                        }
                    } else {
                        eprintln!(
                            "\x1b[1;32m✓\x1b[0m Generated: {}",
                            outcome.generated_path.display()
                        );
                    }
                }
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        }

        Command::Run {
            file,
            backend,
            opt_level,
            fast,
            lto,
            pgo_generate,
            pgo_use,
            call_cache,
            args,
        } => {
            let opt_level = effective_opt_level(opt_level, fast);
            let backend = resolve_backend(backend, true);
            let tuning = ReleaseTuning {
                native_cpu: fast,
                lto,
                pgo_generate,
                pgo_use,
            };
            let features = FeatureFlags {
                call_cache,
            };
            if let Err(e) = validate_release_tuning(backend, &tuning) {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
            if cli.verbose {
                eprintln!("[agamc] Running {}...", file.display());
                if !args.is_empty() {
                    eprintln!("[agamc] Args: {:?}", args);
                }
                eprintln!("[agamc] Optimization level: O{}", opt_level);
                if fast {
                    eprintln!("[agamc] Fast mode enabled (native CPU tuning requested)");
                }
                eprintln!("[agamc] Backend: {:?}", backend);
                if let Some(lto) = tuning.lto {
                    eprintln!("[agamc] LTO: {:?}", lto);
                }
                if let Some(dir) = &tuning.pgo_generate {
                    eprintln!("[agamc] PGO generate: {}", dir.display());
                }
                if let Some(profile) = &tuning.pgo_use {
                    eprintln!("[agamc] PGO use: {}", profile.display());
                }
                if features.call_cache {
                    eprintln!("[agamc] Call cache enabled");
                }
            }

            let exe_path = file.with_extension("exe");

            if backend == Backend::Jit {
                let mut runtime_args = Vec::with_capacity(args.len() + 1);
                runtime_args.push(file.to_string_lossy().to_string());
                runtime_args.extend(args.clone());
                match run_with_jit(
                    &file,
                    &runtime_args,
                    cli.verbose,
                    features,
                ) {
                    Ok(code) => {
                        if code != 0 {
                            process::exit(code);
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
                return;
            }

            match build_file(
                &file,
                &exe_path,
                opt_level,
                backend,
                &tuning,
                features,
                cli.verbose,
            ) {
                Ok(outcome) => {
                    if !outcome.native_binary {
                        eprintln!(
                            "\x1b[1;31merror\x1b[0m: backend {:?} emitted {} but no native executable was produced",
                            backend,
                            outcome.generated_path.display()
                        );
                        process::exit(1);
                    }

                    // Execute the built binary
                    let status = std::process::Command::new(&exe_path).args(&args).status();

                    match status {
                        Ok(s) => {
                            if !s.success() {
                                process::exit(s.code().unwrap_or(1));
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "\x1b[1;31merror\x1b[0m: failed to run {}: {}",
                                exe_path.display(),
                                e
                            );
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
                "\x1b[1;32minfo\x1b[0m: REPL shell is not implemented yet; the first Cranelift JIT runtime now exists, but interactive evaluation still needs a frontend layer"
            );
        }

        Command::Fmt { files, check } => {
            let files = match expand_fmt_inputs(files) {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            let action = if check { "Checking" } else { "Formatting" };
            if cli.verbose {
                eprintln!("[agamc] {} {} file(s)...", action, files.len());
            }

            let mut changed_files = Vec::new();
            for file in &files {
                let source = match std::fs::read_to_string(file) {
                    Ok(source) => source,
                    Err(e) => {
                        eprintln!(
                            "\x1b[1;31merror\x1b[0m: could not read `{}`: {}",
                            file.display(),
                            e
                        );
                        process::exit(1);
                    }
                };

                let formatted = agam_fmt::format_source(&source);
                if formatted.changed {
                    changed_files.push(file.clone());
                    if !check {
                        if let Err(e) = std::fs::write(file, formatted.output) {
                            eprintln!(
                                "\x1b[1;31merror\x1b[0m: could not write `{}`: {}",
                                file.display(),
                                e
                            );
                            process::exit(1);
                        }
                    }
                }
            }

            if check {
                if changed_files.is_empty() {
                    eprintln!("\x1b[1;32m✓\x1b[0m Formatting is clean.");
                } else {
                    for file in &changed_files {
                        eprintln!("needs formatting: {}", file.display());
                    }
                    process::exit(1);
                }
            } else {
                eprintln!(
                    "\x1b[1;32m✓\x1b[0m Formatted {} file(s).",
                    changed_files.len()
                );
            }
        }

        Command::Test { files, coverage } => {
            eprintln!("[agamc] Running tests in {} file(s)...", files.len());
            if coverage {
                eprintln!("[agamc] Code coverage enabled.");
            }
            eprintln!("\x1b[1;32minfo\x1b[0m: test runner not yet implemented (Phase 89)");
        }
    }
}

fn effective_opt_level(opt_level: u8, fast: bool) -> u8 {
    if fast { 3 } else { opt_level.min(3) }
}

fn resolve_backend(requested: Backend, require_native: bool) -> Backend {
    if requested != Backend::Auto {
        return requested;
    }

    let has_clang = command_exists("clang");
    let has_c = command_exists(default_c_compiler());

    if has_clang {
        Backend::Llvm
    } else if has_c || !require_native {
        Backend::C
    } else {
        Backend::Llvm
    }
}

fn command_exists(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn default_c_compiler() -> &'static str {
    if cfg!(windows) { "gcc" } else { "cc" }
}

fn expand_fmt_inputs(files: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
    let inputs = if files.is_empty() {
        vec![
            std::env::current_dir()
                .map_err(|e| format!("could not read current directory: {}", e))?,
        ]
    } else {
        files
    };

    let mut expanded = Vec::new();
    for input in inputs {
        collect_agam_files(&input, &mut expanded)?;
    }
    expanded.sort();
    expanded.dedup();
    Ok(expanded)
}

fn collect_agam_files(path: &PathBuf, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("agam") {
            out.push(path.clone());
        }
        return Ok(());
    }

    if !path.is_dir() {
        return Err(format!("`{}` is not a file or directory", path.display()));
    }

    for entry in std::fs::read_dir(path)
        .map_err(|e| format!("could not read directory `{}`: {}", path.display(), e))?
    {
        let entry = entry.map_err(|e| format!("could not read directory entry: {}", e))?;
        let child = entry.path();
        if child.is_dir() {
            collect_agam_files(&child, out)?;
        } else if child.extension().and_then(|ext| ext.to_str()) == Some("agam") {
            out.push(child);
        }
    }

    Ok(())
}

fn parse_source_file(path: &PathBuf, verbose: bool) -> Result<ParsedSource, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read `{}`: {}", path.display(), e))?;
    let source_file = SourceFile::new(
        SourceId(0),
        path.to_string_lossy().to_string(),
        source.clone(),
    );
    let mut emitter = DiagnosticEmitter::new();
    emitter.add_source(source_file);

    if verbose {
        eprintln!("[agamc] Read {} ({} bytes)", path.display(), source.len());
    }

    let tokens = agam_lexer::tokenize(&source, SourceId(0));
    if verbose {
        eprintln!("[agamc] Lexed {} tokens", tokens.len());
    }

    let mut source_features = source_feature_flags_from_tokens(&tokens);
    let module = agam_parser::parse(tokens, SourceId(0)).map_err(|errors| {
        for err in &errors {
            eprintln!("\x1b[1;31merror\x1b[0m: {}", err.message);
        }
        format!("{} parse error(s)", errors.len())
    })?;

    if verbose {
        eprintln!(
            "[agamc] Parsed {} top-level declarations",
            module.declarations.len()
        );
    }

    merge_function_call_cache_annotations(&module, &mut source_features.call_cache);
    collect_experimental_function_features(&module, &mut source_features.experimental_usages);
    emit_experimental_feature_warnings(&mut emitter, &source_features.experimental_usages);

    Ok(ParsedSource {
        module,
        source_features,
    })
}

fn source_feature_flags_from_tokens(tokens: &[Token]) -> SourceFeatureFlags {
    let mut features = SourceFeatureFlags::default();
    let mut index = skip_trivia_tokens(tokens, 0);

    while index < tokens.len() {
        let Some(annotation) = parse_annotation_name(tokens, index) else {
            break;
        };
        match annotation.name.as_str() {
            "experimental.call_cache" | "lang.feat.call_cache" => {
                features.call_cache.enable_all = true
            }
            "experimental.no_call_cache" | "lang.feat.no_call_cache" => {
                features.call_cache.enable_all = false;
                features.call_cache.optimize_all = false;
            }
            "experimental.call_cache.optimize" => {
                features.call_cache.enable_all = true;
                features.call_cache.optimize_all = true;
                features.experimental_usages.push(ExperimentalFeatureUsage {
                    feature: ExperimentalFeature::CallCacheOptimize,
                    span: annotation.span,
                });
            }
            "experimental.no_call_cache.optimize" => {
                features.call_cache.optimize_all = false;
            }
            _ => {}
        }
        index = skip_trivia_tokens(tokens, annotation.next_index);
    }

    features
}

fn merge_function_call_cache_annotations(
    module: &agam_ast::Module,
    selection: &mut CallCacheSelection,
) {
    for decl in &module.declarations {
        let DeclKind::Function(function) = &decl.kind else {
            continue;
        };
        for annotation in &function.annotations {
            match annotation.name.name.as_str() {
                "experimental.call_cache" | "lang.feat.call_cache" => {
                    selection
                        .exclude_functions
                        .remove(function.name.name.as_str());
                    selection
                        .include_functions
                        .insert(function.name.name.clone());
                }
                "experimental.call_cache.optimize" => {
                    selection
                        .exclude_functions
                        .remove(function.name.name.as_str());
                    selection
                        .include_functions
                        .insert(function.name.name.clone());
                    selection
                        .optimize_functions
                        .insert(function.name.name.clone());
                }
                "experimental.no_call_cache" | "lang.feat.no_call_cache" => {
                    selection
                        .include_functions
                        .remove(function.name.name.as_str());
                    selection
                        .optimize_functions
                        .remove(function.name.name.as_str());
                    selection
                        .exclude_functions
                        .insert(function.name.name.clone());
                }
                "experimental.no_call_cache.optimize" => {
                    selection
                        .optimize_functions
                        .remove(function.name.name.as_str());
                }
                _ => {}
            }
        }
    }
}

fn collect_experimental_function_features(
    module: &agam_ast::Module,
    usages: &mut Vec<ExperimentalFeatureUsage>,
) {
    for decl in &module.declarations {
        let DeclKind::Function(function) = &decl.kind else {
            continue;
        };
        for annotation in &function.annotations {
            match annotation.name.name.as_str() {
                "experimental.call_cache.optimize" => usages.push(ExperimentalFeatureUsage {
                    feature: ExperimentalFeature::CallCacheOptimize,
                    span: annotation.span,
                }),
                _ => {}
            }
        }
    }
}

fn emit_experimental_feature_warnings(
    emitter: &mut DiagnosticEmitter,
    usages: &[ExperimentalFeatureUsage],
) {
    let mut emitted = HashSet::new();
    for usage in usages {
        if !emitted.insert((usage.feature, usage.span)) {
            continue;
        }
        let spec = usage.feature.spec();
        emitter.emit(
            Diagnostic::warning(spec.code, spec.warning)
                .with_label(Label::primary(
                    usage.span,
                    format!("`{}` is enabled here", spec.annotation),
                ))
                .with_help(spec.help),
        );
    }
}

fn skip_trivia_tokens(tokens: &[Token], mut index: usize) -> usize {
    while let Some(token) = tokens.get(index) {
        match token.kind {
            TokenKind::Newline
            | TokenKind::LineComment
            | TokenKind::BlockComment
            | TokenKind::DocComment => index += 1,
            _ => break,
        }
    }
    index
}

struct ParsedAnnotationName {
    name: String,
    span: Span,
    next_index: usize,
}

fn parse_annotation_name(tokens: &[Token], start: usize) -> Option<ParsedAnnotationName> {
    if tokens.get(start)?.kind != TokenKind::At {
        return None;
    }
    let mut index = start + 1;
    let mut parts = Vec::new();
    let start_span = tokens.get(start)?.span.start;
    let source_id = tokens.get(start)?.span.source_id;
    let mut end_span;

    loop {
        let token = tokens.get(index)?;
        if token.kind != TokenKind::Identifier {
            return None;
        }
        parts.push(token.lexeme.clone());
        end_span = token.span.end;
        index += 1;

        match tokens.get(index).map(|token| token.kind) {
            Some(TokenKind::Dot) => {
                index += 1;
            }
            _ => break,
        }
    }

    Some(ParsedAnnotationName {
        name: parts.join("."),
        span: Span::new(source_id, start_span, end_span),
        next_index: index,
    })
}

/// Read, lex, and parse a source file. Returns Ok(()) if no errors.
fn compile_file(path: &PathBuf, verbose: bool) -> Result<(), String> {
    let _ = parse_source_file(path, verbose)?;
    Ok(())
}

fn lower_to_optimized_mir(
    path: &PathBuf,
    verbose: bool,
) -> Result<(agam_mir::ir::MirModule, SourceFeatureFlags), String> {
    let parsed = parse_source_file(path, verbose)?;

    let mut hir_lowering = agam_hir::lower::HirLowering::new();
    let hir = hir_lowering.lower_module(&parsed.module);

    if verbose {
        eprintln!("[agamc] Lowered to HIR: {} functions", hir.functions.len());
    }

    let mut mir_lowering = agam_mir::lower::MirLowering::new();
    let mut mir = mir_lowering.lower_module(&hir);

    let optimized = agam_mir::opt::optimize_module(&mut mir);

    if verbose {
        eprintln!("[agamc] Lowered to MIR: {} functions", mir.functions.len());
        if optimized {
            eprintln!("[agamc] Applied MIR optimization passes");
        }
    }

    Ok((mir, parsed.source_features))
}

/// Full compilation pipeline: Lex → Parse → HIR → MIR → C → gcc → native binary
struct BuildOutcome {
    native_binary: bool,
    generated_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct ReleaseTuning {
    native_cpu: bool,
    lto: Option<LtoMode>,
    pgo_generate: Option<PathBuf>,
    pgo_use: Option<PathBuf>,
}

fn effective_call_cache_selection(cli: FeatureFlags, source: &SourceFeatureFlags) -> CallCacheSelection {
    source.call_cache.merge_cli(cli.call_cache)
}

/// Full compilation pipeline: Lex → Parse → HIR → MIR → backend emission → native binary (when toolchain exists)
fn build_file(
    path: &PathBuf,
    output: &PathBuf,
    opt_level: u8,
    backend: Backend,
    tuning: &ReleaseTuning,
    features: FeatureFlags,
    verbose: bool,
) -> Result<BuildOutcome, String> {
    let (mir, source_features) = lower_to_optimized_mir(path, verbose)?;
    let call_cache = effective_call_cache_selection(features, &source_features);

    if verbose && call_cache.is_enabled() {
        if call_cache.enable_all {
            eprintln!("[agamc] Call cache enabled for all eligible functions");
        } else {
            eprintln!(
                "[agamc] Call cache enabled for {} annotated function(s)",
                call_cache.included_functions().len()
            );
        }
        if !call_cache.exclude_functions.is_empty() {
            eprintln!(
                "[agamc] Call cache excluded for {} function(s)",
                call_cache.exclude_functions.len()
            );
        }
        if call_cache.is_optimized_enabled() {
            eprintln!("[agamc] Call cache optimize mode enabled");
        }
    }

    match backend {
        Backend::Auto => Err("internal error: unresolved auto backend".into()),
        Backend::C => build_with_c_backend(&mir, output, opt_level, tuning, verbose),
        Backend::Llvm => build_with_llvm_backend(
            &mir,
            output,
            opt_level,
            tuning,
            &call_cache,
            verbose,
        ),
        Backend::Jit => Err("`agamc build --backend jit` is not supported because the JIT executes in memory; use `agamc run --backend jit`".into()),
    }
}

fn build_with_c_backend(
    mir: &agam_mir::ir::MirModule,
    output: &PathBuf,
    opt_level: u8,
    tuning: &ReleaseTuning,
    verbose: bool,
) -> Result<BuildOutcome, String> {
    let c_code = agam_codegen::c_emitter::emit_c(mir);

    let c_path = output.with_extension("c");
    std::fs::write(&c_path, &c_code).map_err(|e| format!("failed to write C file: {}", e))?;

    if verbose {
        eprintln!(
            "[agamc] Generated C code: {} ({} bytes)",
            c_path.display(),
            c_code.len()
        );
    }

    let opt_flag = format!("-O{}", opt_level);
    let native_hint = if tuning.native_cpu {
        " -march=native -mtune=native"
    } else {
        ""
    };
    let compiler = default_c_compiler();

    let mut args = vec![
        c_path.to_string_lossy().into_owned(),
        "-o".into(),
        output.to_string_lossy().into_owned(),
        opt_flag.clone(),
    ];
    if tuning.native_cpu {
        args.push("-march=native".into());
        args.push("-mtune=native".into());
    }
    args.push("-lm".into());

    let result = std::process::Command::new(compiler).args(&args).output();

    match result {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("not recognized") || stderr.contains("not found") {
                    eprintln!(
                        "\x1b[1;33mwarning\x1b[0m: C compiler not found, generated C file: {}",
                        c_path.display()
                    );
                    eprintln!(
                        "\x1b[1;32minfo\x1b[0m: compile manually with: gcc {} -o {} {}{} -lm",
                        c_path.display(),
                        output.display()
                        ,
                        opt_flag,
                        native_hint
                    );
                    return Ok(BuildOutcome {
                        native_binary: false,
                        generated_path: c_path,
                    });
                }
                return Err(format!("C compilation failed:\n{}", stderr));
            }
            let _ = std::fs::remove_file(&c_path);
            Ok(BuildOutcome {
                native_binary: true,
                generated_path: output.clone(),
            })
        }
        Err(_) => {
            eprintln!(
                "\x1b[1;33mwarning\x1b[0m: C compiler not found, generated C file: {}",
                c_path.display()
            );
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: compile manually with: gcc {} -o {} {}{} -lm",
                c_path.display(),
                output.display(),
                opt_flag,
                native_hint
            );
            Ok(BuildOutcome {
                native_binary: false,
                generated_path: c_path,
            })
        }
    }
}

fn build_with_llvm_backend(
    mir: &agam_mir::ir::MirModule,
    output: &PathBuf,
    opt_level: u8,
    tuning: &ReleaseTuning,
    call_cache: &CallCacheSelection,
    verbose: bool,
) -> Result<BuildOutcome, String> {
    let native_hint = if tuning.native_cpu {
        " -march=native -mtune=native"
    } else {
        ""
    };
    let llvm_ir = agam_codegen::llvm_emitter::emit_llvm_with_options(
        mir,
        agam_codegen::llvm_emitter::LlvmEmitOptions {
            call_cache: call_cache.enable_all,
            call_cache_only: call_cache.included_functions(),
            call_cache_exclude: call_cache.excluded_functions(),
            call_cache_optimize: call_cache.optimize_all,
            call_cache_optimize_only: call_cache.optimized_functions(),
            ..agam_codegen::llvm_emitter::LlvmEmitOptions::from_env()
        },
    )?;
    let ll_path = output.with_extension("ll");
    std::fs::write(&ll_path, &llvm_ir)
        .map_err(|e| format!("failed to write LLVM IR file: {}", e))?;

    if verbose {
        eprintln!(
            "[agamc] Generated LLVM IR: {} ({} bytes)",
            ll_path.display(),
            llvm_ir.len()
        );
    }

    let opt_flag = format!("-O{}", opt_level);
    let mut args = vec![
        ll_path.to_string_lossy().into_owned(),
        "-o".into(),
        output.to_string_lossy().into_owned(),
        opt_flag.clone(),
    ];
    if let Some(lto) = tuning.lto {
        args.push(lto_flag(lto).into());
    }
    if let Some(dir) = &tuning.pgo_generate {
        args.push(format!("-fprofile-generate={}", dir.to_string_lossy()));
    }
    if let Some(profile) = &tuning.pgo_use {
        args.push(format!("-fprofile-use={}", profile.to_string_lossy()));
    }
    if tuning.native_cpu {
        args.push("-march=native".into());
        args.push("-mtune=native".into());
    }
    args.push("-lm".into());
    let result = std::process::Command::new("clang").args(&args).output();

    match result {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                if stderr.contains("not recognized") || stderr.contains("not found") {
                    eprintln!(
                        "\x1b[1;33mwarning\x1b[0m: clang not found, generated LLVM IR: {}",
                        ll_path.display()
                    );
                    eprintln!(
                        "\x1b[1;32minfo\x1b[0m: compile manually with: clang {} -o {} {}{} -lm",
                        ll_path.display(),
                        output.display(),
                        opt_flag,
                        native_hint
                    );
                    return Ok(BuildOutcome {
                        native_binary: false,
                        generated_path: ll_path,
                    });
                }
                return Err(format!("LLVM compilation failed:\n{}", stderr));
            }
            Ok(BuildOutcome {
                native_binary: true,
                generated_path: ll_path,
            })
        }
        Err(_) => {
            eprintln!(
                "\x1b[1;33mwarning\x1b[0m: clang not found, generated LLVM IR: {}",
                ll_path.display()
            );
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: compile manually with: clang {} -o {} {}{} -lm",
                ll_path.display(),
                output.display(),
                opt_flag,
                native_hint
            );
            Ok(BuildOutcome {
                native_binary: false,
                generated_path: ll_path,
            })
        }
    }
}

fn run_with_jit(
    path: &PathBuf,
    args: &[String],
    verbose: bool,
    features: FeatureFlags,
) -> Result<i32, String> {
    let (mir, source_features) = lower_to_optimized_mir(path, verbose)?;
    let call_cache = effective_call_cache_selection(features, &source_features);

    if verbose && call_cache.is_enabled() {
        if call_cache.enable_all {
            eprintln!("[agamc] Call cache enabled for all eligible functions");
        } else {
            eprintln!(
                "[agamc] Call cache enabled for {} annotated function(s)",
                call_cache.included_functions().len()
            );
        }
        if call_cache.is_optimized_enabled() {
            eprintln!("[agamc] Call cache optimize mode enabled");
        }
    }
    if verbose {
        eprintln!("[agamc] Executing via Cranelift JIT");
    }
    let result = agam_jit::run_main_with_options(
        &mir,
        args,
        agam_jit::JitOptions {
            call_cache: call_cache.enable_all,
            call_cache_only: call_cache.included_functions(),
            call_cache_exclude: call_cache.excluded_functions(),
            call_cache_optimize: call_cache.optimize_all,
            call_cache_optimize_only: call_cache.optimized_functions(),
            ..Default::default()
        },
    );
    if verbose && call_cache.is_enabled() {
        if let Some(stats) = agam_jit::take_last_call_cache_stats() {
            eprintln!(
                "[agamc] JIT call cache: {} hits / {} calls across {} cacheable function(s), {} store(s)",
                stats.total_hits,
                stats.total_calls,
                stats.functions.len(),
                stats.total_stores
            );
            for function in stats
                .functions
                .iter()
                .filter(|function| function.calls > 0 || function.stores > 0)
            {
                eprintln!(
                    "[agamc]   {} -> calls={}, hits={}, stores={}, entries={}",
                    function.name, function.calls, function.hits, function.stores, function.entries
                );
            }
        }
    }
    result
}

fn validate_release_tuning(backend: Backend, tuning: &ReleaseTuning) -> Result<(), String> {
    let requested_release_tuning =
        tuning.lto.is_some() || tuning.pgo_generate.is_some() || tuning.pgo_use.is_some();
    if !requested_release_tuning {
        return Ok(());
    }
    if backend != Backend::Llvm {
        return Err(
            "Phase 14 release tuning flags (`--lto`, `--pgo-generate`, `--pgo-use`) currently require `--backend llvm`"
                .into(),
        );
    }
    if tuning.pgo_generate.is_some() && tuning.pgo_use.is_some() {
        return Err("use either `--pgo-generate` or `--pgo-use`, not both in one build".into());
    }
    Ok(())
}

fn lto_flag(mode: LtoMode) -> &'static str {
    match mode {
        LtoMode::Thin => "-flto=thin",
        LtoMode::Full => "-flto=full",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_source_features(source: &str) -> SourceFeatureFlags {
        let tokens = agam_lexer::tokenize(source, SourceId(0));
        let mut features = source_feature_flags_from_tokens(&tokens);
        let module = agam_parser::parse(tokens, SourceId(0)).expect("source should parse");
        merge_function_call_cache_annotations(&module, &mut features.call_cache);
        features
    }

    #[test]
    fn test_source_call_cache_can_enable_whole_module_and_opt_out_function() {
        let features = parse_source_features(
            r#"
@lang.advance
@lang.feat.call_cache

fn hot(n: i64) -> i64 { return n + 1; }

@lang.feat.no_call_cache
fn cold(n: i64) -> i64 { return n * 2; }
"#,
        );

        assert!(features.call_cache.enable_all);
        assert!(features.call_cache.exclude_functions.contains("cold"));
        assert!(!features.call_cache.include_functions.contains("cold"));
    }

    #[test]
    fn test_source_call_cache_can_target_specific_function_without_global_enable() {
        let features = parse_source_features(
            r#"
@lang.advance
fn main() -> i32 { if hot(1) > 0 { return 0; } return 1; }

@lang.feat.call_cache
fn hot(n: i64) -> i64 { return n + 1; }
"#,
        );

        assert!(!features.call_cache.enable_all);
        assert!(features.call_cache.include_functions.contains("hot"));
        assert!(features.call_cache.exclude_functions.is_empty());
    }

    #[test]
    fn test_source_call_cache_optimize_marks_experimental_usage() {
        let features = parse_source_features(
            r#"
@lang.advance
@experimental.call_cache.optimize

@experimental.call_cache.optimize
fn hot(n: i64) -> i64 { return n + 1; }
"#,
        );

        assert!(features.call_cache.enable_all);
        assert!(features.call_cache.optimize_all);
        assert!(features.call_cache.optimize_functions.contains("hot"));
        assert_eq!(features.experimental_usages.len(), 2);
    }
}
