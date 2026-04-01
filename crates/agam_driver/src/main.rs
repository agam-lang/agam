//! # agamc — The Agam Compiler
//!
//! Entry point for the Agam programming language toolchain.
//!
//! ## Subcommands
//!
//! - `build` — Compile source files to a native binary
//! - `run`   — Build and immediately execute
//! - `package` — Build, inspect, and run portable packages
//! - `check` — Type-check without generating code (fast)
//! - `new`   — Scaffold a first-party Agam project
//! - `dev`   — Run the first-party local development workflow
//! - `cache` — Inspect the local Agam build/package cache
//! - `repl`  — Interactive REPL
//! - `fmt`   — Format source files
//! - `lsp`   — Start the Language Server Protocol server
//! - `test`  — Run tests

use clap::{Parser, Subcommand, ValueEnum};
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
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
    disable_all: bool,
    enable_all: bool,
    optimize_all: bool,
    include_functions: BTreeSet<String>,
    optimize_functions: BTreeSet<String>,
    exclude_functions: BTreeSet<String>,
}

impl CallCacheSelection {
    fn is_enabled(&self) -> bool {
        self.resolved_enable_all()
            || self.optimize_all
            || !self.include_functions.is_empty()
            || !self.optimize_functions.is_empty()
    }

    fn resolved_enable_all(&self) -> bool {
        self.enable_all || !self.disable_all
    }

    fn merge_cli(&self, cli_enabled: bool) -> Self {
        let mut merged = self.clone();
        if cli_enabled {
            merged.disable_all = false;
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
    source: String,
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
        #[arg(
            long = "call-cache",
            alias = "experimental-call-cache",
            alias = "experimental-jit-call-cache"
        )]
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
        #[arg(
            long = "call-cache",
            alias = "experimental-call-cache",
            alias = "experimental-jit-call-cache"
        )]
        call_cache: bool,

        /// Arguments passed to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Build, inspect, and run portable packages
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },

    /// Inspect native backend and SDK readiness on the current machine
    Doctor,

    /// Type-check without generating code (fast feedback)
    Check {
        /// Source file(s) to check
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },

    /// Scaffold a new first-party Agam project layout
    New {
        /// Project directory to create
        #[arg(required = true)]
        path: PathBuf,

        /// Allow creating the layout inside an existing empty directory
        #[arg(long)]
        force: bool,
    },

    /// Run the first-party development workflow for a project or source file
    Dev {
        /// Project directory, manifest path, or source file (defaults to current directory)
        path: Option<PathBuf>,

        /// Code generation backend used for the final run step
        #[arg(long, value_enum, default_value_t = Backend::Auto)]
        backend: Backend,

        /// Optimization level used for the final run step
        #[arg(short = 'O', long, default_value = "3")]
        opt_level: u8,

        /// Apply formatting fixes before checking
        #[arg(long)]
        fix: bool,

        /// Skip the final `run` step after checks pass
        #[arg(long)]
        no_run: bool,

        /// Skip Agam test discovery and execution
        #[arg(long)]
        no_tests: bool,
    },

    /// Inspect the local Agam cache
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
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

    /// Start the Language Server Protocol server over stdio
    Lsp,

    /// Run tests
    Test {
        /// Source file(s) containing tests
        files: Vec<PathBuf>,

        /// Enable code coverage
        #[arg(long)]
        coverage: bool,
    },
}

