//! CLI definitions for the Agam compiler driver.
//!
//! All Clap-derived command-line interface types live here,
//! separated from the implementation for maintainability.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Default daemon poll interval in milliseconds.
pub(crate) const DAEMON_DEFAULT_POLL_MS: u64 = 1_000;
/// The Agam programming language compiler.
#[derive(Parser, Debug)]
#[command(
    name = "agamc",
    version,
    about = "The Agam programming language compiler",
    long_about = "Agam â€” A natively compiled omni-language unifying Python's simplicity\nwith C++'s raw hardware control and Rust's memory safety."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub(crate) verbose: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum Backend {
    Auto,
    C,
    Llvm,
    Jit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum LtoMode {
    Thin,
    Full,
    ThinParallel,
    Distributed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum DependencyTable {
    Main,
    Dev,
    Build,
}

impl DependencyTable {
    pub(crate) fn manifest_label(self) -> &'static str {
        match self {
            Self::Main => "dependencies",
            Self::Dev => "dev-dependencies",
            Self::Build => "build-dependencies",
        }
    }
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// explicitly generate or refresh the workspace lockfile (`agam.lock`)
    Lock {
        /// Workspace root or manifest path (defaults to current directory)
        path: Option<PathBuf>,
    },

    /// Compile source files to a native binary
    Build {
        /// Source file(s) to compile
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Named project-local environment to apply
        #[arg(long)]
        env: Option<String>,

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

        /// Named project-local environment to apply
        #[arg(long)]
        env: Option<String>,

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

    /// Inspect, audit, install, update, and profile source packages in a registry index
    Registry {
        #[command(subcommand)]
        command: RegistryCommand,
    },

    /// List and inspect named project-local environments
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },

    /// Validate and publish a source package into a registry index
    Publish {
        /// Workspace root or manifest path to publish (defaults to current directory)
        path: Option<PathBuf>,

        /// Registry index root directory
        #[arg(long, value_name = "DIR")]
        index: PathBuf,

        /// Package owner handle recorded in the registry entry
        #[arg(long = "owner", value_name = "OWNER")]
        owners: Vec<String>,

        /// Optional publish-time description override
        #[arg(long)]
        description: Option<String>,

        /// Optional publish-time homepage URL
        #[arg(long)]
        homepage: Option<String>,

        /// Optional publish-time repository URL
        #[arg(long)]
        repository: Option<String>,

        /// Optional release download URL recorded in the registry entry
        #[arg(long)]
        download_url: Option<String>,

        /// Publish through the official first-party governance contract
        #[arg(long)]
        official: bool,

        /// Validate and print the publish contract without mutating the index
        #[arg(long)]
        dry_run: bool,
    },

    /// Inspect native backend and SDK readiness on the current machine
    Doctor {
        /// Workspace root or manifest path used for environment-aware diagnostics
        path: Option<PathBuf>,

        /// Named project-local environment to diagnose
        #[arg(long)]
        env: Option<String>,
    },

    /// Generate HTML/JSON documentation for the current package or source files
    Doc {
        /// Source file or workspace path (defaults to current directory)
        path: Option<PathBuf>,

        /// Output directory for rendered documentation (defaults to target/doc)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Open rendered HTML documentation in the default browser
        #[arg(long)]
        open: bool,

        /// Output documentation as structured JSON
        #[arg(long)]
        json: bool,
    },

    /// Extract and execute code examples from doc comments as test cases
    Doctest {
        /// Source file or workspace path (defaults to current directory)
        path: Option<PathBuf>,
    },

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

        /// Named project-local environment to apply
        #[arg(long)]
        env: Option<String>,

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

    /// Execute Agam source through the strict headless execution tool
    Exec {
        /// Read a strict JSON execution request from stdin and emit a JSON response
        #[arg(long)]
        json: bool,

        /// Pretty-print the JSON response
        #[arg(long)]
        pretty: bool,

        /// Read Agam source from a file instead of stdin
        #[arg(long, value_name = "FILE", conflicts_with = "source")]
        file: Option<PathBuf>,

        /// Execute an inline Agam source string instead of reading stdin
        #[arg(long, value_name = "SOURCE", conflicts_with = "file")]
        source: Option<String>,

        /// Filename reported in diagnostics and the temporary execution workspace
        #[arg(long)]
        filename: Option<String>,

        /// Code generation backend
        #[arg(long, value_enum, default_value_t = Backend::Jit)]
        backend: Backend,

        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "2")]
        opt_level: u8,

        /// Use the fastest current execution path
        #[arg(long)]
        fast: bool,

        /// Arguments passed to the executed program
        #[arg(long = "arg")]
        args: Vec<String>,

        /// Sandbox isolation level: "none", "process" (default), or "strict"
        #[arg(long, default_value = "process")]
        sandbox_level: String,

        /// Deny network access from executed programs
        #[arg(long)]
        deny_network: bool,

        /// Deny child process spawning from executed programs
        #[arg(long)]
        deny_process_spawn: bool,
    },

    /// Start the interactive REPL or execute one structured JSON request from stdin
    Repl {
        /// Read a strict JSON execution request from stdin and emit a JSON response
        #[arg(long)]
        json: bool,

        /// Pretty-print the JSON response when `--json` is enabled
        #[arg(long)]
        pretty: bool,
    },

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

    /// Start a persistent incremental compilation daemon
    Daemon {
        /// Workspace root, manifest path, or source file to keep warm (defaults to current directory)
        path: Option<PathBuf>,

        /// Run one warm-state refresh and exit
        #[arg(long)]
        once: bool,

        /// Poll interval in milliseconds while the foreground daemon is running
        #[arg(long, default_value_t = DAEMON_DEFAULT_POLL_MS)]
        poll_ms: u64,

        /// Internal flag: run as a background child process (not for direct user use)
        #[arg(long, hide = true)]
        background_child: bool,

        #[command(subcommand)]
        command: Option<DaemonCommand>,
    },

    /// Run tests
    Test {
        /// Source file(s) containing tests
        files: Vec<PathBuf>,

        /// Enable code coverage
        #[arg(long)]
        coverage: bool,
    },

    /// Start an MCP (Model Context Protocol) server for AI agent integration
    Mcp {
        #[command(subcommand)]
        command: Option<McpCommand>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum McpCommand {
    /// Start the standard stdio MCP JSON-RPC server
    Serve {
        /// Optional workspace root directory
        #[arg(long)]
        workspace: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum PackageCommand {
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
        /// Workspace root or manifest path used for environment-aware SDK metadata
        path: Option<PathBuf>,

        /// Named project-local environment to apply
        #[arg(long)]
        env: Option<String>,

        /// Output directory for the SDK distribution
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Optional bundled LLVM root to copy into the SDK
        #[arg(long, value_name = "DIR")]
        llvm_bundle: Option<PathBuf>,

        /// Optional Android sysroot directory to stage as a target pack
        #[arg(long, value_name = "DIR")]
        android_sysroot: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RegistryCommand {
    /// Inspect package metadata from a registry index
    Inspect {
        /// Registry index root directory
        #[arg(long, value_name = "DIR")]
        index: PathBuf,

        /// Package name recorded in the registry index
        #[arg(required = true)]
        name: String,
    },

    /// Print an audit-friendly release history for a registry package
    Audit {
        /// Registry index root directory
        #[arg(long, value_name = "DIR")]
        index: PathBuf,

        /// Package name recorded in the registry index
        #[arg(required = true)]
        name: String,
    },

    /// Add or pin a registry dependency in `agam.toml`
    Install {
        /// Registry index root directory
        #[arg(long, value_name = "DIR")]
        index: PathBuf,

        /// Workspace root or manifest path (defaults to current directory)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,

        /// Dependency table to update
        #[arg(long, value_enum, default_value_t = DependencyTable::Main)]
        table: DependencyTable,

        /// Package name recorded in the registry index
        #[arg(required = true)]
        name: String,

        /// Optional version requirement to resolve before pinning the selected release
        #[arg(long)]
        version: Option<String>,
    },

    /// Update one or more manifest dependencies from a registry index
    Update {
        /// Registry index root directory
        #[arg(long, value_name = "DIR")]
        index: PathBuf,

        /// Workspace root or manifest path (defaults to current directory)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,

        /// Dependency table to update
        #[arg(long, value_enum, default_value_t = DependencyTable::Main)]
        table: DependencyTable,

        /// Optional dependency keys or package names to update; defaults to all matching entries
        names: Vec<String>,
    },

    /// Mark or unmark a published registry release as yanked
    Yank {
        /// Registry index root directory
        #[arg(long, value_name = "DIR")]
        index: PathBuf,

        /// Package name recorded in the registry index
        #[arg(required = true)]
        name: String,

        /// Published package version to change
        #[arg(required = true)]
        version: String,

        /// Clear the yanked flag instead of setting it
        #[arg(long)]
        undo: bool,
    },

    /// Inspect curated first-party distribution profiles
    Profile {
        #[command(subcommand)]
        command: RegistryProfileCommand,
    },

    /// Print the official first-party package governance contract
    Governance,
}

#[derive(Subcommand, Debug)]
pub(crate) enum RegistryProfileCommand {
    /// List curated first-party distribution profiles
    List,

    /// Inspect one curated first-party distribution profile
    Inspect {
        /// Curated profile name
        #[arg(required = true)]
        name: String,
    },

    /// Install all recommended packages from one curated profile
    Install {
        /// Registry index root directory
        #[arg(long, value_name = "DIR")]
        index: PathBuf,

        /// Workspace root or manifest path (defaults to current directory)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,

        /// Dependency table to update
        #[arg(long, value_enum, default_value_t = DependencyTable::Main)]
        table: DependencyTable,

        /// Curated profile name
        #[arg(required = true)]
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum EnvCommand {
    /// List named environments declared in `agam.toml`
    List {
        /// Workspace root or manifest path (defaults to current directory)
        path: Option<PathBuf>,
    },

    /// Inspect one resolved environment view
    Inspect {
        /// Workspace root or manifest path (defaults to current directory)
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,

        /// Environment name to inspect; defaults to the implicit selection rules
        name: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum CacheCommand {
    /// Print aggregate cache statistics and recent entries
    Status {
        /// Workspace path, manifest path, or source path used to locate the cache
        path: Option<PathBuf>,

        /// Number of recent entries to show
        #[arg(long, default_value = "5")]
        recent: usize,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum DaemonCommand {
    /// Print background daemon status and cached pipeline health
    Status,

    /// Remove persisted daemon status metadata for a workspace
    Clear,

    /// Spawn a background daemon process for the workspace
    Start,

    /// Signal a running background daemon to shut down gracefully
    Stop,
}