#[derive(Subcommand, Debug)]
enum PackageCommand {
    /// Build a portable package from Agam source
    Pack {
        /// Source file to package
        #[arg(required = true)]
        file: PathBuf,

        /// Output package path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Inspect a portable package manifest
    Inspect {
        /// Package file to inspect
        #[arg(required = true)]
        file: PathBuf,
    },

    /// Run a portable package through the runtime/JIT path
    Run {
        /// Package file to execute
        #[arg(required = true)]
        file: PathBuf,

        /// Arguments passed to the packaged program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Assemble a host-native Agam SDK distribution layout
    Sdk {
        /// Output directory for the SDK distribution
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Optional bundled LLVM root to copy into the SDK
        #[arg(long, value_name = "DIR")]
        llvm_bundle: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum CacheCommand {
    /// Print aggregate cache statistics and recent entries
    Status {
        /// Workspace path, manifest path, or source path used to locate the cache
        path: Option<PathBuf>,

        /// Number of recent entries to show
        #[arg(long, default_value = "5")]
        recent: usize,
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
                target: target.clone(),
                native_cpu: fast,
                lto,
                pgo_generate,
                pgo_use,
            };
            let features = FeatureFlags { call_cache };
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
                default_native_binary_output_path(&files[0], tuning.target.as_deref())
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
                target: None,
                native_cpu: fast,
                lto,
                pgo_generate,
                pgo_use,
            };
            let features = FeatureFlags { call_cache };
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

            match run_source_file(
                &file,
                &args,
                backend,
                opt_level,
                &tuning,
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
        }

        Command::Package { command } => match command {
            PackageCommand::Pack { file, output } => {
                let output = output.unwrap_or_else(|| agam_pkg::default_package_path(&file));
                match build_portable_package_file(&file, cli.verbose) {
                    Ok(package) => {
                        if let Err(e) =
                            write_portable_package_with_cache(&file, &output, &package, cli.verbose)
                        {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                            process::exit(1);
                        }
                        eprintln!("\x1b[1;32m✓\x1b[0m Packaged: {}", output.display());
                        if cli.verbose {
                            eprintln!(
                                "[agamc] Package functions: {}",
                                package.manifest.verified_ir.function_count
                            );
                            eprintln!("[agamc] Runtime ABI: v{}", package.runtime.abi.version);
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
            PackageCommand::Inspect { file } => match agam_pkg::read_package_from_path(&file) {
                Ok(package) => print_package_summary(&package),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            PackageCommand::Run { file, args } => {
                match run_portable_package_file(&file, &args, cli.verbose) {
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
            }
            PackageCommand::Sdk {
                output,
                llvm_bundle,
            } => {
                let output = output.unwrap_or_else(default_sdk_distribution_output_dir);
                match package_sdk_distribution(&output, llvm_bundle.as_ref(), cli.verbose) {
                    Ok(outcome) => {
                        eprintln!("\x1b[1;32m✓\x1b[0m SDK staged: {}", outcome.root.display());
                        eprintln!(
                            "\x1b[1;32minfo\x1b[0m: compiler -> {}",
                            outcome.compiler_binary.display()
                        );
                        eprintln!(
                            "\x1b[1;32minfo\x1b[0m: manifest -> {}",
                            outcome.manifest_path.display()
                        );
                        if let Some(bundle_root) = outcome.llvm_bundle_root.as_ref() {
                            eprintln!(
                                "\x1b[1;32minfo\x1b[0m: llvm bundle -> {}",
                                bundle_root.display()
                            );
                        } else {
                            eprintln!(
                                "\x1b[1;33mwarning\x1b[0m: staged SDK does not yet include a bundled LLVM toolchain"
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
        },

        Command::Doctor => match run_doctor(cli.verbose) {
            Ok(healthy) => {
                if !healthy {
                    process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
        },

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

        Command::New { path, force } => match scaffold_project_layout(&path, force, cli.verbose) {
            Ok(layout) => {
                eprintln!(
                    "\x1b[1;32m✓\x1b[0m Created Agam project: {}",
                    layout.root.display()
                );
                eprintln!(
                    "\x1b[1;32minfo\x1b[0m: manifest -> {}",
                    layout.manifest_path.display()
                );
                eprintln!(
                    "\x1b[1;32minfo\x1b[0m: entry -> {}",
                    layout.entry_file.display()
                );
            }
            Err(e) => {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
        },

        Command::Dev {
            path,
            backend,
            opt_level,
            fix,
            no_run,
            no_tests,
        } => {
            if let Err(e) =
                run_dev_workflow(path, backend, opt_level, fix, no_run, no_tests, cli.verbose)
            {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
        }

        Command::Cache { command } => match command {
            CacheCommand::Status { path, recent } => {
                if let Err(e) = print_cache_status(path, recent, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        },

        Command::Repl => {
            println!("Agam REPL v0.1.0");
            println!("Type :help for help, :quit to exit.");
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: REPL shell is not implemented yet; the first Cranelift JIT runtime now exists, but interactive evaluation still needs a frontend layer"
            );
        }

        Command::Fmt { files, check } => {
            let files = match expand_agam_inputs(files) {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            let changed_files = match run_format_paths(&files, check, cli.verbose) {
                Ok(changed_files) => changed_files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

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

        Command::Lsp => {
            if let Err(e) = agam_lsp::run_stdio() {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
        }

        Command::Test { files, coverage } => {
            let files = match expand_agam_inputs(files) {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            if coverage {
                eprintln!(
                    "\x1b[1;33mwarning\x1b[0m: coverage reporting is not implemented yet; running tests without coverage"
                );
            }

            if cli.verbose {
                eprintln!("[agamc] Running tests in {} file(s)...", files.len());
            }

            let totals = match run_agam_tests(&files, cli.verbose) {
                Ok(totals) => totals,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            if totals.failed > 0 {
                eprintln!(
                    "\nresult: \x1b[1;31mFAILED\x1b[0m. {} passed; {} failed.",
                    totals.passed, totals.failed
                );
                process::exit(1);
            } else if totals.total == 0 {
                eprintln!("\x1b[1;33minfo\x1b[0m: no tests found.");
            } else {
                eprintln!(
                    "\nresult: \x1b[1;32mok\x1b[0m. {} passed; 0 failed.",
                    totals.passed
                );
            }
        }
    }
}

fn effective_opt_level(opt_level: u8, fast: bool) -> u8 {
    if fast { 3 } else { opt_level.min(3) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LlvmToolchain {
    Native,
    Wsl,
}

const DEV_WSL_LLVM_ENV: &str = "AGAM_DEV_USE_WSL_LLVM";
const LLVM_CLANG_ENV: &str = "AGAM_LLVM_CLANG";
const LLVM_BUNDLE_DIR_ENV: &str = "AGAM_LLVM_BUNDLE_DIR";
const LLVM_SYSROOT_ENV: &str = "AGAM_LLVM_SYSROOT";
const LLVM_SDKROOT_ENV: &str = "AGAM_LLVM_SDKROOT";
const BUILD_CACHE_SIGNATURE_VERSION: &str = "compiler-build-v2";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LlvmTargetPlatform {
    Windows,
    Linux,
    MacOs,
    Ios,
    Android,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LlvmTargetConfig {
    target_triple: Option<String>,
    platform: LlvmTargetPlatform,
    sysroot: Option<PathBuf>,
    sdk_root: Option<PathBuf>,
}

fn resolve_backend(requested: Backend, require_native: bool) -> Backend {
    let allow_dev_wsl_llvm = allow_dev_wsl_llvm();
    resolve_backend_with_toolchains(
        requested,
        require_native,
        resolve_native_llvm_toolchain().is_some(),
        wsl_command_exists("clang"),
        allow_dev_wsl_llvm,
        command_exists(default_c_compiler()),
    )
}

fn default_native_binary_output_path(source: &Path, target: Option<&str>) -> PathBuf {
    let stem = source
        .file_stem()
        .map(|stem| stem.to_os_string())
        .unwrap_or_else(|| "a.out".into());
    let mut output = source.with_file_name(stem);
    if native_binary_extension(target) == Some("exe") {
        output.set_extension("exe");
    }
    output
}

fn native_binary_extension(target: Option<&str>) -> Option<&'static str> {
    match classify_llvm_target_platform(target) {
        LlvmTargetPlatform::Windows => Some("exe"),
        _ => None,
    }
}

fn classify_llvm_target_platform(target: Option<&str>) -> LlvmTargetPlatform {
    if let Some(target) = target {
        let target = target.trim().to_ascii_lowercase();
        if target.is_empty() {
            return host_llvm_target_platform();
        }
        if target.contains("android") {
            return LlvmTargetPlatform::Android;
        }
        if target.contains("apple-ios")
            || target.ends_with("-ios")
            || target.contains("-ios-")
            || target.contains("iphoneos")
        {
            return LlvmTargetPlatform::Ios;
        }
        if target.contains("apple-darwin") || target.contains("macos") || target.contains("darwin")
        {
            return LlvmTargetPlatform::MacOs;
        }
        if target.contains("windows") || target.contains("mingw") || target.contains("msvc") {
            return LlvmTargetPlatform::Windows;
        }
        if target.contains("linux") {
            return LlvmTargetPlatform::Linux;
        }
        return LlvmTargetPlatform::Unknown;
    }
    host_llvm_target_platform()
}

fn host_llvm_target_platform() -> LlvmTargetPlatform {
    if cfg!(windows) {
        LlvmTargetPlatform::Windows
    } else if cfg!(target_os = "macos") {
        LlvmTargetPlatform::MacOs
    } else if cfg!(target_os = "linux") {
        LlvmTargetPlatform::Linux
    } else {
        LlvmTargetPlatform::Unknown
    }
}

fn configured_llvm_clang_override() -> Option<String> {
    std::env::var(LLVM_CLANG_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn llvm_driver_file_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["clang.exe", "clang++.exe"]
    } else {
        &["clang", "clang++"]
    }
}

fn bundled_llvm_platform_dir() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x86_64",
        ("windows", "aarch64") => "windows-aarch64",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("macos", "aarch64") => "darwin-aarch64",
        _ => "unknown",
    }
}

fn bundled_llvm_candidate_paths(root: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for driver in llvm_driver_file_names() {
        candidates.push(root.join(driver));
        candidates.push(root.join("bin").join(driver));
        candidates.push(
            root.join(bundled_llvm_platform_dir())
                .join("bin")
                .join(driver),
        );
        candidates.push(root.join("llvm").join("bin").join(driver));
        candidates.push(
            root.join("llvm")
                .join(bundled_llvm_platform_dir())
                .join("bin")
                .join(driver),
        );
        candidates.push(
            root.join("toolchains")
                .join("llvm")
                .join("bin")
                .join(driver),
        );
        candidates.push(
            root.join("toolchains")
                .join("llvm")
                .join(bundled_llvm_platform_dir())
                .join("bin")
                .join(driver),
        );
    }
    candidates
}

fn discover_bundled_llvm_clang() -> Option<String> {
    let mut roots = Vec::new();
    if let Some(explicit_root) = env_path(LLVM_BUNDLE_DIR_ENV) {
        roots.push(explicit_root);
    }
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(exe_dir) = current_exe.parent() {
            roots.push(exe_dir.to_path_buf());
            if let Some(parent) = exe_dir.parent() {
                roots.push(parent.to_path_buf());
            }
        }
    }

    let mut seen = BTreeSet::new();
    for root in roots {
        let rendered = root.to_string_lossy().to_string();
        if !seen.insert(rendered) {
            continue;
        }
        if let Some(candidate) = bundled_llvm_candidate_paths(&root)
            .into_iter()
            .find(|path| path.is_file())
        {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn windows_vswhere_path() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    env_path("ProgramFiles(x86)").map(|root| {
        root.join("Microsoft Visual Studio")
            .join("Installer")
            .join("vswhere.exe")
    })
}

fn discover_visual_studio_installation_path() -> Option<PathBuf> {
    let vswhere = windows_vswhere_path()?;
    if !vswhere.is_file() {
        return None;
    }
    let output = std::process::Command::new(vswhere)
        .args(["-latest", "-products", "*", "-property", "installationPath"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!path.is_empty()).then_some(PathBuf::from(path))
}

fn visual_studio_llvm_candidate_paths(install_root: &Path) -> Vec<PathBuf> {
    vec![
        install_root
            .join("VC")
            .join("Tools")
            .join("Llvm")
            .join("x64")
            .join("bin")
            .join("clang.exe"),
        install_root
            .join("VC")
            .join("Tools")
            .join("Llvm")
            .join("bin")
            .join("clang.exe"),
        install_root
            .join("VC")
            .join("Tools")
            .join("Llvm")
            .join("arm64")
            .join("bin")
            .join("clang.exe"),
    ]
}

fn standalone_windows_llvm_install_roots() -> Vec<PathBuf> {
    if !cfg!(windows) {
        return Vec::new();
    }

    let mut roots = Vec::new();
    let mut seen = BTreeSet::new();
    for env_name in ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(base) = env_path(env_name) {
            let candidate = base.join("LLVM");
            let rendered = candidate.to_string_lossy().to_string();
            if seen.insert(rendered) {
                roots.push(candidate);
            }
        }
    }
    roots
}

fn standalone_windows_llvm_candidate_paths(install_root: &Path) -> Vec<PathBuf> {
    llvm_driver_file_names()
        .iter()
        .map(|driver| install_root.join("bin").join(driver))
        .collect()
}

fn discover_standalone_windows_llvm_clang() -> Option<String> {
    if !cfg!(windows) {
        return None;
    }

    standalone_windows_llvm_install_roots()
        .into_iter()
        .flat_map(|root| standalone_windows_llvm_candidate_paths(&root))
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

fn discover_visual_studio_llvm_clang() -> Option<String> {
    let install_root = discover_visual_studio_installation_path()?;
    visual_studio_llvm_candidate_paths(&install_root)
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

fn native_llvm_clang_candidates() -> Vec<String> {
    if let Some(explicit) = configured_llvm_clang_override() {
        return vec![explicit];
    }

    let mut candidates = Vec::new();
    if let Some(bundled) = discover_bundled_llvm_clang() {
        candidates.push(bundled);
    }
    if let Some(vs_clang) = discover_visual_studio_llvm_clang() {
        if !candidates.iter().any(|candidate| candidate == &vs_clang) {
            candidates.push(vs_clang);
        }
    }
    if let Some(standalone_clang) = discover_standalone_windows_llvm_clang() {
        if !candidates
            .iter()
            .any(|candidate| candidate == &standalone_clang)
        {
            candidates.push(standalone_clang);
        }
    }
    for path_candidate in ["clang", "clang++"] {
        if !candidates
            .iter()
            .any(|candidate| candidate == path_candidate)
        {
            candidates.push(path_candidate.into());
        }
    }
    candidates
}

fn resolve_native_llvm_command() -> Option<String> {
    native_llvm_clang_candidates()
        .into_iter()
        .find(|candidate| command_exists(candidate))
}

fn configured_llvm_clang() -> String {
    resolve_native_llvm_command()
        .or_else(configured_llvm_clang_override)
        .unwrap_or_else(|| "clang".into())
}

fn windows_native_llvm_install_hint() -> Option<String> {
    if !cfg!(windows) {
        return None;
    }
    let base = if let Some(install_root) = discover_visual_studio_installation_path() {
        format!(
            "install the LLVM/Clang tools in Visual Studio Installer for `{}`",
            install_root.display()
        )
    } else if !standalone_windows_llvm_install_roots().is_empty() {
        "repair or reinstall the official LLVM toolchain under `C:\\Program Files\\LLVM`".into()
    } else {
        "install a native Windows LLVM/Clang toolchain (for example through Visual Studio Installer or the official LLVM installer)".into()
    };
    Some(format!(
        "ship a bundled LLVM toolchain next to `agamc` under `toolchains/llvm/{}/bin`, {base}, or set `{}` / `{}` explicitly",
        bundled_llvm_platform_dir(),
        LLVM_BUNDLE_DIR_ENV,
        LLVM_CLANG_ENV
    ))
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var(name)
        .ok()
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn android_ndk_host_tag() -> Option<&'static str> {
    if cfg!(windows) {
        Some("windows-x86_64")
    } else if cfg!(target_os = "linux") {
        Some("linux-x86_64")
    } else if cfg!(target_os = "macos") {
        match std::env::consts::ARCH {
            "aarch64" => Some("darwin-arm64"),
            "x86_64" => Some("darwin-x86_64"),
            _ => None,
        }
    } else {
        None
    }
}

fn resolve_android_ndk_sysroot() -> Option<PathBuf> {
    let ndk_root = env_path("ANDROID_NDK_HOME").or_else(|| env_path("ANDROID_NDK_ROOT"))?;
    let host_tag = android_ndk_host_tag()?;
    let sysroot = ndk_root
        .join("toolchains")
        .join("llvm")
        .join("prebuilt")
        .join(host_tag)
        .join("sysroot");
    sysroot.exists().then_some(sysroot)
}

fn resolve_llvm_target_config(tuning: &ReleaseTuning) -> LlvmTargetConfig {
    let target_triple = tuning
        .target
        .clone()
        .or_else(|| {
            std::env::var("AGAM_LLVM_TARGET_TRIPLE")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .map(|value| value.trim().to_string());
    let platform = classify_llvm_target_platform(target_triple.as_deref());
    let sysroot = env_path(LLVM_SYSROOT_ENV).or_else(|| {
        if platform == LlvmTargetPlatform::Android {
            resolve_android_ndk_sysroot()
        } else {
            None
        }
    });
    let sdk_root = env_path(LLVM_SDKROOT_ENV).or_else(|| env_path("SDKROOT"));
    LlvmTargetConfig {
        target_triple,
        platform,
        sysroot,
        sdk_root,
    }
}

fn resolve_backend_with_toolchains(
    requested: Backend,
    require_native: bool,
    has_native_clang: bool,
    has_wsl_clang: bool,
    allow_dev_wsl_llvm: bool,
    has_c: bool,
) -> Backend {
    if requested != Backend::Auto {
        return requested;
    }

    let has_run_clang = has_native_clang || (allow_dev_wsl_llvm && has_wsl_clang);
    if require_native {
        if has_run_clang {
            Backend::Llvm
        } else if has_c {
            Backend::C
        } else {
            Backend::Jit
        }
    } else if has_native_clang {
        Backend::Llvm
    } else if has_c {
        Backend::C
    } else {
        Backend::C
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

fn allow_dev_wsl_llvm() -> bool {
    cfg!(windows) && env_flag_enabled(DEV_WSL_LLVM_ENV)
}

fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn wsl_command_exists(command: &str) -> bool {
    if !cfg!(windows) {
        return false;
    }
    std::process::Command::new("wsl")
        .args([
            "bash",
            "-lc",
            &format!("command -v {command} >/dev/null 2>&1"),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn resolve_native_llvm_toolchain() -> Option<LlvmToolchain> {
    if resolve_native_llvm_command().is_some() {
        Some(LlvmToolchain::Native)
    } else {
        None
    }
}

fn resolve_llvm_run_toolchain() -> Option<LlvmToolchain> {
    resolve_llvm_run_toolchain_with_opt_in(allow_dev_wsl_llvm())
}

fn resolve_llvm_run_toolchain_with_opt_in(allow_dev_wsl_llvm: bool) -> Option<LlvmToolchain> {
    if let Some(native) = resolve_native_llvm_toolchain() {
        Some(native)
    } else if allow_dev_wsl_llvm && wsl_command_exists("clang") {
        Some(LlvmToolchain::Wsl)
    } else {
        None
    }
}

fn llvm_math_link_required(platform: LlvmTargetPlatform) -> bool {
    !matches!(platform, LlvmTargetPlatform::Windows)
}

fn build_native_llvm_clang_args(
    ll_path: &Path,
    output: &Path,
    opt_level: u8,
    tuning: &ReleaseTuning,
    target_config: &LlvmTargetConfig,
) -> Vec<String> {
    let mut args = vec![
        ll_path.to_string_lossy().into_owned(),
        "-o".into(),
        output.to_string_lossy().into_owned(),
        format!("-O{}", opt_level),
    ];

    if let Some(target) = target_config.target_triple.as_ref() {
        args.push(format!("--target={target}"));
    }
    if let Some(sysroot) = target_config.sysroot.as_ref() {
        args.push(format!("--sysroot={}", sysroot.to_string_lossy()));
    }
    if let Some(sdk_root) = target_config.sdk_root.as_ref() {
        if matches!(
            target_config.platform,
            LlvmTargetPlatform::MacOs | LlvmTargetPlatform::Ios
        ) {
            args.push("-isysroot".into());
            args.push(sdk_root.to_string_lossy().into_owned());
        }
    }
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
    if llvm_math_link_required(target_config.platform) {
        args.push("-lm".into());
    }

    args
}

fn render_shellish_command(command: &str, args: &[String]) -> String {
    let rendered_args = args
        .iter()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{arg}\"")
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{command} {rendered_args}")
}

fn validate_llvm_target_config(tuning: &ReleaseTuning) -> Result<(), String> {
    let target_config = resolve_llvm_target_config(tuning);
    if tuning.native_cpu && target_config.target_triple.is_some() {
        return Err(
            "`--fast`/native CPU tuning is only valid for host-native LLVM builds; remove `--fast` when using `--target`"
                .into(),
        );
    }
    match target_config.platform {
        LlvmTargetPlatform::Android if target_config.target_triple.is_some() => {
            if target_config.sysroot.is_none() {
                return Err(format!(
                    "Android LLVM targets require a sysroot; set `{LLVM_SYSROOT_ENV}` or `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT`"
                ));
            }
        }
        LlvmTargetPlatform::Ios if target_config.target_triple.is_some() => {
            if target_config.sdk_root.is_none() {
                return Err(format!(
                    "iOS LLVM targets require an Apple SDK root; set `{LLVM_SDKROOT_ENV}` or `SDKROOT`"
                ));
            }
        }
        _ => {}
    }
    Ok(())
}

fn default_c_compiler() -> &'static str {
    if cfg!(windows) { "gcc" } else { "cc" }
}

fn expand_agam_inputs(files: Vec<PathBuf>) -> Result<Vec<PathBuf>, String> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceLayout {
    root: PathBuf,
    manifest_path: Option<PathBuf>,
    entry_file: PathBuf,
    source_files: Vec<PathBuf>,
    test_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectScaffold {
    root: PathBuf,
    manifest_path: PathBuf,
    entry_file: PathBuf,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TestRunTotals {
    total: usize,
    passed: usize,
    failed: usize,
}

fn scaffold_project_layout(
    path: &Path,
    force: bool,
    verbose: bool,
) -> Result<ProjectScaffold, String> {
    let root = path.to_path_buf();
    if root.exists() {
        if !root.is_dir() {
            return Err(format!(
                "`{}` already exists and is not a directory",
                root.display()
            ));
        }
        let mut entries = std::fs::read_dir(&root)
            .map_err(|e| format!("failed to inspect `{}`: {}", root.display(), e))?;
        if entries
            .next()
            .transpose()
            .map_err(|e| {
                format!(
                    "failed to inspect directory entries for `{}`: {}",
                    root.display(),
                    e
                )
            })?
            .is_some()
        {
            return Err(format!(
                "`{}` is not empty; scaffold into a new directory instead",
                root.display()
            ));
        }
        if !force {
            return Err(format!(
                "`{}` already exists; pass `--force` to scaffold inside the existing empty directory",
                root.display()
            ));
        }
    } else {
        std::fs::create_dir_all(&root)
            .map_err(|e| format!("failed to create `{}`: {}", root.display(), e))?;
    }

    let project_name = sanitize_project_name(
        root.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("agam-app"),
    );
    let manifest_path = root.join("agam.toml");
    let entry_dir = root.join("src");
    let entry_file = entry_dir.join("main.agam");
    let tests_dir = root.join("tests");
    let smoke_test = tests_dir.join("smoke.agam");
    let gitignore_path = root.join(".gitignore");

    std::fs::create_dir_all(&entry_dir)
        .map_err(|e| format!("failed to create `{}`: {}", entry_dir.display(), e))?;
    std::fs::create_dir_all(&tests_dir)
        .map_err(|e| format!("failed to create `{}`: {}", tests_dir.display(), e))?;

    write_text_file(&manifest_path, &render_project_manifest(&project_name))?;
    write_text_file(&entry_file, &render_project_entry(&project_name))?;
    write_text_file(&smoke_test, &render_project_smoke_test())?;
    write_text_file(&gitignore_path, PROJECT_GITIGNORE)?;

    if verbose {
        eprintln!("[agamc] Scaffolded project `{}`", project_name);
    }

    Ok(ProjectScaffold {
        root,
        manifest_path,
        entry_file,
    })
}

fn write_text_file(path: &Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents)
        .map_err(|e| format!("failed to write `{}`: {}", path.display(), e))
}

fn sanitize_project_name(raw: &str) -> String {
    let mut sanitized = String::with_capacity(raw.len());
    let mut last_was_sep = false;
    for ch in raw.chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            last_was_sep = false;
            Some(ch.to_ascii_lowercase())
        } else if matches!(ch, '-' | '_') {
            if last_was_sep {
                None
            } else {
                last_was_sep = true;
                Some('-')
            }
        } else {
            if last_was_sep {
                None
            } else {
                last_was_sep = true;
                Some('-')
            }
        };
        if let Some(ch) = normalized {
            sanitized.push(ch);
        }
    }
    sanitized = sanitized.trim_matches('-').to_string();
    if sanitized.is_empty() {
        "agam-app".into()
    } else {
        sanitized
    }
}

const PROJECT_GITIGNORE: &str = ".agam_cache/\ndist/\n*.agpkg.json\n*.c\n*.ll\n*.exe\nsrc/main\n";

fn render_project_manifest(project_name: &str) -> String {
    format!("[project]\nname = \"{project_name}\"\nversion = \"0.1.0\"\nagam = \"0.1\"\n")
}

fn render_project_entry(project_name: &str) -> String {
    format!(
        "@lang.advance\n\nfn main() -> i32 {{\n    println(\"Hello from {project_name}\");\n    return 0;\n}}\n"
    )
}

fn render_project_smoke_test() -> String {
    "@test\nfn arithmetic_is_sound() -> bool:\n    return (20 + 22) == 42\n".into()
}

fn resolve_workspace_layout(path: Option<PathBuf>) -> Result<WorkspaceLayout, String> {
    let hint = match path {
        Some(path) => path,
        None => std::env::current_dir()
            .map_err(|e| format!("failed to read current directory: {}", e))?,
    };

    resolve_workspace_layout_from_path(&hint)
}

fn resolve_workspace_layout_from_path(path: &Path) -> Result<WorkspaceLayout, String> {
    if path.is_file() {
        if path.file_name().and_then(|name| name.to_str()) == Some("agam.toml") {
            let root = path
                .parent()
                .ok_or_else(|| format!("manifest `{}` has no parent directory", path.display()))?;
            return workspace_layout_from_root(root, Some(path.to_path_buf()), None);
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("agam") {
            return Err(format!(
                "`{}` is not an Agam source file or `agam.toml` manifest",
                path.display()
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| format!("source file `{}` has no parent directory", path.display()))?;
        let manifest = find_workspace_manifest(parent);
        let root = manifest
            .as_ref()
            .and_then(|manifest_path| manifest_path.parent())
            .unwrap_or(parent)
            .to_path_buf();
        return workspace_layout_from_root(&root, manifest, Some(path.to_path_buf()));
    }

    if !path.exists() {
        return Err(format!("`{}` does not exist", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("`{}` is not a directory", path.display()));
    }

    let manifest = find_workspace_manifest(path);
    let root = manifest
        .as_ref()
        .and_then(|manifest_path| manifest_path.parent())
        .unwrap_or(path)
        .to_path_buf();
    workspace_layout_from_root(&root, manifest, None)
}

fn find_workspace_manifest(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let manifest = ancestor.join("agam.toml");
        if manifest.is_file() {
            return Some(manifest);
        }
    }
    None
}

fn workspace_layout_from_root(
    root: &Path,
    manifest_path: Option<PathBuf>,
    entry_override: Option<PathBuf>,
) -> Result<WorkspaceLayout, String> {
    let entry_file = entry_override.unwrap_or_else(|| root.join("src").join("main.agam"));
    if !entry_file.is_file() {
        return Err(format!(
            "could not find entry file `{}`; create a project with `agamc new <name>` or pass an explicit `.agam` file",
            entry_file.display()
        ));
    }

    let mut source_files = Vec::new();
    let src_dir = root.join("src");
    if src_dir.is_dir() {
        collect_agam_files(&src_dir, &mut source_files)?;
    }
    if !source_files.iter().any(|file| file == &entry_file) {
        source_files.push(entry_file.clone());
    }
    source_files.sort();
    source_files.dedup();

    let mut test_files = Vec::new();
    let tests_dir = root.join("tests");
    if tests_dir.is_dir() {
        collect_agam_files(&tests_dir, &mut test_files)?;
    }
    test_files.sort();
    test_files.dedup();

    Ok(WorkspaceLayout {
        root: root.to_path_buf(),
        manifest_path,
        entry_file,
        source_files,
        test_files,
    })
}

fn run_format_paths(files: &[PathBuf], check: bool, verbose: bool) -> Result<Vec<PathBuf>, String> {
    let action = if check { "Checking" } else { "Formatting" };
    if verbose {
        eprintln!("[agamc] {} {} file(s)...", action, files.len());
    }

    let mut changed_files = Vec::new();
    for file in files {
        let source = std::fs::read_to_string(file)
            .map_err(|e| format!("could not read `{}`: {}", file.display(), e))?;
        let formatted = agam_fmt::format_source(&source);
        if formatted.changed {
            changed_files.push(file.clone());
            if !check {
                std::fs::write(file, formatted.output)
                    .map_err(|e| format!("could not write `{}`: {}", file.display(), e))?;
            }
        }
    }
    Ok(changed_files)
}

fn run_agam_tests(files: &[PathBuf], verbose: bool) -> Result<TestRunTotals, String> {
    let mut totals = TestRunTotals::default();

    for file in files {
        let summary = agam_test::run_file(file)?;
        if summary.results.is_empty() && verbose {
            eprintln!("[agamc] {} — no tests found", file.display());
        }
        for result in &summary.results {
            let status = if result.passed {
                "\x1b[1;32mok\x1b[0m"
            } else {
                "\x1b[1;31mFAILED\x1b[0m"
            };
            eprintln!(
                "test {}:{}:{} {} ... {}",
                file.display(),
                result.case.line,
                result.case.column,
                result.case.name,
                status
            );
            if let Some(message) = &result.message {
                eprintln!("  {}", message);
            }
        }

        totals.total += summary.total();
        totals.passed += summary.passed();
        totals.failed += summary.failed();
    }

    Ok(totals)
}

fn run_source_file(
    file: &PathBuf,
    args: &[String],
    backend: Backend,
    opt_level: u8,
    tuning: &ReleaseTuning,
    verbose: bool,
    features: FeatureFlags,
) -> Result<i32, String> {
    let exe_path = default_native_binary_output_path(file, tuning.target.as_deref());

    if backend == Backend::Jit {
        let mut runtime_args = Vec::with_capacity(args.len() + 1);
        runtime_args.push(file.to_string_lossy().to_string());
        runtime_args.extend(args.iter().cloned());
        return run_with_jit(file, &runtime_args, verbose, features);
    }

    if backend == Backend::Llvm {
        return run_with_llvm(file, args, opt_level, tuning, verbose, features);
    }

    let outcome = build_file(
        file, &exe_path, opt_level, backend, tuning, features, verbose,
    )?;
    if !outcome.native_binary {
        return Err(format!(
            "backend {:?} emitted {} but no native executable was produced",
            backend,
            outcome.generated_path.display()
        ));
    }

    let status = std::process::Command::new(&exe_path)
        .args(args)
        .status()
        .map_err(|e| format!("failed to run {}: {}", exe_path.display(), e))?;
    Ok(status.code().unwrap_or(1))
}

fn print_cache_status(path: Option<PathBuf>, recent: usize, verbose: bool) -> Result<(), String> {
    let hint = match path {
        Some(path) => path,
        None => std::env::current_dir()
            .map_err(|e| format!("failed to read current directory: {}", e))?,
    };
    let cache = agam_runtime::cache::CacheStore::for_path(&hint)?;
    let status = cache.status(recent)?;

    println!("Agam Cache");
    println!("root: {}", status.root.display());
    println!("entries: {}", status.entry_count);
    println!("size: {}", human_bytes(status.total_bytes));

    if status.by_kind.is_empty() {
        println!("kinds: empty");
    } else {
        println!("kinds:");
        for kind in &status.by_kind {
            println!(
                "  {}: {} entr{} / {}",
                kind.kind.label(),
                kind.entries,
                if kind.entries == 1 { "y" } else { "ies" },
                human_bytes(kind.bytes)
            );
        }
    }

    if !status.recent_entries.is_empty() {
        println!("recent:");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        for entry in &status.recent_entries {
            println!(
                "  {} {} ({})",
                entry.artifact_kind.label(),
                entry.source_path,
                relative_age(now.saturating_sub(entry.last_used_unix_ms))
            );
        }
    } else if verbose {
        println!("recent: none");
    }

    Ok(())
}

fn run_dev_workflow(
    path: Option<PathBuf>,
    backend: Backend,
    opt_level: u8,
    fix: bool,
    no_run: bool,
    no_tests: bool,
    verbose: bool,
) -> Result<(), String> {
    let workspace = resolve_workspace_layout(path)?;
    let project_name = workspace
        .manifest_path
        .as_ref()
        .and_then(|path| path.parent())
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("agam-workspace");
    let cache = agam_runtime::cache::CacheStore::for_path(&workspace.root)?;
    let cache_status = cache.status(3)?;
    let native_llvm = resolve_native_llvm_command();
    let resolved_backend = resolve_backend(backend, !no_run);

    println!("Agam Dev");
    println!("workspace: {}", workspace.root.display());
    if let Some(manifest) = workspace.manifest_path.as_ref() {
        println!("manifest: {}", manifest.display());
    } else {
        println!("manifest: none");
    }
    println!("project: {}", project_name);
    println!("entry: {}", workspace.entry_file.display());
    println!("sources: {}", workspace.source_files.len());
    println!("tests: {}", workspace.test_files.len());
    println!(
        "cache: {} / {}",
        cache_status.entry_count,
        human_bytes(cache_status.total_bytes)
    );
    println!(
        "toolchain: {}",
        native_llvm
            .map(|driver| format!("native llvm via `{driver}`"))
            .unwrap_or_else(|| {
                if command_exists(default_c_compiler()) {
                    format!("c fallback via `{}`", default_c_compiler())
                } else {
                    "jit-only".into()
                }
            })
    );

    let mut files_to_format = workspace.source_files.clone();
    files_to_format.extend(workspace.test_files.iter().cloned());
    files_to_format.sort();
    files_to_format.dedup();

    let changed = run_format_paths(&files_to_format, !fix, verbose)?;
    if !fix && !changed.is_empty() {
        for file in &changed {
            eprintln!("needs formatting: {}", file.display());
        }
        return Err("formatting is not clean; re-run with `agamc dev --fix` or `agamc fmt`".into());
    }
    if fix && !changed.is_empty() {
        eprintln!("\x1b[1;32m✓\x1b[0m Formatted {} file(s).", changed.len());
    }

    for file in &workspace.source_files {
        compile_file(file, verbose)?;
    }
    for file in &workspace.test_files {
        compile_file(file, verbose)?;
    }
    eprintln!("\x1b[1;32m✓\x1b[0m Type checks passed.");

    if !no_tests && !workspace.test_files.is_empty() {
        let totals = run_agam_tests(&workspace.test_files, verbose)?;
        if totals.failed > 0 {
            return Err(format!(
                "Agam tests failed: {} passed; {} failed",
                totals.passed, totals.failed
            ));
        }
        eprintln!("\x1b[1;32m✓\x1b[0m Agam tests passed: {}", totals.passed);
    }

    if no_run {
        eprintln!("\x1b[1;32m✓\x1b[0m Dev checks completed.");
        return Ok(());
    }

    let tuning = ReleaseTuning {
        target: None,
        native_cpu: true,
        lto: None,
        pgo_generate: None,
        pgo_use: None,
    };
    let features = FeatureFlags::default();
    let code = run_source_file(
        &workspace.entry_file,
        &[],
        resolved_backend,
        opt_level.min(3),
        &tuning,
        verbose,
        features,
    )?;
    if code != 0 {
        return Err(format!("program exited with status {}", code));
    }

    eprintln!("\x1b[1;32m✓\x1b[0m Dev run completed.");
    Ok(())
}

fn human_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= GIB {
        format!("{:.1} GiB", bytes_f / GIB)
    } else if bytes_f >= MIB {
        format!("{:.1} MiB", bytes_f / MIB)
    } else if bytes_f >= KIB {
        format!("{:.1} KiB", bytes_f / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn relative_age(delta_ms: u128) -> String {
    const SECOND: u128 = 1000;
    const MINUTE: u128 = 60 * SECOND;
    const HOUR: u128 = 60 * MINUTE;
    if delta_ms >= HOUR {
        format!("{}h ago", delta_ms / HOUR)
    } else if delta_ms >= MINUTE {
        format!("{}m ago", delta_ms / MINUTE)
    } else if delta_ms >= SECOND {
        format!("{}s ago", delta_ms / SECOND)
    } else {
        "just now".into()
    }
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
        source,
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
                features.call_cache.disable_all = false;
                features.call_cache.enable_all = true;
            }
            "experimental.no_call_cache" | "lang.feat.no_call_cache" => {
                features.call_cache.disable_all = true;
                features.call_cache.enable_all = false;
                features.call_cache.optimize_all = false;
            }
            "experimental.call_cache.optimize" => {
                features.call_cache.disable_all = false;
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

fn lower_parsed_to_optimized_mir(parsed: &ParsedSource, verbose: bool) -> agam_mir::ir::MirModule {
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

    // Run escape analysis + stack promotion as a post-optimization pass.
    let purity = agam_mir::opt::escape::CalleePurityInfo::default();
    let (escape_results, promo_results) = agam_mir::opt::run_escape_and_promote(&mut mir, &purity);

    if verbose {
        eprintln!(
            "[agamc] Escape analysis: {} function(s) analyzed",
            escape_results.functions.len()
        );
        if promo_results.total_promoted > 0 {
            eprintln!(
                "[agamc] Stack promotion: {} local(s) promoted, {} ARC elision(s)",
                promo_results.total_promoted, promo_results.total_arc_elided
            );
        }
        for (func_name, fr) in &promo_results.functions {
            if !fr.promoted_locals.is_empty() {
                eprintln!(
                    "[agamc]   {}: promoted [{}]",
                    func_name,
                    fr.promoted_locals.join(", ")
                );
            }
            for (local, reason) in &fr.skipped {
                eprintln!("[agamc]   {}: skipped `{}` ({})", func_name, local, reason);
            }
        }
    }

    mir
}

fn lower_to_optimized_mir(
    path: &PathBuf,
    verbose: bool,
) -> Result<(agam_mir::ir::MirModule, SourceFeatureFlags), String> {
    let parsed = parse_source_file(path, verbose)?;
    let mir = lower_parsed_to_optimized_mir(&parsed, verbose);

    Ok((mir, parsed.source_features))
}

fn build_portable_package_file(
    path: &PathBuf,
    verbose: bool,
) -> Result<agam_pkg::PortablePackage, String> {
    let parsed = parse_source_file(path, verbose)?;
    let mir = lower_parsed_to_optimized_mir(&parsed, verbose);
    Ok(agam_pkg::build_portable_package(
        path,
        &parsed.source,
        &parsed.module,
        &mir,
        agam_runtime::contract::RuntimeBackend::Jit,
    ))
}

fn write_portable_package_with_cache(
    source_path: &PathBuf,
    output: &PathBuf,
    package: &agam_pkg::PortablePackage,
    verbose: bool,
) -> Result<(), String> {
    let cache = agam_runtime::cache::CacheStore::for_path(source_path)?;
    let source = std::fs::read(source_path).map_err(|e| {
        format!(
            "failed to read `{}` for cache key generation: {}",
            source_path.display(),
            e
        )
    })?;
    let package_hash = agam_runtime::cache::hash_bytes(&source);
    let semantic_hash = agam_runtime::cache::hash_serializable(&package.manifest)?;
    let key = agam_runtime::cache::default_cache_key(
        package_hash,
        semantic_hash,
        agam_runtime::contract::RuntimeBackend::Jit,
        0,
        "package".to_string(),
    );

    if let Some(hit) = cache.lookup(&key)? {
        if verbose {
            eprintln!("[agamc] Package cache hit: {}", hit.id);
        }
        return cache.restore_to_path(&hit, output);
    }

    let bytes = serde_json::to_vec_pretty(package)
        .map_err(|e| format!("failed to serialize package for cache: {}", e))?;
    let hit = cache.store_bytes(
        &key,
        agam_runtime::cache::CacheArtifactKind::PortablePackage,
        source_path,
        output
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package.agpkg.json"),
        &bytes,
    )?;
    if verbose {
        eprintln!("[agamc] Package cache miss; stored {}", hit.id);
    }
    cache.restore_to_path(&hit, output)
}

fn run_portable_package_file(
    path: &PathBuf,
    args: &[String],
    verbose: bool,
) -> Result<i32, String> {
    let package = agam_pkg::read_package_from_path(path)?;
    let host = agam_runtime::contract::host_runtime();
    let plan = agam_runtime::contract::plan_package_load(
        &package.runtime,
        agam_runtime::contract::RuntimeBackend::Auto,
        &host,
    )?;

    if verbose {
        eprintln!(
            "[agamc] Loaded portable package `{}` via {:?} on {} / {} / {}-bit",
            package.manifest.name,
            plan.backend,
            plan.host.os,
            plan.host.arch,
            plan.host.pointer_width
        );
    }

    match plan.backend {
        agam_runtime::contract::RuntimeBackend::Jit => {
            let mut runtime_args = Vec::with_capacity(args.len() + 1);
            runtime_args.push(path.to_string_lossy().to_string());
            runtime_args.extend(args.iter().cloned());
            agam_jit::run_main(&package.mir, &runtime_args)
        }
        backend => Err(format!(
            "portable package execution is currently implemented only through the JIT runtime; package requested {:?}",
            backend
        )),
    }
}

fn print_package_summary(package: &agam_pkg::PortablePackage) {
    println!("package: {}", package.manifest.name);
    println!("source: {}", package.manifest.source_path);
    println!("entry: {}", package.manifest.entry_function);
    println!(
        "runtime ABI: v{} ({:?})",
        package.runtime.abi.version, package.runtime.requirements.preferred_backend
    );
    println!(
        "build host: {} / {} / {}-bit",
        package.runtime.build_host.os,
        package.runtime.build_host.arch,
        package.runtime.build_host.pointer_width
    );
    println!(
        "verified functions: {}",
        package.manifest.verified_ir.function_count
    );
    println!("source map entries: {}", package.manifest.source_map.len());
    println!(
        "declared effects: {}",
        package.manifest.effects.declared_effects.len()
    );
}

fn print_doctor_status(label: &str, status: &str, detail: &str) {
    println!("{label}: {status}");
    println!("  {detail}");
}

fn run_doctor(verbose: bool) -> Result<bool, String> {
    let host = current_host_sdk_platform();
    let bundled_root = detect_packaged_llvm_bundle_root();
    let bundled_driver = discover_bundled_llvm_clang();
    let override_driver = configured_llvm_clang_override();
    let native_driver = resolve_native_llvm_command();
    let vs_install = discover_visual_studio_installation_path();
    let vs_driver = discover_visual_studio_llvm_clang();
    let wsl_clang = wsl_command_exists("clang");
    let c_driver = command_exists(default_c_compiler());
    let android_sysroot = resolve_android_ndk_sysroot();

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("failed to locate current compiler executable: {}", e))?;

    println!("Agam Doctor");
    println!("host: {host}");
    println!("core compiler: {}", current_exe.display());

    match native_driver.as_ref() {
        Some(driver) => {
            print_doctor_status("native llvm", "ok", &format!("using `{driver}`"));
        }
        None => {
            let hint = if cfg!(windows) {
                windows_native_llvm_install_hint().unwrap_or_else(|| {
                    format!(
                        "install a native LLVM/Clang toolchain, bundle one next to agamc, or set `{LLVM_CLANG_ENV}`"
                    )
                })
            } else {
                format!(
                    "install a native LLVM/Clang toolchain, bundle one next to agamc, or set `{LLVM_CLANG_ENV}`"
                )
            };
            print_doctor_status("native llvm", "missing", &hint);
        }
    }

    match bundled_root.as_ref() {
        Some(root) => print_doctor_status(
            "bundled llvm",
            "ok",
            &format!("bundle root `{}`", root.display()),
        ),
        None => print_doctor_status(
            "bundled llvm",
            "missing",
            &format!(
                "no bundled LLVM found; expected `toolchains/llvm/{}/bin` near `agamc` or set `{}`",
                bundled_llvm_platform_dir(),
                LLVM_BUNDLE_DIR_ENV
            ),
        ),
    }

    if let Some(driver) = bundled_driver.as_ref() {
        print_doctor_status("bundled driver", "ok", &format!("driver `{driver}`"));
    } else if verbose {
        print_doctor_status(
            "bundled driver",
            "missing",
            "no bundled clang/clang++ executable resolved from the bundle search paths",
        );
    }

    if cfg!(windows) {
        match vs_install.as_ref() {
            Some(path) => print_doctor_status(
                "visual studio",
                "ok",
                &format!("installation `{}`", path.display()),
            ),
            None => print_doctor_status(
                "visual studio",
                "missing",
                "Visual Studio installation not detected via vswhere",
            ),
        }
        match vs_driver.as_ref() {
            Some(path) => {
                print_doctor_status("visual studio llvm", "ok", &format!("driver `{path}`"))
            }
            None => print_doctor_status(
                "visual studio llvm",
                "missing",
                "LLVM/Clang component is not currently installed in Visual Studio",
            ),
        }
        if wsl_clang {
            print_doctor_status(
                "wsl llvm",
                "available",
                &format!(
                    "development-only fallback; enable with `{DEV_WSL_LLVM_ENV}=1` for `agamc run --backend llvm`"
                ),
            );
        } else if verbose {
            print_doctor_status("wsl llvm", "missing", "WSL clang was not detected");
        }
    }

    if let Some(driver) = override_driver.as_ref() {
        print_doctor_status(
            "llvm override",
            "configured",
            &format!("`{LLVM_CLANG_ENV}` -> `{driver}`"),
        );
    } else if verbose {
        print_doctor_status(
            "llvm override",
            "unset",
            &format!("set `{LLVM_CLANG_ENV}` to pin `clang` or `clang++`"),
        );
    }

    if c_driver {
        print_doctor_status(
            "c fallback",
            "ok",
            &format!("`{}` detected", default_c_compiler()),
        );
    } else {
        print_doctor_status(
            "c fallback",
            "missing",
            &format!("`{}` was not detected on PATH", default_c_compiler()),
        );
    }

    match android_sysroot.as_ref() {
        Some(path) => print_doctor_status(
            "android sysroot",
            "ok",
            &format!("resolved `{}`", path.display()),
        ),
        None => print_doctor_status(
            "android sysroot",
            "missing",
            &format!(
                "set `{LLVM_SYSROOT_ENV}` or `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT` for Android LLVM builds"
            ),
        ),
    }

    println!(
        "recommended sdk command: agamc package sdk --output {}",
        default_sdk_distribution_output_dir().display()
    );

    Ok(native_driver.is_some())
}

#[derive(Debug)]
struct SdkDistributionOutcome {
    root: PathBuf,
    compiler_binary: PathBuf,
    manifest_path: PathBuf,
    llvm_bundle_root: Option<PathBuf>,
}

fn current_host_sdk_platform() -> String {
    bundled_llvm_platform_dir().to_string()
}

fn default_sdk_distribution_output_dir() -> PathBuf {
    PathBuf::from("dist").join(current_host_sdk_platform())
}

fn relative_path_string(root: &Path, path: &Path) -> Result<String, String> {
    path.strip_prefix(root)
        .map_err(|_| {
            format!(
                "failed to compute relative path for `{}` under `{}`",
                path.display(),
                root.display()
            )
        })
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
}

fn default_host_target_triple() -> String {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "x86_64-pc-windows-msvc".into(),
        ("windows", "aarch64") => "aarch64-pc-windows-msvc".into(),
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu".into(),
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu".into(),
        ("macos", "x86_64") => "x86_64-apple-darwin".into(),
        ("macos", "aarch64") => "aarch64-apple-darwin".into(),
        _ => format!(
            "{}-unknown-{}",
            std::env::consts::ARCH,
            std::env::consts::OS
        ),
    }
}

fn sdk_supported_targets() -> Vec<agam_pkg::SdkTargetProfile> {
    let mut targets = vec![agam_pkg::SdkTargetProfile {
        name: "host-native".into(),
        target_triple: default_host_target_triple(),
        backend: agam_runtime::contract::RuntimeBackend::Llvm,
        sysroot_env: None,
        sdk_env: None,
    }];

    if matches!(
        host_llvm_target_platform(),
        LlvmTargetPlatform::Windows | LlvmTargetPlatform::Linux
    ) {
        targets.push(agam_pkg::SdkTargetProfile {
            name: "android-arm64".into(),
            target_triple: "aarch64-linux-android21".into(),
            backend: agam_runtime::contract::RuntimeBackend::Llvm,
            sysroot_env: Some(LLVM_SYSROOT_ENV.into()),
            sdk_env: None,
        });
    }

    targets
}

fn detect_packaged_llvm_bundle_root() -> Option<PathBuf> {
    if let Some(explicit_root) = env_path(LLVM_BUNDLE_DIR_ENV) {
        if explicit_root.is_dir() {
            return Some(explicit_root);
        }
    }
    let current_exe = std::env::current_exe().ok()?;
    let exe_dir = current_exe.parent()?;
    for base in [Some(exe_dir), exe_dir.parent()].into_iter().flatten() {
        let candidate = base.join("toolchains").join("llvm");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

fn resolve_sdk_llvm_bundle_source(explicit: Option<&PathBuf>) -> Option<PathBuf> {
    explicit.cloned().or_else(detect_packaged_llvm_bundle_root)
}

fn copy_directory_recursive(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "directory copy source `{}` does not exist or is not a directory",
            source.display()
        ));
    }
    std::fs::create_dir_all(destination).map_err(|e| {
        format!(
            "failed to create directory `{}`: {}",
            destination.display(),
            e
        )
    })?;
    for entry in std::fs::read_dir(source)
        .map_err(|e| format!("failed to read directory `{}`: {}", source.display(), e))?
    {
        let entry = entry.map_err(|e| {
            format!(
                "failed to read directory entry in `{}`: {}",
                source.display(),
                e
            )
        })?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|e| {
            format!(
                "failed to read file type for `{}`: {}",
                source_path.display(),
                e
            )
        })?;
        if file_type.is_dir() {
            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    format!("failed to create directory `{}`: {}", parent.display(), e)
                })?;
            }
            std::fs::copy(&source_path, &destination_path).map_err(|e| {
                format!(
                    "failed to copy `{}` to `{}`: {}",
                    source_path.display(),
                    destination_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

fn stage_llvm_bundle_into_sdk(source: &Path, output_root: &Path) -> Result<PathBuf, String> {
    let host_platform = bundled_llvm_platform_dir();
    let dest_root = output_root.join("toolchains").join("llvm");
    if source.join(host_platform).is_dir() || source.join("bin").is_dir() {
        copy_directory_recursive(source, &dest_root)?;
        return Ok(dest_root);
    }
    if source
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name == host_platform)
        .unwrap_or(false)
    {
        let destination = dest_root.join(host_platform);
        copy_directory_recursive(source, &destination)?;
        return Ok(dest_root);
    }
    Err(format!(
        "LLVM bundle source `{}` must be a bundle root or `{}` platform directory",
        source.display(),
        host_platform
    ))
}

fn package_sdk_distribution(
    output_root: &Path,
    llvm_bundle: Option<&PathBuf>,
    verbose: bool,
) -> Result<SdkDistributionOutcome, String> {
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("failed to locate current compiler executable: {}", e))?;
    let compiler_name = current_exe.file_name().ok_or_else(|| {
        format!(
            "failed to determine compiler filename from `{}`",
            current_exe.display()
        )
    })?;
    let compiler_destination = output_root.join("bin").join(compiler_name);
    if let Some(parent) = compiler_destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory `{}`: {}", parent.display(), e))?;
    }
    std::fs::copy(&current_exe, &compiler_destination).map_err(|e| {
        format!(
            "failed to copy compiler binary `{}` to `{}`: {}",
            current_exe.display(),
            compiler_destination.display(),
            e
        )
    })?;

    let llvm_bundle_root = match resolve_sdk_llvm_bundle_source(llvm_bundle) {
        Some(source) => {
            let staged = stage_llvm_bundle_into_sdk(&source, output_root)?;
            if verbose {
                eprintln!("[agamc] staged bundled LLVM from {}", source.display());
            }
            Some(staged)
        }
        None => None,
    };

    let preferred_llvm_driver = llvm_bundle_root.as_ref().and_then(|root| {
        bundled_llvm_candidate_paths(root)
            .into_iter()
            .find(|path| path.is_file())
    });
    let manifest = agam_pkg::SdkDistributionManifest {
        format_version: agam_pkg::SDK_DISTRIBUTION_FORMAT_VERSION,
        sdk_name: format!("agam-sdk-{}", current_host_sdk_platform()),
        host_platform: current_host_sdk_platform(),
        compiler_binary: relative_path_string(output_root, &compiler_destination)?,
        llvm_bundle_root: llvm_bundle_root
            .as_ref()
            .map(|path| relative_path_string(output_root, path))
            .transpose()?,
        preferred_llvm_driver: preferred_llvm_driver
            .as_ref()
            .map(|path| relative_path_string(output_root, path))
            .transpose()?,
        supported_targets: sdk_supported_targets(),
        notes: vec![
            "native llvm is the preferred production backend".into(),
            "wsl remains a development-only fallback and is not part of the shipped sdk contract"
                .into(),
        ],
    };
    let manifest_path = output_root.join("sdk-manifest.json");
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create directory `{}`: {}", parent.display(), e))?;
    }
    agam_pkg::write_sdk_distribution_manifest_to_path(&manifest_path, &manifest)?;

    Ok(SdkDistributionOutcome {
        root: output_root.to_path_buf(),
        compiler_binary: compiler_destination,
        manifest_path,
        llvm_bundle_root,
    })
}

fn runtime_backend_for_cache(backend: Backend) -> agam_runtime::contract::RuntimeBackend {
    match backend {
        Backend::Auto => agam_runtime::contract::RuntimeBackend::Auto,
        Backend::C => agam_runtime::contract::RuntimeBackend::C,
        Backend::Llvm => agam_runtime::contract::RuntimeBackend::Llvm,
        Backend::Jit => agam_runtime::contract::RuntimeBackend::Jit,
    }
}

fn runtime_backend_label(backend: agam_runtime::contract::RuntimeBackend) -> &'static str {
    match backend {
        agam_runtime::contract::RuntimeBackend::Auto => "auto",
        agam_runtime::contract::RuntimeBackend::Jit => "jit",
        agam_runtime::contract::RuntimeBackend::Llvm => "llvm",
        agam_runtime::contract::RuntimeBackend::C => "c",
    }
}

fn call_cache_signature(call_cache: &CallCacheSelection) -> String {
    let mut parts = Vec::new();
    parts.push("strategy=auto-v1".to_string());
    parts.push(format!("disable_all={}", call_cache.disable_all));
    parts.push(format!("enable_all={}", call_cache.enable_all));
    parts.push(format!("optimize_all={}", call_cache.optimize_all));
    parts.push(format!(
        "include={}",
        call_cache
            .include_functions
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    ));
    parts.push(format!(
        "optimize={}",
        call_cache
            .optimize_functions
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    ));
    parts.push(format!(
        "exclude={}",
        call_cache
            .exclude_functions
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    ));
    parts.join(";")
}

fn build_feature_signature(
    backend: Backend,
    call_cache: &CallCacheSelection,
    allow_wsl_llvm: bool,
    tuning: &ReleaseTuning,
) -> String {
    let mut signature = format!("build_cache={BUILD_CACHE_SIGNATURE_VERSION}");
    signature.push(';');
    signature.push_str(&call_cache_signature(call_cache));
    if backend == Backend::Llvm {
        let target_config = resolve_llvm_target_config(tuning);
        let toolchain = match if allow_wsl_llvm {
            resolve_llvm_run_toolchain()
        } else {
            resolve_native_llvm_toolchain()
        } {
            Some(LlvmToolchain::Native) => "native",
            Some(LlvmToolchain::Wsl) => "wsl",
            None => "missing",
        };
        signature.push_str(&format!(";llvm_toolchain={toolchain}"));
        signature.push_str(&format!(
            ";llvm_wsl_allowed={}",
            if allow_wsl_llvm { "true" } else { "false" }
        ));
        signature.push_str(&format!(
            ";llvm_clang={}",
            configured_llvm_clang().replace(';', "_")
        ));
        signature.push_str(&format!(
            ";llvm_target={}",
            target_config.target_triple.as_deref().unwrap_or("host")
        ));
        signature.push_str(&format!(
            ";llvm_sysroot={}",
            target_config
                .sysroot
                .as_ref()
                .map(|path| path.to_string_lossy().replace(';', "_"))
                .unwrap_or_else(|| "none".into())
        ));
        signature.push_str(&format!(
            ";llvm_sdkroot={}",
            target_config
                .sdk_root
                .as_ref()
                .map(|path| path.to_string_lossy().replace(';', "_"))
                .unwrap_or_else(|| "none".into())
        ));
    }
    signature
}

fn build_cache_key(
    path: &PathBuf,
    mir: &agam_mir::ir::MirModule,
    backend: Backend,
    opt_level: u8,
    call_cache: &CallCacheSelection,
    allow_wsl_llvm: bool,
    tuning: &ReleaseTuning,
) -> Result<agam_runtime::cache::CacheKey, String> {
    let source = std::fs::read(path).map_err(|e| {
        format!(
            "failed to read `{}` for cache key generation: {}",
            path.display(),
            e
        )
    })?;
    let package_hash = agam_runtime::cache::hash_bytes(&source);
    let semantic_hash = agam_runtime::cache::hash_serializable(mir)?;
    Ok(agam_runtime::cache::default_cache_key(
        package_hash,
        semantic_hash,
        runtime_backend_for_cache(backend),
        opt_level,
        build_feature_signature(backend, call_cache, allow_wsl_llvm, tuning),
    ))
}

fn cached_build_output_path(
    output: &PathBuf,
    artifact_kind: agam_runtime::cache::CacheArtifactKind,
) -> PathBuf {
    match artifact_kind {
        agam_runtime::cache::CacheArtifactKind::NativeBinary => output.clone(),
        agam_runtime::cache::CacheArtifactKind::LlvmIr => output.with_extension("ll"),
        agam_runtime::cache::CacheArtifactKind::CSource => output.with_extension("c"),
        agam_runtime::cache::CacheArtifactKind::PortablePackage => {
            output.with_extension("agpkg.json")
        }
        agam_runtime::cache::CacheArtifactKind::ProfileJson => {
            output.with_extension("call_profile.json")
        }
    }
}

fn profile_cache_key_for_backend(
    path: &PathBuf,
    mir: &agam_mir::ir::MirModule,
    call_cache: &CallCacheSelection,
    backend: agam_runtime::contract::RuntimeBackend,
    namespace: &str,
) -> Result<agam_runtime::cache::CacheKey, String> {
    let source = std::fs::read(path).map_err(|e| {
        format!(
            "failed to read `{}` for profile cache key generation: {}",
            path.display(),
            e
        )
    })?;
    let package_hash = agam_runtime::cache::hash_bytes(&source);
    let semantic_hash = agam_runtime::cache::hash_serializable(mir)?;
    Ok(agam_runtime::cache::default_cache_key(
        package_hash,
        semantic_hash,
        backend,
        0,
        format!("{namespace};{}", call_cache_signature(call_cache)),
    ))
}

fn load_persisted_call_profile(
    path: &PathBuf,
    mir: &agam_mir::ir::MirModule,
    call_cache: &CallCacheSelection,
    backend: agam_runtime::contract::RuntimeBackend,
    namespace: &str,
    verbose: bool,
) -> Option<agam_profile::PersistentCallCacheProfile> {
    let cache = agam_runtime::cache::CacheStore::for_path(path).ok()?;
    let key = profile_cache_key_for_backend(path, mir, call_cache, backend, namespace).ok()?;
    let hit = match cache.lookup(&key) {
        Ok(hit) => hit?,
        Err(e) => {
            if verbose {
                eprintln!(
                    "[agamc] {} profile cache lookup failed: {}",
                    runtime_backend_label(backend).to_uppercase(),
                    e
                );
            }
            return None;
        }
    };
    let json = match std::fs::read_to_string(&hit.artifact_path) {
        Ok(json) => json,
        Err(e) => {
            if verbose {
                eprintln!(
                    "[agamc] Failed to read persisted JIT profile `{}`: {}",
                    hit.artifact_path.display(),
                    e
                );
            }
            return None;
        }
    };
    match serde_json::from_str::<agam_profile::PersistentCallCacheProfile>(&json) {
        Ok(profile) => {
            if profile.schema_version != agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION {
                if verbose {
                    eprintln!(
                        "[agamc] Ignoring persisted {} profile with schema v{} (expected v{})",
                        runtime_backend_label(backend).to_uppercase(),
                        profile.schema_version,
                        agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION
                    );
                }
                return None;
            }
            if profile.backend != runtime_backend_label(backend) {
                if verbose {
                    eprintln!(
                        "[agamc] Ignoring persisted call-cache profile for backend `{}`",
                        profile.backend
                    );
                }
                return None;
            }
            Some(profile)
        }
        Err(e) => {
            if verbose {
                eprintln!(
                    "[agamc] Failed to parse persisted JIT profile `{}`: {}",
                    hit.artifact_path.display(),
                    e
                );
            }
            None
        }
    }
}

fn store_persisted_call_profile(
    path: &PathBuf,
    mir: &agam_mir::ir::MirModule,
    call_cache: &CallCacheSelection,
    backend: agam_runtime::contract::RuntimeBackend,
    namespace: &str,
    profile: &agam_profile::PersistentCallCacheProfile,
    verbose: bool,
) {
    let Ok(cache) = agam_runtime::cache::CacheStore::for_path(path) else {
        return;
    };
    let Ok(key) = profile_cache_key_for_backend(path, mir, call_cache, backend, namespace) else {
        return;
    };
    let Ok(bytes) = serde_json::to_vec_pretty(profile) else {
        return;
    };
    let artifact_name = format!(
        "{}.{}_profile.json",
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("profile"),
        runtime_backend_label(backend)
    );
    match cache.store_bytes(
        &key,
        agam_runtime::cache::CacheArtifactKind::ProfileJson,
        path,
        &artifact_name,
        &bytes,
    ) {
        Ok(hit) => {
            if verbose {
                eprintln!(
                    "[agamc] Stored persisted {} profile: {} (runs={})",
                    runtime_backend_label(backend).to_uppercase(),
                    hit.id,
                    profile.runs
                );
            }
        }
        Err(e) => {
            if verbose {
                eprintln!(
                    "[agamc] Failed to store persisted {} profile: {}",
                    runtime_backend_label(backend).to_uppercase(),
                    e
                );
            }
        }
    }
}

fn load_persisted_jit_profile(
    path: &PathBuf,
    mir: &agam_mir::ir::MirModule,
    call_cache: &CallCacheSelection,
    verbose: bool,
) -> Option<agam_profile::PersistentCallCacheProfile> {
    load_persisted_call_profile(
        path,
        mir,
        call_cache,
        agam_runtime::contract::RuntimeBackend::Jit,
        "jit_profile_v1",
        verbose,
    )
}

fn store_persisted_jit_profile(
    path: &PathBuf,
    mir: &agam_mir::ir::MirModule,
    call_cache: &CallCacheSelection,
    profile: &agam_profile::PersistentCallCacheProfile,
    verbose: bool,
) {
    store_persisted_call_profile(
        path,
        mir,
        call_cache,
        agam_runtime::contract::RuntimeBackend::Jit,
        "jit_profile_v1",
        profile,
        verbose,
    )
}

fn load_persisted_llvm_profile(
    path: &PathBuf,
    mir: &agam_mir::ir::MirModule,
    call_cache: &CallCacheSelection,
    verbose: bool,
) -> Option<agam_profile::PersistentCallCacheProfile> {
    load_persisted_call_profile(
        path,
        mir,
        call_cache,
        agam_runtime::contract::RuntimeBackend::Llvm,
        "llvm_profile_v1",
        verbose,
    )
}

fn store_persisted_llvm_profile(
    path: &PathBuf,
    mir: &agam_mir::ir::MirModule,
    call_cache: &CallCacheSelection,
    profile: &agam_profile::PersistentCallCacheProfile,
    verbose: bool,
) {
    store_persisted_call_profile(
        path,
        mir,
        call_cache,
        agam_runtime::contract::RuntimeBackend::Llvm,
        "llvm_profile_v1",
        profile,
        verbose,
    )
}

fn jit_stats_to_run_profile(
    stats: &agam_jit::JitCallCacheStats,
) -> agam_profile::CallCacheRunProfile {
    agam_profile::CallCacheRunProfile {
        backend: "jit".into(),
        total_calls: stats.total_calls,
        total_hits: stats.total_hits,
        total_stores: stats.total_stores,
        functions: stats
            .functions
            .iter()
            .map(|function| agam_profile::CallCacheFunctionSnapshot {
                name: function.name.clone(),
                calls: function.calls,
                hits: function.hits,
                stores: function.stores,
                entries: function.entries,
                profile: function.profile.clone(),
            })
            .collect(),
    }
}

fn parse_llvm_call_cache_run_profile(
    text: &str,
) -> Result<agam_profile::CallCacheRunProfile, String> {
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return Err("empty LLVM call-cache profile".into());
    };
    let header = header.trim();
    if header != "AGAM_LLVM_CALL_CACHE_PROFILE_V1"
        && header != "AGAM_LLVM_CALL_CACHE_PROFILE_V2"
        && header != "AGAM_LLVM_CALL_CACHE_PROFILE_V3"
        && header != "AGAM_LLVM_CALL_CACHE_PROFILE_V4"
    {
        return Err(format!(
            "unsupported LLVM call-cache profile header `{header}`"
        ));
    }

    let mut functions = Vec::new();
    let mut function_indexes = std::collections::HashMap::new();
    let mut total_calls = 0u64;
    let mut total_hits = 0u64;
    let mut total_stores = 0u64;

    for (line_index, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<_> = line.split('\t').collect();
        match parts.first().copied() {
            Some("FN") => {
                if parts.len() != 6 {
                    return Err(format!(
                        "invalid LLVM call-cache profile line {}: `{}`",
                        line_index + 2,
                        line
                    ));
                }

                let calls = parts[2].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache call count on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                let hits = parts[3].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache hit count on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                let stores = parts[4].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache store count on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                let entries = parts[5].parse::<usize>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache entry count on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;

                total_calls = total_calls.saturating_add(calls);
                total_hits = total_hits.saturating_add(hits);
                total_stores = total_stores.saturating_add(stores);
                let name = parts[1].to_string();
                let function_index = functions.len();
                function_indexes.insert(name.clone(), function_index);
                functions.push(agam_profile::CallCacheFunctionSnapshot {
                    name,
                    calls,
                    hits,
                    stores,
                    entries,
                    profile: agam_profile::CallCacheFunctionProfile {
                        unique_keys: entries.max(stores as usize),
                        ..Default::default()
                    },
                });
            }
            Some("SV") => {
                if parts.len() != 5 {
                    return Err(format!(
                        "invalid LLVM call-cache stable-value line {}: `{}`",
                        line_index + 2,
                        line
                    ));
                }
                let Some(function_index) = function_indexes.get(parts[1]).copied() else {
                    return Err(format!(
                        "LLVM call-cache stable-value line {} references unknown function `{}`",
                        line_index + 2,
                        parts[1]
                    ));
                };
                let arg_index = parts[2].parse::<usize>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache stable-value index on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                let raw_bits = parts[3].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache stable-value bits on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                let matches = parts[4].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache stable-value score on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                if matches > 0 {
                    functions[function_index].profile.stable_values.push(
                        agam_profile::StableScalarValueProfile {
                            index: arg_index,
                            raw_bits,
                            matches,
                        },
                    );
                }
            }
            Some("RD") => {
                if parts.len() != 5 {
                    return Err(format!(
                        "invalid LLVM call-cache reuse-distance line {}: `{}`",
                        line_index + 2,
                        line
                    ));
                }
                let Some(function_index) = function_indexes.get(parts[1]).copied() else {
                    return Err(format!(
                        "LLVM call-cache reuse-distance line {} references unknown function `{}`",
                        line_index + 2,
                        parts[1]
                    ));
                };
                let avg_reuse_distance = parts[2].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache avg reuse distance on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                let max_reuse_distance = parts[3].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache max reuse distance on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                let samples = parts[4].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache reuse sample count on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                if samples > 0 {
                    functions[function_index].profile.avg_reuse_distance = Some(avg_reuse_distance);
                    functions[function_index].profile.max_reuse_distance = Some(max_reuse_distance);
                }
            }
            Some("SP") => {
                if parts.len() != 4 {
                    return Err(format!(
                        "invalid LLVM call-cache specialization line {}: `{}`",
                        line_index + 2,
                        line
                    ));
                }
                let Some(function_index) = function_indexes.get(parts[1]).copied() else {
                    return Err(format!(
                        "LLVM call-cache specialization line {} references unknown function `{}`",
                        line_index + 2,
                        parts[1]
                    ));
                };
                let guard_hits = parts[2].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache specialization hit count on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                let guard_fallbacks = parts[3].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache specialization fallback count on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                functions[function_index].profile.specialization_guard_hits = guard_hits;
                functions[function_index]
                    .profile
                    .specialization_guard_fallbacks = guard_fallbacks;
            }
            _ => {
                return Err(format!(
                    "invalid LLVM call-cache profile line {}: `{}`",
                    line_index + 2,
                    line
                ));
            }
        }
    }

    for function in &mut functions {
        function.profile.specialization_hint =
            agam_profile::specialization_hint(function.calls, &function.profile);
    }

    Ok(agam_profile::CallCacheRunProfile {
        backend: "llvm".into(),
        total_calls,
        total_hits,
        total_stores,
        functions,
    })
}

fn apply_persisted_optimize_profile(
    selection: &CallCacheSelection,
    profile: Option<&agam_profile::PersistentCallCacheProfile>,
) -> (CallCacheSelection, Vec<String>) {
    let Some(profile) = profile else {
        return (selection.clone(), Vec::new());
    };

    let mut merged = selection.clone();
    let mut promoted = Vec::new();
    for function in agam_profile::recommended_optimize_functions(profile) {
        if merged.exclude_functions.contains(&function) {
            continue;
        }
        let selectable = merged.optimize_all
            || merged.resolved_enable_all()
            || merged.include_functions.contains(&function)
            || merged.optimize_functions.contains(&function);
        if !selectable {
            continue;
        }
        if merged.optimize_functions.insert(function.clone()) {
            promoted.push(function);
        }
    }
    (merged, promoted)
}

fn apply_persisted_specialization_profile(
    selection: &CallCacheSelection,
    profile: Option<&agam_profile::PersistentCallCacheProfile>,
) -> Vec<agam_profile::CallCacheSpecializationPlan> {
    let Some(profile) = profile else {
        return Vec::new();
    };

    agam_profile::recommended_specializations(profile)
        .into_iter()
        .filter(|plan| !selection.exclude_functions.contains(&plan.name))
        .filter(|plan| selection.optimize_all || selection.optimize_functions.contains(&plan.name))
        .collect()
}

/// Full compilation pipeline: Lex → Parse → HIR → MIR → C → gcc → native binary
struct BuildOutcome {
    native_binary: bool,
    generated_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct ReleaseTuning {
    target: Option<String>,
    native_cpu: bool,
    lto: Option<LtoMode>,
    pgo_generate: Option<PathBuf>,
    pgo_use: Option<PathBuf>,
}

fn effective_call_cache_selection(
    cli: FeatureFlags,
    source: &SourceFeatureFlags,
) -> CallCacheSelection {
    source.call_cache.merge_cli(cli.call_cache)
}

fn log_call_cache_analysis(
    backend_label: &str,
    selection: &CallCacheSelection,
    analysis: &agam_mir::analysis::CallCacheAnalysis,
) {
    let selected = analysis
        .functions
        .iter()
        .filter(|function| function.eligible)
        .count();
    let optimized = analysis
        .functions
        .iter()
        .filter(|function| {
            matches!(
                function.mode,
                Some(agam_mir::analysis::CallCacheMode::Optimize)
            )
        })
        .count();
    let rejected: Vec<_> = analysis
        .functions
        .iter()
        .filter(|function| function.requested && !function.eligible)
        .collect();

    if !selection.resolved_enable_all()
        && selection.include_functions.is_empty()
        && selection.optimize_functions.is_empty()
    {
        eprintln!("[agamc] Automatic call cache disabled for {backend_label}");
        return;
    }

    eprintln!(
        "[agamc] Automatic call cache on {backend_label}: selected {selected} function(s), rejected {}",
        rejected.len()
    );
    if optimized > 0 {
        eprintln!("[agamc]   optimize mode active for {optimized} function(s)");
    }
    if !selection.exclude_functions.is_empty() {
        eprintln!(
            "[agamc]   source-level opt-out on {} function(s)",
            selection.exclude_functions.len()
        );
    }
    for function in rejected {
        let reasons = function
            .rejection_reasons
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ");
        eprintln!("[agamc]   rejected `{}`: {}", function.name, reasons);
    }
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
    build_prelowered_file(
        path,
        output,
        opt_level,
        backend,
        tuning,
        &mir,
        &call_cache,
        &[],
        false,
        verbose,
    )
}

fn build_prelowered_file(
    path: &PathBuf,
    output: &PathBuf,
    opt_level: u8,
    backend: Backend,
    tuning: &ReleaseTuning,
    mir: &agam_mir::ir::MirModule,
    call_cache: &CallCacheSelection,
    llvm_specializations: &[agam_profile::CallCacheSpecializationPlan],
    allow_wsl_llvm: bool,
    verbose: bool,
) -> Result<BuildOutcome, String> {
    let cache_store = match agam_runtime::cache::CacheStore::for_path(path) {
        Ok(store) => Some(store),
        Err(e) => {
            if verbose {
                eprintln!("[agamc] cache disabled: {}", e);
            }
            None
        }
    };
    let cache_key = match build_cache_key(
        path,
        mir,
        backend,
        opt_level,
        call_cache,
        allow_wsl_llvm,
        tuning,
    ) {
        Ok(key) => Some(key),
        Err(e) => {
            if verbose {
                eprintln!("[agamc] cache key generation failed: {}", e);
            }
            None
        }
    };

    if let (Some(cache), Some(key)) = (&cache_store, &cache_key) {
        match cache.lookup(key) {
            Ok(Some(hit)) => {
                let restored_path = cached_build_output_path(output, hit.entry.artifact_kind);
                cache.restore_to_path(&hit, &restored_path)?;
                if verbose {
                    eprintln!("[agamc] Build cache hit: {}", hit.id);
                }
                return Ok(BuildOutcome {
                    native_binary: hit.entry.artifact_kind
                        == agam_runtime::cache::CacheArtifactKind::NativeBinary,
                    generated_path: restored_path,
                });
            }
            Ok(None) => {
                if verbose {
                    eprintln!("[agamc] Build cache miss");
                }
            }
            Err(e) => {
                if verbose {
                    eprintln!("[agamc] Build cache lookup failed: {}", e);
                }
            }
        }
    }

    let outcome = match backend {
        Backend::Auto => Err("internal error: unresolved auto backend".into()),
        Backend::C => build_with_c_backend(mir, output, opt_level, tuning, verbose),
        Backend::Llvm => build_with_llvm_backend(
            mir,
            output,
            opt_level,
            tuning,
            call_cache,
            llvm_specializations,
            allow_wsl_llvm,
            verbose,
        ),
        Backend::Jit => Err("`agamc build --backend jit` is not supported because the JIT executes in memory; use `agamc run --backend jit`".into()),
    }?;

    if let (Some(cache), Some(key)) = (&cache_store, &cache_key) {
        let artifact_kind = if outcome.native_binary {
            agam_runtime::cache::CacheArtifactKind::NativeBinary
        } else {
            match backend {
                Backend::C => agam_runtime::cache::CacheArtifactKind::CSource,
                Backend::Llvm => agam_runtime::cache::CacheArtifactKind::LlvmIr,
                Backend::Auto | Backend::Jit => {
                    agam_runtime::cache::CacheArtifactKind::NativeBinary
                }
            }
        };
        let artifact_path = if outcome.native_binary {
            output
        } else {
            &outcome.generated_path
        };

        if artifact_path.exists() {
            match cache.store_file(key, artifact_kind, path, artifact_path) {
                Ok(hit) => {
                    if verbose {
                        eprintln!("[agamc] Stored build artifact in cache: {}", hit.id);
                    }
                }
                Err(e) => {
                    if verbose {
                        eprintln!("[agamc] Failed to store build cache artifact: {}", e);
                    }
                }
            }
        }
    }

    Ok(outcome)
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
                        output.display(),
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
    llvm_specializations: &[agam_profile::CallCacheSpecializationPlan],
    allow_wsl_llvm: bool,
    verbose: bool,
) -> Result<BuildOutcome, String> {
    let target_config = resolve_llvm_target_config(tuning);
    let mut llvm_options = agam_codegen::llvm_emitter::LlvmEmitOptions::from_env();
    llvm_options.target_triple = target_config.target_triple.clone();
    llvm_options.call_cache = call_cache.resolved_enable_all();
    llvm_options.call_cache_only = call_cache.included_functions();
    llvm_options.call_cache_exclude = call_cache.excluded_functions();
    llvm_options.call_cache_optimize = call_cache.optimize_all;
    llvm_options.call_cache_optimize_only = call_cache.optimized_functions();
    let llvm_options = agam_codegen::llvm_emitter::LlvmEmitOptions {
        target_triple: llvm_options.target_triple,
        data_layout: llvm_options.data_layout,
        call_cache: call_cache.resolved_enable_all(),
        call_cache_only: call_cache.included_functions(),
        call_cache_exclude: call_cache.excluded_functions(),
        call_cache_optimize: call_cache.optimize_all,
        call_cache_optimize_only: call_cache.optimized_functions(),
        call_cache_specializations: llvm_specializations.to_vec(),
        call_cache_capacity: llvm_options.call_cache_capacity,
        call_cache_warmup: llvm_options.call_cache_warmup,
    };
    if verbose {
        let analysis = agam_codegen::llvm_emitter::analyze_call_cache(mir, &llvm_options);
        log_call_cache_analysis("LLVM", call_cache, &analysis);
    }
    let llvm_ir = agam_codegen::llvm_emitter::emit_llvm_with_options(mir, llvm_options)?;
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
    let clang_command = configured_llvm_clang();
    let manual_args =
        build_native_llvm_clang_args(&ll_path, output, opt_level, tuning, &target_config);
    if verbose {
        eprintln!("[agamc] LLVM driver: {}", clang_command);
        if let Some(target) = target_config.target_triple.as_ref() {
            eprintln!("[agamc] LLVM target: {}", target);
        }
        if let Some(sysroot) = target_config.sysroot.as_ref() {
            eprintln!("[agamc] LLVM sysroot: {}", sysroot.display());
        }
    }
    let toolchain = if allow_wsl_llvm {
        resolve_llvm_run_toolchain()
    } else {
        resolve_native_llvm_toolchain()
    };
    if matches!(toolchain, None) {
        eprintln!(
            "\x1b[1;33mwarning\x1b[0m: native LLVM driver not found, generated LLVM IR: {}",
            ll_path.display()
        );
        if cfg!(windows) && wsl_command_exists("clang") && !allow_wsl_llvm {
            let native_hint = windows_native_llvm_install_hint().unwrap_or_else(|| {
                format!(
                    "install a native LLVM/Clang toolchain or set `{LLVM_CLANG_ENV}` to `clang` or `clang++`"
                )
            });
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: native Windows LLVM build/run requires a native Windows clang toolchain; {native_hint}. For development-only WSL execution, set {DEV_WSL_LLVM_ENV}=1 for `agamc run --backend llvm`"
            );
        } else {
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: compile manually with: {}",
                render_shellish_command(&clang_command, &manual_args)
            );
        }
        return Ok(BuildOutcome {
            native_binary: false,
            generated_path: ll_path,
        });
    }

    let result = match toolchain.expect("toolchain checked above") {
        LlvmToolchain::Native => {
            let args =
                build_native_llvm_clang_args(&ll_path, output, opt_level, tuning, &target_config);
            std::process::Command::new(&clang_command)
                .args(&args)
                .output()
        }
        LlvmToolchain::Wsl => {
            let ll_wsl = path_to_wsl(&ll_path)?;
            let output_wsl = path_to_wsl(output)?;
            let mut args = vec![
                "clang".to_string(),
                ll_wsl,
                "-o".into(),
                output_wsl,
                opt_flag.clone(),
            ];
            if let Some(target) = target_config.target_triple.as_ref() {
                args.push(format!("--target={target}"));
            }
            if let Some(sysroot) = target_config.sysroot.as_ref() {
                args.push(format!("--sysroot={}", path_to_wsl(sysroot)?));
            }
            if let Some(sdk_root) = target_config.sdk_root.as_ref() {
                if matches!(
                    target_config.platform,
                    LlvmTargetPlatform::MacOs | LlvmTargetPlatform::Ios
                ) {
                    args.push("-isysroot".into());
                    args.push(path_to_wsl(sdk_root)?);
                }
            }
            if let Some(lto) = tuning.lto {
                args.push(lto_flag(lto).into());
            }
            if let Some(dir) = &tuning.pgo_generate {
                args.push(format!("-fprofile-generate={}", path_to_wsl(dir)?));
            }
            if let Some(profile) = &tuning.pgo_use {
                args.push(format!("-fprofile-use={}", path_to_wsl(profile)?));
            }
            if tuning.native_cpu {
                args.push("-march=native".into());
                args.push("-mtune=native".into());
            }
            if llvm_math_link_required(target_config.platform) {
                args.push("-lm".into());
            }
            if verbose {
                eprintln!("[agamc] LLVM native compilation via dev-only WSL clang fallback");
            }
            std::process::Command::new("wsl").args(&args).output()
        }
    };

    match result {
        Ok(out) => {
            if !out.status.success() {
                let stderr = String::from_utf8_lossy(&out.stderr);
                return Err(format!("LLVM compilation failed:\n{}", stderr));
            }
            Ok(BuildOutcome {
                native_binary: true,
                generated_path: ll_path,
            })
        }
        Err(_) => {
            eprintln!(
                "\x1b[1;33mwarning\x1b[0m: native LLVM driver not found, generated LLVM IR: {}",
                ll_path.display()
            );
            eprintln!(
                "\x1b[1;32minfo\x1b[0m: compile manually with: {}",
                render_shellish_command(&clang_command, &manual_args)
            );
            Ok(BuildOutcome {
                native_binary: false,
                generated_path: ll_path,
            })
        }
    }
}

fn llvm_profile_capture_path(output: &PathBuf) -> PathBuf {
    output.with_extension("llvm_call_profile.txt")
}

fn path_to_wsl(path: &std::path::Path) -> Result<String, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| {
                format!(
                    "failed to resolve current directory for `{}`: {}",
                    path.display(),
                    e
                )
            })?
            .join(path)
    };
    let rendered = absolute.to_string_lossy().replace('\\', "/");
    let bytes = rendered.as_bytes();
    if bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        Ok(format!("/mnt/{drive}/{}", &rendered[3..]))
    } else {
        Err(format!(
            "cannot translate path `{}` into a WSL mount path",
            absolute.display()
        ))
    }
}

fn run_with_llvm(
    path: &PathBuf,
    args: &[String],
    opt_level: u8,
    tuning: &ReleaseTuning,
    verbose: bool,
    features: FeatureFlags,
) -> Result<i32, String> {
    let allow_dev_wsl_llvm = allow_dev_wsl_llvm();
    let (mir, source_features) = lower_to_optimized_mir(path, verbose)?;
    let call_cache = effective_call_cache_selection(features, &source_features);
    let persisted_profile = if call_cache.is_enabled() {
        load_persisted_llvm_profile(path, &mir, &call_cache, verbose)
    } else {
        None
    };
    let (effective_call_cache, persisted_promotions) =
        apply_persisted_optimize_profile(&call_cache, persisted_profile.as_ref());
    let specialization_plans =
        apply_persisted_specialization_profile(&effective_call_cache, persisted_profile.as_ref());

    if verbose {
        if let Some(profile) = persisted_profile.as_ref() {
            eprintln!(
                "[agamc] Loaded persisted LLVM profile: {} run(s), {} function(s), {} total call(s)",
                profile.runs,
                profile.functions.len(),
                profile.total_calls
            );
            if !persisted_promotions.is_empty() {
                eprintln!(
                    "[agamc]   pre-promoted {} function(s) from prior runs: {}",
                    persisted_promotions.len(),
                    persisted_promotions.join(", ")
                );
            }
            if !specialization_plans.is_empty() {
                let rendered = specialization_plans
                    .iter()
                    .map(|plan| {
                        let slots = plan
                            .stable_values
                            .iter()
                            .map(|value| format!("arg{}=0x{:X}", value.index, value.raw_bits))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{} [{}]", plan.name, slots)
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                eprintln!(
                    "[agamc]   prepared {} guarded LLVM specialization clone(s): {}",
                    specialization_plans.len(),
                    rendered
                );
            }
        }
        if matches!(resolve_llvm_run_toolchain(), Some(LlvmToolchain::Wsl)) {
            eprintln!("[agamc] Executing LLVM backend through dev-only WSL fallback");
        }
    }

    let exe_path = path.with_extension("exe");
    let outcome = build_prelowered_file(
        path,
        &exe_path,
        opt_level,
        Backend::Llvm,
        tuning,
        &mir,
        &effective_call_cache,
        &specialization_plans,
        allow_dev_wsl_llvm,
        verbose,
    )?;
    if !outcome.native_binary {
        if cfg!(windows) && wsl_command_exists("clang") && !allow_dev_wsl_llvm {
            let native_hint = windows_native_llvm_install_hint().unwrap_or_else(|| {
                format!(
                    "install a native LLVM/Clang toolchain or set `{LLVM_CLANG_ENV}` to `clang` or `clang++`"
                )
            });
            return Err(format!(
                "LLVM run requires a native Windows clang toolchain; {native_hint}. For development-only WSL execution, set {DEV_WSL_LLVM_ENV}=1 to opt into the WSL clang fallback for `agamc run --backend llvm`"
            ));
        }
        return Err(format!(
            "backend {:?} emitted {} but no native executable was produced",
            Backend::Llvm,
            outcome.generated_path.display()
        ));
    }

    let profile_capture = llvm_profile_capture_path(&exe_path);
    let _ = std::fs::remove_file(&profile_capture);
    let toolchain = resolve_llvm_run_toolchain();
    let mut command = match toolchain {
        Some(LlvmToolchain::Wsl) => {
            let exe_wsl = path_to_wsl(&exe_path)?;
            let mut command = std::process::Command::new("wsl");
            if effective_call_cache.is_enabled() {
                let profile_wsl = path_to_wsl(&profile_capture)?;
                command.arg("env");
                command.arg(format!("AGAM_LLVM_CALL_CACHE_PROFILE_OUT={profile_wsl}"));
            }
            command.arg(exe_wsl);
            command
        }
        _ => {
            let mut command = std::process::Command::new(&exe_path);
            if effective_call_cache.is_enabled() {
                command.env("AGAM_LLVM_CALL_CACHE_PROFILE_OUT", &profile_capture);
            }
            command
        }
    };
    command.args(args);
    let status = command
        .status()
        .map_err(|e| format!("failed to run {}: {}", exe_path.display(), e))?;
    let exit_code = status.code().unwrap_or(1);

    if effective_call_cache.is_enabled() {
        match std::fs::read_to_string(&profile_capture) {
            Ok(profile_text) => match parse_llvm_call_cache_run_profile(&profile_text) {
                Ok(run_profile) => {
                    if verbose {
                        eprintln!(
                            "[agamc] LLVM call cache: {} hits / {} calls across {} cacheable function(s), {} store(s)",
                            run_profile.total_hits,
                            run_profile.total_calls,
                            run_profile.functions.len(),
                            run_profile.total_stores
                        );
                        for function in &run_profile.functions {
                            if function.calls > 0 || function.stores > 0 {
                                eprintln!(
                                    "[agamc]   {} -> calls={}, hits={}, stores={}, entries={}",
                                    function.name,
                                    function.calls,
                                    function.hits,
                                    function.stores,
                                    function.entries
                                );
                                if function.profile.avg_reuse_distance.is_some()
                                    || function.profile.max_reuse_distance.is_some()
                                {
                                    let avg_reuse = function
                                        .profile
                                        .avg_reuse_distance
                                        .map(|value| value.to_string())
                                        .unwrap_or_else(|| "n/a".into());
                                    let max_reuse = function
                                        .profile
                                        .max_reuse_distance
                                        .map(|value| value.to_string())
                                        .unwrap_or_else(|| "n/a".into());
                                    eprintln!(
                                        "[agamc]      reuse distance: avg={}, max={}",
                                        avg_reuse, max_reuse
                                    );
                                }
                                if !function.profile.stable_values.is_empty() {
                                    let stable = function
                                        .profile
                                        .stable_values
                                        .iter()
                                        .map(|value| {
                                            format!(
                                                "arg{}=0x{:X} (score {})",
                                                value.index, value.raw_bits, value.matches
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    eprintln!("[agamc]      stable scalars: {}", stable);
                                }
                                let specialization_attempts =
                                    function.profile.specialization_guard_hits.saturating_add(
                                        function.profile.specialization_guard_fallbacks,
                                    );
                                if specialization_attempts > 0 {
                                    let hit_rate = function
                                        .profile
                                        .specialization_guard_hits
                                        .saturating_mul(100)
                                        / specialization_attempts.max(1);
                                    eprintln!(
                                        "[agamc]      specialization guard: hits={}, fallbacks={}, matched={}%",
                                        function.profile.specialization_guard_hits,
                                        function.profile.specialization_guard_fallbacks,
                                        hit_rate
                                    );
                                }
                                if !matches!(
                                    function.profile.specialization_hint,
                                    agam_profile::CallCacheSpecializationHint::None
                                ) {
                                    eprintln!(
                                        "[agamc]      specialization hint: {}",
                                        function.profile.specialization_hint
                                    );
                                }
                            }
                        }
                    }
                    let merged_profile =
                        agam_profile::merge_persistent_profile(persisted_profile, &run_profile);
                    store_persisted_llvm_profile(path, &mir, &call_cache, &merged_profile, verbose);
                }
                Err(e) => {
                    if verbose {
                        eprintln!(
                            "[agamc] Failed to parse LLVM call-cache profile `{}`: {}",
                            profile_capture.display(),
                            e
                        );
                    }
                }
            },
            Err(e) => {
                if verbose
                    && effective_call_cache.is_enabled()
                    && e.kind() != std::io::ErrorKind::NotFound
                {
                    eprintln!(
                        "[agamc] Failed to read LLVM call-cache profile `{}`: {}",
                        profile_capture.display(),
                        e
                    );
                }
            }
        }
        let _ = std::fs::remove_file(&profile_capture);
    }

    Ok(exit_code)
}

fn run_with_jit(
    path: &PathBuf,
    args: &[String],
    verbose: bool,
    features: FeatureFlags,
) -> Result<i32, String> {
    let (mir, source_features) = lower_to_optimized_mir(path, verbose)?;
    let call_cache = effective_call_cache_selection(features, &source_features);
    let persisted_profile = if call_cache.is_enabled() {
        load_persisted_jit_profile(path, &mir, &call_cache, verbose)
    } else {
        None
    };
    let (effective_call_cache, persisted_promotions) =
        apply_persisted_optimize_profile(&call_cache, persisted_profile.as_ref());
    let specialization_plans =
        apply_persisted_specialization_profile(&effective_call_cache, persisted_profile.as_ref());
    let jit_options = agam_jit::JitOptions {
        call_cache: effective_call_cache.resolved_enable_all(),
        call_cache_only: effective_call_cache.included_functions(),
        call_cache_exclude: effective_call_cache.excluded_functions(),
        call_cache_optimize: effective_call_cache.optimize_all,
        call_cache_optimize_only: effective_call_cache.optimized_functions(),
        call_cache_specializations: specialization_plans.clone(),
        ..Default::default()
    };

    if verbose {
        if let Some(profile) = persisted_profile.as_ref() {
            eprintln!(
                "[agamc] Loaded persisted JIT profile: {} run(s), {} function(s), {} total call(s)",
                profile.runs,
                profile.functions.len(),
                profile.total_calls
            );
            if !persisted_promotions.is_empty() {
                eprintln!(
                    "[agamc]   pre-promoted {} function(s) from prior runs: {}",
                    persisted_promotions.len(),
                    persisted_promotions.join(", ")
                );
            }
            if !specialization_plans.is_empty() {
                let rendered = specialization_plans
                    .iter()
                    .map(|plan| {
                        let slots = plan
                            .stable_values
                            .iter()
                            .map(|value| format!("arg{}=0x{:X}", value.index, value.raw_bits))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("{} [{}]", plan.name, slots)
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                eprintln!(
                    "[agamc]   prepared {} guarded specialization clone(s): {}",
                    specialization_plans.len(),
                    rendered
                );
            }
        }
        let analysis = agam_jit::analyze_call_cache(&mir, &jit_options);
        log_call_cache_analysis("JIT", &effective_call_cache, &analysis);
        eprintln!("[agamc] Executing via Cranelift JIT");
    }
    let result = agam_jit::run_main_with_options(&mir, args, jit_options);
    if effective_call_cache.is_enabled() {
        let stats = agam_jit::take_last_call_cache_stats();
        if verbose {
            if let Some(stats) = stats.as_ref() {
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
                        function.name,
                        function.calls,
                        function.hits,
                        function.stores,
                        function.entries
                    );
                    if function.profile.unique_keys > 0 {
                        let avg_reuse = function
                            .profile
                            .avg_reuse_distance
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "n/a".into());
                        let max_reuse = function
                            .profile
                            .max_reuse_distance
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "n/a".into());
                        eprintln!(
                            "[agamc]      profile: unique_keys={}, hottest_key_hits={}, avg_reuse_distance={}, max_reuse_distance={}",
                            function.profile.unique_keys,
                            function.profile.hottest_key_hits,
                            avg_reuse,
                            max_reuse
                        );
                    }
                    if !function.profile.stable_values.is_empty() {
                        let stable = function
                            .profile
                            .stable_values
                            .iter()
                            .map(|value| {
                                format!(
                                    "arg{}=0x{:X} ({} matches)",
                                    value.index, value.raw_bits, value.matches
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        eprintln!("[agamc]      stable scalars: {}", stable);
                    }
                    let specialization_attempts = function
                        .profile
                        .specialization_guard_hits
                        .saturating_add(function.profile.specialization_guard_fallbacks);
                    if specialization_attempts > 0 {
                        let hit_rate = function
                            .profile
                            .specialization_guard_hits
                            .saturating_mul(100)
                            / specialization_attempts.max(1);
                        eprintln!(
                            "[agamc]      specialization guard: hits={}, fallbacks={}, matched={}%",
                            function.profile.specialization_guard_hits,
                            function.profile.specialization_guard_fallbacks,
                            hit_rate
                        );
                    }
                    if !matches!(
                        function.profile.specialization_hint,
                        agam_profile::CallCacheSpecializationHint::None
                    ) {
                        eprintln!(
                            "[agamc]      specialization hint: {}",
                            function.profile.specialization_hint
                        );
                    }
                }
            }
        }
        if result.is_ok() {
            if let Some(stats) = stats.as_ref() {
                let run_profile = jit_stats_to_run_profile(stats);
                let merged_profile =
                    agam_profile::merge_persistent_profile(persisted_profile, &run_profile);
                store_persisted_jit_profile(path, &mir, &call_cache, &merged_profile, verbose);
            }
        }
    }
    result
}

fn validate_release_tuning(backend: Backend, tuning: &ReleaseTuning) -> Result<(), String> {
    let requested_release_tuning =
        tuning.lto.is_some() || tuning.pgo_generate.is_some() || tuning.pgo_use.is_some();
    let requested_target = tuning.target.is_some();
    if !requested_release_tuning && !requested_target {
        return validate_llvm_target_config(tuning);
    }
    if backend != Backend::Llvm && (requested_release_tuning || requested_target) {
        return Err(
            "Phase 14/15 LLVM tuning flags (`--target`, `--lto`, `--pgo-generate`, `--pgo-use`) currently require `--backend llvm`"
                .into(),
        );
    }
    if tuning.pgo_generate.is_some() && tuning.pgo_use.is_some() {
        return Err("use either `--pgo-generate` or `--pgo-use`, not both in one build".into());
    }
    validate_llvm_target_config(tuning)
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
    use std::fs;

    fn parse_source_features(source: &str) -> SourceFeatureFlags {
        let tokens = agam_lexer::tokenize(source, SourceId(0));
        let mut features = source_feature_flags_from_tokens(&tokens);
        let module = agam_parser::parse(tokens, SourceId(0)).expect("source should parse");
        merge_function_call_cache_annotations(&module, &mut features.call_cache);
        features
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "agam_driver_{prefix}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn test_sanitize_project_name_collapses_non_identifier_runs() {
        assert_eq!(sanitize_project_name("  Hello__Agam!!  "), "hello-agam");
        assert_eq!(sanitize_project_name("###"), "agam-app");
    }

    #[test]
    fn test_scaffold_project_layout_creates_first_party_files() {
        let root = temp_dir("scaffold");
        let project_root = root.join("hello-app");

        let scaffold =
            scaffold_project_layout(&project_root, false, false).expect("scaffold should work");

        assert_eq!(scaffold.manifest_path, project_root.join("agam.toml"));
        assert_eq!(
            scaffold.entry_file,
            project_root.join("src").join("main.agam")
        );
        assert!(project_root.join("tests").join("smoke.agam").is_file());
        assert!(
            fs::read_to_string(project_root.join("agam.toml"))
                .expect("read manifest")
                .contains("name = \"hello-app\"")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_resolve_workspace_layout_uses_manifest_root_entry_and_tests() {
        let root = temp_dir("workspace");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        let test_file = root.join("tests").join("smoke.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        fs::create_dir_all(test_file.parent().expect("test parent")).expect("create tests");
        fs::write(&manifest, render_project_manifest("workspace")).expect("write manifest");
        fs::write(&entry, render_project_entry("workspace")).expect("write entry");
        fs::write(&test_file, render_project_smoke_test()).expect("write test");

        let layout =
            resolve_workspace_layout(Some(root.clone())).expect("workspace layout should resolve");

        assert_eq!(layout.root, root);
        assert_eq!(layout.manifest_path.as_ref(), Some(&manifest));
        assert_eq!(layout.entry_file, entry);
        assert_eq!(layout.test_files, vec![test_file]);

        let _ = fs::remove_dir_all(layout.root);
    }

    #[test]
    fn test_resolve_workspace_layout_supports_single_source_file_without_manifest() {
        let root = temp_dir("single_file");
        let file = root.join("script.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        let layout =
            resolve_workspace_layout(Some(file.clone())).expect("single source should resolve");

        assert!(layout.manifest_path.is_none());
        assert_eq!(layout.entry_file, file);
        assert_eq!(layout.source_files, vec![layout.entry_file.clone()]);

        let _ = fs::remove_dir_all(root);
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
        assert!(!features.call_cache.disable_all);
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
        assert!(!features.call_cache.disable_all);
        assert!(features.call_cache.include_functions.contains("hot"));
        assert!(features.call_cache.exclude_functions.is_empty());
    }

    #[test]
    fn test_source_no_call_cache_disables_automatic_service() {
        let features = parse_source_features(
            r#"
@lang.advance
@lang.feat.no_call_cache

fn main() -> i32 { return 0; }
"#,
        );

        assert!(features.call_cache.disable_all);
        assert!(!features.call_cache.enable_all);
        assert!(!features.call_cache.optimize_all);
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
        assert!(!features.call_cache.disable_all);
        assert!(features.call_cache.optimize_all);
        assert!(features.call_cache.optimize_functions.contains("hot"));
        assert_eq!(features.experimental_usages.len(), 2);
    }

    #[test]
    fn test_persisted_profile_prepromotes_selectable_hot_functions() {
        let profile = agam_profile::PersistentCallCacheProfile {
            schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 48,
            total_stores: 2,
            functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
                name: "hot".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 24,
                total_stores: 1,
                last_entries: 1,
                profile: agam_profile::CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 32,
                    avg_reuse_distance: Some(1),
                    max_reuse_distance: Some(1),
                    stable_values: vec![agam_profile::StableScalarValueProfile {
                        index: 0,
                        raw_bits: 33,
                        matches: 32,
                    }],
                    specialization_hint:
                        agam_profile::CallCacheSpecializationHint::StableArgumentsAndHotKey {
                            slots: vec![0],
                            hits: 32,
                            unique_keys: 1,
                        },
                    ..Default::default()
                },
            }],
        };

        let (selection, promoted) =
            apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));

        assert_eq!(promoted, vec!["hot".to_string()]);
        assert!(selection.optimize_functions.contains("hot"));
    }

    #[test]
    fn test_persisted_profile_respects_disabled_automatic_service_and_exclusions() {
        let profile = agam_profile::PersistentCallCacheProfile {
            schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 1,
            total_calls: 32,
            total_hits: 24,
            total_stores: 1,
            functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
                name: "hot".into(),
                runs: 1,
                total_calls: 32,
                total_hits: 24,
                total_stores: 1,
                last_entries: 1,
                profile: agam_profile::CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 32,
                    avg_reuse_distance: Some(1),
                    max_reuse_distance: Some(1),
                    stable_values: vec![],
                    specialization_hint: agam_profile::CallCacheSpecializationHint::HotKey {
                        hits: 32,
                        unique_keys: 1,
                    },
                    ..Default::default()
                },
            }],
        };

        let selection = CallCacheSelection {
            disable_all: true,
            exclude_functions: ["hot".to_string()].into_iter().collect(),
            ..Default::default()
        };

        let (selection, promoted) = apply_persisted_optimize_profile(&selection, Some(&profile));

        assert!(promoted.is_empty());
        assert!(selection.optimize_functions.is_empty());
    }

    #[test]
    fn test_persisted_profile_builds_specialization_plans_for_optimized_functions() {
        let profile = agam_profile::PersistentCallCacheProfile {
            schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 48,
            total_stores: 2,
            functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
                name: "hot".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 24,
                total_stores: 1,
                last_entries: 1,
                profile: agam_profile::CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 32,
                    avg_reuse_distance: Some(1),
                    max_reuse_distance: Some(1),
                    stable_values: vec![
                        agam_profile::StableScalarValueProfile {
                            index: 0,
                            raw_bits: 33,
                            matches: 24,
                        },
                        agam_profile::StableScalarValueProfile {
                            index: 1,
                            raw_bits: 7,
                            matches: 18,
                        },
                    ],
                    specialization_hint:
                        agam_profile::CallCacheSpecializationHint::StableArgumentsAndHotKey {
                            slots: vec![0, 1],
                            hits: 32,
                            unique_keys: 1,
                        },
                    ..Default::default()
                },
            }],
        };

        let (selection, _) =
            apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));
        let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].name, "hot");
        assert_eq!(plans[0].stable_values.len(), 2);
        assert_eq!(plans[1].stable_values.len(), 1);
        assert_eq!(plans[1].stable_values[0].raw_bits, 33);
    }

    #[test]
    fn test_parse_llvm_call_cache_run_profile() {
        let profile = parse_llvm_call_cache_run_profile(
            "AGAM_LLVM_CALL_CACHE_PROFILE_V4\nFN\thot\t32\t24\t2\t1\nSP\thot\t12\t4\nSV\thot\t0\t33\t24\nRD\thot\t1\t3\t24\nFN\twarm\t8\t0\t0\t0\nSP\twarm\t0\t0\nSV\twarm\t0\t7\t0\nRD\twarm\t0\t0\t0\n",
        )
        .expect("profile should parse");

        assert_eq!(profile.backend, "llvm");
        assert_eq!(profile.total_calls, 40);
        assert_eq!(profile.total_hits, 24);
        assert_eq!(profile.total_stores, 2);
        assert_eq!(profile.functions.len(), 2);
        assert_eq!(profile.functions[0].name, "hot");
        assert_eq!(profile.functions[0].entries, 1);
        assert_eq!(profile.functions[0].profile.stable_values.len(), 1);
        assert_eq!(profile.functions[0].profile.stable_values[0].raw_bits, 33);
        assert_eq!(profile.functions[0].profile.avg_reuse_distance, Some(1));
        assert_eq!(profile.functions[0].profile.max_reuse_distance, Some(3));
        assert_eq!(profile.functions[0].profile.specialization_guard_hits, 12);
        assert_eq!(
            profile.functions[0].profile.specialization_guard_fallbacks,
            4
        );
    }

    #[test]
    fn test_persisted_profile_skips_unfavorable_specialization_feedback() {
        let profile = agam_profile::PersistentCallCacheProfile {
            schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 48,
            total_stores: 2,
            functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
                name: "hot".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 24,
                total_stores: 1,
                last_entries: 1,
                profile: agam_profile::CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 32,
                    avg_reuse_distance: Some(1),
                    max_reuse_distance: Some(1),
                    stable_values: vec![agam_profile::StableScalarValueProfile {
                        index: 0,
                        raw_bits: 33,
                        matches: 24,
                    }],
                    specialization_guard_hits: 1,
                    specialization_guard_fallbacks: 15,
                    specialization_hint:
                        agam_profile::CallCacheSpecializationHint::StableArgumentsAndHotKey {
                            slots: vec![0],
                            hits: 32,
                            unique_keys: 1,
                        },
                },
            }],
        };

        let (selection, _) =
            apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));
        let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

        assert!(plans.is_empty());
    }

    #[test]
    fn test_persisted_profile_does_not_prepromote_unfavorable_specialization_only_signal() {
        let profile = agam_profile::PersistentCallCacheProfile {
            schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 6,
            total_stores: 2,
            functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
                name: "thrashy".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 3,
                total_stores: 1,
                last_entries: 8,
                profile: agam_profile::CallCacheFunctionProfile {
                    unique_keys: 8,
                    hottest_key_hits: 6,
                    avg_reuse_distance: None,
                    max_reuse_distance: None,
                    stable_values: vec![agam_profile::StableScalarValueProfile {
                        index: 0,
                        raw_bits: 33,
                        matches: 12,
                    }],
                    specialization_guard_hits: 1,
                    specialization_guard_fallbacks: 15,
                    specialization_hint:
                        agam_profile::CallCacheSpecializationHint::StableArguments {
                            slots: vec![0],
                        },
                },
            }],
        };

        let (selection, promoted) =
            apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));

        assert!(promoted.is_empty());
        assert!(!selection.optimize_functions.contains("thrashy"));
    }

    #[test]
    fn test_persisted_profile_retains_specialization_from_favorable_feedback() {
        let profile = agam_profile::PersistentCallCacheProfile {
            schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 48,
            total_stores: 2,
            functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
                name: "retained".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 24,
                total_stores: 1,
                last_entries: 1,
                profile: agam_profile::CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 24,
                    avg_reuse_distance: Some(1),
                    max_reuse_distance: Some(1),
                    stable_values: vec![
                        agam_profile::StableScalarValueProfile {
                            index: 0,
                            raw_bits: 33,
                            matches: 4,
                        },
                        agam_profile::StableScalarValueProfile {
                            index: 1,
                            raw_bits: 7,
                            matches: 3,
                        },
                    ],
                    specialization_guard_hits: 12,
                    specialization_guard_fallbacks: 4,
                    specialization_hint:
                        agam_profile::CallCacheSpecializationHint::StableArguments {
                            slots: vec![0, 1],
                        },
                },
            }],
        };

        let (selection, promoted) =
            apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));
        let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

        assert_eq!(promoted, vec!["retained".to_string()]);
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].name, "retained");
        assert_eq!(plans[0].stable_values.len(), 2);
        assert_eq!(plans[1].stable_values[0].raw_bits, 33);
    }

    #[test]
    fn test_build_feature_signature_includes_cache_generation() {
        let signature = build_feature_signature(
            Backend::Llvm,
            &CallCacheSelection::default(),
            false,
            &ReleaseTuning::default(),
        );

        assert!(signature.contains("build_cache=compiler-build-v2"));
    }

    #[test]
    fn test_auto_run_backend_falls_back_to_jit_without_external_toolchains() {
        let resolved =
            resolve_backend_with_toolchains(Backend::Auto, true, false, false, false, false);
        assert_eq!(resolved, Backend::Jit);
    }

    #[test]
    fn test_auto_build_backend_keeps_aot_fallback_without_external_toolchains() {
        let resolved =
            resolve_backend_with_toolchains(Backend::Auto, false, false, false, false, false);
        assert_eq!(resolved, Backend::C);
    }

    #[test]
    fn test_auto_run_backend_ignores_wsl_llvm_without_dev_opt_in() {
        let resolved =
            resolve_backend_with_toolchains(Backend::Auto, true, false, true, false, false);
        assert_eq!(resolved, Backend::Jit);
    }

    #[test]
    fn test_auto_run_backend_accepts_wsl_llvm_with_dev_opt_in() {
        let resolved =
            resolve_backend_with_toolchains(Backend::Auto, true, false, true, true, false);
        assert_eq!(resolved, Backend::Llvm);
    }

    #[test]
    fn test_auto_build_backend_does_not_treat_wsl_llvm_as_native_aot_toolchain() {
        let resolved =
            resolve_backend_with_toolchains(Backend::Auto, false, false, true, true, false);
        assert_eq!(resolved, Backend::C);
    }

    #[test]
    fn test_default_native_binary_output_path_uses_target_platform_extension() {
        let windows = default_native_binary_output_path(
            Path::new("examples/hello.agam"),
            Some("x86_64-pc-windows-msvc"),
        );
        let linux = default_native_binary_output_path(
            Path::new("examples/hello.agam"),
            Some("x86_64-unknown-linux-gnu"),
        );

        assert_eq!(
            windows.file_name().and_then(|name| name.to_str()),
            Some("hello.exe")
        );
        assert_eq!(
            linux.file_name().and_then(|name| name.to_str()),
            Some("hello")
        );
    }

    #[test]
    fn test_default_sdk_distribution_output_dir_uses_host_platform() {
        let output = default_sdk_distribution_output_dir();
        assert_eq!(
            output,
            PathBuf::from("dist").join(current_host_sdk_platform())
        );
    }

    #[test]
    fn test_sdk_supported_targets_begin_with_host_native() {
        let targets = sdk_supported_targets();
        assert!(!targets.is_empty());
        assert_eq!(targets[0].name, "host-native");
        assert_eq!(
            targets[0].backend,
            agam_runtime::contract::RuntimeBackend::Llvm
        );
    }

    #[test]
    fn test_visual_studio_llvm_candidate_paths_include_expected_clang_locations() {
        let candidates = visual_studio_llvm_candidate_paths(Path::new("C:/VS"));

        assert_eq!(
            candidates[0],
            PathBuf::from("C:/VS/VC/Tools/Llvm/x64/bin/clang.exe")
        );
        assert_eq!(
            candidates[1],
            PathBuf::from("C:/VS/VC/Tools/Llvm/bin/clang.exe")
        );
    }

    #[test]
    fn test_standalone_windows_llvm_candidate_paths_include_program_files_layout() {
        let candidates =
            standalone_windows_llvm_candidate_paths(Path::new("C:/Program Files/LLVM"));

        let primary = if cfg!(windows) {
            PathBuf::from("C:/Program Files/LLVM/bin/clang.exe")
        } else {
            PathBuf::from("C:/Program Files/LLVM/bin/clang")
        };
        let secondary = if cfg!(windows) {
            PathBuf::from("C:/Program Files/LLVM/bin/clang++.exe")
        } else {
            PathBuf::from("C:/Program Files/LLVM/bin/clang++")
        };

        assert!(candidates.iter().any(|candidate| candidate == &primary));
        assert!(candidates.iter().any(|candidate| candidate == &secondary));
    }

    #[test]
    fn test_bundled_llvm_candidate_paths_include_packaged_toolchain_layout() {
        let root = Path::new("C:/agam");
        let candidates = bundled_llvm_candidate_paths(root);
        let expected = root
            .join("toolchains")
            .join("llvm")
            .join(bundled_llvm_platform_dir())
            .join("bin")
            .join(if cfg!(windows) { "clang.exe" } else { "clang" });

        assert!(candidates.iter().any(|candidate| candidate == &expected));
    }

    #[test]
    fn test_stage_llvm_bundle_into_sdk_accepts_bundle_root_layout() {
        let unique = format!(
            "agam_sdk_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should be monotonic enough")
                .as_nanos()
        );
        let temp_root = std::env::temp_dir().join(unique);
        let bundle_root = temp_root.join("bundle");
        let output_root = temp_root.join("out");
        let driver = bundle_root
            .join(bundled_llvm_platform_dir())
            .join("bin")
            .join(if cfg!(windows) { "clang.exe" } else { "clang" });
        std::fs::create_dir_all(driver.parent().expect("driver should have parent"))
            .expect("create bundle layout");
        std::fs::write(&driver, b"clang").expect("write fake driver");

        let staged = stage_llvm_bundle_into_sdk(&bundle_root, &output_root)
            .expect("bundle root layout should stage");

        assert_eq!(staged, output_root.join("toolchains").join("llvm"));
        assert!(
            staged
                .join(bundled_llvm_platform_dir())
                .join("bin")
                .join(if cfg!(windows) { "clang.exe" } else { "clang" })
                .is_file()
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_bundled_llvm_candidate_paths_support_bundle_root_override_layout() {
        let root = Path::new("C:/agam/toolchains/llvm");
        let candidates = bundled_llvm_candidate_paths(root);
        let expected = root
            .join(bundled_llvm_platform_dir())
            .join("bin")
            .join(if cfg!(windows) { "clang.exe" } else { "clang" });

        assert!(candidates.iter().any(|candidate| candidate == &expected));
    }

    #[test]
    fn test_native_llvm_clang_args_include_cross_target_and_sysroot() {
        let tuning = ReleaseTuning {
            target: Some("aarch64-linux-android21".into()),
            native_cpu: false,
            lto: Some(LtoMode::Thin),
            pgo_generate: None,
            pgo_use: None,
        };
        let target_config = LlvmTargetConfig {
            target_triple: tuning.target.clone(),
            platform: LlvmTargetPlatform::Android,
            sysroot: Some(PathBuf::from("/ndk/sysroot")),
            sdk_root: None,
        };

        let args = build_native_llvm_clang_args(
            Path::new("hello.ll"),
            Path::new("hello"),
            3,
            &tuning,
            &target_config,
        );

        assert!(
            args.iter()
                .any(|arg| arg == "--target=aarch64-linux-android21")
        );
        assert!(args.iter().any(|arg| arg == "--sysroot=/ndk/sysroot"));
        assert!(args.iter().any(|arg| arg == "-flto=thin"));
        assert!(args.iter().any(|arg| arg == "-lm"));
    }

    #[test]
    fn test_native_llvm_clang_args_omit_math_library_on_windows() {
        let tuning = ReleaseTuning {
            target: Some("x86_64-pc-windows-msvc".into()),
            native_cpu: false,
            lto: None,
            pgo_generate: None,
            pgo_use: None,
        };
        let target_config = LlvmTargetConfig {
            target_triple: tuning.target.clone(),
            platform: LlvmTargetPlatform::Windows,
            sysroot: None,
            sdk_root: None,
        };

        let args = build_native_llvm_clang_args(
            Path::new("hello.ll"),
            Path::new("hello.exe"),
            2,
            &tuning,
            &target_config,
        );

        assert!(!args.iter().any(|arg| arg == "-lm"));
    }

    #[test]
    fn test_validate_release_tuning_rejects_target_for_non_llvm_backend() {
        let tuning = ReleaseTuning {
            target: Some("x86_64-unknown-linux-gnu".into()),
            native_cpu: false,
            lto: None,
            pgo_generate: None,
            pgo_use: None,
        };

        let error =
            validate_release_tuning(Backend::C, &tuning).expect_err("target should require llvm");
        assert!(error.contains("--target"));
    }

    #[test]
    fn test_validate_release_tuning_rejects_native_cpu_for_cross_target() {
        let tuning = ReleaseTuning {
            target: Some("x86_64-unknown-linux-gnu".into()),
            native_cpu: true,
            lto: None,
            pgo_generate: None,
            pgo_use: None,
        };

        let error = validate_release_tuning(Backend::Llvm, &tuning)
            .expect_err("cross target should reject native cpu");
        assert!(error.contains("--fast"));
    }
}
