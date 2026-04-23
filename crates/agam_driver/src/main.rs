//! # agamc — The Agam Compiler
//!
//! Entry point for the Agam programming language toolchain.
//!
//! ## Subcommands
//!
//! - `build` — Compile source files to a native binary
//! - `run`   — Build and immediately execute
//! - `package` — Build, inspect, and run portable packages
//! - `publish` — Validate and publish source packages to a registry index
//! - `registry` — Inspect, audit, install, update, and profile source packages in a registry index
//! - `env` — List and inspect named project-local environments
//! - `check` — Type-check without generating code (fast)
//! - `new`   — Scaffold a first-party Agam project
//! - `dev`   — Run the first-party local development workflow
//! - `cache` — Inspect the local Agam build/package cache
//! - `repl`  — Interactive REPL
//! - `exec`  — Strict agent-facing headless execution tool
//! - `fmt`   — Format source files
//! - `lsp`   — Start the Language Server Protocol server
//! - `test`  — Run tests

use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::ffi::c_void;
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Stdio};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
use std::time::Duration;

use agam_ast::decl::DeclKind;
use agam_errors::{Diagnostic, DiagnosticEmitter, Label, SourceFile, SourceId, Span};
use agam_lexer::{Token, TokenKind};
use agam_notebook::{
    HeadlessExecutionBackend, HeadlessExecutionPolicy, HeadlessExecutionRequest,
    HeadlessExecutionResponse,
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum DependencyTable {
    Main,
    Dev,
    Build,
}

impl DependencyTable {
    fn manifest_label(self) -> &'static str {
        match self {
            Self::Main => "dependencies",
            Self::Dev => "dev-dependencies",
            Self::Build => "build-dependencies",
        }
    }
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

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
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

    fn caches_function(&self, function: &str) -> bool {
        if self.exclude_functions.contains(function) {
            return false;
        }

        self.resolved_enable_all()
            || self.optimize_all
            || self.include_functions.contains(function)
            || self.optimize_functions.contains(function)
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

const DAEMON_STATUS_SCHEMA_VERSION: u32 = 1;
const DAEMON_HEARTBEAT_STALE_MS: u128 = 5_000;
const DAEMON_DEFAULT_POLL_MS: u64 = 1_000;
const NESTED_BUILD_REQUEST_ENV: &str = "AGAM_NESTED_BUILD_REQUEST";
const NESTED_CHECK_REQUEST_ENV: &str = "AGAM_NESTED_CHECK_REQUEST";
const HEADLESS_EXEC_WORKER_ENV: &str = "AGAM_HEADLESS_EXEC_WORKER";
const HEADLESS_SANDBOX_ROOT_ENV: &str = "AGAM_HEADLESS_SANDBOX_ROOT";

#[derive(Clone, Debug, PartialEq, Eq)]
struct BuildRequest {
    file: PathBuf,
    output: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CheckRequest {
    file: PathBuf,
}

#[derive(Debug)]
struct BuildRequestResult {
    request: BuildRequest,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    succeeded: bool,
    launch_error: Option<String>,
}

#[derive(Debug)]
struct CheckRequestResult {
    request: CheckRequest,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    succeeded: bool,
    launch_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestRequest {
    file: PathBuf,
}

#[derive(Debug)]
#[allow(dead_code)]
struct TestRequestResult {
    request: TestRequest,
    summary: Option<agam_test::FileTestSummary>,
    error: Option<String>,
}

#[derive(Debug)]
struct DaemonPrewarmedEntry {
    package: agam_pkg::PortablePackage,
    call_cache: CallCacheSelection,
}

#[derive(Subcommand, Debug)]
enum Command {
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
enum RegistryCommand {
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
enum RegistryProfileCommand {
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
enum EnvCommand {
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

#[derive(Subcommand, Debug)]
enum DaemonCommand {
    /// Print background daemon status and cached pipeline health
    Status,

    /// Remove persisted daemon status metadata for a workspace
    Clear,

    /// Spawn a background daemon process for the workspace
    Start,

    /// Signal a running background daemon to shut down gracefully
    Stop,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct DaemonDiffSummary {
    pub added_files: usize,
    pub changed_files: usize,
    pub removed_files: usize,
    pub unchanged_files: usize,
    pub manifest_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum DaemonRunMode {
    OneShot,
    ForegroundLoop,
    BackgroundService,
}

impl Default for DaemonRunMode {
    fn default() -> Self {
        Self::ForegroundLoop
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DaemonStatusRecord {
    pub schema_version: u32,
    #[serde(default)]
    pub run_mode: DaemonRunMode,
    pub workspace_root: String,
    pub project_name: String,
    pub pid: u32,
    pub session_started_unix_ms: u128,
    pub last_heartbeat_unix_ms: u128,
    pub snapshot_file_count: usize,
    pub warmed_file_count: usize,
    pub warmed_version_count: usize,
    pub ast_decl_count: usize,
    pub hir_function_count: usize,
    pub mir_function_count: usize,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub prewarm: DaemonPrewarmSummary,
    pub last_diff: DaemonDiffSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DaemonIpcRequest {
    Status,
    GetWarmMir {
        file_path: String,
        content_hash: String,
    },
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DaemonIpcResponse {
    Status(DaemonStatusRecord),
    WarmMir {
        found: bool,
        mir_json: Option<String>,
        call_cache_json: Option<String>,
    },
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DaemonLiveness {
    Running,
    Snapshot,
    Stale,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct WarmSummary {
    pub warmed_files: usize,
    pub reused_files: usize,
    pub warmed_version_count: usize,
    pub ast_decl_count: usize,
    pub hir_function_count: usize,
    pub mir_function_count: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct WarmCacheSummary {
    pub file_count: usize,
    pub version_count: usize,
    pub ast_decl_count: usize,
    pub hir_function_count: usize,
    pub mir_function_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct DaemonPrewarmSummary {
    #[serde(default)]
    pub package_ready: bool,
    #[serde(default)]
    pub entry_path: Option<String>,
    #[serde(default)]
    pub entry_content_hash: Option<String>,
    #[serde(default)]
    pub package_artifact_path: Option<String>,
    #[serde(default)]
    pub call_cache: CallCacheSelection,
    #[serde(default)]
    pub build_ready: bool,
    #[serde(default)]
    pub build_backend: Option<String>,
    #[serde(default)]
    pub build_artifact_kind: Option<String>,
    #[serde(default)]
    pub prewarmed_file_count: usize,
    #[serde(default)]
    pub prewarmed_total_files: usize,
    #[serde(default)]
    pub last_error: Option<String>,
}

enum DaemonCycleOutcome {
    Success {
        status: DaemonStatusRecord,
        diff_summary: DaemonDiffSummary,
        prewarm_ran: bool,
    },
    Error {
        status: DaemonStatusRecord,
        error: String,
    },
}

/// Daemon-owned compilation state for a specific file version.
#[derive(Debug)]
struct WarmState {
    pub source_features: Option<SourceFeatureFlags>,
    pub module: Option<agam_ast::Module>,
    pub hir: Option<agam_hir::nodes::HirModule>,
    pub mir: Option<agam_mir::ir::MirModule>,
}

#[derive(Debug, Serialize)]
struct DaemonWarmArtifact<'a> {
    mir: &'a agam_mir::ir::MirModule,
    #[serde(skip_serializing_if = "Option::is_none")]
    call_cache: Option<&'a CallCacheSelection>,
}

#[derive(Debug, Deserialize)]
struct DaemonWarmArtifactOwned {
    mir: agam_mir::ir::MirModule,
    #[serde(default)]
    call_cache: Option<CallCacheSelection>,
}

/// Daemon pipeline and state owner.
#[derive(Debug, Default)]
struct DaemonSession {
    pub snapshot: Option<agam_pkg::WorkspaceSnapshot>,
    pub cache: BTreeMap<PathBuf, BTreeMap<String, WarmState>>,
    pub last_prewarm: DaemonPrewarmSummary,
}

/// Pipeline that takes a diff and reuses warm state where possible.
struct IncrementalPipeline<'a> {
    pub session: &'a mut DaemonSession,
}

impl<'a> IncrementalPipeline<'a> {
    pub fn new(session: &'a mut DaemonSession) -> Self {
        Self { session }
    }

    pub fn apply_diff(
        &mut self,
        next_snapshot: agam_pkg::WorkspaceSnapshot,
        diff: &agam_pkg::WorkspaceSnapshotDiff,
    ) {
        let manifest_changed = self
            .session
            .snapshot
            .as_ref()
            .map(|previous| snapshot_diff_touches_manifest(previous, &next_snapshot, diff))
            .unwrap_or(false);
        if manifest_changed {
            self.session.cache.clear();
            self.session.snapshot = Some(next_snapshot);
            return;
        }

        // Remove caches for deleted files entirely
        for removed in &diff.removed_files {
            self.session.cache.remove(removed);
        }

        // Keep the previous good cache for changed files until the replacement
        // version successfully warms. `warm_workspace_session` will clear the old
        // version only after a new hash has been built.

        // Unchanged files maintain their WarmState entries securely in `session.cache`.

        self.session.snapshot = Some(next_snapshot);
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Lock { path } => {
            let session = match resolve_workspace_session_for_driver(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not resolve workspace: {e}");
                    std::process::exit(1);
                }
            };
            if session.manifest.is_none() {
                eprintln!("error: no `agam.toml` manifest found in this directory");
                std::process::exit(1);
            }

            match agam_pkg::generate_or_refresh_lockfile(&session) {
                Ok(lockfile) => {
                    let manifest = session.manifest.as_ref().unwrap();
                    let diagnostics = agam_pkg::lockfile_diagnostics(manifest, &lockfile);
                    for d in diagnostics {
                        eprintln!("warning: {d}");
                    }
                    let drift = agam_pkg::lockfile_content_drift(&session.layout.root, &lockfile);
                    for (name, _, _) in drift {
                        eprintln!(
                            "warning: path dependency `{name}` has changed since lockfile was generated"
                        );
                    }
                    println!("locked {} package(s)", lockfile.packages.len());
                }
                Err(e) => {
                    eprintln!("error: failed to resolve dependencies: {e}");
                    std::process::exit(1);
                }
            }
        }
        Command::Build {
            files,
            env,
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
            let environment = match maybe_resolve_build_environment(&files, env.as_deref()) {
                Ok(environment) => environment,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };
            let target = selected_target_for_command(target, environment.as_ref());
            let requested_backend = requested_backend_for_command(
                backend,
                environment.as_ref(),
                false,
                target.as_deref(),
            );
            let opt_level = effective_opt_level(opt_level, fast);
            let backend = resolve_backend(requested_backend, false);
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
            if cli.verbose && !is_nested_build_request() {
                eprintln!("[agamc] Building {} file(s)...", files.len());
                if let Some(environment) = environment.as_ref() {
                    eprintln!(
                        "[agamc] Environment: {}",
                        environment_selection_label(environment)
                    );
                    if requested_backend_from_environment(&environment.environment, false).is_none()
                        && matches!(
                            environment.environment.preferred_backend,
                            Some(agam_runtime::contract::RuntimeBackend::Jit)
                        )
                    {
                        eprintln!(
                            "[agamc] Environment backend `jit` does not apply to `build`; using normal AOT backend selection"
                        );
                    }
                }
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

            // Lockfile refresh: attempt to resolve and refresh agam.lock for
            // the workspace containing the build input(s).
            if !is_nested_build_request() {
                if let Some(first_file) = files.first() {
                    match resolve_workspace_session_for_driver(Some(first_file.clone())) {
                        Ok(session) => {
                            if let Err(e) = try_lockfile_refresh(&session, cli.verbose) {
                                if cli.verbose {
                                    eprintln!("[agamc] lockfile warning: {e}");
                                }
                            }
                        }
                        Err(_) => {
                            // No resolvable workspace — skip lockfile.
                        }
                    }
                }
            }

            let build_requests =
                match resolve_build_requests(&files, output, tuning.target.as_deref()) {
                    Ok(requests) => requests,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };

            if build_requests.len() > 1 && !is_nested_build_request() {
                let parallelism = build_request_parallelism(build_requests.len());
                if cli.verbose {
                    eprintln!(
                        "[agamc] Scheduling {} independent build request(s) across {} worker(s)",
                        build_requests.len(),
                        parallelism
                    );
                }

                let results =
                    execute_build_requests_with_runner(&build_requests, parallelism, |request| {
                        run_nested_build_request(
                            request,
                            opt_level,
                            backend,
                            &tuning,
                            features,
                            cli.verbose,
                        )
                    });

                let mut had_errors = false;
                for result in results {
                    if let Err(error) = replay_build_request_output(&result) {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", error);
                        had_errors = true;
                    }
                    if !result.succeeded {
                        had_errors = true;
                    }
                }

                if had_errors {
                    process::exit(1);
                }
                return;
            }

            let mut had_errors = false;
            for request in build_requests {
                let file = &request.file;
                let out_path = &request.output;
                match build_file(
                    file,
                    out_path,
                    opt_level,
                    backend,
                    &tuning,
                    features,
                    cli.verbose,
                ) {
                    Ok(outcome) => {
                        if outcome.native_binary {
                            eprintln!(
                                "\x1b[1;32m✓\x1b[0m Built: {} -> {}",
                                file.display(),
                                out_path.display()
                            );
                            if outcome.generated_path != *out_path {
                                eprintln!(
                                    "\x1b[1;32minfo\x1b[0m: Generated IR: {}",
                                    outcome.generated_path.display()
                                );
                            }
                        } else {
                            eprintln!(
                                "\x1b[1;32m✓\x1b[0m Generated: {} -> {}",
                                file.display(),
                                outcome.generated_path.display()
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {} ({})", e, file.display());
                        had_errors = true;
                    }
                }
            }

            if had_errors {
                process::exit(1);
            }
        }

        Command::Run {
            file,
            env,
            backend,
            opt_level,
            fast,
            lto,
            pgo_generate,
            pgo_use,
            call_cache,
            args,
        } => {
            let environment =
                match maybe_resolve_workspace_environment(Some(file.clone()), env.as_deref()) {
                    Ok(environment) => environment,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };
            let opt_level = effective_opt_level(opt_level, fast);
            let requested_target = environment
                .as_ref()
                .and_then(|report| report.environment.target.clone());
            let requested_backend = requested_backend_for_command(
                backend,
                environment.as_ref(),
                true,
                requested_target.as_deref(),
            );
            let backend = resolve_backend(requested_backend, true);
            let tuning = ReleaseTuning {
                target: requested_target,
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
            let file = match resolve_entry_source_path(&file) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };
            if cli.verbose {
                eprintln!("[agamc] Running {}...", file.display());
                if let Some(environment) = environment.as_ref() {
                    eprintln!(
                        "[agamc] Environment: {}",
                        environment_selection_label(environment)
                    );
                }
                if !args.is_empty() {
                    eprintln!("[agamc] Args: {:?}", args);
                }
                if let Some(target) = tuning.target.as_ref() {
                    eprintln!("[agamc] Target: {}", target);
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
                let file = match resolve_entry_source_path(&file) {
                    Ok(file) => file,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };
                let output = match output {
                    Some(output) => output,
                    None => match default_package_output_path(&file) {
                        Ok(output) => output,
                        Err(e) => {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                            process::exit(1);
                        }
                    },
                };
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
                path,
                env,
                output,
                llvm_bundle,
                android_sysroot,
            } => {
                let environment =
                    match maybe_resolve_optional_workspace_environment(path, env.as_deref()) {
                        Ok(environment) => environment,
                        Err(e) => {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                            process::exit(1);
                        }
                    };
                let output = output.unwrap_or_else(default_sdk_distribution_output_dir);
                match package_sdk_distribution(
                    &output,
                    llvm_bundle.as_ref(),
                    android_sysroot.as_ref(),
                    environment.as_ref(),
                    cli.verbose,
                ) {
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
                        if let Some(android_sysroot_root) = outcome.android_sysroot_root.as_ref() {
                            eprintln!(
                                "\x1b[1;32minfo\x1b[0m: android target pack -> {}",
                                android_sysroot_root.display()
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

        Command::Registry { command } => match command {
            RegistryCommand::Inspect { index, name } => {
                match inspect_registry_package(&index, &name) {
                    Ok(report) => print_registry_inspect_report(&report, cli.verbose),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
            RegistryCommand::Audit { index, name } => {
                match audit_registry_index_package(&index, &name) {
                    Ok(report) => print_registry_audit_report(&report),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
            RegistryCommand::Install {
                index,
                path,
                table,
                name,
                version,
            } => match install_registry_dependency(
                path,
                &index,
                table,
                &name,
                version.as_deref(),
                cli.verbose,
            ) {
                Ok(report) => print_registry_install_report(&report, cli.verbose),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            RegistryCommand::Update {
                index,
                path,
                table,
                names,
            } => match update_registry_dependencies(path, &index, table, &names, cli.verbose) {
                Ok(report) => print_registry_update_report(&report, cli.verbose),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            RegistryCommand::Yank {
                index,
                name,
                version,
                undo,
            } => match yank_registry_release(&index, &name, &version, undo) {
                Ok(report) => print_registry_yank_report(&report),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            RegistryCommand::Profile { command } => match command {
                RegistryProfileCommand::List => {
                    print_registry_profile_list_report(&list_registry_profiles())
                }
                RegistryProfileCommand::Inspect { name } => match inspect_registry_profile(&name) {
                    Ok(report) => print_registry_profile_inspect_report(&report),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                },
                RegistryProfileCommand::Install {
                    index,
                    path,
                    table,
                    name,
                } => match install_registry_profile(path, &index, table, &name, cli.verbose) {
                    Ok(report) => print_registry_profile_install_report(&report, cli.verbose),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                },
            },
            RegistryCommand::Governance => {
                print_registry_governance_report(&registry_governance_report())
            }
        },

        Command::Env { command } => match command {
            EnvCommand::List { path } => match list_workspace_environments(path) {
                Ok(report) => print_environment_list_report(&report),
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            },
            EnvCommand::Inspect { path, name } => {
                match inspect_workspace_environment(path, name.as_deref()) {
                    Ok(report) => print_environment_inspect_report(&report),
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                }
            }
        },

        Command::Publish {
            path,
            index,
            owners,
            description,
            homepage,
            repository,
            download_url,
            official,
            dry_run,
        } => match publish_workspace_to_registry(
            path,
            &index,
            &owners,
            description.as_ref(),
            homepage.as_ref(),
            repository.as_ref(),
            download_url.as_ref(),
            official,
            dry_run,
            cli.verbose,
        ) {
            Ok(report) => print_publish_report(&report, cli.verbose),
            Err(e) => {
                eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                process::exit(1);
            }
        },

        Command::Doctor { path, env } => {
            let environment =
                match maybe_resolve_optional_workspace_environment(path, env.as_deref()) {
                    Ok(environment) => environment,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };
            match run_doctor(environment.as_ref(), cli.verbose) {
                Ok(healthy) => {
                    if !healthy {
                        process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        }

        Command::Check { files } => {
            let files = match agam_pkg::expand_agam_inputs(files) {
                Ok(files) => files,
                Err(e) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            };

            let nested_check = is_nested_check_request();
            if cli.verbose && !nested_check {
                eprintln!("[agamc] Checking {} file(s)...", files.len());
            }

            // Lockfile refresh for the workspace containing the check input(s).
            if !nested_check {
                if let Some(first_file) = files.first() {
                    match resolve_workspace_session_for_driver(Some(first_file.clone())) {
                        Ok(session) => {
                            if let Err(e) = try_lockfile_refresh(&session, cli.verbose) {
                                if cli.verbose {
                                    eprintln!("[agamc] lockfile warning: {e}");
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }
            }

            let mut had_errors = false;
            if !nested_check && files.len() > 1 {
                let requests = files
                    .iter()
                    .cloned()
                    .map(|file| CheckRequest { file })
                    .collect::<Vec<_>>();
                let results = execute_parallel_check_requests(&requests, cli.verbose);
                for result in &results {
                    match replay_check_request_output(result) {
                        Ok(succeeded) => had_errors |= !succeeded,
                        Err(error) => {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", error);
                            had_errors = true;
                        }
                    }
                }
            } else {
                for file in &files {
                    match run_check_request_locally(file, cli.verbose) {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                            had_errors = true;
                        }
                    }
                }
            }

            if had_errors {
                process::exit(1);
            } else if !nested_check {
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
            env,
            backend,
            opt_level,
            fix,
            no_run,
            no_tests,
        } => {
            let environment =
                match maybe_resolve_workspace_environment(path.clone(), env.as_deref()) {
                    Ok(environment) => environment,
                    Err(e) => {
                        eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                        process::exit(1);
                    }
                };
            if let Err(e) = run_dev_workflow(
                path,
                environment,
                backend,
                opt_level,
                fix,
                no_run,
                no_tests,
                cli.verbose,
            ) {
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

        Command::Exec {
            json,
            pretty,
            file,
            source,
            filename,
            backend,
            opt_level,
            fast,
            args,
            sandbox_level,
            deny_network,
            deny_process_spawn,
        } => {
            match run_exec_tool(
                json,
                pretty,
                file,
                source,
                filename,
                backend,
                opt_level,
                fast,
                args,
                cli.verbose,
                sandbox_level,
                deny_network,
                deny_process_spawn,
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

        Command::Repl { json, pretty } => {
            let outcome = if json {
                run_headless_json_request(pretty, cli.verbose)
            } else {
                run_repl_shell(cli.verbose)
            };
            match outcome {
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

        Command::Fmt { files, check } => {
            let files = match agam_pkg::expand_agam_inputs(files) {
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

            let changed_files = match agam_fmt::format_paths(&files, check) {
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

        Command::Daemon {
            path,
            once,
            poll_ms,
            background_child,
            command,
        } => match command {
            Some(DaemonCommand::Status) => {
                if let Err(e) = print_daemon_status(path, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
            Some(DaemonCommand::Clear) => {
                if let Err(e) = clear_daemon_status(path, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
            Some(DaemonCommand::Start) => {
                if let Err(e) = start_daemon_background(path.clone(), poll_ms, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
            Some(DaemonCommand::Stop) => {
                if let Err(e) = stop_daemon_background(path, cli.verbose) {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
            None => {
                let is_background = background_child;
                if let Err(e) =
                    run_daemon_foreground(path, once, poll_ms, is_background, cli.verbose)
                {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", e);
                    process::exit(1);
                }
            }
        },

        Command::Test { files, coverage } => {
            let files = match agam_pkg::expand_agam_inputs(files) {
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

fn resolve_entry_source_path(path: &Path) -> Result<PathBuf, String> {
    Ok(resolve_workspace_layout(Some(path.to_path_buf()))?.entry_file)
}

fn ensure_build_output_parent_dir(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(parent).map_err(|e| {
        format!(
            "failed to create build output directory `{}`: {e}",
            parent.display()
        )
    })
}

fn default_package_output_path(path: &Path) -> Result<PathBuf, String> {
    let layout = resolve_workspace_layout(Some(path.to_path_buf()))?;
    if layout.manifest_path.is_some() {
        return Ok(layout
            .root
            .join("dist")
            .join(format!("{}.agpkg.json", layout.project_name)));
    }
    Ok(agam_pkg::default_package_path(&layout.entry_file))
}

fn default_build_output_path(path: &Path, target: Option<&str>) -> Result<PathBuf, String> {
    let layout = resolve_workspace_layout(Some(path.to_path_buf()))?;
    if layout.manifest_path.is_some() {
        return Ok(layout.root.join("dist").join({
            let mut name = std::ffi::OsString::from(layout.project_name);
            if native_binary_extension(target) == Some("exe") {
                name.push(".exe");
            }
            name
        }));
    }
    Ok(default_native_binary_output_path(
        &layout.entry_file,
        target,
    ))
}

fn resolve_build_requests(
    files: &[PathBuf],
    output: Option<PathBuf>,
    target: Option<&str>,
) -> Result<Vec<BuildRequest>, String> {
    if files.is_empty() {
        return Err("at least one source file is required".into());
    }

    if let Some(output) = output {
        if files.len() > 1 {
            return Err(
                "`--output` only supports a single input file; omit it to compile each file to its default output path"
                    .into(),
            );
        }
        return Ok(vec![BuildRequest {
            file: resolve_entry_source_path(&files[0])?,
            output,
        }]);
    }

    let mut seen = BTreeSet::new();
    let mut requests = Vec::new();
    for path in files {
        let file = resolve_entry_source_path(path)?;
        if !seen.insert(file.clone()) {
            continue;
        }
        let output = default_native_binary_output_path(&file, target);
        requests.push(BuildRequest { file, output });
    }

    Ok(requests)
}

fn is_nested_build_request() -> bool {
    std::env::var_os(NESTED_BUILD_REQUEST_ENV).is_some()
}

fn is_nested_check_request() -> bool {
    std::env::var_os(NESTED_CHECK_REQUEST_ENV).is_some()
}

fn render_backend_cli_value(backend: Backend) -> &'static str {
    match backend {
        Backend::Auto => "auto",
        Backend::C => "c",
        Backend::Llvm => "llvm",
        Backend::Jit => "jit",
    }
}

fn render_lto_cli_value(mode: LtoMode) -> &'static str {
    match mode {
        LtoMode::Thin => "thin",
        LtoMode::Full => "full",
    }
}

fn build_request_parallelism(request_count: usize) -> usize {
    request_parallelism(request_count)
}

fn check_request_parallelism(request_count: usize) -> usize {
    request_parallelism(request_count)
}

fn request_parallelism(request_count: usize) -> usize {
    if request_count <= 1 {
        return 1;
    }

    let available = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1);
    request_count.min(available)
}

fn execute_check_requests_with_runner<F>(
    requests: &[CheckRequest],
    parallelism: usize,
    runner: F,
) -> Vec<CheckRequestResult>
where
    F: Fn(&CheckRequest) -> CheckRequestResult + Sync,
{
    if requests.is_empty() {
        return Vec::new();
    }

    let worker_count = parallelism.max(1).min(requests.len());
    let next_index = AtomicUsize::new(0);
    let results = Mutex::new(
        std::iter::repeat_with(|| None)
            .take(requests.len())
            .collect::<Vec<Option<CheckRequestResult>>>(),
    );

    std::thread::scope(|scope| {
        let runner = &runner;
        for _ in 0..worker_count {
            scope.spawn(|| {
                loop {
                    let index = next_index.fetch_add(1, Ordering::Relaxed);
                    if index >= requests.len() {
                        break;
                    }

                    let result = runner(&requests[index]);
                    results.lock().expect("check results mutex poisoned")[index] = Some(result);
                }
            });
        }
    });

    results
        .into_inner()
        .expect("check results mutex poisoned")
        .into_iter()
        .map(|result| result.expect("check request result missing"))
        .collect()
}

fn execute_test_requests_with_runner<F>(
    requests: &[TestRequest],
    parallelism: usize,
    runner: F,
) -> Vec<TestRequestResult>
where
    F: Fn(&TestRequest) -> TestRequestResult + Sync,
{
    if requests.is_empty() {
        return Vec::new();
    }

    let worker_count = parallelism.max(1).min(requests.len());
    let next_index = AtomicUsize::new(0);
    let results = Mutex::new(
        std::iter::repeat_with(|| None)
            .take(requests.len())
            .collect::<Vec<Option<TestRequestResult>>>(),
    );

    std::thread::scope(|scope| {
        let runner = &runner;
        for _ in 0..worker_count {
            scope.spawn(|| {
                loop {
                    let index = next_index.fetch_add(1, Ordering::Relaxed);
                    if index >= requests.len() {
                        break;
                    }

                    let result = runner(&requests[index]);
                    results.lock().expect("test results mutex poisoned")[index] = Some(result);
                }
            });
        }
    });

    results
        .into_inner()
        .expect("test results mutex poisoned")
        .into_iter()
        .map(|result| result.expect("test request result missing"))
        .collect()
}

fn execute_build_requests_with_runner<F>(
    requests: &[BuildRequest],
    parallelism: usize,
    runner: F,
) -> Vec<BuildRequestResult>
where
    F: Fn(&BuildRequest) -> BuildRequestResult + Sync,
{
    if requests.is_empty() {
        return Vec::new();
    }

    let worker_count = parallelism.max(1).min(requests.len());
    let next_index = AtomicUsize::new(0);
    let results = Mutex::new(
        std::iter::repeat_with(|| None)
            .take(requests.len())
            .collect::<Vec<Option<BuildRequestResult>>>(),
    );

    std::thread::scope(|scope| {
        let runner = &runner;
        for _ in 0..worker_count {
            scope.spawn(|| {
                loop {
                    let index = next_index.fetch_add(1, Ordering::Relaxed);
                    if index >= requests.len() {
                        break;
                    }

                    let result = runner(&requests[index]);
                    results.lock().expect("build results mutex poisoned")[index] = Some(result);
                }
            });
        }
    });

    results
        .into_inner()
        .expect("build results mutex poisoned")
        .into_iter()
        .map(|result| result.expect("build request result missing"))
        .collect()
}

fn run_nested_build_request(
    request: &BuildRequest,
    opt_level: u8,
    backend: Backend,
    tuning: &ReleaseTuning,
    features: FeatureFlags,
    verbose: bool,
) -> BuildRequestResult {
    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            return BuildRequestResult {
                request: request.clone(),
                stdout: Vec::new(),
                stderr: Vec::new(),
                succeeded: false,
                launch_error: Some(format!(
                    "failed to locate the current agamc executable for `{}`: {}",
                    request.file.display(),
                    error
                )),
            };
        }
    };

    let mut command = std::process::Command::new(current_exe);
    if verbose {
        command.arg("--verbose");
    }
    command
        .arg("build")
        .arg(&request.file)
        .arg("--output")
        .arg(&request.output)
        .arg("-O")
        .arg(opt_level.to_string())
        .arg("--backend")
        .arg(render_backend_cli_value(backend))
        .env(NESTED_BUILD_REQUEST_ENV, "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if tuning.native_cpu {
        command.arg("--fast");
    }
    if let Some(target) = tuning.target.as_ref() {
        command.arg("--target").arg(target);
    }
    if let Some(lto) = tuning.lto {
        command.arg("--lto").arg(render_lto_cli_value(lto));
    }
    if let Some(dir) = tuning.pgo_generate.as_ref() {
        command.arg("--pgo-generate").arg(dir);
    }
    if let Some(profile) = tuning.pgo_use.as_ref() {
        command.arg("--pgo-use").arg(profile);
    }
    if features.call_cache {
        command.arg("--call-cache");
    }

    match command.output() {
        Ok(output) => BuildRequestResult {
            request: request.clone(),
            stdout: output.stdout,
            stderr: output.stderr,
            succeeded: output.status.success(),
            launch_error: None,
        },
        Err(error) => BuildRequestResult {
            request: request.clone(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            succeeded: false,
            launch_error: Some(format!(
                "failed to launch nested build for `{}`: {}",
                request.file.display(),
                error
            )),
        },
    }
}

fn run_nested_check_request(request: &CheckRequest, verbose: bool) -> CheckRequestResult {
    let current_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            return CheckRequestResult {
                request: request.clone(),
                stdout: Vec::new(),
                stderr: Vec::new(),
                succeeded: false,
                launch_error: Some(format!(
                    "failed to locate the current agamc executable for `{}`: {}",
                    request.file.display(),
                    error
                )),
            };
        }
    };

    let mut command = std::process::Command::new(current_exe);
    if verbose {
        command.arg("--verbose");
    }
    command
        .arg("check")
        .arg(&request.file)
        .env(NESTED_CHECK_REQUEST_ENV, "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match command.output() {
        Ok(output) => CheckRequestResult {
            request: request.clone(),
            stdout: output.stdout,
            stderr: output.stderr,
            succeeded: output.status.success(),
            launch_error: None,
        },
        Err(error) => CheckRequestResult {
            request: request.clone(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            succeeded: false,
            launch_error: Some(format!(
                "failed to launch nested check for `{}`: {}",
                request.file.display(),
                error
            )),
        },
    }
}

fn replay_build_request_output(result: &BuildRequestResult) -> Result<(), String> {
    if !result.stdout.is_empty() {
        std::io::stdout()
            .write_all(&result.stdout)
            .map_err(|error| format!("failed to replay build stdout: {error}"))?;
        std::io::stdout()
            .flush()
            .map_err(|error| format!("failed to flush build stdout: {error}"))?;
    }

    if !result.stderr.is_empty() {
        std::io::stderr()
            .write_all(&result.stderr)
            .map_err(|error| format!("failed to replay build stderr: {error}"))?;
        std::io::stderr()
            .flush()
            .map_err(|error| format!("failed to flush build stderr: {error}"))?;
    }

    if let Some(error) = result.launch_error.as_ref() {
        eprintln!("\x1b[1;31merror\x1b[0m: {}", error);
    } else if !result.succeeded && result.stderr.is_empty() && result.stdout.is_empty() {
        eprintln!(
            "\x1b[1;31merror\x1b[0m: nested build failed for `{}` without diagnostic output",
            result.request.file.display()
        );
    }

    Ok(())
}

fn replay_check_request_output(result: &CheckRequestResult) -> Result<bool, String> {
    if !result.stdout.is_empty() {
        std::io::stdout()
            .write_all(&result.stdout)
            .map_err(|error| format!("failed to replay check stdout: {error}"))?;
        std::io::stdout()
            .flush()
            .map_err(|error| format!("failed to flush check stdout: {error}"))?;
    }

    if !result.stderr.is_empty() {
        std::io::stderr()
            .write_all(&result.stderr)
            .map_err(|error| format!("failed to replay check stderr: {error}"))?;
        std::io::stderr()
            .flush()
            .map_err(|error| format!("failed to flush check stderr: {error}"))?;
    }

    if let Some(error) = result.launch_error.as_ref() {
        eprintln!("\x1b[1;31merror\x1b[0m: {}", error);
        return Ok(false);
    }
    if !result.succeeded && result.stderr.is_empty() && result.stdout.is_empty() {
        eprintln!(
            "\x1b[1;31merror\x1b[0m: nested check failed for `{}` without diagnostic output",
            result.request.file.display()
        );
        return Ok(false);
    }

    Ok(result.succeeded)
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

fn packaged_sdk_root_for_executable(executable: &Path) -> Option<PathBuf> {
    let exe_dir = executable.parent()?;
    for candidate in [Some(exe_dir), exe_dir.parent()].into_iter().flatten() {
        if candidate.join("sdk-manifest.json").is_file() {
            return Some(candidate.to_path_buf());
        }
    }
    None
}

fn detect_packaged_sdk_manifest() -> Option<(PathBuf, agam_pkg::SdkDistributionManifest)> {
    let current_exe = std::env::current_exe().ok()?;
    let root = packaged_sdk_root_for_executable(&current_exe)?;
    let manifest =
        agam_pkg::read_sdk_distribution_manifest_from_path(&root.join("sdk-manifest.json")).ok()?;
    Some((root, manifest))
}

fn resolve_packaged_android_sysroot(target_triple: Option<&str>) -> Option<PathBuf> {
    let (sdk_root, manifest) = detect_packaged_sdk_manifest()?;
    let mut best_match: Option<(u8, PathBuf)> = None;
    for profile in manifest.supported_targets {
        if classify_llvm_target_platform(Some(profile.target_triple.as_str()))
            != LlvmTargetPlatform::Android
        {
            continue;
        }
        let packaged_sysroot = match profile.packaged_sysroot {
            Some(path) => sdk_root.join(path),
            None => continue,
        };
        if !packaged_sysroot.is_dir() {
            continue;
        }
        let priority = match target_triple {
            Some(target) if target == profile.target_triple => 2,
            Some(_) => 1,
            None => 1,
        };
        match &best_match {
            Some((best_priority, _)) if *best_priority >= priority => {}
            _ => best_match = Some((priority, packaged_sysroot)),
        }
    }
    best_match.map(|(_, path)| path)
}

fn resolve_android_sysroot_for_target(target_triple: Option<&str>) -> Option<PathBuf> {
    env_path(LLVM_SYSROOT_ENV)
        .or_else(resolve_android_ndk_sysroot)
        .or_else(|| resolve_packaged_android_sysroot(target_triple))
}

fn resolve_sdk_android_sysroot_source(explicit: Option<&PathBuf>) -> Option<PathBuf> {
    explicit
        .cloned()
        .or_else(|| resolve_android_sysroot_for_target(None))
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
    let sysroot = if platform == LlvmTargetPlatform::Android {
        resolve_android_sysroot_for_target(target_triple.as_deref())
    } else {
        env_path(LLVM_SYSROOT_ENV)
    };
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

type WorkspaceLayout = agam_pkg::WorkspaceLayout;

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
    let manifest = agam_pkg::scaffold_workspace_manifest(&project_name);
    let manifest_path = agam_pkg::default_manifest_path(&root);
    let entry_file = manifest_entry_path(&root, &manifest)?;
    let entry_dir = entry_file
        .parent()
        .ok_or_else(|| {
            format!(
                "entry file `{}` has no parent directory",
                entry_file.display()
            )
        })?
        .to_path_buf();
    let tests_dir = root.join("tests");
    let smoke_test = tests_dir.join("smoke.agam");
    let gitignore_path = root.join(".gitignore");

    std::fs::create_dir_all(&entry_dir)
        .map_err(|e| format!("failed to create `{}`: {}", entry_dir.display(), e))?;
    std::fs::create_dir_all(&tests_dir)
        .map_err(|e| format!("failed to create `{}`: {}", tests_dir.display(), e))?;

    agam_pkg::write_workspace_manifest_to_path(&manifest_path, &manifest)?;
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

const PROJECT_GITIGNORE: &str = ".agam_cache/\ndist/\n*.agpkg.json\n*.c\n*.ll\n*.exe\n";

fn render_project_entry(project_name: &str) -> String {
    format!(
        "@lang.advance\n\nfn main() -> i32 {{\n    println(\"Hello from {project_name}\");\n    return 0;\n}}\n"
    )
}

fn render_project_smoke_test() -> String {
    "@test\nfn arithmetic_is_sound() -> bool:\n    return (20 + 22) == 42\n".into()
}

fn resolve_workspace_layout(path: Option<PathBuf>) -> Result<WorkspaceLayout, String> {
    agam_pkg::resolve_workspace_layout(path)
}

fn resolve_workspace_session_for_driver(
    path: Option<PathBuf>,
) -> Result<agam_pkg::WorkspaceSession, String> {
    agam_pkg::resolve_workspace_session(path)
}

/// Attempt lockfile generation/refresh for a workspace session.
///
/// Returns `Ok(Some(lockfile))` when a lockfile was generated or is fresh,
/// `Ok(None)` when the workspace has no manifest (no lockfile needed),
/// and `Err` on resolution failures.
fn try_lockfile_refresh(
    session: &agam_pkg::WorkspaceSession,
    verbose: bool,
) -> Result<Option<agam_pkg::WorkspaceLockfile>, String> {
    if session.manifest.is_none() {
        return Ok(None);
    }

    let lockfile_path = agam_pkg::default_lockfile_path(&session.layout.root);
    let had_lockfile = lockfile_path.is_file();

    let lockfile = agam_pkg::generate_or_refresh_lockfile(session)?;

    if verbose {
        let manifest = session.manifest.as_ref().expect("manifest checked above");
        let diagnostics = agam_pkg::lockfile_diagnostics(manifest, &lockfile);
        if diagnostics.is_empty() {
            if had_lockfile {
                eprintln!(
                    "[agamc] lockfile: fresh ({} package(s))",
                    lockfile.packages.len()
                );
            } else {
                eprintln!(
                    "[agamc] lockfile: generated agam.lock ({} package(s))",
                    lockfile.packages.len()
                );
            }
        } else {
            for diagnostic in &diagnostics {
                eprintln!("[agamc] lockfile warning: {diagnostic}");
            }
        }

        // Drift detection: warn about path deps that changed since lockfile generation.
        let drift = agam_pkg::lockfile_content_drift(&session.layout.root, &lockfile);
        for (name, _old, _new) in &drift {
            eprintln!(
                "[agamc] lockfile drift: path dependency `{name}` has changed since lockfile was generated"
            );
        }
    }

    Ok(Some(lockfile))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishReport {
    dry_run: bool,
    official: bool,
    workspace_root: PathBuf,
    manifest_path: PathBuf,
    index_root: PathBuf,
    index_name: String,
    index_path: String,
    owners: Vec<String>,
    manifest: agam_pkg::PublishManifest,
    receipt: Option<agam_pkg::PublishReceipt>,
    bootstrapped_config: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryInspectReport {
    index_root: PathBuf,
    index_name: String,
    index_path: String,
    entry: agam_pkg::RegistryPackageEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryAuditReport {
    index_root: PathBuf,
    index_name: String,
    index_path: String,
    lines: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryProfileListReport {
    profiles: Vec<agam_pkg::FirstPartyDistributionProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryProfileInspectReport {
    profile: agam_pkg::FirstPartyDistributionProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryGovernanceReport {
    governance: agam_pkg::OfficialPackageGovernance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EnvironmentListReport {
    workspace_root: PathBuf,
    manifest_path: PathBuf,
    default_environment: Option<String>,
    environments: Vec<agam_pkg::ResolvedEnvironment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EnvironmentInspectReport {
    workspace_root: PathBuf,
    manifest_path: PathBuf,
    selected_by_default: bool,
    environment: agam_pkg::ResolvedEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryYankReport {
    index_root: PathBuf,
    index_name: String,
    package_name: String,
    version: String,
    yanked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryInstallReport {
    workspace_root: PathBuf,
    manifest_path: PathBuf,
    index_root: PathBuf,
    index_name: String,
    dependency_table: DependencyTable,
    dependency_key: String,
    package_name: String,
    requested_version: Option<String>,
    selected_version: String,
    added_new_entry: bool,
    changed_manifest: bool,
    lockfile_package_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryProfileInstallItem {
    package_name: String,
    requested_version: String,
    selected_version: String,
    added_new_entry: bool,
    changed_manifest: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryProfileInstallReport {
    workspace_root: PathBuf,
    manifest_path: PathBuf,
    index_root: PathBuf,
    index_name: String,
    dependency_table: DependencyTable,
    profile: agam_pkg::FirstPartyDistributionProfile,
    items: Vec<RegistryProfileInstallItem>,
    lockfile_package_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryUpdateItem {
    dependency_key: String,
    package_name: String,
    previous_version: Option<String>,
    selected_version: String,
    updated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryUpdateReport {
    workspace_root: PathBuf,
    manifest_path: PathBuf,
    index_root: PathBuf,
    index_name: String,
    dependency_table: DependencyTable,
    items: Vec<RegistryUpdateItem>,
    lockfile_package_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RegistryDependencyTarget {
    dependency_key: String,
    package_name: String,
}

fn backend_from_runtime_backend(backend: agam_runtime::contract::RuntimeBackend) -> Backend {
    match backend {
        agam_runtime::contract::RuntimeBackend::Auto => Backend::Auto,
        agam_runtime::contract::RuntimeBackend::C => Backend::C,
        agam_runtime::contract::RuntimeBackend::Llvm => Backend::Llvm,
        agam_runtime::contract::RuntimeBackend::Jit => Backend::Jit,
    }
}

fn requested_backend_from_environment(
    environment: &agam_pkg::ResolvedEnvironment,
    allow_jit: bool,
) -> Option<Backend> {
    match environment.preferred_backend {
        Some(agam_runtime::contract::RuntimeBackend::Jit) if !allow_jit => None,
        Some(backend) => Some(backend_from_runtime_backend(backend)),
        None => None,
    }
}

fn requested_backend_for_command(
    cli_backend: Backend,
    environment: Option<&EnvironmentInspectReport>,
    allow_jit: bool,
    target: Option<&str>,
) -> Backend {
    if cli_backend != Backend::Auto {
        return cli_backend;
    }
    if let Some(environment) = environment {
        if let Some(backend) =
            requested_backend_from_environment(&environment.environment, allow_jit)
        {
            return backend;
        }
    }
    if target.is_some() {
        return Backend::Llvm;
    }
    Backend::Auto
}

fn selected_target_for_command(
    cli_target: Option<String>,
    environment: Option<&EnvironmentInspectReport>,
) -> Option<String> {
    cli_target.or_else(|| environment.and_then(|report| report.environment.target.clone()))
}

fn environment_selection_label(report: &EnvironmentInspectReport) -> String {
    if report.selected_by_default {
        format!("{} (default)", report.environment.name)
    } else {
        report.environment.name.clone()
    }
}

fn maybe_resolve_workspace_environment(
    path: Option<PathBuf>,
    requested: Option<&str>,
) -> Result<Option<EnvironmentInspectReport>, String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let Some(manifest_path) = session.layout.manifest_path.clone() else {
        if requested.is_some() {
            return Err(
                "`--env` requires a workspace rooted by `agam.toml`; single-file sessions do not define environments"
                    .into(),
            );
        }
        return Ok(None);
    };
    let Some(manifest) = session.manifest.clone() else {
        if requested.is_some() {
            return Err(format!(
                "`--env` requires a manifest at `{}`",
                manifest_path.display()
            ));
        }
        return Ok(None);
    };
    if manifest.environments.is_empty() {
        if let Some(requested) = requested {
            return Err(format!(
                "workspace `{}` defines no named environments; cannot select `{requested}`",
                manifest.project.name
            ));
        }
        return Ok(None);
    }

    let lockfile = agam_pkg::resolve_dependencies(&session)?;
    let selected_by_default = requested.is_none();
    let environment = agam_pkg::resolve_environment(&manifest, &lockfile, requested)?
        .ok_or_else(|| "no environment selected".to_string())?;

    Ok(Some(EnvironmentInspectReport {
        workspace_root: session.layout.root,
        manifest_path,
        selected_by_default,
        environment,
    }))
}

fn maybe_resolve_optional_workspace_environment(
    path: Option<PathBuf>,
    requested: Option<&str>,
) -> Result<Option<EnvironmentInspectReport>, String> {
    let Some(path) = path else {
        return if requested.is_some() {
            Err(
                "`--env` requires a workspace rooted by `agam.toml`; no workspace path was provided"
                    .into(),
            )
        } else {
            Ok(None)
        };
    };

    match maybe_resolve_workspace_environment(Some(path), requested) {
        Ok(environment) => Ok(environment),
        Err(_) if requested.is_none() => Ok(None),
        Err(error) => Err(error),
    }
}

fn maybe_resolve_build_environment(
    files: &[PathBuf],
    requested: Option<&str>,
) -> Result<Option<EnvironmentInspectReport>, String> {
    let mut selected: Option<EnvironmentInspectReport> = None;
    let mut saw_environment = false;
    let mut saw_environment_free = false;
    let mut seen = BTreeSet::new();

    for input in files {
        let file = resolve_entry_source_path(input)?;
        if !seen.insert(file.clone()) {
            continue;
        }

        match maybe_resolve_workspace_environment(Some(file), requested)? {
            Some(report) => {
                saw_environment = true;
                if let Some(existing) = selected.as_ref() {
                    let existing_backend =
                        requested_backend_from_environment(&existing.environment, false);
                    let report_backend =
                        requested_backend_from_environment(&report.environment, false);
                    if existing.environment.target != report.environment.target
                        || existing_backend != report_backend
                    {
                        return Err(format!(
                            "build inputs resolve to incompatible environments: `{}` -> `{}` (target={}, backend={}); `{}` -> `{}` (target={}, backend={})",
                            existing.workspace_root.display(),
                            environment_selection_label(existing),
                            existing.environment.target.as_deref().unwrap_or("host"),
                            existing_backend
                                .map(render_backend_cli_value)
                                .unwrap_or("auto"),
                            report.workspace_root.display(),
                            environment_selection_label(&report),
                            report.environment.target.as_deref().unwrap_or("host"),
                            report_backend
                                .map(render_backend_cli_value)
                                .unwrap_or("auto"),
                        ));
                    }
                } else {
                    selected = Some(report);
                }
            }
            None => saw_environment_free = true,
        }
    }

    if saw_environment && saw_environment_free {
        return Err(
            "build inputs mix environment-aware workspaces with environment-free inputs; build them separately or add a consistent project-local environment contract"
                .into(),
        );
    }

    Ok(selected)
}

struct RegistryIndexEnvRestore {
    key: String,
    previous: Option<std::ffi::OsString>,
}

impl RegistryIndexEnvRestore {
    fn capture(key: &str) -> Self {
        Self {
            key: key.to_string(),
            previous: std::env::var_os(key),
        }
    }
}

impl Drop for RegistryIndexEnvRestore {
    fn drop(&mut self) {
        match self.previous.as_ref() {
            Some(previous) => unsafe {
                std::env::set_var(&self.key, previous);
            },
            None => unsafe {
                std::env::remove_var(&self.key);
            },
        }
    }
}

fn registry_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_registry_index_env<R>(registry: &str, index_root: &Path, f: impl FnOnce() -> R) -> R {
    let _guard = registry_env_lock()
        .lock()
        .expect("registry env lock should not be poisoned");
    let key = agam_pkg::registry_index_env_var(registry);
    let restore = RegistryIndexEnvRestore::capture(&key);
    unsafe {
        std::env::set_var(&key, index_root);
    }
    let result = f();
    drop(restore);
    result
}

fn registry_field_for_index(index_name: &str) -> Option<String> {
    (index_name != "agam").then(|| index_name.to_string())
}

fn dependency_registry_name(spec: &agam_pkg::DependencySpec) -> &str {
    spec.registry.as_deref().unwrap_or("agam")
}

fn workspace_member_names(session: &agam_pkg::WorkspaceSession) -> BTreeSet<String> {
    session
        .members
        .iter()
        .map(|member| member.layout.project_name.clone())
        .collect()
}

fn dependency_table(
    manifest: &agam_pkg::WorkspaceManifest,
    table: DependencyTable,
) -> &BTreeMap<String, agam_pkg::DependencySpec> {
    match table {
        DependencyTable::Main => &manifest.dependencies,
        DependencyTable::Dev => &manifest.dev_dependencies,
        DependencyTable::Build => &manifest.build_dependencies,
    }
}

fn dependency_table_mut(
    manifest: &mut agam_pkg::WorkspaceManifest,
    table: DependencyTable,
) -> &mut BTreeMap<String, agam_pkg::DependencySpec> {
    match table {
        DependencyTable::Main => &mut manifest.dependencies,
        DependencyTable::Dev => &mut manifest.dev_dependencies,
        DependencyTable::Build => &mut manifest.build_dependencies,
    }
}

fn is_registry_dependency(
    dependency_key: &str,
    spec: &agam_pkg::DependencySpec,
    workspace_members: &BTreeSet<String>,
) -> bool {
    !workspace_members.contains(dependency_key)
        && spec.path.is_none()
        && spec.git.is_none()
        && (spec.version.is_some() || spec.registry.is_some())
}

fn ensure_registry_dependency_slot(
    dependency_key: &str,
    package_name: &str,
    spec: &agam_pkg::DependencySpec,
    index_name: &str,
    workspace_members: &BTreeSet<String>,
    table: DependencyTable,
) -> Result<(), String> {
    if workspace_members.contains(dependency_key) {
        return Err(format!(
            "cannot modify `{dependency_key}` in `{}` because it resolves to a workspace member",
            table.manifest_label()
        ));
    }
    if spec.path.is_some() || spec.git.is_some() {
        return Err(format!(
            "cannot modify `{dependency_key}` in `{}` because it already uses a non-registry source",
            table.manifest_label()
        ));
    }

    let existing_package = spec.package.as_deref().unwrap_or(dependency_key);
    if existing_package != package_name {
        return Err(format!(
            "cannot modify `{dependency_key}` in `{}` because it already targets package `{existing_package}`",
            table.manifest_label()
        ));
    }

    let existing_registry = dependency_registry_name(spec);
    if existing_registry != index_name {
        return Err(format!(
            "cannot modify `{dependency_key}` in `{}` because it targets registry `{existing_registry}` instead of `{index_name}`",
            table.manifest_label()
        ));
    }

    Ok(())
}

fn collect_registry_update_targets(
    manifest: &agam_pkg::WorkspaceManifest,
    table: DependencyTable,
    names: &[String],
    index_name: &str,
    workspace_members: &BTreeSet<String>,
) -> Result<Vec<RegistryDependencyTarget>, String> {
    let dependencies = dependency_table(manifest, table);
    if names.is_empty() {
        let targets = dependencies
            .iter()
            .filter(|(dependency_key, spec)| {
                is_registry_dependency(dependency_key, spec, workspace_members)
                    && dependency_registry_name(spec) == index_name
            })
            .map(|(dependency_key, spec)| RegistryDependencyTarget {
                dependency_key: dependency_key.clone(),
                package_name: spec
                    .package
                    .clone()
                    .unwrap_or_else(|| dependency_key.clone()),
            })
            .collect::<Vec<_>>();

        if targets.is_empty() {
            return Err(format!(
                "no registry dependencies in `{}` target registry `{index_name}`",
                table.manifest_label()
            ));
        }

        return Ok(targets);
    }

    let mut targets = Vec::new();
    let mut seen = BTreeSet::new();
    for raw_name in names {
        let name = raw_name.trim();
        if name.is_empty() {
            return Err("registry update names cannot be empty".into());
        }

        if let Some(spec) = dependencies.get(name) {
            ensure_registry_dependency_slot(
                name,
                spec.package.as_deref().unwrap_or(name),
                spec,
                index_name,
                workspace_members,
                table,
            )?;
            if seen.insert(name.to_string()) {
                targets.push(RegistryDependencyTarget {
                    dependency_key: name.to_string(),
                    package_name: spec.package.clone().unwrap_or_else(|| name.to_string()),
                });
            }
            continue;
        }

        let matches = dependencies
            .iter()
            .filter(|(dependency_key, spec)| {
                spec.package.as_deref() == Some(name)
                    && is_registry_dependency(dependency_key, spec, workspace_members)
                    && dependency_registry_name(spec) == index_name
            })
            .map(|(dependency_key, _)| dependency_key.clone())
            .collect::<Vec<_>>();

        match matches.len() {
            0 => {
                return Err(format!(
                    "dependency or package `{name}` was not found in `{}` for registry `{index_name}`",
                    table.manifest_label()
                ));
            }
            1 => {
                let dependency_key = matches[0].clone();
                if seen.insert(dependency_key.clone()) {
                    targets.push(RegistryDependencyTarget {
                        dependency_key,
                        package_name: name.to_string(),
                    });
                }
            }
            _ => {
                return Err(format!(
                    "package `{name}` maps to multiple dependency keys in `{}`; update by dependency key instead",
                    table.manifest_label()
                ));
            }
        }
    }

    Ok(targets)
}

fn refresh_lockfile_with_registry_index(
    workspace_root: &Path,
    index_name: &str,
    index_root: &Path,
    verbose: bool,
) -> Result<usize, String> {
    with_registry_index_env(index_name, index_root, || {
        let session = resolve_workspace_session_for_driver(Some(workspace_root.to_path_buf()))?;
        Ok(try_lockfile_refresh(&session, verbose)?
            .map(|lockfile| lockfile.packages.len())
            .unwrap_or(0))
    })
}

fn persist_manifest_and_refresh_lockfile(
    original_manifest: &agam_pkg::WorkspaceManifest,
    updated_manifest: &agam_pkg::WorkspaceManifest,
    manifest_path: &Path,
    workspace_root: &Path,
    index_name: &str,
    index_root: &Path,
    verbose: bool,
) -> Result<usize, String> {
    agam_pkg::validate_workspace_manifest(workspace_root, updated_manifest)?;
    agam_pkg::write_workspace_manifest_to_path(manifest_path, updated_manifest)?;

    match refresh_lockfile_with_registry_index(workspace_root, index_name, index_root, verbose) {
        Ok(lockfile_package_count) => Ok(lockfile_package_count),
        Err(error) => {
            let restore =
                agam_pkg::write_workspace_manifest_to_path(manifest_path, original_manifest);
            match restore {
                Ok(()) => Err(error),
                Err(restore_error) => Err(format!(
                    "{error}; failed to restore manifest `{}` after the lockfile refresh failed: {restore_error}",
                    manifest_path.display()
                )),
            }
        }
    }
}

fn install_registry_dependency(
    path: Option<PathBuf>,
    index_root: &Path,
    table: DependencyTable,
    package_name: &str,
    version_req: Option<&str>,
    verbose: bool,
) -> Result<RegistryInstallReport, String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "registry install requires a workspace rooted by `agam.toml`; single-file sessions are not installable"
            .to_string()
    })?;
    let index_name = resolve_registry_index_name(index_root)?;
    let selected_release =
        agam_pkg::select_registry_release(index_root, package_name, version_req)?;
    let workspace_members = workspace_member_names(&session);
    if workspace_members.contains(package_name) {
        return Err(format!(
            "cannot install `{package_name}` into `{}` because it already resolves to a workspace member",
            table.manifest_label()
        ));
    }

    let original_manifest = session.manifest.clone().ok_or_else(|| {
        format!(
            "registry install requires a manifest at `{}`",
            manifest_path.display()
        )
    })?;
    let mut updated_manifest = original_manifest.clone();
    let dependency_key = package_name.to_string();
    let dependencies = dependency_table_mut(&mut updated_manifest, table);

    let mut next_spec = dependencies
        .get(&dependency_key)
        .cloned()
        .unwrap_or_else(agam_pkg::DependencySpec::default);
    let mut added_new_entry = true;
    if let Some(existing) = dependencies.get(&dependency_key) {
        ensure_registry_dependency_slot(
            &dependency_key,
            package_name,
            existing,
            &index_name,
            &workspace_members,
            table,
        )?;
        added_new_entry = false;
    }

    let previous_version = next_spec.version.clone();
    let previous_registry = next_spec.registry.clone();
    next_spec.version = Some(selected_release.version.clone());
    next_spec.registry = registry_field_for_index(&index_name);
    next_spec.path = None;
    next_spec.git = None;
    next_spec.rev = None;
    next_spec.branch = None;
    next_spec.package = None;
    let changed_manifest =
        previous_version != next_spec.version || previous_registry != next_spec.registry;
    dependencies.insert(dependency_key.clone(), next_spec);

    let lockfile_package_count = if changed_manifest {
        persist_manifest_and_refresh_lockfile(
            &original_manifest,
            &updated_manifest,
            &manifest_path,
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    } else {
        refresh_lockfile_with_registry_index(
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    };

    Ok(RegistryInstallReport {
        workspace_root: session.layout.root,
        manifest_path,
        index_root: index_root.to_path_buf(),
        index_name,
        dependency_table: table,
        dependency_key,
        package_name: package_name.to_string(),
        requested_version: version_req.map(str::to_string),
        selected_version: selected_release.version,
        added_new_entry,
        changed_manifest,
        lockfile_package_count,
    })
}

fn update_registry_dependencies(
    path: Option<PathBuf>,
    index_root: &Path,
    table: DependencyTable,
    names: &[String],
    verbose: bool,
) -> Result<RegistryUpdateReport, String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "registry update requires a workspace rooted by `agam.toml`; single-file sessions are not installable"
            .to_string()
    })?;
    let index_name = resolve_registry_index_name(index_root)?;
    let workspace_members = workspace_member_names(&session);
    let original_manifest = session.manifest.clone().ok_or_else(|| {
        format!(
            "registry update requires a manifest at `{}`",
            manifest_path.display()
        )
    })?;
    let targets = collect_registry_update_targets(
        &original_manifest,
        table,
        names,
        &index_name,
        &workspace_members,
    )?;

    let mut updated_manifest = original_manifest.clone();
    let dependencies = dependency_table_mut(&mut updated_manifest, table);
    let mut items = Vec::new();
    let mut any_manifest_change = false;

    for target in targets {
        let selected_release =
            agam_pkg::select_registry_release(index_root, &target.package_name, None)?;
        let spec = dependencies
            .get_mut(&target.dependency_key)
            .ok_or_else(|| {
                format!(
                    "dependency `{}` disappeared from `{}` while preparing the update",
                    target.dependency_key,
                    table.manifest_label()
                )
            })?;

        let previous_version = spec.version.clone();
        let previous_registry = spec.registry.clone();
        spec.version = Some(selected_release.version.clone());
        spec.registry = registry_field_for_index(&index_name);
        spec.path = None;
        spec.git = None;
        spec.rev = None;
        spec.branch = None;
        if target.dependency_key == target.package_name {
            spec.package = None;
        } else {
            spec.package = Some(target.package_name.clone());
        }

        let updated = previous_version != spec.version || previous_registry != spec.registry;
        any_manifest_change |= updated;
        items.push(RegistryUpdateItem {
            dependency_key: target.dependency_key,
            package_name: target.package_name,
            previous_version,
            selected_version: selected_release.version,
            updated,
        });
    }

    let lockfile_package_count = if any_manifest_change {
        persist_manifest_and_refresh_lockfile(
            &original_manifest,
            &updated_manifest,
            &manifest_path,
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    } else {
        refresh_lockfile_with_registry_index(
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    };

    Ok(RegistryUpdateReport {
        workspace_root: session.layout.root,
        manifest_path,
        index_root: index_root.to_path_buf(),
        index_name,
        dependency_table: table,
        items,
        lockfile_package_count,
    })
}

fn yank_registry_release(
    index_root: &Path,
    package_name: &str,
    version: &str,
    undo: bool,
) -> Result<RegistryYankReport, String> {
    let index_name = resolve_registry_index_name(index_root)?;
    let release = agam_pkg::set_registry_release_yanked(index_root, package_name, version, !undo)?;
    Ok(RegistryYankReport {
        index_root: index_root.to_path_buf(),
        index_name,
        package_name: package_name.to_string(),
        version: release.version,
        yanked: release.yanked,
    })
}

fn list_registry_profiles() -> RegistryProfileListReport {
    RegistryProfileListReport {
        profiles: agam_pkg::first_party_distribution_profiles(),
    }
}

fn inspect_registry_profile(name: &str) -> Result<RegistryProfileInspectReport, String> {
    let profile = agam_pkg::first_party_distribution_profile(name)
        .ok_or_else(|| format!("unknown curated first-party profile `{name}`"))?;
    Ok(RegistryProfileInspectReport { profile })
}

fn registry_governance_report() -> RegistryGovernanceReport {
    RegistryGovernanceReport {
        governance: agam_pkg::official_package_governance(),
    }
}

fn install_registry_profile(
    path: Option<PathBuf>,
    index_root: &Path,
    table: DependencyTable,
    profile_name: &str,
    verbose: bool,
) -> Result<RegistryProfileInstallReport, String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "registry profile install requires a workspace rooted by `agam.toml`; single-file sessions are not installable"
            .to_string()
    })?;
    let profile = agam_pkg::first_party_distribution_profile(profile_name)
        .ok_or_else(|| format!("unknown curated first-party profile `{profile_name}`"))?;
    let index_name = resolve_registry_index_name(index_root)?;
    let workspace_members = workspace_member_names(&session);
    let original_manifest = session.manifest.clone().ok_or_else(|| {
        format!(
            "registry profile install requires a manifest at `{}`",
            manifest_path.display()
        )
    })?;

    let mut updated_manifest = original_manifest.clone();
    let dependencies = dependency_table_mut(&mut updated_manifest, table);
    let mut items = Vec::new();
    let mut any_manifest_change = false;

    for recommendation in &profile.packages {
        if workspace_members.contains(&recommendation.name) {
            return Err(format!(
                "cannot install profile `{}` because `{}` already resolves to a workspace member",
                profile.name, recommendation.name
            ));
        }

        let selected_release = agam_pkg::select_registry_release(
            index_root,
            &recommendation.name,
            Some(&recommendation.version_req),
        )?;
        let dependency_key = recommendation.name.clone();
        let mut next_spec = dependencies
            .get(&dependency_key)
            .cloned()
            .unwrap_or_else(agam_pkg::DependencySpec::default);
        let added_new_entry = !dependencies.contains_key(&dependency_key);
        if let Some(existing) = dependencies.get(&dependency_key) {
            ensure_registry_dependency_slot(
                &dependency_key,
                &recommendation.name,
                existing,
                &index_name,
                &workspace_members,
                table,
            )?;
        }

        let previous_version = next_spec.version.clone();
        let previous_registry = next_spec.registry.clone();
        next_spec.version = Some(selected_release.version.clone());
        next_spec.registry = registry_field_for_index(&index_name);
        next_spec.path = None;
        next_spec.git = None;
        next_spec.rev = None;
        next_spec.branch = None;
        next_spec.package = None;
        let changed_manifest =
            previous_version != next_spec.version || previous_registry != next_spec.registry;
        any_manifest_change |= changed_manifest;
        dependencies.insert(dependency_key, next_spec);

        items.push(RegistryProfileInstallItem {
            package_name: recommendation.name.clone(),
            requested_version: recommendation.version_req.clone(),
            selected_version: selected_release.version,
            added_new_entry,
            changed_manifest,
        });
    }

    let lockfile_package_count = if any_manifest_change {
        persist_manifest_and_refresh_lockfile(
            &original_manifest,
            &updated_manifest,
            &manifest_path,
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    } else {
        refresh_lockfile_with_registry_index(
            &session.layout.root,
            &index_name,
            index_root,
            verbose,
        )?
    };

    Ok(RegistryProfileInstallReport {
        workspace_root: session.layout.root,
        manifest_path,
        index_root: index_root.to_path_buf(),
        index_name,
        dependency_table: table,
        profile,
        items,
        lockfile_package_count,
    })
}

fn resolve_environment_session_and_lockfile(
    path: Option<PathBuf>,
) -> Result<
    (
        agam_pkg::WorkspaceSession,
        PathBuf,
        agam_pkg::WorkspaceManifest,
        agam_pkg::WorkspaceLockfile,
    ),
    String,
> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "environment commands require a workspace rooted by `agam.toml`; single-file sessions do not define environments"
            .to_string()
    })?;
    let manifest = session.manifest.clone().ok_or_else(|| {
        format!(
            "environment commands require a manifest at `{}`",
            manifest_path.display()
        )
    })?;
    let lockfile = agam_pkg::resolve_dependencies(&session)?;
    Ok((session, manifest_path, manifest, lockfile))
}

fn list_workspace_environments(path: Option<PathBuf>) -> Result<EnvironmentListReport, String> {
    let (session, manifest_path, manifest, lockfile) =
        resolve_environment_session_and_lockfile(path)?;
    let default_environment = agam_pkg::default_environment_name(&manifest);
    let environments = agam_pkg::resolve_environment_catalog(&manifest, &lockfile)
        .into_values()
        .collect();

    Ok(EnvironmentListReport {
        workspace_root: session.layout.root,
        manifest_path,
        default_environment,
        environments,
    })
}

fn inspect_workspace_environment(
    path: Option<PathBuf>,
    name: Option<&str>,
) -> Result<EnvironmentInspectReport, String> {
    let (session, manifest_path, manifest, lockfile) =
        resolve_environment_session_and_lockfile(path)?;
    let selected_by_default = name.is_none();
    let environment =
        agam_pkg::resolve_environment(&manifest, &lockfile, name)?.ok_or_else(|| {
            if manifest.environments.is_empty() {
                "workspace defines no named environments".to_string()
            } else {
                "no environment selected".to_string()
            }
        })?;

    Ok(EnvironmentInspectReport {
        workspace_root: session.layout.root,
        manifest_path,
        selected_by_default,
        environment,
    })
}

fn publish_workspace_to_registry(
    path: Option<PathBuf>,
    index_root: &Path,
    owners: &[String],
    description: Option<&String>,
    homepage: Option<&String>,
    repository: Option<&String>,
    download_url: Option<&String>,
    official: bool,
    dry_run: bool,
    _verbose: bool,
) -> Result<PublishReport, String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let manifest_path = session.layout.manifest_path.clone().ok_or_else(|| {
        "publish requires a workspace rooted by `agam.toml`; single-file sessions are not publishable"
            .to_string()
    })?;

    let mut manifest = agam_pkg::build_publish_manifest(&session)?;
    if let Some(description) = normalize_publish_text(description.map(String::as_str)) {
        manifest.description = Some(description);
    }
    if let Some(homepage) = normalize_publish_text(homepage.map(String::as_str)) {
        manifest.homepage = Some(homepage);
    }
    if let Some(repository) = normalize_publish_text(repository.map(String::as_str)) {
        manifest.repository = Some(repository);
    }
    if let Some(download_url) = normalize_publish_text(download_url.map(String::as_str)) {
        manifest.download_url = Some(download_url);
    }

    let owners = normalize_publish_owners(owners);
    let (index_name, bootstrapped_config) = ensure_registry_index_ready(index_root, dry_run)?;
    let index_path = agam_pkg::registry_index_path(&manifest.name);

    if official {
        agam_pkg::validate_official_publish_manifest(&manifest, &index_name, &owners)?;
    } else {
        agam_pkg::validate_publish_manifest(&manifest)?;
    }

    let receipt = if dry_run {
        None
    } else if official {
        Some(agam_pkg::publish_official_package_to_registry_index(
            index_root,
            &manifest,
            &owners,
            &publish_timestamp(),
            &index_name,
        )?)
    } else {
        Some(agam_pkg::publish_to_registry_index(
            index_root,
            &manifest,
            &owners,
            &publish_timestamp(),
        )?)
    };

    Ok(PublishReport {
        dry_run,
        official,
        workspace_root: session.layout.root,
        manifest_path,
        index_root: index_root.to_path_buf(),
        index_name,
        index_path,
        owners,
        manifest,
        receipt,
        bootstrapped_config,
    })
}

fn normalize_publish_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn normalize_publish_owners(owners: &[String]) -> Vec<String> {
    owners
        .iter()
        .map(|owner| owner.trim())
        .filter(|owner| !owner.is_empty())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn inferred_registry_index_name(index_root: &Path) -> String {
    index_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("agam")
        .to_string()
}

fn resolve_registry_index_name(index_root: &Path) -> Result<String, String> {
    if !index_root.exists() {
        return Err(format!(
            "registry index root `{}` does not exist",
            index_root.display()
        ));
    }
    if !index_root.is_dir() {
        return Err(format!(
            "registry index root `{}` is not a directory",
            index_root.display()
        ));
    }

    let config_path = index_root.join("config.json");
    if config_path.exists() && !config_path.is_file() {
        return Err(format!(
            "registry config path `{}` is not a file",
            config_path.display()
        ));
    }

    if config_path.is_file() {
        let config = agam_pkg::read_registry_config(index_root)?;
        if config.format_version != agam_pkg::REGISTRY_INDEX_FORMAT_VERSION {
            return Err(format!(
                "registry index `{}` uses unsupported format version {}; expected {}",
                index_root.display(),
                config.format_version,
                agam_pkg::REGISTRY_INDEX_FORMAT_VERSION
            ));
        }
        Ok(config
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| inferred_registry_index_name(index_root)))
    } else {
        Ok(inferred_registry_index_name(index_root))
    }
}

fn ensure_registry_index_ready(index_root: &Path, dry_run: bool) -> Result<(String, bool), String> {
    if index_root.exists() && !index_root.is_dir() {
        return Err(format!(
            "registry index root `{}` is not a directory",
            index_root.display()
        ));
    }

    let config_path = index_root.join("config.json");
    if config_path.exists() && !config_path.is_file() {
        return Err(format!(
            "registry config path `{}` is not a file",
            config_path.display()
        ));
    }

    if config_path.is_file() {
        let config = agam_pkg::read_registry_config(index_root)?;
        if config.format_version != agam_pkg::REGISTRY_INDEX_FORMAT_VERSION {
            return Err(format!(
                "registry index `{}` uses unsupported format version {}; expected {}",
                index_root.display(),
                config.format_version,
                agam_pkg::REGISTRY_INDEX_FORMAT_VERSION
            ));
        }
        let index_name = config
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| inferred_registry_index_name(index_root));
        return Ok((index_name, false));
    }

    let index_name = inferred_registry_index_name(index_root);
    if dry_run {
        return Ok((index_name, false));
    }

    agam_pkg::write_registry_config(
        index_root,
        &agam_pkg::RegistryConfig {
            format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
            api_url: None,
            download_url: None,
            name: Some(index_name.clone()),
        },
    )?;

    Ok((index_name, true))
}

fn publish_timestamp() -> String {
    now_unix_ms().to_string()
}

fn inspect_registry_package(
    index_root: &Path,
    name: &str,
) -> Result<RegistryInspectReport, String> {
    let index_name = resolve_registry_index_name(index_root)?;
    let entry = agam_pkg::read_registry_package_entry(index_root, name)?;
    Ok(RegistryInspectReport {
        index_root: index_root.to_path_buf(),
        index_name,
        index_path: agam_pkg::registry_index_path(name),
        entry,
    })
}

fn audit_registry_index_package(
    index_root: &Path,
    name: &str,
) -> Result<RegistryAuditReport, String> {
    let index_name = resolve_registry_index_name(index_root)?;
    let lines = agam_pkg::audit_registry_package(index_root, name)?;
    Ok(RegistryAuditReport {
        index_root: index_root.to_path_buf(),
        index_name,
        index_path: agam_pkg::registry_index_path(name),
        lines,
    })
}

fn print_publish_report(report: &PublishReport, verbose: bool) {
    println!("publish: {}", if report.dry_run { "dry-run" } else { "ok" });
    println!(
        "package: {}@{}",
        report.manifest.name, report.manifest.version
    );
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("index path: {}", report.index_path);
    println!("checksum: {}", report.manifest.checksum);
    println!("manifest checksum: {}", report.manifest.manifest_checksum);
    println!("agam version: {}", report.manifest.agam_version);
    println!("official: {}", if report.official { "yes" } else { "no" });
    println!(
        "owners: {}",
        if report.owners.is_empty() {
            "none".to_string()
        } else {
            report.owners.join(", ")
        }
    );
    println!("dependencies: {}", report.manifest.dependencies.len());

    if let Some(description) = report.manifest.description.as_deref() {
        println!("description: {description}");
    } else if verbose {
        println!("description: none");
    }
    if let Some(homepage) = report.manifest.homepage.as_deref() {
        println!("homepage: {homepage}");
    } else if verbose {
        println!("homepage: none");
    }
    if let Some(repository) = report.manifest.repository.as_deref() {
        println!("repository: {repository}");
    } else if verbose {
        println!("repository: none");
    }
    if let Some(download_url) = report.manifest.download_url.as_deref() {
        println!("download: {download_url}");
    } else if verbose {
        println!("download: registry default or none");
    }
    if !report.manifest.keywords.is_empty() {
        println!("keywords: {}", report.manifest.keywords.join(", "));
    } else if verbose {
        println!("keywords: none");
    }

    if verbose && !report.manifest.dependencies.is_empty() {
        println!("dependency detail:");
        for dependency in &report.manifest.dependencies {
            let mut line = format!("  {} {}", dependency.name, dependency.version_req);
            if let Some(registry) = dependency.registry.as_deref() {
                line.push_str(&format!(" [registry: {registry}]"));
            }
            if dependency.optional {
                line.push_str(" [optional]");
            }
            if !dependency.features.is_empty() {
                line.push_str(&format!(" [features: {}]", dependency.features.join(", ")));
            }
            println!("{line}");
        }
    }

    if report.bootstrapped_config {
        println!("registry config: initialized config.json");
    } else if verbose {
        println!("registry config: existing or skipped");
    }

    if let Some(receipt) = report.receipt.as_ref() {
        println!("published at: {}", receipt.published_at);
    } else if verbose {
        println!("published at: pending (dry-run)");
    }
}

fn print_registry_inspect_report(report: &RegistryInspectReport, verbose: bool) {
    println!("package: {}", report.entry.name);
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("index path: {}", report.index_path);
    println!("created: {}", report.entry.created_at);
    println!(
        "owners: {}",
        if report.entry.owners.is_empty() {
            "none".to_string()
        } else {
            report.entry.owners.join(", ")
        }
    );
    println!("releases: {}", report.entry.releases.len());

    if let Some(description) = report.entry.description.as_deref() {
        println!("description: {description}");
    } else if verbose {
        println!("description: none");
    }
    if let Some(homepage) = report.entry.homepage.as_deref() {
        println!("homepage: {homepage}");
    } else if verbose {
        println!("homepage: none");
    }
    if let Some(repository) = report.entry.repository.as_deref() {
        println!("repository: {repository}");
    } else if verbose {
        println!("repository: none");
    }
    if !report.entry.keywords.is_empty() {
        println!("keywords: {}", report.entry.keywords.join(", "));
    } else if verbose {
        println!("keywords: none");
    }

    if verbose && !report.entry.releases.is_empty() {
        println!("release detail:");
        for release in &report.entry.releases {
            let yanked = if release.yanked { " [yanked]" } else { "" };
            println!(
                "  {} (checksum: {}, agam: {}, published: {}{}, deps: {}, features: {})",
                release.version,
                release.checksum,
                release.agam_version,
                release.published_at,
                yanked,
                release.dependencies.len(),
                release.features.len()
            );
            if let Some(download_url) = release.download_url.as_deref() {
                println!("    download: {download_url}");
            }
            if let Some(provenance) = release.provenance.as_ref() {
                println!(
                    "    provenance: source={}, manifest={}",
                    provenance.source_checksum, provenance.manifest_checksum
                );
                if let Some(published_by) = provenance.published_by.as_deref() {
                    println!("    published by: {published_by}");
                }
                if let Some(source_repository) = provenance.source_repository.as_deref() {
                    println!("    source repository: {source_repository}");
                }
            }
        }
    }
}

fn print_registry_audit_report(report: &RegistryAuditReport) {
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("index path: {}", report.index_path);
    for line in &report.lines {
        println!("{line}");
    }
}

fn print_registry_profile_list_report(report: &RegistryProfileListReport) {
    println!("profiles: {}", report.profiles.len());
    for profile in &report.profiles {
        println!(
            "{} | packages={} | {}",
            profile.name,
            profile.packages.len(),
            profile.summary
        );
    }
}

fn print_registry_profile_inspect_report(report: &RegistryProfileInspectReport) {
    println!("profile: {}", report.profile.name);
    println!("summary: {}", report.profile.summary);
    println!("description: {}", report.profile.description);
    println!("packages: {}", report.profile.packages.len());
    for package in &report.profile.packages {
        println!(
            "  {} {} | {}",
            package.name, package.version_req, package.rationale
        );
    }
    if !report.profile.notes.is_empty() {
        println!("notes:");
        for note in &report.profile.notes {
            println!("  {note}");
        }
    }
}

fn print_registry_governance_report(report: &RegistryGovernanceReport) {
    println!("registry: {}", report.governance.registry);
    println!("reserved prefix: {}", report.governance.reserved_prefix);
    println!(
        "repository namespace: {}",
        report.governance.repository_namespace
    );
    println!(
        "owners: {}",
        if report.governance.owner_handles.is_empty() {
            "none".to_string()
        } else {
            report.governance.owner_handles.join(", ")
        }
    );
    println!("rules: {}", report.governance.publication_rules.len());
    for rule in &report.governance.publication_rules {
        println!("  {rule}");
    }
}

fn print_registry_install_report(report: &RegistryInstallReport, verbose: bool) {
    println!(
        "install: {}",
        if report.changed_manifest {
            "ok"
        } else {
            "up-to-date"
        }
    );
    println!(
        "package: {}@{}",
        report.package_name, report.selected_version
    );
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("table: {}", report.dependency_table.manifest_label());
    println!("dependency: {}", report.dependency_key);
    println!(
        "manifest change: {}",
        if report.added_new_entry {
            "added dependency"
        } else if report.changed_manifest {
            "updated existing dependency"
        } else {
            "unchanged"
        }
    );
    println!("lockfile packages: {}", report.lockfile_package_count);

    if let Some(requested_version) = report.requested_version.as_deref() {
        println!("requested: {requested_version}");
    } else if verbose {
        println!("requested: latest");
    }
}

fn print_registry_profile_install_report(report: &RegistryProfileInstallReport, verbose: bool) {
    let changed = report
        .items
        .iter()
        .filter(|item| item.changed_manifest)
        .count();
    let unchanged = report.items.len().saturating_sub(changed);

    println!(
        "profile install: {}",
        if changed > 0 { "ok" } else { "up-to-date" }
    );
    println!("profile: {}", report.profile.name);
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("table: {}", report.dependency_table.manifest_label());
    println!("packages: {}", report.items.len());
    println!("updated: {changed}");
    println!("unchanged: {unchanged}");
    println!("lockfile packages: {}", report.lockfile_package_count);

    for item in &report.items {
        if item.changed_manifest || verbose {
            let status = if item.added_new_entry {
                "added"
            } else if item.changed_manifest {
                "updated"
            } else {
                "unchanged"
            };
            println!(
                "{}: {} -> {} ({status})",
                item.package_name, item.requested_version, item.selected_version
            );
        }
    }
}

fn print_registry_update_report(report: &RegistryUpdateReport, verbose: bool) {
    let updated = report.items.iter().filter(|item| item.updated).count();
    let unchanged = report.items.len().saturating_sub(updated);

    println!("update: ok");
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
    println!("table: {}", report.dependency_table.manifest_label());
    println!("updated: {updated}");
    println!("unchanged: {unchanged}");
    println!("lockfile packages: {}", report.lockfile_package_count);

    for item in &report.items {
        if item.updated || verbose {
            let label = if item.dependency_key == item.package_name {
                item.dependency_key.clone()
            } else {
                format!("{} ({})", item.dependency_key, item.package_name)
            };
            let previous = item.previous_version.as_deref().unwrap_or("*");
            println!("{}: {} -> {}", label, previous, item.selected_version);
        }
    }
}

fn print_registry_yank_report(report: &RegistryYankReport) {
    println!(
        "yank: {}",
        if report.yanked { "yanked" } else { "available" }
    );
    println!("package: {}@{}", report.package_name, report.version);
    println!(
        "registry: {} ({})",
        report.index_name,
        report.index_root.display()
    );
}

fn print_environment_list_report(report: &EnvironmentListReport) {
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!("environments: {}", report.environments.len());
    println!(
        "default: {}",
        report.default_environment.as_deref().unwrap_or("none")
    );

    for environment in &report.environments {
        let default_marker =
            if report.default_environment.as_deref() == Some(environment.name.as_str()) {
                " [default]"
            } else {
                ""
            };
        println!(
            "{}{} | compiler={} | sdk={} | target={} | backend={} | profiles={} | packages={}",
            environment.name,
            default_marker,
            environment.compiler,
            environment.sdk.as_deref().unwrap_or("none"),
            environment.target.as_deref().unwrap_or("none"),
            environment
                .preferred_backend
                .map(|backend| format!("{backend:?}").to_lowercase())
                .unwrap_or_else(|| "none".to_string()),
            if environment.profiles.is_empty() {
                "none".to_string()
            } else {
                environment.profiles.join(", ")
            },
            environment.packages.len()
        );
    }
}

fn print_environment_inspect_report(report: &EnvironmentInspectReport) {
    println!("workspace: {}", report.workspace_root.display());
    println!("manifest: {}", report.manifest_path.display());
    println!("environment: {}", report.environment.name);
    println!(
        "selected by: {}",
        if report.selected_by_default {
            "implicit default rules"
        } else {
            "explicit request"
        }
    );
    println!("compiler: {}", report.environment.compiler);
    println!(
        "sdk: {}",
        report.environment.sdk.as_deref().unwrap_or("none")
    );
    println!(
        "target: {}",
        report.environment.target.as_deref().unwrap_or("none")
    );
    println!(
        "runtime abi: {}",
        report
            .environment
            .runtime_abi
            .map(|abi| abi.to_string())
            .unwrap_or_else(|| "none".to_string())
    );
    println!(
        "backend: {}",
        report
            .environment
            .preferred_backend
            .map(|backend| format!("{backend:?}").to_lowercase())
            .unwrap_or_else(|| "none".to_string())
    );
    println!(
        "profiles: {}",
        if report.environment.profiles.is_empty() {
            "none".to_string()
        } else {
            report.environment.profiles.join(", ")
        }
    );
    println!("packages: {}", report.environment.packages.len());
    for package in &report.environment.packages {
        println!("  {package}");
    }
}

struct DaemonWorkspaceTarget {
    root: PathBuf,
    project_name: String,
}

fn daemon_workspace_target_from_layout(layout: WorkspaceLayout) -> DaemonWorkspaceTarget {
    DaemonWorkspaceTarget {
        root: layout.root,
        project_name: layout.project_name,
    }
}

fn daemon_workspace_target_from_root(root: PathBuf) -> DaemonWorkspaceTarget {
    let project_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("agam-workspace")
        .to_string();
    DaemonWorkspaceTarget { root, project_name }
}

fn daemon_refresh_snapshot_hint(workspace: &WorkspaceLayout) -> PathBuf {
    if workspace.manifest_path.is_none() {
        workspace.entry_file.clone()
    } else {
        workspace.root.clone()
    }
}

fn resolve_daemon_workspace_target(path: Option<PathBuf>) -> Result<DaemonWorkspaceTarget, String> {
    let hint = match path {
        Some(path) => path,
        None => {
            std::env::current_dir().map_err(|e| format!("failed to read current directory: {e}"))?
        }
    };
    if hint.exists() {
        if let Ok(layout) = resolve_workspace_layout(Some(hint.clone())) {
            return Ok(daemon_workspace_target_from_layout(layout));
        }
        if hint.is_dir() {
            return Ok(daemon_workspace_target_from_root(hint));
        }
    }

    let is_source_hint = hint.extension().and_then(|ext| ext.to_str()) == Some("agam");
    let is_manifest_hint = hint.file_name().and_then(|name| name.to_str()) == Some("agam.toml");
    let root = if is_source_hint || is_manifest_hint {
        hint.parent()
            .ok_or_else(|| {
                format!(
                    "`{}` does not exist and has no parent directory to resolve daemon status from",
                    hint.display()
                )
            })?
            .to_path_buf()
    } else {
        hint.clone()
    };
    if !root.exists() {
        return Err(format!("`{}` does not exist", hint.display()));
    }
    if let Ok(layout) = resolve_workspace_layout(Some(root.clone())) {
        return Ok(daemon_workspace_target_from_layout(layout));
    }

    if is_source_hint || is_manifest_hint {
        return Ok(daemon_workspace_target_from_root(root));
    }

    Err(format!("`{}` does not exist", hint.display()))
}

fn manifest_entry_path(
    root: &Path,
    manifest: &agam_pkg::WorkspaceManifest,
) -> Result<PathBuf, String> {
    let entry = manifest.project.entry.as_deref().unwrap_or("src/main.agam");
    workspace_relative_path(root, entry, "`project.entry`")
}

fn workspace_relative_path(
    root: &Path,
    relative: &str,
    field_name: &str,
) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if relative.trim().is_empty() {
        return Err(format!("{field_name} cannot be empty"));
    }
    if path.is_absolute() {
        return Err(format!(
            "{field_name} must stay relative to the workspace root; got `{}`",
            relative
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return Err(format!(
            "{field_name} must stay inside the workspace root; got `{}`",
            relative
        ));
    }
    Ok(root.join(path))
}

fn run_agam_tests(files: &[PathBuf], verbose: bool) -> Result<TestRunTotals, String> {
    let mut totals = TestRunTotals::default();

    let requests = files
        .iter()
        .cloned()
        .map(|file| TestRequest { file })
        .collect::<Vec<_>>();
    let results = execute_parallel_test_requests(&requests);

    for result in results {
        if let Some(error) = result.error {
            return Err(error);
        }
        let file_summary = result
            .summary
            .ok_or_else(|| "internal error: missing Agam test summary".to_string())?;
        let file = &file_summary.path;
        let summary = &file_summary.summary;
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
    if let Some(warm_state) = load_daemon_prewarmed_warm_state(file, verbose) {
        return run_source_file_with_optional_warm_state(
            file,
            args,
            backend,
            opt_level,
            tuning,
            verbose,
            features,
            Some(&warm_state),
        );
    }

    // Fallback: try the multi-file warm index
    if let Some(warm_state) = load_daemon_warm_state_for_file(file, verbose) {
        if warm_state_supports_runnable_reuse(&warm_state) {
            return run_source_file_with_optional_warm_state(
                file,
                args,
                backend,
                opt_level,
                tuning,
                verbose,
                features,
                Some(&warm_state),
            );
        } else if verbose && warm_state.mir.is_some() {
            eprintln!(
                "[agamc] warm state for `{}` is incomplete for runnable reuse; falling back to local compilation",
                file.display()
            );
        }
    }

    run_source_file_with_optional_warm_state(
        file, args, backend, opt_level, tuning, verbose, features, None,
    )
}

fn warm_state_mir<'a>(
    file: &Path,
    warm_state: &'a WarmState,
) -> Result<&'a agam_mir::ir::MirModule, String> {
    warm_state
        .mir
        .as_ref()
        .ok_or_else(|| format!("warm MIR state missing for `{}`", file.display()))
}

fn warm_state_source_features<'a>(
    file: &Path,
    warm_state: &'a WarmState,
) -> Result<&'a SourceFeatureFlags, String> {
    warm_state
        .source_features
        .as_ref()
        .ok_or_else(|| format!("warm source features missing for `{}`", file.display()))
}

fn source_features_from_call_cache(call_cache: CallCacheSelection) -> SourceFeatureFlags {
    SourceFeatureFlags {
        call_cache,
        experimental_usages: Vec::new(),
    }
}

fn warm_state_supports_runnable_reuse(warm_state: &WarmState) -> bool {
    warm_state.mir.is_some() && warm_state.source_features.is_some()
}

fn warm_state_module<'a>(
    file: &Path,
    warm_state: &'a WarmState,
) -> Result<&'a agam_ast::Module, String> {
    warm_state
        .module
        .as_ref()
        .ok_or_else(|| format!("warm AST module missing for `{}`", file.display()))
}

fn load_daemon_prewarmed_entry(path: &PathBuf, verbose: bool) -> Option<DaemonPrewarmedEntry> {
    let workspace = match resolve_daemon_workspace_target(Some(path.clone())) {
        Ok(workspace) => workspace,
        Err(error) => {
            if verbose {
                eprintln!("[agamc] daemon prewarm lookup skipped: {error}");
            }
            return None;
        }
    };
    let status = match read_daemon_status(&workspace.root) {
        Ok(Some(status)) => status,
        Ok(None) => return None,
        Err(error) => {
            if verbose {
                eprintln!("[agamc] daemon prewarm status unavailable: {error}");
            }
            return None;
        }
    };
    let prewarm = &status.prewarm;
    if !prewarm.package_ready {
        return None;
    }

    let Some(entry_path) = prewarm.entry_path.as_deref() else {
        return None;
    };
    if Path::new(entry_path) != path.as_path() {
        return None;
    }

    let source = match std::fs::read(path) {
        Ok(source) => source,
        Err(error) => {
            if verbose {
                eprintln!(
                    "[agamc] daemon prewarm source hash check failed for `{}`: {}",
                    path.display(),
                    error
                );
            }
            return None;
        }
    };
    let source_hash = agam_runtime::cache::hash_bytes(&source);
    if prewarm.entry_content_hash.as_deref() != Some(source_hash.as_str()) {
        return None;
    }

    let Some(package_artifact_path) = prewarm.package_artifact_path.as_ref() else {
        return None;
    };
    let artifact_path = PathBuf::from(package_artifact_path);
    let package = match agam_pkg::read_package_from_path(&artifact_path) {
        Ok(package) => package,
        Err(error) => {
            if verbose {
                eprintln!(
                    "[agamc] daemon prewarm package load failed from `{}`: {}",
                    artifact_path.display(),
                    error
                );
            }
            return None;
        }
    };

    if verbose {
        eprintln!(
            "[agamc] Reused daemon prewarmed entry package: {}",
            artifact_path.display()
        );
    }

    Some(DaemonPrewarmedEntry {
        package,
        call_cache: prewarm.call_cache.clone(),
    })
}

fn warm_state_from_daemon_prewarmed_entry(prewarmed: DaemonPrewarmedEntry) -> WarmState {
    WarmState {
        source_features: Some(SourceFeatureFlags {
            call_cache: prewarmed.call_cache,
            experimental_usages: Vec::new(),
        }),
        module: None,
        hir: None,
        mir: Some(prewarmed.package.mir),
    }
}

fn load_daemon_prewarmed_warm_state(path: &PathBuf, verbose: bool) -> Option<WarmState> {
    load_daemon_prewarmed_entry(path, verbose).map(warm_state_from_daemon_prewarmed_entry)
}

fn run_source_file_with_optional_warm_state(
    file: &PathBuf,
    args: &[String],
    backend: Backend,
    opt_level: u8,
    tuning: &ReleaseTuning,
    verbose: bool,
    features: FeatureFlags,
    warm_state: Option<&WarmState>,
) -> Result<i32, String> {
    let exe_path = default_build_output_path(file, tuning.target.as_deref())?;

    if backend == Backend::Jit {
        let mut runtime_args = Vec::with_capacity(args.len() + 1);
        runtime_args.push(file.to_string_lossy().to_string());
        runtime_args.extend(args.iter().cloned());
        return match warm_state {
            Some(warm_state) => run_with_jit_prelowered(
                file,
                &runtime_args,
                warm_state_mir(file, warm_state)?,
                warm_state_source_features(file, warm_state)?,
                verbose,
                features,
            ),
            None => run_with_jit(file, &runtime_args, verbose, features),
        };
    }

    if backend == Backend::Llvm {
        return match warm_state {
            Some(warm_state) => run_with_llvm_prelowered(
                file,
                args,
                opt_level,
                tuning,
                warm_state_mir(file, warm_state)?,
                warm_state_source_features(file, warm_state)?,
                verbose,
                features,
            ),
            None => run_with_llvm(file, args, opt_level, tuning, verbose, features),
        };
    }

    let outcome = match warm_state {
        Some(warm_state) => {
            let call_cache = effective_call_cache_selection(
                features,
                warm_state_source_features(file, warm_state)?,
            );
            build_prelowered_file(
                file,
                &exe_path,
                opt_level,
                backend,
                tuning,
                warm_state_mir(file, warm_state)?,
                &call_cache,
                &[],
                false,
                verbose,
            )?
        }
        None => build_file(
            file, &exe_path, opt_level, backend, tuning, features, verbose,
        )?,
    };
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

fn dev_daemon_status_message(root: &Path) -> Result<String, String> {
    let Some(status) = read_daemon_status(root)? else {
        return Ok(
            "daemon: not connected (run `agamc daemon` for incremental warm-state reuse)".into(),
        );
    };

    let now = now_unix_ms();
    if let Some(error) = status.last_error.as_ref() {
        return Ok(format!(
            "daemon: last warm refresh failed ({}; run `agamc daemon` again after fixing the workspace)",
            error
        ));
    }
    Ok(match daemon_liveness(&status, now) {
        DaemonLiveness::Running => format!(
            "daemon: connected (warm-state pipeline active; {} file(s) warm)",
            status.warmed_file_count
        ),
        DaemonLiveness::Snapshot => format!(
            "daemon: snapshot available (last warm refresh {}; run `agamc daemon` for continuous incremental reuse)",
            relative_age(now.saturating_sub(status.last_heartbeat_unix_ms))
        ),
        DaemonLiveness::Stale => format!(
            "daemon: stale (last heartbeat {}; run `agamc daemon` for incremental warm-state reuse)",
            relative_age(now.saturating_sub(status.last_heartbeat_unix_ms))
        ),
    })
}

fn run_dev_workflow(
    path: Option<PathBuf>,
    environment: Option<EnvironmentInspectReport>,
    backend: Backend,
    opt_level: u8,
    fix: bool,
    no_run: bool,
    no_tests: bool,
    verbose: bool,
) -> Result<(), String> {
    let session = resolve_workspace_session_for_driver(path)?;
    let workspace = session.layout.clone();
    let cache = agam_runtime::cache::CacheStore::for_path(&workspace.root)?;
    let cache_status = cache.status(3)?;
    let native_llvm = resolve_native_llvm_command();
    let requested_target = environment
        .as_ref()
        .and_then(|report| report.environment.target.clone());
    let requested_backend = requested_backend_for_command(
        backend,
        environment.as_ref(),
        true,
        requested_target.as_deref(),
    );
    let resolved_backend = resolve_backend(requested_backend, !no_run);

    // Resolve or refresh the lockfile for manifested workspaces.
    let lockfile = try_lockfile_refresh(&session, verbose)?;

    println!("Agam Dev");
    println!("workspace: {}", workspace.root.display());
    if let Some(manifest) = workspace.manifest_path.as_ref() {
        println!("manifest: {}", manifest.display());
    } else {
        println!("manifest: none");
    }
    println!("project: {}", workspace.project_name);
    println!("entry: {}", workspace.entry_file.display());
    if let Some(environment) = environment.as_ref() {
        println!("environment: {}", environment_selection_label(environment));
        println!(
            "environment target: {}",
            environment.environment.target.as_deref().unwrap_or("host")
        );
        println!(
            "environment backend: {}",
            environment
                .environment
                .preferred_backend
                .map(runtime_backend_label)
                .unwrap_or("auto")
        );
    }
    println!("sources: {}", workspace.source_files.len());
    println!("tests: {}", workspace.test_files.len());
    if let Some(ref lf) = lockfile {
        println!("dependencies: {} (locked)", lf.packages.len());
    }
    println!(
        "cache: {} / {}",
        cache_status.entry_count,
        human_bytes(cache_status.total_bytes)
    );
    println!("{}", dev_daemon_status_message(&workspace.root)?);
    if let Some(status) = read_daemon_status(&workspace.root)? {
        if let Some(message) = daemon_prewarm_status_message(&status.prewarm) {
            println!("{message}");
        }
    }
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

    if verbose {
        let action = if !fix { "Checking" } else { "Formatting" };
        eprintln!("[agamc] {} {} file(s)...", action, files_to_format.len());
    }
    let changed = agam_fmt::format_paths(&files_to_format, !fix)?;
    if !fix && !changed.is_empty() {
        for file in &changed {
            eprintln!("needs formatting: {}", file.display());
        }
        return Err("formatting is not clean; re-run with `agamc dev --fix` or `agamc fmt`".into());
    }
    if fix && !changed.is_empty() {
        eprintln!("\x1b[1;32m✓\x1b[0m Formatted {} file(s).", changed.len());
    }

    let mut ordered_check_files = workspace
        .source_files
        .iter()
        .map(|file| (file.clone(), *file == workspace.entry_file && !no_run))
        .collect::<Vec<_>>();
    ordered_check_files.extend(
        workspace
            .test_files
            .iter()
            .cloned()
            .map(|file| (file, false)),
    );

    let nested_check_requests = ordered_check_files
        .iter()
        .filter(|(_, keep_warm_state)| !keep_warm_state)
        .map(|(file, _)| CheckRequest { file: file.clone() })
        .collect::<Vec<_>>();
    let parallel_nested_checks = nested_check_requests.len() > 1;
    let nested_results = if parallel_nested_checks {
        execute_parallel_check_requests(&nested_check_requests, verbose)
    } else {
        Vec::new()
    };

    let mut had_errors = false;
    let mut warmed_entry_state = None;
    let mut next_nested_result = 0usize;
    for (file, keep_warm_state) in &ordered_check_files {
        if *keep_warm_state {
            match compile_dev_source_file(file, true, verbose) {
                Ok(warm) => warmed_entry_state = warm,
                Err(error) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", error);
                    had_errors = true;
                }
            }
            continue;
        }

        if parallel_nested_checks {
            let result = &nested_results[next_nested_result];
            next_nested_result += 1;
            match replay_check_request_output(result) {
                Ok(succeeded) => had_errors |= !succeeded,
                Err(error) => {
                    eprintln!("\x1b[1;31merror\x1b[0m: {}", error);
                    had_errors = true;
                }
            }
        } else if let Err(error) = run_check_request_locally(file, verbose) {
            eprintln!("\x1b[1;31merror\x1b[0m: {}", error);
            had_errors = true;
        }
    }

    if had_errors {
        return Err("type checks failed".into());
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
        target: requested_target,
        native_cpu: true,
        lto: None,
        pgo_generate: None,
        pgo_use: None,
    };
    let features = FeatureFlags::default();
    let code = run_source_file_with_optional_warm_state(
        &workspace.entry_file,
        &[],
        resolved_backend,
        opt_level.min(3),
        &tuning,
        verbose,
        features,
        warmed_entry_state.as_ref(),
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

fn now_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn daemon_status_path(root: &Path) -> PathBuf {
    root.join(".agam_cache").join("daemon").join("status.json")
}

fn daemon_pid_path(root: &Path) -> PathBuf {
    root.join(".agam_cache").join("daemon").join("daemon.pid")
}

fn daemon_shutdown_path(root: &Path) -> PathBuf {
    root.join(".agam_cache")
        .join("daemon")
        .join("shutdown_requested")
}

fn daemon_port_path(root: &Path) -> PathBuf {
    root.join(".agam_cache").join("daemon").join("daemon.port")
}

fn ensure_daemon_status_dir(root: &Path) -> Result<PathBuf, String> {
    let dir = root.join(".agam_cache").join("daemon");
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "failed to create daemon status directory `{}`: {e}",
            dir.display()
        )
    })?;
    Ok(dir)
}

fn read_daemon_status(root: &Path) -> Result<Option<DaemonStatusRecord>, String> {
    let path = daemon_status_path(root);
    if !path.is_file() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read daemon status `{}`: {e}", path.display()))?;
    let status: DaemonStatusRecord = serde_json::from_str(&json)
        .map_err(|e| format!("failed to parse daemon status `{}`: {e}", path.display()))?;
    if status.schema_version != DAEMON_STATUS_SCHEMA_VERSION {
        return Ok(None);
    }
    Ok(Some(status))
}

fn write_daemon_status(root: &Path, status: &DaemonStatusRecord) -> Result<(), String> {
    ensure_daemon_status_dir(root)?;
    let path = daemon_status_path(root);
    let json = serde_json::to_vec_pretty(status)
        .map_err(|e| format!("failed to serialize daemon status: {e}"))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("failed to write daemon status `{}`: {e}", path.display()))
}

fn clear_daemon_status(path: Option<PathBuf>, verbose: bool) -> Result<(), String> {
    let workspace = resolve_daemon_workspace_target(path)?;
    let status_path = daemon_status_path(&workspace.root);
    let warm_index_path = agam_pkg::daemon_warm_index_path(&workspace.root);
    let prewarm_dir = daemon_prewarm_stage_dir(&workspace.root);
    let pid_path = daemon_pid_path(&workspace.root);
    let shutdown_path = daemon_shutdown_path(&workspace.root);
    let port_path = daemon_port_path(&workspace.root);

    if status_path.is_file() {
        std::fs::remove_file(&status_path).map_err(|e| {
            format!(
                "failed to remove daemon status `{}`: {e}",
                status_path.display()
            )
        })?;
        println!("Agam Daemon");
        println!("workspace: {}", workspace.root.display());
        println!("status: cleared");
    } else {
        println!("Agam Daemon");
        println!("workspace: {}", workspace.root.display());
        println!("status: already clear");
    }

    // Clean warm index
    if warm_index_path.is_file() {
        let _ = std::fs::remove_file(&warm_index_path);
        if verbose {
            println!("warm-index: cleared");
        }
    }

    // Clean prewarm directory (MIR artifacts)
    if prewarm_dir.is_dir() {
        let _ = std::fs::remove_dir_all(&prewarm_dir);
        if verbose {
            println!("prewarm-dir: cleared");
        }
    }

    // Clean PID lock and shutdown sentinel
    if pid_path.is_file() {
        let _ = std::fs::remove_file(&pid_path);
        if verbose {
            println!("pid-lock: cleared");
        }
    }
    if shutdown_path.is_file() {
        let _ = std::fs::remove_file(&shutdown_path);
        if verbose {
            println!("shutdown-sentinel: cleared");
        }
    }
    if port_path.is_file() {
        let _ = std::fs::remove_file(&port_path);
        if verbose {
            println!("ipc-port: cleared");
        }
    }

    if verbose {
        println!("status-file: {}", status_path.display());
    }
    Ok(())
}

fn daemon_liveness(status: &DaemonStatusRecord, now: u128) -> DaemonLiveness {
    if status.run_mode == DaemonRunMode::OneShot {
        return DaemonLiveness::Snapshot;
    }
    if now.saturating_sub(status.last_heartbeat_unix_ms) <= DAEMON_HEARTBEAT_STALE_MS {
        DaemonLiveness::Running
    } else {
        DaemonLiveness::Stale
    }
}

#[cfg(test)]
fn active_daemon_status(root: &Path) -> Result<Option<DaemonStatusRecord>, String> {
    let Some(status) = read_daemon_status(root)? else {
        return Ok(None);
    };
    if status.last_error.is_some() {
        return Ok(None);
    }
    if daemon_liveness(&status, now_unix_ms()) == DaemonLiveness::Running {
        Ok(Some(status))
    } else {
        Ok(None)
    }
}

fn tracked_snapshot_file_count(snapshot: &agam_pkg::WorkspaceSnapshot) -> usize {
    snapshot.manifests.len() + snapshot.source_files.len() + snapshot.test_files.len()
}

fn workspace_diff_is_empty(diff: &agam_pkg::WorkspaceSnapshotDiff) -> bool {
    diff.added_files.is_empty() && diff.changed_files.is_empty() && diff.removed_files.is_empty()
}

fn snapshot_diff_touches_manifest(
    previous: &agam_pkg::WorkspaceSnapshot,
    next: &agam_pkg::WorkspaceSnapshot,
    diff: &agam_pkg::WorkspaceSnapshotDiff,
) -> bool {
    let previous_manifests = previous
        .manifests
        .iter()
        .map(|file| &file.path)
        .collect::<BTreeSet<_>>();
    let next_manifests = next
        .manifests
        .iter()
        .map(|file| &file.path)
        .collect::<BTreeSet<_>>();
    diff.added_files
        .iter()
        .chain(&diff.changed_files)
        .chain(&diff.removed_files)
        .any(|path| previous_manifests.contains(path) || next_manifests.contains(path))
}

fn summarize_workspace_diff(
    previous: Option<&agam_pkg::WorkspaceSnapshot>,
    next: &agam_pkg::WorkspaceSnapshot,
    diff: Option<&agam_pkg::WorkspaceSnapshotDiff>,
) -> DaemonDiffSummary {
    match (previous, diff) {
        (Some(previous), Some(diff)) => DaemonDiffSummary {
            added_files: diff.added_files.len(),
            changed_files: diff.changed_files.len(),
            removed_files: diff.removed_files.len(),
            unchanged_files: diff.unchanged_files.len(),
            manifest_changed: snapshot_diff_touches_manifest(previous, next, diff),
        },
        _ => DaemonDiffSummary {
            added_files: tracked_snapshot_file_count(next),
            ..DaemonDiffSummary::default()
        },
    }
}

fn daemon_diff_has_changes(summary: &DaemonDiffSummary) -> bool {
    summary.manifest_changed
        || summary.added_files > 0
        || summary.changed_files > 0
        || summary.removed_files > 0
}

fn daemon_entry_snapshot<'a>(
    snapshot: &'a agam_pkg::WorkspaceSnapshot,
) -> Option<&'a agam_pkg::WorkspaceFileSnapshot> {
    snapshot
        .source_files
        .iter()
        .chain(&snapshot.test_files)
        .find(|file| file.path == snapshot.session.layout.entry_file)
}

fn warm_state_for_snapshot_file<'a>(
    session: &'a DaemonSession,
    file: &agam_pkg::WorkspaceFileSnapshot,
) -> Option<&'a WarmState> {
    session
        .cache
        .get(&file.path)
        .and_then(|versions| versions.get(&file.content_hash))
}

fn record_prewarm_error(summary: &mut DaemonPrewarmSummary, message: String) {
    match summary.last_error.as_mut() {
        Some(existing) => {
            existing.push_str(" | ");
            existing.push_str(&message);
        }
        None => summary.last_error = Some(message),
    }
}

fn daemon_prewarm_stage_dir(root: &Path) -> PathBuf {
    root.join(".agam_cache").join("daemon").join("prewarm")
}

fn ensure_daemon_prewarm_stage_dir(root: &Path) -> Result<PathBuf, String> {
    let dir = daemon_prewarm_stage_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "failed to create daemon prewarm directory `{}`: {e}",
            dir.display()
        )
    })?;
    Ok(dir)
}

fn daemon_prewarm_stage_prefix(
    root: &Path,
    entry_file: &Path,
    suffix: &str,
) -> Result<PathBuf, String> {
    let dir = ensure_daemon_prewarm_stage_dir(root)?;
    let hash = agam_runtime::cache::hash_bytes(entry_file.to_string_lossy().as_bytes());
    let stem = entry_file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("entry")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    Ok(dir.join(format!("{stem}_{suffix}_{hash}")))
}

fn daemon_prewarm_package_output(root: &Path, entry_file: &Path) -> Result<PathBuf, String> {
    Ok(daemon_prewarm_stage_prefix(root, entry_file, "package")?.with_extension("agpkg.json"))
}

fn daemon_prewarm_build_output(
    root: &Path,
    entry_file: &Path,
    target: Option<&str>,
    backend: Backend,
) -> Result<PathBuf, String> {
    let mut output =
        daemon_prewarm_stage_prefix(root, entry_file, render_backend_cli_value(backend))?;
    if native_binary_extension(target) == Some("exe") {
        output.set_extension("exe");
    }
    Ok(output)
}

fn clean_prewarm_output(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

fn build_outcome_artifact_kind_label(backend: Backend, outcome: &BuildOutcome) -> &'static str {
    if outcome.native_binary {
        "native-binary"
    } else {
        match backend {
            Backend::C => "c-source",
            Backend::Llvm => "llvm-ir",
            Backend::Auto | Backend::Jit => "artifact",
        }
    }
}

fn daemon_prewarm_package_artifact_missing(prewarm: &DaemonPrewarmSummary) -> bool {
    if !prewarm.package_ready {
        return false;
    }
    prewarm
        .package_artifact_path
        .as_deref()
        .map(|path| !Path::new(path).is_file())
        .unwrap_or(true)
}

fn daemon_prewarm_needs_refresh(prewarm: &DaemonPrewarmSummary) -> bool {
    daemon_prewarm_package_artifact_missing(prewarm)
}

fn daemon_prewarm_status_message(prewarm: &DaemonPrewarmSummary) -> Option<String> {
    if !prewarm.package_ready
        && !prewarm.build_ready
        && prewarm.build_backend.is_none()
        && prewarm.last_error.is_none()
        && prewarm.prewarmed_file_count == 0
    {
        return None;
    }

    let package = if daemon_prewarm_package_artifact_missing(prewarm) {
        "stale (artifact missing)"
    } else if prewarm.package_ready {
        "ready"
    } else {
        "cold"
    };
    let build = match prewarm.build_backend.as_deref() {
        Some("jit") => "warm MIR only via jit".to_string(),
        Some(backend) if prewarm.build_ready => {
            let artifact = prewarm.build_artifact_kind.as_deref().unwrap_or("artifact");
            format!("ready via {backend} ({artifact})")
        }
        Some(backend) => format!("cold via {backend}"),
        None => "none".to_string(),
    };
    let files = if prewarm.prewarmed_total_files > 0 {
        format!(
            ", warm files {}/{}",
            prewarm.prewarmed_file_count, prewarm.prewarmed_total_files
        )
    } else {
        String::new()
    };

    Some(format!("prewarm: package {package}, build {build}{files}"))
}

fn prewarm_daemon_entry_artifacts(
    session: &DaemonSession,
    snapshot: &agam_pkg::WorkspaceSnapshot,
    verbose: bool,
) -> DaemonPrewarmSummary {
    let mut summary = DaemonPrewarmSummary::default();

    // --- Multi-file warm index prewarm ---
    let root = &snapshot.session.layout.root;
    let all_files: Vec<_> = snapshot
        .source_files
        .iter()
        .chain(&snapshot.test_files)
        .collect();
    summary.prewarmed_total_files = all_files.len();

    let mut warm_index = agam_pkg::DaemonWarmIndex {
        format_version: agam_pkg::DAEMON_WARM_INDEX_FORMAT_VERSION,
        files: BTreeMap::new(),
    };

    for file_snapshot in &all_files {
        let Some(warm_state) = warm_state_for_snapshot_file(session, file_snapshot) else {
            continue;
        };
        let Some(mir) = warm_state.mir.as_ref() else {
            // File was parsed/checked but not lowered — record at a lower warm level
            let warm_level = if warm_state.hir.is_some() {
                agam_pkg::DaemonWarmLevel::Checked
            } else {
                agam_pkg::DaemonWarmLevel::Parsed
            };
            warm_index.files.insert(
                file_snapshot.path.display().to_string(),
                agam_pkg::DaemonWarmFileEntry {
                    content_hash: file_snapshot.content_hash.clone(),
                    mir_hash: None,
                    artifact_path: None,
                    warm_level,
                },
            );
            summary.prewarmed_file_count += 1;
            continue;
        };

        // Serialize per-file MIR artifact to the prewarm staging directory
        let mir_hash = agam_runtime::cache::hash_serializable(mir).unwrap_or_default();
        let artifact_path = match daemon_prewarm_mir_artifact_path(root, &file_snapshot.path) {
            Ok(path) => path,
            Err(error) => {
                if verbose {
                    eprintln!(
                        "[agamc] daemon warm index: skipped `{}`: {error}",
                        file_snapshot.path.display()
                    );
                }
                continue;
            }
        };

        match write_warm_artifact(
            &artifact_path,
            mir,
            warm_state
                .source_features
                .as_ref()
                .map(|features| &features.call_cache),
        ) {
            Ok(()) => {
                warm_index.files.insert(
                    file_snapshot.path.display().to_string(),
                    agam_pkg::DaemonWarmFileEntry {
                        content_hash: file_snapshot.content_hash.clone(),
                        mir_hash: Some(mir_hash),
                        artifact_path: Some(artifact_path.display().to_string()),
                        warm_level: agam_pkg::DaemonWarmLevel::Lowered,
                    },
                );
                summary.prewarmed_file_count += 1;
            }
            Err(error) => {
                if verbose {
                    eprintln!(
                        "[agamc] daemon warm index: failed to write MIR for `{}`: {error}",
                        file_snapshot.path.display()
                    );
                }
            }
        }
    }

    // Write the warm index
    if let Err(error) = agam_pkg::write_daemon_warm_index(root, &warm_index) {
        record_prewarm_error(
            &mut summary,
            format!("failed to write daemon warm index: {error}"),
        );
    } else if verbose {
        eprintln!(
            "[agamc] daemon warm index: {}/{} file(s) indexed",
            summary.prewarmed_file_count, summary.prewarmed_total_files
        );
    }

    // Clean stale MIR artifacts that are no longer in the warm index
    let valid_mir_paths: HashSet<PathBuf> = warm_index
        .files
        .values()
        .filter_map(|entry| entry.artifact_path.as_deref().map(PathBuf::from))
        .collect();
    let prewarm_dir = daemon_prewarm_stage_dir(root);
    if prewarm_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&prewarm_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let is_mir_artifact = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.contains("_mir_") && name.ends_with(".json"))
                    .unwrap_or(false);
                if is_mir_artifact && !valid_mir_paths.contains(&path) {
                    if verbose {
                        eprintln!(
                            "[agamc] daemon warm index: cleaning stale MIR artifact `{}`",
                            path.display()
                        );
                    }
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }

    // --- Entry-file portable package prewarm (existing behavior, preserved) ---
    let Some(entry_snapshot) = daemon_entry_snapshot(snapshot) else {
        record_prewarm_error(
            &mut summary,
            format!(
                "entry file `{}` is missing from the daemon snapshot",
                snapshot.session.layout.entry_file.display()
            ),
        );
        return summary;
    };
    let Some(warm_state) = warm_state_for_snapshot_file(session, entry_snapshot) else {
        record_prewarm_error(
            &mut summary,
            format!(
                "warm state is missing for daemon entry file `{}`",
                entry_snapshot.path.display()
            ),
        );
        return summary;
    };

    let entry_file = &entry_snapshot.path;
    summary.entry_path = Some(entry_file.display().to_string());
    summary.entry_content_hash = Some(entry_snapshot.content_hash.clone());
    let source = match std::fs::read_to_string(entry_file) {
        Ok(source) => source,
        Err(error) => {
            record_prewarm_error(
                &mut summary,
                format!(
                    "failed to read daemon entry file `{}` for prewarm: {error}",
                    entry_file.display()
                ),
            );
            return summary;
        }
    };

    let mir = match warm_state_mir(entry_file, warm_state) {
        Ok(mir) => mir,
        Err(error) => {
            record_prewarm_error(&mut summary, error);
            return summary;
        }
    };
    let module = match warm_state_module(entry_file, warm_state) {
        Ok(module) => module,
        Err(error) => {
            record_prewarm_error(&mut summary, error);
            return summary;
        }
    };
    let source_features = match warm_state_source_features(entry_file, warm_state) {
        Ok(features) => features,
        Err(error) => {
            record_prewarm_error(&mut summary, error);
            return summary;
        }
    };
    summary.call_cache = source_features.call_cache.clone();

    match daemon_prewarm_package_output(root, entry_file) {
        Ok(output) => {
            let package = agam_pkg::build_portable_package(
                entry_file,
                &source,
                module,
                mir,
                agam_runtime::contract::RuntimeBackend::Jit,
            );
            match write_portable_package_with_cache(entry_file, &output, &package, verbose) {
                Ok(hit) => {
                    summary.package_ready = true;
                    summary.package_artifact_path = Some(hit.artifact_path.display().to_string());
                    clean_prewarm_output(&output);
                }
                Err(error) => record_prewarm_error(
                    &mut summary,
                    format!(
                        "portable package prewarm failed for `{}`: {error}",
                        entry_file.display()
                    ),
                ),
            }
        }
        Err(error) => record_prewarm_error(&mut summary, error),
    }

    let build_backend = resolve_backend(Backend::Auto, true);
    summary.build_backend = Some(render_backend_cli_value(build_backend).to_string());
    if build_backend == Backend::Jit {
        return summary;
    }

    let tuning = ReleaseTuning {
        target: None,
        native_cpu: true,
        lto: None,
        pgo_generate: None,
        pgo_use: None,
    };
    let call_cache = effective_call_cache_selection(FeatureFlags::default(), source_features);
    match daemon_prewarm_build_output(root, entry_file, tuning.target.as_deref(), build_backend) {
        Ok(output) => {
            let allow_wsl_llvm = build_backend == Backend::Llvm && allow_dev_wsl_llvm();
            match build_prelowered_file(
                &entry_snapshot.path,
                &output,
                3,
                build_backend,
                &tuning,
                mir,
                &call_cache,
                &[],
                allow_wsl_llvm,
                verbose,
            ) {
                Ok(outcome) => {
                    summary.build_ready = true;
                    summary.build_artifact_kind = Some(
                        build_outcome_artifact_kind_label(build_backend, &outcome).to_string(),
                    );
                    clean_prewarm_output(&output);
                    if outcome.generated_path != output {
                        clean_prewarm_output(&outcome.generated_path);
                    }
                }
                Err(error) => record_prewarm_error(
                    &mut summary,
                    format!(
                        "build prewarm failed for `{}` via {}: {error}",
                        entry_file.display(),
                        render_backend_cli_value(build_backend)
                    ),
                ),
            }
        }
        Err(error) => record_prewarm_error(&mut summary, error),
    }

    summary
}

fn daemon_prewarm_mir_artifact_path(root: &Path, file: &Path) -> Result<PathBuf, String> {
    let dir = ensure_daemon_prewarm_stage_dir(root)?;
    let hash = agam_runtime::cache::hash_bytes(file.to_string_lossy().as_bytes());
    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("file")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    Ok(dir.join(format!("{stem}_mir_{hash}.json")))
}

fn write_warm_artifact(
    path: &Path,
    mir: &agam_mir::ir::MirModule,
    call_cache: Option<&CallCacheSelection>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "failed to create MIR artifact directory `{}`: {e}",
                parent.display()
            )
        })?;
    }
    let json = serde_json::to_vec(&DaemonWarmArtifact { mir, call_cache })
        .map_err(|e| format!("failed to serialize daemon warm artifact: {e}"))?;
    std::fs::write(path, json)
        .map_err(|e| format!("failed to write MIR artifact `{}`: {e}", path.display()))
}

fn read_warm_artifact(path: &Path) -> Result<WarmState, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read MIR artifact `{}`: {e}", path.display()))?;
    if let Ok(artifact) = serde_json::from_str::<DaemonWarmArtifactOwned>(&json) {
        return Ok(WarmState {
            source_features: artifact.call_cache.map(source_features_from_call_cache),
            module: None,
            hir: None,
            mir: Some(artifact.mir),
        });
    }

    let mir = serde_json::from_str(&json)
        .map_err(|e| format!("failed to parse MIR artifact `{}`: {e}", path.display()))?;
    Ok(WarmState {
        source_features: None,
        module: None,
        hir: None,
        mir: Some(mir),
    })
}

/// Attempt to load daemon-prewarmed warm state for any file via the IPC or warm index.
fn load_daemon_warm_state_for_file(path: &Path, verbose: bool) -> Option<WarmState> {
    let workspace = match resolve_daemon_workspace_target(Some(path.to_path_buf())) {
        Ok(workspace) => workspace,
        Err(error) => {
            if verbose {
                eprintln!("[agamc] warm state lookup skipped: {}", error);
            }
            return None;
        }
    };

    let source_bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            if verbose {
                eprintln!(
                    "[agamc] warm state hash check failed for `{}`: {}",
                    path.display(),
                    error
                );
            }
            return None;
        }
    };
    let current_hash = agam_runtime::cache::hash_bytes(&source_bytes);

    // 1. Try IPC query first
    let req = DaemonIpcRequest::GetWarmMir {
        file_path: path.display().to_string(),
        content_hash: current_hash.clone(),
    };

    if let Ok(DaemonIpcResponse::WarmMir {
        found,
        mir_json,
        call_cache_json,
    }) = send_daemon_ipc_request(&workspace.root, req)
    {
        if found {
            if verbose {
                eprintln!("[agamc] IPC warm cache hit for `{}`", path.display());
            }
            let mut warm = WarmState {
                source_features: None,
                module: None,
                hir: None,
                mir: None,
            };
            if let Some(json) = mir_json {
                if let Ok(mir) = serde_json::from_str(&json) {
                    warm.mir = Some(mir);
                } else if verbose {
                    eprintln!("[agamc] IPC warm cache parse err for `{}`", path.display());
                }
            }
            if let Some(json) = call_cache_json {
                if let Ok(call_cache) = serde_json::from_str(&json) {
                    warm.source_features = Some(source_features_from_call_cache(call_cache));
                } else if verbose {
                    eprintln!(
                        "[agamc] IPC warm cache call-cache parse err for `{}`",
                        path.display()
                    );
                }
            }
            return Some(warm);
        } else {
            if verbose {
                eprintln!("[agamc] IPC warm cache miss for `{}`", path.display());
            }
            return None; // Daemon definitively doesn't have it matching the hash
        }
    }

    // 2. Fallback to Disk Index
    let index = match agam_pkg::read_daemon_warm_index(&workspace.root) {
        Ok(Some(index)) => index,
        _ => return None,
    };

    let key = path.display().to_string();
    let entry = match index.files.get(&key) {
        Some(e) => e,
        None => return None,
    };

    if current_hash != entry.content_hash {
        if verbose {
            eprintln!("[agamc] disk warm index stale for `{}`", path.display());
        }
        return None;
    }

    if entry.warm_level == agam_pkg::DaemonWarmLevel::Checked {
        if verbose {
            eprintln!("[agamc] Reused checked warm state for `{}`", path.display());
        }
        return Some(WarmState {
            source_features: None,
            module: None,
            hir: None,
            mir: None,
        });
    }

    if entry.warm_level == agam_pkg::DaemonWarmLevel::Lowered {
        let warm_state = entry.artifact_path.as_deref().and_then(|artifact_path| {
            let artifact = Path::new(artifact_path);
            if !artifact.is_file() {
                return None;
            }
            read_warm_artifact(artifact).ok()
        });
        if verbose && warm_state.is_some() {
            eprintln!("[agamc] Reused disk warm state for `{}`", path.display());
        }
        return warm_state;
    }
    None
}

fn build_daemon_status(
    snapshot: &agam_pkg::WorkspaceSnapshot,
    warm: WarmSummary,
    last_diff: DaemonDiffSummary,
    prewarm: DaemonPrewarmSummary,
    session_started_unix_ms: u128,
    run_mode: DaemonRunMode,
) -> DaemonStatusRecord {
    DaemonStatusRecord {
        schema_version: DAEMON_STATUS_SCHEMA_VERSION,
        run_mode,
        workspace_root: snapshot.session.layout.root.display().to_string(),
        project_name: snapshot.session.layout.project_name.clone(),
        pid: process::id(),
        session_started_unix_ms,
        last_heartbeat_unix_ms: now_unix_ms(),
        snapshot_file_count: tracked_snapshot_file_count(snapshot),
        warmed_file_count: snapshot.source_files.len() + snapshot.test_files.len(),
        warmed_version_count: warm.warmed_version_count,
        ast_decl_count: warm.ast_decl_count,
        hir_function_count: warm.hir_function_count,
        mir_function_count: warm.mir_function_count,
        last_error: None,
        prewarm,
        last_diff,
    }
}

fn build_daemon_error_status(
    snapshot: &agam_pkg::WorkspaceSnapshot,
    warm_cache: WarmCacheSummary,
    prewarm: DaemonPrewarmSummary,
    session_started_unix_ms: u128,
    run_mode: DaemonRunMode,
    error: String,
) -> DaemonStatusRecord {
    DaemonStatusRecord {
        schema_version: DAEMON_STATUS_SCHEMA_VERSION,
        run_mode,
        workspace_root: snapshot.session.layout.root.display().to_string(),
        project_name: snapshot.session.layout.project_name.clone(),
        pid: process::id(),
        session_started_unix_ms,
        last_heartbeat_unix_ms: now_unix_ms(),
        snapshot_file_count: tracked_snapshot_file_count(snapshot),
        warmed_file_count: warm_cache.file_count,
        warmed_version_count: warm_cache.version_count,
        ast_decl_count: warm_cache.ast_decl_count,
        hir_function_count: warm_cache.hir_function_count,
        mir_function_count: warm_cache.mir_function_count,
        last_error: Some(error),
        prewarm,
        last_diff: DaemonDiffSummary::default(),
    }
}

fn print_daemon_status(path: Option<PathBuf>, verbose: bool) -> Result<(), String> {
    let workspace = resolve_daemon_workspace_target(path)?;
    let now = now_unix_ms();

    println!("Agam Daemon Status");
    println!("workspace: {}", workspace.root.display());
    println!("project: {}", workspace.project_name);

    let Some(status) = read_daemon_status(&workspace.root)? else {
        println!("status: not running");
        if verbose {
            println!(
                "status-file: {}",
                daemon_status_path(&workspace.root).display()
            );
        }
        return Ok(());
    };

    let heartbeat_age = now.saturating_sub(status.last_heartbeat_unix_ms);
    if status.last_error.is_some() {
        println!("status: error");
    } else {
        match daemon_liveness(&status, now) {
            DaemonLiveness::Running => println!("status: running"),
            DaemonLiveness::Snapshot => println!("status: snapshot"),
            DaemonLiveness::Stale => println!("status: stale"),
        }
    }
    println!("pid: {}", status.pid);
    match status.run_mode {
        DaemonRunMode::BackgroundService => println!("mode: background service"),
        DaemonRunMode::ForegroundLoop => println!("mode: foreground loop"),
        DaemonRunMode::OneShot => println!("mode: one-shot snapshot"),
    }
    println!("heartbeat: {}", relative_age(heartbeat_age));
    println!("tracked files: {}", status.snapshot_file_count);
    println!("warm files: {}", status.warmed_file_count);
    println!("warm versions: {}", status.warmed_version_count);
    println!("parsed declarations: {}", status.ast_decl_count);
    println!(
        "lowered functions: HIR {} / MIR {}",
        status.hir_function_count, status.mir_function_count
    );
    if status.last_diff.manifest_changed {
        println!("last diff: manifest changed, full warm-state reset");
    } else {
        println!(
            "last diff: +{} ~{} -{} ={}",
            status.last_diff.added_files,
            status.last_diff.changed_files,
            status.last_diff.removed_files,
            status.last_diff.unchanged_files
        );
    }
    if let Some(error) = status.last_error.as_ref() {
        println!("last error: {error}");
    }
    if status.prewarm.prewarmed_total_files > 0 {
        println!(
            "warm index: {}/{} file(s) prewarmed",
            status.prewarm.prewarmed_file_count, status.prewarm.prewarmed_total_files
        );
    }
    if let Some(message) = daemon_prewarm_status_message(&status.prewarm) {
        println!("{message}");
    }
    if let Some(error) = status.prewarm.last_error.as_ref() {
        println!("last prewarm error: {error}");
    }
    if verbose {
        println!(
            "status-file: {}",
            daemon_status_path(&workspace.root).display()
        );
    }

    Ok(())
}

fn refresh_daemon_session(
    session: &mut DaemonSession,
    next_snapshot: agam_pkg::WorkspaceSnapshot,
    verbose: bool,
) -> Result<(WarmSummary, DaemonDiffSummary), String> {
    let diff_summary = if let Some(previous) = session.snapshot.as_ref() {
        let diff = agam_pkg::diff_workspace_snapshots(previous, &next_snapshot);
        let summary = summarize_workspace_diff(Some(previous), &next_snapshot, Some(&diff));
        if workspace_diff_is_empty(&diff) {
            session.snapshot = Some(next_snapshot);
        } else {
            let mut pipeline = IncrementalPipeline::new(session);
            pipeline.apply_diff(next_snapshot, &diff);
        }
        summary
    } else {
        let summary = summarize_workspace_diff(None, &next_snapshot, None);
        session.snapshot = Some(next_snapshot);
        summary
    };

    let snapshot = session
        .snapshot
        .clone()
        .ok_or_else(|| "internal error: daemon snapshot missing after refresh".to_string())?;
    let warm = warm_workspace_session(session, &snapshot, verbose)?;
    Ok((warm, diff_summary))
}

fn run_daemon_cycle(
    session: &mut DaemonSession,
    refresh_hint: &Path,
    initial_snapshot: &agam_pkg::WorkspaceSnapshot,
    session_started_unix_ms: u128,
    run_mode: DaemonRunMode,
    verbose: bool,
    first_cycle: bool,
) -> Result<DaemonCycleOutcome, String> {
    let snapshot = if first_cycle {
        initial_snapshot.clone()
    } else {
        match agam_pkg::snapshot_workspace_from_path(refresh_hint) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let status = build_daemon_error_status(
                    session.snapshot.as_ref().unwrap_or(initial_snapshot),
                    summarize_warm_cache(&session.cache),
                    session.last_prewarm.clone(),
                    session_started_unix_ms,
                    run_mode,
                    error.clone(),
                );
                return Ok(DaemonCycleOutcome::Error { status, error });
            }
        }
    };
    let (warm, diff_summary) = match refresh_daemon_session(session, snapshot.clone(), verbose) {
        Ok(result) => result,
        Err(error) => {
            let status = build_daemon_error_status(
                &snapshot,
                summarize_warm_cache(&session.cache),
                session.last_prewarm.clone(),
                session_started_unix_ms,
                run_mode,
                error.clone(),
            );
            return Ok(DaemonCycleOutcome::Error { status, error });
        }
    };
    let should_prewarm = first_cycle
        || daemon_diff_has_changes(&diff_summary)
        || session.last_prewarm.last_error.is_some()
        || daemon_prewarm_needs_refresh(&session.last_prewarm);
    if should_prewarm {
        session.last_prewarm = prewarm_daemon_entry_artifacts(session, &snapshot, verbose);
    }
    let snapshot = session
        .snapshot
        .clone()
        .ok_or_else(|| "internal error: daemon snapshot missing".to_string())?;
    let status = build_daemon_status(
        &snapshot,
        warm,
        diff_summary.clone(),
        session.last_prewarm.clone(),
        session_started_unix_ms,
        run_mode,
    );
    Ok(DaemonCycleOutcome::Success {
        status,
        diff_summary,
        prewarm_ran: should_prewarm,
    })
}
fn spawn_ipc_server(
    workspace_root: &Path,
) -> Result<
    std::sync::mpsc::Receiver<(DaemonIpcRequest, std::sync::mpsc::Sender<DaemonIpcResponse>)>,
    String,
> {
    use std::io::Read;
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("failed to bind IPC listener: {e}"))?;

    let port_path = daemon_port_path(workspace_root);
    if let Some(parent) = port_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let port = listener
        .local_addr()
        .map_err(|e| format!("failed to get IPC port: {e}"))?
        .port();

    std::fs::write(&port_path, format!("{port}"))
        .map_err(|e| format!("failed to write port file: {e}"))?;

    let (req_tx, req_rx) =
        std::sync::mpsc::channel::<(DaemonIpcRequest, std::sync::mpsc::Sender<DaemonIpcResponse>)>(
        );

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut payload = String::new();
                if stream.read_to_string(&mut payload).is_ok() {
                    if let Ok(req) = serde_json::from_str::<DaemonIpcRequest>(&payload) {
                        let (resp_tx, resp_rx) = std::sync::mpsc::channel();
                        if req_tx.send((req, resp_tx)).is_ok() {
                            if let Ok(resp) = resp_rx.recv() {
                                let _ = serde_json::to_writer(&stream, &resp);
                            }
                        }
                    }
                }
            }
        }
    });

    Ok(req_rx)
}

fn send_daemon_ipc_request(
    root: &Path,
    req: DaemonIpcRequest,
) -> Result<DaemonIpcResponse, String> {
    let port_path = daemon_port_path(root);
    let port_str = std::fs::read_to_string(&port_path).map_err(|e| format!("no port file: {e}"))?;
    let port: u16 = port_str
        .trim()
        .parse()
        .map_err(|e| format!("invalid port: {e}"))?;

    let mut stream = std::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .map_err(|e| format!("failed to connect to IPC socket: {e}"))?;

    serde_json::to_writer(&stream, &req).map_err(|e| format!("failed to write JSON: {e}"))?;
    stream.shutdown(std::net::Shutdown::Write).ok();

    use std::io::Read;
    let mut resp_payload = String::new();
    stream
        .read_to_string(&mut resp_payload)
        .map_err(|e| format!("failed to read IPC response: {e}"))?;

    serde_json::from_str(&resp_payload).map_err(|e| format!("failed to parse IPC response: {e}"))
}

fn run_daemon_foreground(
    path: Option<PathBuf>,
    once: bool,
    poll_ms: u64,
    is_background: bool,
    verbose: bool,
) -> Result<(), String> {
    let initial_snapshot = agam_pkg::snapshot_workspace(path)?;
    let workspace = initial_snapshot.session.layout.clone();
    let session_started_unix_ms = now_unix_ms();
    let mut session = DaemonSession::default();
    let mut first_cycle = true;
    let mut last_error = None;
    let refresh_hint = daemon_refresh_snapshot_hint(&workspace);
    let run_mode = if once {
        DaemonRunMode::OneShot
    } else if is_background {
        DaemonRunMode::BackgroundService
    } else {
        DaemonRunMode::ForegroundLoop
    };

    // Write PID lock for background daemon
    if is_background {
        let pid_path = daemon_pid_path(&workspace.root);
        if let Some(parent) = pid_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&pid_path, format!("{}", std::process::id()));
        // Remove any stale shutdown sentinel
        let _ = std::fs::remove_file(daemon_shutdown_path(&workspace.root));
    }

    if !is_background {
        println!("Agam Daemon");
        println!("workspace: {}", workspace.root.display());
        println!("project: {}", workspace.project_name);
        if let Some(manifest) = workspace.manifest_path.as_ref() {
            println!("manifest: {}", manifest.display());
        } else {
            println!("manifest: none");
        }
        if once {
            println!("mode: one-shot warm refresh");
        } else {
            println!("mode: foreground warm loop ({poll_ms} ms poll)");
            println!(
                "status-file: {}",
                daemon_status_path(&workspace.root).display()
            );
        }
    }

    let ipc_rx = if !once {
        spawn_ipc_server(&workspace.root).ok()
    } else {
        None
    };

    loop {
        // Check for shutdown request (background daemon)
        if is_background {
            let shutdown_path = daemon_shutdown_path(&workspace.root);
            if shutdown_path.is_file() {
                // Clean up and exit gracefully
                let _ = std::fs::remove_file(&shutdown_path);
                let _ = std::fs::remove_file(daemon_pid_path(&workspace.root));
                return Ok(());
            }
        }

        match run_daemon_cycle(
            &mut session,
            &refresh_hint,
            &initial_snapshot,
            session_started_unix_ms,
            run_mode,
            verbose,
            first_cycle,
        )? {
            DaemonCycleOutcome::Success {
                status,
                diff_summary,
                prewarm_ran,
            } => {
                let should_log = !is_background
                    && (first_cycle
                        || daemon_diff_has_changes(&diff_summary)
                        || last_error.take().is_some());
                if should_log {
                    println!(
                        "warm: {} file(s), {} version(s), AST {}, HIR {}, MIR {}",
                        status.warmed_file_count,
                        status.warmed_version_count,
                        status.ast_decl_count,
                        status.hir_function_count,
                        status.mir_function_count
                    );
                    if diff_summary.manifest_changed {
                        println!("invalidate: manifest changed, full warm-state reset");
                    } else if diff_summary.added_files > 0
                        || diff_summary.changed_files > 0
                        || diff_summary.removed_files > 0
                    {
                        println!(
                            "invalidate: +{} ~{} -{} ={}",
                            diff_summary.added_files,
                            diff_summary.changed_files,
                            diff_summary.removed_files,
                            diff_summary.unchanged_files
                        );
                    }
                    if let Some(message) = daemon_prewarm_status_message(&status.prewarm) {
                        println!("{message}");
                    }
                }
                if prewarm_ran && !is_background {
                    if let Some(error) = status.prewarm.last_error.as_ref() {
                        eprintln!("[agamc] daemon prewarm failed: {error}");
                    } else if verbose {
                        eprintln!("[agamc] daemon prewarm refreshed");
                    }
                }

                write_daemon_status(&workspace.root, &status)?;
                if once {
                    return Ok(());
                }
            }
            DaemonCycleOutcome::Error { status, error } => {
                write_daemon_status(&workspace.root, &status)?;
                if !is_background && last_error.as_ref() != Some(&error) {
                    eprintln!("[agamc] daemon refresh failed: {error}");
                }
                last_error = Some(error.clone());
                if once {
                    // Clean up PID lock on error exit
                    if is_background {
                        let _ = std::fs::remove_file(daemon_pid_path(&workspace.root));
                    }
                    return Err(error);
                }
            }
        }

        let timeout = std::time::Duration::from_millis(poll_ms.max(100));
        let sleep_start = std::time::Instant::now();

        while sleep_start.elapsed() < timeout {
            let remain = timeout.saturating_sub(sleep_start.elapsed());
            if remain.is_zero() {
                break;
            }
            if let Some(rx) = &ipc_rx {
                match rx.recv_timeout(remain) {
                    Ok((req, resp_tx)) => {
                        let resp = match req {
                            DaemonIpcRequest::Status => {
                                // For status, just write standard status and return it
                                // Or read it from file, but we can reconstruct a basic one or return dummy
                                // Let's just return what's on disk for simplicity
                                if let Ok(Some(st)) = read_daemon_status(&workspace.root) {
                                    DaemonIpcResponse::Status(st)
                                } else {
                                    DaemonIpcResponse::Error("status unknown".into())
                                }
                            }
                            DaemonIpcRequest::GetWarmMir {
                                file_path,
                                content_hash,
                            } => {
                                let pb = PathBuf::from(&file_path);
                                let mut found = false;
                                let mut mir_json = None;
                                let mut call_cache_json = None;
                                if let Some(versions) = session.cache.get(&pb) {
                                    if let Some(state) = versions.get(&content_hash) {
                                        found = true;
                                        if let Some(mir) = &state.mir {
                                            mir_json = serde_json::to_string(mir).ok();
                                        }
                                        if let Some(source_features) = &state.source_features {
                                            call_cache_json =
                                                serde_json::to_string(&source_features.call_cache)
                                                    .ok();
                                        }
                                    }
                                }
                                DaemonIpcResponse::WarmMir {
                                    found,
                                    mir_json,
                                    call_cache_json,
                                }
                            }
                            DaemonIpcRequest::Stop => {
                                let _ = resp_tx.send(DaemonIpcResponse::Error("stopping".into()));
                                let _ = std::fs::remove_file(daemon_pid_path(&workspace.root));
                                let _ = std::fs::remove_file(daemon_port_path(&workspace.root));
                                return Ok(());
                            }
                        };
                        let _ = resp_tx.send(resp);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                    Err(_) => break, // Channel disconnected
                }
            } else {
                std::thread::sleep(remain);
                break;
            }
        }

        first_cycle = false;
    }
}

/// Spawn a background daemon process for the workspace.
fn start_daemon_background(
    path: Option<PathBuf>,
    poll_ms: u64,
    verbose: bool,
) -> Result<(), String> {
    let workspace = resolve_daemon_workspace_target(path.clone())?;
    let pid_path = daemon_pid_path(&workspace.root);

    // Check if a daemon is already running
    if pid_path.is_file() {
        if let Ok(pid_str) = std::fs::read_to_string(&pid_path) {
            if let Ok(pid) = pid_str.trim().parse::<u32>() {
                // Check if process is still alive by reading status
                if let Ok(Some(status)) = read_daemon_status(&workspace.root) {
                    let now = now_unix_ms();
                    if daemon_liveness(&status, now) == DaemonLiveness::Running {
                        println!("Agam Daemon");
                        println!("workspace: {}", workspace.root.display());
                        println!("status: already running (pid {pid})");
                        return Ok(());
                    }
                }
            }
        }
        // Stale PID file — remove it
        let _ = std::fs::remove_file(&pid_path);
    }

    // Find our own executable
    let exe =
        std::env::current_exe().map_err(|e| format!("failed to find agamc executable: {e}"))?;

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("daemon");
    if let Some(ref p) = path {
        cmd.arg(p);
    }
    cmd.arg("--background-child");
    cmd.arg("--poll-ms");
    cmd.arg(poll_ms.to_string());

    // Platform-specific detach
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    // Redirect stdio to prevent blocking
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    let child = cmd
        .spawn()
        .map_err(|e| format!("failed to spawn background daemon: {e}"))?;
    let child_pid = child.id();

    // Ensure daemon directory exists and write PID
    if let Some(parent) = pid_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&pid_path, format!("{child_pid}"));

    println!("Agam Daemon");
    println!("workspace: {}", workspace.root.display());
    println!("started background daemon (pid {child_pid})");
    if verbose {
        println!("pid-file: {}", pid_path.display());
    }

    Ok(())
}

/// Signal a running background daemon to shut down gracefully.
fn stop_daemon_background(path: Option<PathBuf>, verbose: bool) -> Result<(), String> {
    let workspace = resolve_daemon_workspace_target(path)?;
    let pid_path = daemon_pid_path(&workspace.root);
    let shutdown_path = daemon_shutdown_path(&workspace.root);

    let pid_str = std::fs::read_to_string(&pid_path)
        .map_err(|_| "no running background daemon found (no PID file)".to_string())?;
    let pid: u32 = pid_str
        .trim()
        .parse()
        .map_err(|_| format!("invalid PID in daemon lock file: {}", pid_str.trim()))?;

    // First try IPC stop for immediate clean shutdown
    let mut ipc_success = false;
    if let Ok(DaemonIpcResponse::Error(_)) =
        send_daemon_ipc_request(&workspace.root, DaemonIpcRequest::Stop)
    {
        ipc_success = true;
    }

    // Fallback to sentinel file if IPC failed
    if !ipc_success {
        if let Some(parent) = shutdown_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&shutdown_path, format!("{}", now_unix_ms()))
            .map_err(|e| format!("failed to create shutdown sentinel: {e}"))?;
    }

    println!("Agam Daemon");
    println!("workspace: {}", workspace.root.display());
    println!("signalled shutdown for daemon pid {pid}");
    if verbose {
        if ipc_success {
            println!("transport: IPC synchronous shutdown");
        } else {
            println!("transport: fallback sentinel {}", shutdown_path.display());
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
        source,
    })
}

fn emit_resolve_error(emitter: &mut DiagnosticEmitter, error: &agam_sema::resolver::ResolveError) {
    let diagnostic = if error.span.is_dummy() {
        Diagnostic::error("E3001", error.message.clone())
    } else {
        Diagnostic::error("E3001", error.message.clone())
            .with_label(Label::primary(error.span, error.message.clone()))
    };
    emitter.emit(diagnostic);
}

fn emit_type_error(emitter: &mut DiagnosticEmitter, error: &agam_sema::checker::TypeError) {
    let diagnostic = if error.span.is_dummy() {
        Diagnostic::error("E3002", error.message.clone())
    } else {
        Diagnostic::error("E3002", error.message.clone())
            .with_label(Label::primary(error.span, error.message.clone()))
    };
    emitter.emit(diagnostic);
}

fn semantic_check_parsed_source(
    path: &PathBuf,
    parsed: &ParsedSource,
    verbose: bool,
) -> Result<(), String> {
    let source_file = SourceFile::new(
        SourceId(0),
        path.to_string_lossy().to_string(),
        parsed.source.clone(),
    );
    let mut emitter = DiagnosticEmitter::new();
    emitter.add_source(source_file);

    let mut resolver = agam_sema::resolver::Resolver::new();
    resolver.resolve_module(&parsed.module);
    let resolve_error_count = resolver.errors.len();
    if verbose {
        eprintln!("[agamc] Name resolution: {} error(s)", resolve_error_count);
    }
    for error in &resolver.errors {
        emit_resolve_error(&mut emitter, error);
    }
    if resolve_error_count > 0 {
        return Err(format!("{resolve_error_count} semantic error(s)"));
    }

    let mut checker = agam_sema::checker::TypeChecker::from_resolver(resolver);
    checker.check_module(&parsed.module);
    let type_error_count = checker.errors.len();
    if verbose {
        eprintln!("[agamc] Type checking: {} error(s)", type_error_count);
    }
    for error in &checker.errors {
        emit_type_error(&mut emitter, error);
    }
    if type_error_count > 0 {
        return Err(format!("{type_error_count} type error(s)"));
    }

    Ok(())
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

fn execute_parallel_check_requests(
    requests: &[CheckRequest],
    verbose: bool,
) -> Vec<CheckRequestResult> {
    let parallelism = check_request_parallelism(requests.len());
    execute_check_requests_with_runner(requests, parallelism, |request| {
        run_nested_check_request(request, verbose)
    })
}

fn execute_parallel_test_requests(requests: &[TestRequest]) -> Vec<TestRequestResult> {
    let parallelism = request_parallelism(requests.len());
    execute_test_requests_with_runner(requests, parallelism, |request| {
        match agam_test::run_file(&request.file) {
            Ok(summary) => TestRequestResult {
                request: request.clone(),
                summary: Some(agam_test::FileTestSummary {
                    path: request.file.clone(),
                    summary,
                }),
                error: None,
            },
            Err(error) => TestRequestResult {
                request: request.clone(),
                summary: None,
                error: Some(error),
            },
        }
    })
}

fn run_check_request_locally(path: &PathBuf, verbose: bool) -> Result<(), String> {
    compile_file(path, verbose)?;
    if verbose {
        eprintln!("[agamc] {} — OK", path.display());
    }
    Ok(())
}

/// Read, parse, and run semantic checks without lowering or code generation.
fn compile_file(path: &PathBuf, verbose: bool) -> Result<(), String> {
    if load_daemon_prewarmed_warm_state(path, verbose).is_some() {
        return Ok(());
    }
    // Fallback: try the multi-file warm index (Checked or Lowered level skips all work)
    if load_daemon_warm_state_for_file(path, verbose).is_some() {
        return Ok(());
    }
    let parsed = parse_source_file(path, verbose)?;
    semantic_check_parsed_source(path, &parsed, verbose)?;
    Ok(())
}

/// Compile a file for `agamc dev`; only the runnable entry file needs warm lowered state.
fn compile_dev_source_file(
    path: &PathBuf,
    keep_warm_state: bool,
    verbose: bool,
) -> Result<Option<WarmState>, String> {
    if keep_warm_state {
        if let Some(warm_state) = load_daemon_prewarmed_warm_state(path, verbose) {
            return Ok(Some(warm_state));
        }
        // Fallback: try the warm index (only if it includes MIR)
        if let Some(warm_state) = load_daemon_warm_state_for_file(path, verbose) {
            if warm_state_supports_runnable_reuse(&warm_state) {
                return Ok(Some(warm_state));
            }
            if verbose && warm_state.mir.is_some() {
                eprintln!(
                    "[agamc] warm state for `{}` is incomplete for runnable reuse; rebuilding locally",
                    path.display()
                );
            }
        }
        Ok(Some(compile_file_with_warm_state(path, verbose)?))
    } else {
        compile_file(path, verbose)?;
        Ok(None)
    }
}

fn lower_module_to_hir_and_optimized_mir(
    module: &agam_ast::Module,
    verbose: bool,
) -> (agam_hir::nodes::HirModule, agam_mir::ir::MirModule) {
    let mut hir_lowering = agam_hir::lower::HirLowering::new();
    let hir = hir_lowering.lower_module(module);

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

    (hir, mir)
}

fn build_warm_state(
    path: &PathBuf,
    parsed: ParsedSource,
    verbose: bool,
) -> Result<WarmState, String> {
    semantic_check_parsed_source(path, &parsed, verbose)?;
    let ParsedSource {
        module,
        source_features,
        ..
    } = parsed;
    let (hir, mir) = lower_module_to_hir_and_optimized_mir(&module, verbose);
    Ok(WarmState {
        source_features: Some(source_features),
        module: Some(module),
        hir: Some(hir),
        mir: Some(mir),
    })
}

fn summarize_warm_cache(
    cache: &BTreeMap<PathBuf, BTreeMap<String, WarmState>>,
) -> WarmCacheSummary {
    let mut summary = WarmCacheSummary::default();
    for versions in cache.values() {
        if !versions.is_empty() {
            summary.file_count += 1;
        }
        summary.version_count += versions.len();
        for state in versions.values() {
            if let Some(module) = state.module.as_ref() {
                summary.ast_decl_count += module.declarations.len();
            }
            if let Some(hir) = state.hir.as_ref() {
                summary.hir_function_count += hir.functions.len();
            }
            if let Some(mir) = state.mir.as_ref() {
                summary.mir_function_count += mir.functions.len();
            }
        }
    }
    summary
}

fn warm_workspace_session(
    session: &mut DaemonSession,
    snapshot: &agam_pkg::WorkspaceSnapshot,
    verbose: bool,
) -> Result<WarmSummary, String> {
    let mut summary = WarmSummary::default();

    // Partition files into cache hits (reused) and cache misses (need warming)
    let mut files_to_warm = Vec::new();
    for file in snapshot.source_files.iter().chain(&snapshot.test_files) {
        let versions = session.cache.entry(file.path.clone()).or_default();
        if versions.contains_key(&file.content_hash) {
            summary.reused_files += 1;
        } else {
            files_to_warm.push(file.clone());
        }
    }

    // Warm cache-miss files in parallel
    let parallelism = request_parallelism(files_to_warm.len());
    let warmed_results: Vec<(agam_pkg::WorkspaceFileSnapshot, Result<WarmState, String>)> =
        if files_to_warm.len() <= 1 || parallelism <= 1 {
            // Sequential fast path for single file or no parallelism
            files_to_warm
                .into_iter()
                .map(|file| {
                    let result = parse_source_file(&file.path, verbose)
                        .and_then(|parsed| build_warm_state(&file.path, parsed, verbose));
                    (file, result)
                })
                .collect()
        } else {
            // Parallel warm using scoped threads with work-stealing
            let next_index = AtomicUsize::new(0);
            let results: Mutex<
                Vec<Option<(agam_pkg::WorkspaceFileSnapshot, Result<WarmState, String>)>>,
            > = Mutex::new(
                std::iter::repeat_with(|| None)
                    .take(files_to_warm.len())
                    .collect(),
            );
            let worker_count = parallelism.max(1).min(files_to_warm.len());

            std::thread::scope(|scope| {
                let files_ref = &files_to_warm;
                let next_ref = &next_index;
                let results_ref = &results;
                for _ in 0..worker_count {
                    scope.spawn(move || {
                        loop {
                            let index = next_ref.fetch_add(1, Ordering::Relaxed);
                            if index >= files_ref.len() {
                                break;
                            }
                            let file = &files_ref[index];
                            let result = parse_source_file(&file.path, verbose)
                                .and_then(|parsed| build_warm_state(&file.path, parsed, verbose));
                            results_ref.lock().expect("warm results mutex poisoned")[index] =
                                Some((file.clone(), result));
                        }
                    });
                }
            });

            results
                .into_inner()
                .expect("warm results mutex poisoned")
                .into_iter()
                .map(|r| r.expect("warm result missing"))
                .collect()
        };

    // Merge results into session cache
    for (file, result) in warmed_results {
        let warm_state = result?;
        let versions = session.cache.entry(file.path.clone()).or_default();
        versions.clear();
        versions.insert(file.content_hash.clone(), warm_state);
        summary.warmed_files += 1;
    }

    let cache_summary = summarize_warm_cache(&session.cache);
    summary.warmed_version_count = cache_summary.version_count;
    summary.ast_decl_count = cache_summary.ast_decl_count;
    summary.hir_function_count = cache_summary.hir_function_count;
    summary.mir_function_count = cache_summary.mir_function_count;

    Ok(summary)
}

fn compile_file_with_warm_state(path: &PathBuf, verbose: bool) -> Result<WarmState, String> {
    let parsed = parse_source_file(path, verbose)?;
    build_warm_state(path, parsed, verbose)
}

fn lower_parsed_to_optimized_mir(parsed: &ParsedSource, verbose: bool) -> agam_mir::ir::MirModule {
    let (_, mir) = lower_module_to_hir_and_optimized_mir(&parsed.module, verbose);
    mir
}

fn lower_to_optimized_mir(
    path: &PathBuf,
    verbose: bool,
) -> Result<(agam_mir::ir::MirModule, SourceFeatureFlags), String> {
    let parsed = parse_source_file(path, verbose)?;
    semantic_check_parsed_source(path, &parsed, verbose)?;
    let mir = lower_parsed_to_optimized_mir(&parsed, verbose);

    Ok((mir, parsed.source_features))
}

fn build_portable_package_file(
    path: &PathBuf,
    verbose: bool,
) -> Result<agam_pkg::PortablePackage, String> {
    if let Some(prewarmed) = load_daemon_prewarmed_entry(path, verbose) {
        return Ok(prewarmed.package);
    }

    let parsed = parse_source_file(path, verbose)?;
    semantic_check_parsed_source(path, &parsed, verbose)?;
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
) -> Result<agam_runtime::cache::CacheHit, String> {
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
        cache.restore_to_path(&hit, output)?;
        return Ok(hit);
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
    cache.restore_to_path(&hit, output)?;
    Ok(hit)
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplSession {
    request: HeadlessExecutionRequest,
}

impl Default for ReplSession {
    fn default() -> Self {
        Self {
            request: HeadlessExecutionRequest {
                filename: "repl.agam".into(),
                ..HeadlessExecutionRequest::default()
            },
        }
    }
}

impl ReplSession {
    fn append_line(&mut self, line: &str) {
        self.request.source.push_str(line);
        self.request.source.push('\n');
    }

    fn replace_source(&mut self, filename: String, source: String) {
        self.request.filename = filename;
        self.request.source = source;
    }

    fn clear(&mut self) {
        self.request.source.clear();
    }
}

#[derive(Debug)]
struct ReplExecutionCache {
    root: PathBuf,
    manifest_path: PathBuf,
    source_path: PathBuf,
    filename: String,
    source_hash: Option<String>,
    daemon_session: DaemonSession,
}

impl ReplExecutionCache {
    fn new(filename: &str) -> Result<Self, String> {
        let root = create_headless_temp_dir()?;
        let filename = sanitize_headless_filename(filename);
        let manifest_path = agam_pkg::default_manifest_path(&root);
        write_repl_workspace_manifest(&manifest_path, &filename)?;
        let source_path = repl_workspace_entry_path(&root, &filename);
        Ok(Self {
            root,
            manifest_path,
            source_path,
            filename,
            source_hash: None,
            daemon_session: DaemonSession::default(),
        })
    }

    fn source_path(&self) -> &PathBuf {
        &self.source_path
    }

    fn materialize_request(&mut self, request: &HeadlessExecutionRequest) -> Result<(), String> {
        let filename = sanitize_headless_filename(&request.filename);
        if filename != self.filename {
            let previous_source_path = self.source_path.clone();
            self.filename = filename.clone();
            self.source_path = repl_workspace_entry_path(&self.root, &filename);
            write_repl_workspace_manifest(&self.manifest_path, &filename)?;
            if previous_source_path.is_file() && previous_source_path != self.source_path {
                std::fs::remove_file(&previous_source_path).map_err(|error| {
                    format!(
                        "failed to remove stale REPL source `{}`: {error}",
                        previous_source_path.display()
                    )
                })?;
            }
            self.source_hash = None;
        }

        let source_hash = agam_runtime::cache::hash_bytes(request.source.as_bytes());
        if self.source_hash.as_deref() == Some(source_hash.as_str()) && self.source_path.is_file() {
            return Ok(());
        }

        if let Some(parent) = self.source_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "failed to create REPL temp dir `{}`: {error}",
                    parent.display()
                )
            })?;
        }
        std::fs::write(&self.source_path, &request.source).map_err(|error| {
            format!(
                "failed to write REPL source `{}`: {error}",
                self.source_path.display()
            )
        })?;
        self.source_hash = Some(source_hash);
        Ok(())
    }

    fn ensure_materialized_warm_state(&mut self, verbose: bool) -> Result<&WarmState, String> {
        let snapshot = agam_pkg::snapshot_workspace(Some(self.root.clone()))?;
        let (_, diff_summary) =
            refresh_daemon_session(&mut self.daemon_session, snapshot.clone(), verbose)?;
        if verbose && !daemon_diff_has_changes(&diff_summary) {
            eprintln!(
                "[agamc] Reused REPL daemon warm state for `{}`",
                self.source_path.display()
            );
        }
        let file = daemon_entry_snapshot(&snapshot)
            .filter(|file| file.path == self.source_path)
            .ok_or_else(|| {
                format!(
                    "internal error: REPL snapshot entry missing for `{}`",
                    self.source_path.display()
                )
            })?;
        warm_state_for_snapshot_file(&self.daemon_session, file).ok_or_else(|| {
            format!(
                "internal error: REPL warm state missing for `{}`",
                self.source_path.display()
            )
        })
    }
}

impl Drop for ReplExecutionCache {
    fn drop(&mut self) {
        cleanup_headless_temp_dir(&self.root, false);
    }
}

fn repl_workspace_entry_relative_path(filename: &str) -> String {
    format!("src/{filename}")
}

fn repl_workspace_entry_path(root: &Path, filename: &str) -> PathBuf {
    root.join("src").join(filename)
}

fn write_repl_workspace_manifest(manifest_path: &Path, filename: &str) -> Result<(), String> {
    let mut manifest = agam_pkg::scaffold_workspace_manifest("repl-session");
    manifest.project.entry = Some(repl_workspace_entry_relative_path(filename));
    agam_pkg::write_workspace_manifest_to_path(manifest_path, &manifest)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReplCommandKind {
    Help,
    Quit,
    Reset,
    Show,
    Run,
    Load(PathBuf),
    Backend(HeadlessExecutionBackend),
    Opt(u8),
    Fast(bool),
}

fn run_repl_shell(verbose: bool) -> Result<i32, String> {
    let mut session = ReplSession::default();
    let mut execution_cache = ReplExecutionCache::new(&session.request.filename)?;
    let stdin = std::io::stdin();
    let mut handle = stdin.lock();

    println!("Agam REPL v0.1.0");
    println!("Type :help for commands, :quit to exit.");

    loop {
        print!("agam> ");
        std::io::stdout()
            .flush()
            .map_err(|error| format!("failed to flush REPL prompt: {error}"))?;

        let mut line = String::new();
        let read = handle
            .read_line(&mut line)
            .map_err(|error| format!("failed to read REPL input: {error}"))?;
        if read == 0 {
            println!();
            break;
        }

        let line = line.trim_end_matches(['\r', '\n']);
        match parse_repl_command(line)? {
            Some(ReplCommandKind::Help) => print_repl_help(),
            Some(ReplCommandKind::Quit) => break,
            Some(ReplCommandKind::Reset) => {
                session.clear();
                println!("session cleared");
            }
            Some(ReplCommandKind::Show) => {
                if session.request.source.is_empty() {
                    println!("(empty)");
                } else {
                    print!("{}", session.request.source);
                    if !session.request.source.ends_with('\n') {
                        println!();
                    }
                }
            }
            Some(ReplCommandKind::Run) => {
                if session.request.source.trim().is_empty() {
                    eprintln!("buffer is empty; add Agam source before `:run`");
                    continue;
                }
                match execute_repl_request(&session.request, &mut execution_cache, verbose) {
                    Ok(code) => {
                        if code != 0 {
                            eprintln!("[agamc] exit code {code}");
                        }
                    }
                    Err(error) => eprintln!("[agamc] {error}"),
                }
            }
            Some(ReplCommandKind::Load(path)) => {
                let source = std::fs::read_to_string(&path).map_err(|error| {
                    format!("failed to read `{}` for `:load`: {error}", path.display())
                })?;
                let filename = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| sanitize_headless_filename(name))
                    .unwrap_or_else(|| "repl.agam".into());
                session.replace_source(filename, source);
                println!("loaded {}", path.display());
            }
            Some(ReplCommandKind::Backend(backend)) => {
                session.request.backend = backend;
                println!(
                    "backend = {}",
                    render_headless_backend_label(session.request.backend)
                );
            }
            Some(ReplCommandKind::Opt(opt_level)) => {
                session.request.opt_level = opt_level;
                println!("opt_level = {opt_level}");
            }
            Some(ReplCommandKind::Fast(fast)) => {
                session.request.fast = fast;
                println!("fast = {}", if fast { "on" } else { "off" });
            }
            None => session.append_line(line),
        }
    }

    Ok(0)
}

fn print_repl_help() {
    println!("Commands:");
    println!("  :help                 show this help");
    println!("  :run                  execute the buffered Agam source");
    println!("  :show                 print the current source buffer");
    println!("  :reset                clear the current source buffer");
    println!("  :load <path>          replace the buffer with a file");
    println!("  :backend <name>       set backend to auto, c, llvm, or jit");
    println!("  :opt <0-3>            set optimization level used for non-JIT runs");
    println!("  :fast <on|off>        toggle fast-mode run requests");
    println!("  :quit                 exit the REPL");
    println!("Notes:");
    println!("  Free-form lines are appended to the current buffer.");
    println!("  `:run` expects the buffer to be a valid Agam source file.");
}

fn parse_repl_command(input: &str) -> Result<Option<ReplCommandKind>, String> {
    let trimmed = input.trim();
    if !trimmed.starts_with(':') {
        return Ok(None);
    }

    let body = trimmed[1..].trim();
    if body.is_empty() {
        return Err("empty repl command".into());
    }

    let command = body
        .split_whitespace()
        .next()
        .ok_or_else(|| "empty repl command".to_string())?;
    let tail = body[command.len()..].trim();

    match command {
        "help" => Ok(Some(ReplCommandKind::Help)),
        "q" | "quit" | "exit" => Ok(Some(ReplCommandKind::Quit)),
        "reset" | "clear" => Ok(Some(ReplCommandKind::Reset)),
        "show" => Ok(Some(ReplCommandKind::Show)),
        "run" => Ok(Some(ReplCommandKind::Run)),
        "load" => {
            if tail.is_empty() {
                return Err("`:load` requires a path".into());
            }
            Ok(Some(ReplCommandKind::Load(PathBuf::from(tail))))
        }
        "backend" => Ok(Some(ReplCommandKind::Backend(
            parse_headless_backend_label(tail)?,
        ))),
        "opt" => {
            if tail.is_empty() {
                return Err("`:opt` requires a value from 0 to 3".into());
            }
            let opt_level = tail
                .parse::<u8>()
                .map_err(|_| format!("invalid optimization level `{tail}`"))?;
            if opt_level > 3 {
                return Err(format!("optimization level `{opt_level}` must be 0..=3"));
            }
            Ok(Some(ReplCommandKind::Opt(opt_level)))
        }
        "fast" => Ok(Some(ReplCommandKind::Fast(parse_repl_fast_flag(tail)?))),
        _ => Err(format!("unknown repl command `:{command}`")),
    }
}

fn parse_repl_fast_flag(value: &str) -> Result<bool, String> {
    match value {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        _ => Err("`:fast` expects `on` or `off`".into()),
    }
}

fn run_exec_tool(
    json: bool,
    pretty: bool,
    file: Option<PathBuf>,
    source: Option<String>,
    filename: Option<String>,
    backend: Backend,
    opt_level: u8,
    fast: bool,
    args: Vec<String>,
    verbose: bool,
    sandbox_level: String,
    deny_network: bool,
    deny_process_spawn: bool,
) -> Result<i32, String> {
    if json {
        return run_headless_json_request(pretty, verbose);
    }

    let request = build_exec_request(
        file,
        source,
        filename,
        backend,
        opt_level,
        fast,
        args,
        sandbox_level,
        deny_network,
        deny_process_spawn,
    )?;

    // Activate the sandbox guard around execution based on the policy sandbox_level.
    let _sandbox_guard = if request.policy.sandbox_level != "none" {
        let sandbox_policy = agam_runtime::sandbox::SandboxPolicy {
            deny_network: request.policy.deny_network,
            deny_process_spawn: request.policy.deny_process_spawn,
            ..agam_runtime::sandbox::SandboxPolicy::default()
        };
        match agam_runtime::sandbox::SandboxGuard::acquire(&sandbox_policy) {
            Ok(guard) => Some(guard),
            Err(error) => {
                if verbose {
                    eprintln!("[agamc] sandbox activation failed: {error}");
                }
                None
            }
        }
    } else {
        None
    };

    let response = execute_headless_request(&request, verbose);
    let exit_code = headless_response_exit_code(&response);
    write_headless_response(&response, pretty)?;
    Ok(exit_code)
}

fn build_exec_request(
    file: Option<PathBuf>,
    source: Option<String>,
    filename: Option<String>,
    backend: Backend,
    opt_level: u8,
    fast: bool,
    args: Vec<String>,
    sandbox_level: String,
    deny_network: bool,
    deny_process_spawn: bool,
) -> Result<HeadlessExecutionRequest, String> {
    let (source, request_filename) = if let Some(source) = source {
        (
            source,
            filename.unwrap_or_else(agam_notebook::default_headless_filename),
        )
    } else if let Some(file) = file {
        let source = std::fs::read_to_string(&file)
            .map_err(|error| format!("failed to read Agam source `{}`: {error}", file.display()))?;
        let request_filename = filename.unwrap_or_else(|| {
            file.file_name()
                .and_then(|name| name.to_str())
                .map(str::to_string)
                .unwrap_or_else(agam_notebook::default_headless_filename)
        });
        (source, request_filename)
    } else {
        let mut source = String::new();
        std::io::stdin()
            .read_to_string(&mut source)
            .map_err(|error| format!("failed to read Agam source from stdin: {error}"))?;
        (
            source,
            filename.unwrap_or_else(agam_notebook::default_headless_filename),
        )
    };
    let mut policy = HeadlessExecutionPolicy::default();
    if !matches!(backend, Backend::Jit) {
        policy.allow_native_backends = true;
    }
    policy.sandbox_level = sandbox_level;
    policy.deny_network = deny_network;
    policy.deny_process_spawn = deny_process_spawn;

    Ok(HeadlessExecutionRequest {
        source,
        filename: request_filename,
        args,
        backend: backend_to_headless_backend(backend),
        opt_level,
        fast,
        policy,
    })
}

fn run_headless_json_request(pretty: bool, verbose: bool) -> Result<i32, String> {
    let mut payload = String::new();
    std::io::stdin()
        .read_to_string(&mut payload)
        .map_err(|error| format!("failed to read JSON request from stdin: {error}"))?;

    let request = match serde_json::from_str::<HeadlessExecutionRequest>(&payload) {
        Ok(request) => request,
        Err(error) => {
            let response = HeadlessExecutionResponse::execution_error(
                &HeadlessExecutionRequest::default(),
                format!("failed to parse JSON request: {error}"),
                String::new(),
            );
            write_headless_response(&response, pretty)?;
            return Ok(1);
        }
    };

    let response = execute_headless_request(&request, verbose);
    let exit_code = headless_response_exit_code(&response);
    write_headless_response(&response, pretty)?;
    Ok(exit_code)
}

fn write_headless_response(
    response: &HeadlessExecutionResponse,
    pretty: bool,
) -> Result<(), String> {
    if pretty {
        serde_json::to_writer_pretty(std::io::stdout().lock(), response)
            .map_err(|error| format!("failed to serialize JSON response: {error}"))?;
    } else {
        serde_json::to_writer(std::io::stdout().lock(), response)
            .map_err(|error| format!("failed to serialize JSON response: {error}"))?;
    }
    println!();
    Ok(())
}

fn headless_response_exit_code(response: &HeadlessExecutionResponse) -> i32 {
    if let Some(code) = response.exit_code {
        code
    } else if response.success {
        0
    } else {
        1
    }
}

fn backend_to_headless_backend(backend: Backend) -> HeadlessExecutionBackend {
    match backend {
        Backend::Auto => HeadlessExecutionBackend::Auto,
        Backend::C => HeadlessExecutionBackend::C,
        Backend::Llvm => HeadlessExecutionBackend::Llvm,
        Backend::Jit => HeadlessExecutionBackend::Jit,
    }
}

fn render_headless_parse_errors(errors: &[agam_parser::ParseError]) -> String {
    let mut stderr = String::new();
    for error in errors {
        stderr.push_str("\x1b[1;31merror\x1b[0m: ");
        stderr.push_str(&error.message);
        stderr.push('\n');
    }
    stderr
}

fn build_headless_warm_state(
    request: &HeadlessExecutionRequest,
    verbose: bool,
) -> Result<(WarmState, String), String> {
    let source = request.source.clone();
    let source_file = SourceFile::new(SourceId(0), request.filename.clone(), source.clone());
    let mut parse_emitter = DiagnosticEmitter::buffered();
    parse_emitter.add_source(source_file);

    if verbose {
        eprintln!(
            "[agamc] Read headless source {} ({} bytes)",
            request.filename,
            source.len()
        );
    }

    let tokens = agam_lexer::tokenize(&source, SourceId(0));
    if verbose {
        eprintln!("[agamc] Lexed {} tokens", tokens.len());
    }

    let mut source_features = source_feature_flags_from_tokens(&tokens);
    let module = match agam_parser::parse(tokens, SourceId(0)) {
        Ok(module) => module,
        Err(errors) => {
            let mut stderr = render_headless_parse_errors(&errors);
            stderr.push_str(&parse_emitter.take_rendered_output());
            return Err(stderr);
        }
    };

    if verbose {
        eprintln!(
            "[agamc] Parsed {} top-level declarations",
            module.declarations.len()
        );
    }

    merge_function_call_cache_annotations(&module, &mut source_features.call_cache);
    collect_experimental_function_features(&module, &mut source_features.experimental_usages);
    emit_experimental_feature_warnings(&mut parse_emitter, &source_features.experimental_usages);

    let mut stderr = parse_emitter.take_rendered_output();
    let mut sema_emitter = DiagnosticEmitter::buffered();
    sema_emitter.add_source(SourceFile::new(
        SourceId(0),
        request.filename.clone(),
        source.clone(),
    ));

    let mut resolver = agam_sema::resolver::Resolver::new();
    resolver.resolve_module(&module);
    let resolve_error_count = resolver.errors.len();
    if verbose {
        eprintln!("[agamc] Name resolution: {} error(s)", resolve_error_count);
    }
    for error in &resolver.errors {
        emit_resolve_error(&mut sema_emitter, error);
    }
    if resolve_error_count > 0 {
        stderr.push_str(&sema_emitter.take_rendered_output());
        return Err(stderr);
    }

    let mut checker = agam_sema::checker::TypeChecker::from_resolver(resolver);
    checker.check_module(&module);
    let type_error_count = checker.errors.len();
    if verbose {
        eprintln!("[agamc] Type checking: {} error(s)", type_error_count);
    }
    for error in &checker.errors {
        emit_type_error(&mut sema_emitter, error);
    }
    if type_error_count > 0 {
        stderr.push_str(&sema_emitter.take_rendered_output());
        return Err(stderr);
    }

    stderr.push_str(&sema_emitter.take_rendered_output());
    let (hir, mir) = lower_module_to_hir_and_optimized_mir(&module, verbose);
    Ok((
        WarmState {
            source_features: Some(source_features),
            module: Some(module),
            hir: Some(hir),
            mir: Some(mir),
        },
        stderr,
    ))
}

fn run_with_jit_prelowered_captured(
    args: &[String],
    mir: &agam_mir::ir::MirModule,
    source_features: &SourceFeatureFlags,
    verbose: bool,
    features: FeatureFlags,
) -> Result<(i32, String), String> {
    let call_cache = effective_call_cache_selection(features, source_features);
    let jit_options = agam_jit::JitOptions {
        call_cache: call_cache.resolved_enable_all(),
        call_cache_only: call_cache.included_functions(),
        call_cache_exclude: call_cache.excluded_functions(),
        call_cache_optimize: call_cache.optimize_all,
        call_cache_optimize_only: call_cache.optimized_functions(),
        ..Default::default()
    };

    if verbose {
        let analysis = agam_jit::analyze_call_cache(mir, &jit_options);
        log_call_cache_analysis("JIT", &call_cache, &analysis);
        eprintln!("[agamc] Executing via Cranelift JIT");
    }

    let (exit_code, stdout) = agam_jit::run_main_with_options_captured(mir, args, jit_options)?;

    if call_cache.is_enabled() {
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
            }
        }
    }

    Ok((exit_code, stdout))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CapturedExecution {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

fn capture_command_output(
    command: &mut std::process::Command,
    program: &Path,
) -> Result<CapturedExecution, String> {
    let output = command
        .output()
        .map_err(|error| format!("failed to run {}: {}", program.display(), error))?;
    Ok(CapturedExecution {
        exit_code: output.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

fn run_with_c_prelowered_captured(
    path: &PathBuf,
    args: &[String],
    opt_level: u8,
    tuning: &ReleaseTuning,
    mir: &agam_mir::ir::MirModule,
    source_features: &SourceFeatureFlags,
    verbose: bool,
    features: FeatureFlags,
) -> Result<CapturedExecution, String> {
    if !command_exists(default_c_compiler()) {
        return Err(format!(
            "C run requires `{}` on PATH; headless execution cannot shell through the legacy CLI bridge anymore",
            default_c_compiler()
        ));
    }

    let exe_path = default_native_binary_output_path(path, tuning.target.as_deref());
    let call_cache = effective_call_cache_selection(features, source_features);
    let outcome = build_prelowered_file(
        path,
        &exe_path,
        opt_level,
        Backend::C,
        tuning,
        mir,
        &call_cache,
        &[],
        false,
        verbose,
    )?;
    if !outcome.native_binary {
        return Err(format!(
            "backend {:?} emitted {} but no native executable was produced",
            Backend::C,
            outcome.generated_path.display()
        ));
    }

    let mut command = std::process::Command::new(&exe_path);
    command.args(args);
    capture_command_output(&mut command, &exe_path)
}

fn run_with_llvm_prelowered_captured(
    path: &PathBuf,
    args: &[String],
    opt_level: u8,
    tuning: &ReleaseTuning,
    mir: &agam_mir::ir::MirModule,
    source_features: &SourceFeatureFlags,
    verbose: bool,
    features: FeatureFlags,
) -> Result<CapturedExecution, String> {
    let allow_dev_wsl_llvm = allow_dev_wsl_llvm();
    let toolchain = resolve_llvm_run_toolchain();
    if matches!(toolchain, None) {
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
            "LLVM run requires a native LLVM toolchain or bundled clang; use `agamc doctor` to inspect readiness for `{}`",
            path.display()
        ));
    }

    let call_cache = effective_call_cache_selection(features, source_features);
    let persisted_profile = if call_cache.is_enabled() {
        load_persisted_llvm_profile(path, mir, &call_cache, verbose)
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
        if matches!(toolchain, Some(LlvmToolchain::Wsl)) {
            eprintln!("[agamc] Executing LLVM backend through dev-only WSL fallback");
        }
    }

    let exe_path = default_native_binary_output_path(path, tuning.target.as_deref());
    let outcome = build_prelowered_file(
        path,
        &exe_path,
        opt_level,
        Backend::Llvm,
        tuning,
        mir,
        &effective_call_cache,
        &specialization_plans,
        allow_dev_wsl_llvm,
        verbose,
    )?;
    if !outcome.native_binary {
        return Err(format!(
            "backend {:?} emitted {} but no native executable was produced",
            Backend::Llvm,
            outcome.generated_path.display()
        ));
    }

    let profile_capture = llvm_profile_capture_path(&exe_path);
    let _ = std::fs::remove_file(&profile_capture);
    let mut command = match toolchain.expect("toolchain checked above") {
        LlvmToolchain::Wsl => {
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
        LlvmToolchain::Native => {
            let mut command = std::process::Command::new(&exe_path);
            if effective_call_cache.is_enabled() {
                command.env("AGAM_LLVM_CALL_CACHE_PROFILE_OUT", &profile_capture);
            }
            command
        }
    };
    command.args(args);
    let captured = capture_command_output(&mut command, &exe_path)?;

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
                    store_persisted_llvm_profile(path, mir, &call_cache, &merged_profile, verbose);
                }
                Err(error) => {
                    if verbose {
                        eprintln!(
                            "[agamc] Failed to parse LLVM call-cache profile `{}`: {}",
                            profile_capture.display(),
                            error
                        );
                    }
                }
            },
            Err(error) => {
                if verbose && error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!(
                        "[agamc] Failed to read LLVM call-cache profile `{}`: {}",
                        profile_capture.display(),
                        error
                    );
                }
            }
        }
        let _ = std::fs::remove_file(&profile_capture);
    }

    Ok(captured)
}

fn execute_headless_request_in_process(
    request: &HeadlessExecutionRequest,
    backend: Backend,
    verbose: bool,
) -> HeadlessExecutionResponse {
    let temp_root = match create_headless_temp_dir() {
        Ok(path) => path,
        Err(error) => {
            return HeadlessExecutionResponse::execution_error(request, error, String::new());
        }
    };
    let source_path = temp_root.join(&request.filename);

    let response = if let Some(parent) = source_path.parent() {
        match std::fs::create_dir_all(parent) {
            Ok(()) => None,
            Err(error) => Some(HeadlessExecutionResponse::execution_error(
                request,
                format!(
                    "failed to create headless source directory `{}`: {error}",
                    parent.display()
                ),
                String::new(),
            )),
        }
    } else {
        None
    };

    let response = response.unwrap_or_else(|| {
        if let Err(error) = std::fs::write(&source_path, &request.source) {
            return HeadlessExecutionResponse::execution_error(
                request,
                format!(
                    "failed to write headless source `{}`: {error}",
                    source_path.display()
                ),
                String::new(),
            );
        }

        let (warm_state, mut stderr) = match build_headless_warm_state(request, verbose) {
            Ok(result) => result,
            Err(stderr) => {
                return HeadlessExecutionResponse::execution_error(
                    request,
                    "failed to compile headless Agam request",
                    stderr,
                );
            }
        };

        let Some(mir) = warm_state.mir.as_ref() else {
            return HeadlessExecutionResponse::execution_error(
                request,
                "internal error: headless warm state is missing MIR",
                stderr,
            );
        };
        let Some(source_features) = warm_state.source_features.as_ref() else {
            return HeadlessExecutionResponse::execution_error(
                request,
                "internal error: headless warm state is missing source features",
                stderr,
            );
        };

        let tuning = ReleaseTuning {
            target: None,
            native_cpu: request.fast,
            lto: None,
            pgo_generate: None,
            pgo_use: None,
        };
        if let Err(error) = validate_release_tuning(backend, &tuning) {
            return HeadlessExecutionResponse::execution_error(request, error, stderr);
        }

        let captured = match backend {
            Backend::Jit => {
                let mut runtime_args = Vec::with_capacity(request.args.len() + 1);
                runtime_args.push(source_path.to_string_lossy().to_string());
                runtime_args.extend(request.args.iter().cloned());
                match run_with_jit_prelowered_captured(
                    &runtime_args,
                    mir,
                    source_features,
                    verbose,
                    FeatureFlags::default(),
                ) {
                    Ok((exit_code, stdout)) => CapturedExecution {
                        exit_code,
                        stdout,
                        stderr: String::new(),
                    },
                    Err(error) => {
                        return HeadlessExecutionResponse::execution_error(request, error, stderr);
                    }
                }
            }
            Backend::C => match run_with_c_prelowered_captured(
                &source_path,
                &request.args,
                request.opt_level,
                &tuning,
                mir,
                source_features,
                verbose,
                FeatureFlags::default(),
            ) {
                Ok(captured) => captured,
                Err(error) => {
                    return HeadlessExecutionResponse::execution_error(request, error, stderr);
                }
            },
            Backend::Llvm => match run_with_llvm_prelowered_captured(
                &source_path,
                &request.args,
                request.opt_level,
                &tuning,
                mir,
                source_features,
                verbose,
                FeatureFlags::default(),
            ) {
                Ok(captured) => captured,
                Err(error) => {
                    return HeadlessExecutionResponse::execution_error(request, error, stderr);
                }
            },
            Backend::Auto => {
                return HeadlessExecutionResponse::execution_error(
                    request,
                    "internal error: unresolved auto backend",
                    stderr,
                );
            }
        };

        stderr.push_str(&captured.stderr);
        HeadlessExecutionResponse::process_result(
            request,
            captured.exit_code,
            captured.stdout,
            stderr,
        )
    });

    cleanup_headless_temp_dir(&temp_root, verbose);
    response
}

fn should_execute_headless_request_in_process() -> bool {
    std::env::var_os(HEADLESS_EXEC_WORKER_ENV).is_some() || cfg!(test)
}

fn execute_headless_request_in_worker(
    request: &HeadlessExecutionRequest,
    verbose: bool,
) -> HeadlessExecutionResponse {
    let sandbox_root = match create_headless_sandbox_root() {
        Ok(path) => path,
        Err(error) => {
            return HeadlessExecutionResponse::execution_error(request, error, String::new());
        }
    };

    let response = (|| {
        let payload = serde_json::to_vec(request).map_err(|error| {
            HeadlessExecutionResponse::execution_error(
                request,
                format!("failed to serialize headless worker request: {error}"),
                String::new(),
            )
        })?;

        let mut command = build_headless_worker_command(request, verbose, &sandbox_root)
            .map_err(|error| {
                HeadlessExecutionResponse::execution_error(request, error, String::new())
            })?;

        let mut child = command.spawn().map_err(|error| {
            HeadlessExecutionResponse::execution_error(
                request,
                format!("failed to spawn isolated headless worker: {error}"),
                String::new(),
            )
        })?;

        #[cfg(windows)]
        let _job = attach_headless_worker_job(&child, request, verbose);

        let Some(mut stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(HeadlessExecutionResponse::execution_error(
                request,
                "isolated headless worker did not expose stdin",
                String::new(),
            ));
        };
        if let Err(error) = stdin.write_all(&payload) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(HeadlessExecutionResponse::execution_error(
                request,
                format!("failed to write isolated headless worker request: {error}"),
                String::new(),
            ));
        }
        drop(stdin);

        let (output, timed_out) =
            wait_for_headless_worker_output(child, request.policy.max_runtime_ms).map_err(
                |error| HeadlessExecutionResponse::execution_error(request, error, String::new()),
            )?;

        if timed_out {
            return Err(HeadlessExecutionResponse::execution_error(
                request,
                format!(
                    "headless execution exceeded the wall-clock policy limit of {} ms",
                    request.policy.max_runtime_ms
                ),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }

        serde_json::from_slice::<HeadlessExecutionResponse>(&output.stdout).map_err(|error| {
            HeadlessExecutionResponse::execution_error(
                request,
                format!(
                    "isolated headless worker returned invalid JSON: {error} (status: {:?}, stdout: {:?})",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stdout)
                ),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            )
        })
    })()
    .unwrap_or_else(|response| response);

    cleanup_headless_temp_dir(&sandbox_root, verbose);
    response
}

fn execute_headless_request(
    request: &HeadlessExecutionRequest,
    verbose: bool,
) -> HeadlessExecutionResponse {
    let request = match normalize_headless_request(request) {
        Ok(request) => request,
        Err(error) => {
            return HeadlessExecutionResponse::execution_error(request, error, String::new());
        }
    };

    if should_execute_headless_request_in_process() {
        let backend = resolve_backend(headless_backend_to_backend(request.backend), true);
        execute_headless_request_in_process(&request, backend, verbose)
    } else {
        execute_headless_request_in_worker(&request, verbose)
    }
}

fn execute_repl_request(
    request: &HeadlessExecutionRequest,
    execution_cache: &mut ReplExecutionCache,
    verbose: bool,
) -> Result<i32, String> {
    let request = normalize_headless_request(request)?;
    let backend = resolve_backend(headless_backend_to_backend(request.backend), true);
    let tuning = ReleaseTuning {
        target: None,
        native_cpu: request.fast,
        lto: None,
        pgo_generate: None,
        pgo_use: None,
    };
    let features = FeatureFlags::default();
    validate_release_tuning(backend, &tuning)?;
    execution_cache.materialize_request(&request)?;
    let source_path = execution_cache.source_path().clone();
    let warm_state = execution_cache.ensure_materialized_warm_state(verbose)?;
    run_source_file_with_optional_warm_state(
        &source_path,
        &request.args,
        backend,
        request.opt_level,
        &tuning,
        verbose,
        features,
        Some(warm_state),
    )
}

fn build_headless_worker_command(
    request: &HeadlessExecutionRequest,
    verbose: bool,
    sandbox_root: &Path,
) -> Result<std::process::Command, String> {
    let current_exe = std::env::current_exe()
        .map_err(|error| format!("failed to resolve current `agamc` executable: {error}"))?;
    let mut command = std::process::Command::new(current_exe);
    if verbose {
        command.arg("--verbose");
    }
    command.arg("exec").arg("--json");
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    command.current_dir(sandbox_root);
    configure_headless_worker_environment(&mut command, sandbox_root, request);
    configure_headless_worker_platform_before_spawn(&mut command, request)?;
    Ok(command)
}

fn configure_headless_worker_environment(
    command: &mut std::process::Command,
    sandbox_root: &Path,
    request: &HeadlessExecutionRequest,
) {
    if request.policy.inherit_environment {
        command.env(HEADLESS_EXEC_WORKER_ENV, "1");
        command.env(HEADLESS_SANDBOX_ROOT_ENV, sandbox_root);
    } else {
        command.env_clear();
        for (key, value) in std::env::vars_os() {
            if key
                .to_str()
                .is_some_and(should_forward_headless_worker_env_var)
            {
                command.env(&key, &value);
            }
        }
        command.env(HEADLESS_EXEC_WORKER_ENV, "1");
        command.env(HEADLESS_SANDBOX_ROOT_ENV, sandbox_root);
    }
    command.env_remove(NESTED_BUILD_REQUEST_ENV);
    command.env_remove(NESTED_CHECK_REQUEST_ENV);
}

fn should_forward_headless_worker_env_var(key: &str) -> bool {
    key.starts_with("AGAM_LLVM_")
        || key == DEV_WSL_LLVM_ENV
        || matches!(
            key,
            "PATH"
                | "Path"
                | "PATHEXT"
                | "TEMP"
                | "TMP"
                | "TMPDIR"
                | "HOME"
                | "USERPROFILE"
                | "LOCALAPPDATA"
                | "APPDATA"
                | "SystemRoot"
                | "SYSTEMROOT"
                | "SystemDrive"
                | "WINDIR"
                | "ComSpec"
                | "COMSPEC"
                | "ProgramFiles"
                | "ProgramFiles(x86)"
                | "ProgramW6432"
                | "INCLUDE"
                | "LIB"
                | "LIBPATH"
                | "VCINSTALLDIR"
                | "VSINSTALLDIR"
                | "WindowsSdkDir"
                | "WindowsSDKDir"
                | "WindowsSDKVersion"
                | "UniversalCRTSdkDir"
                | "UCRTVersion"
                | "SDKROOT"
                | "ANDROID_NDK_HOME"
                | "ANDROID_NDK_ROOT"
                | "LD_LIBRARY_PATH"
                | "DYLD_LIBRARY_PATH"
                | "DYLD_FALLBACK_LIBRARY_PATH"
        )
}

#[cfg(unix)]
fn configure_headless_worker_platform_before_spawn(
    command: &mut std::process::Command,
    request: &HeadlessExecutionRequest,
) -> Result<(), String> {
    use std::os::unix::process::CommandExt;

    let max_memory_bytes = request.policy.max_memory_bytes;
    unsafe {
        command.pre_exec(move || {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }

            let core_limit = libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            };
            if libc::setrlimit(libc::RLIMIT_CORE, &core_limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }

            let memory_limit = libc::rlimit {
                rlim_cur: max_memory_bytes as libc::rlim_t,
                rlim_max: max_memory_bytes as libc::rlim_t,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &memory_limit) != 0 {
                return Err(std::io::Error::last_os_error());
            }

            Ok(())
        });
    }
    Ok(())
}

#[cfg(not(unix))]
fn configure_headless_worker_platform_before_spawn(
    _command: &mut std::process::Command,
    _request: &HeadlessExecutionRequest,
) -> Result<(), String> {
    Ok(())
}

fn wait_for_headless_worker_output(
    child: std::process::Child,
    timeout_ms: u64,
) -> Result<(std::process::Output, bool), String> {
    let pid = child.id();
    let timeout = Duration::from_millis(timeout_ms.max(1));
    let timed_out = Arc::new(AtomicBool::new(false));
    let timed_out_flag = Arc::clone(&timed_out);
    let (cancel_tx, cancel_rx) = mpsc::channel::<()>();
    let killer = std::thread::spawn(move || {
        if cancel_rx.recv_timeout(timeout).is_err() {
            timed_out_flag.store(true, Ordering::SeqCst);
            let _ = terminate_headless_worker_process(pid);
        }
    });

    let output = child
        .wait_with_output()
        .map_err(|error| format!("failed while waiting for isolated headless worker: {error}"))?;
    let _ = cancel_tx.send(());
    let _ = killer.join();
    Ok((output, timed_out.load(Ordering::SeqCst)))
}

#[cfg(unix)]
fn terminate_headless_worker_process(pid: u32) -> Result<(), String> {
    let result = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
    if result == 0 {
        Ok(())
    } else {
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(code) if code == libc::ESRCH => Ok(()),
            _ => Err(format!(
                "failed to terminate isolated headless worker: {error}"
            )),
        }
    }
}

#[cfg(windows)]
fn terminate_headless_worker_process(pid: u32) -> Result<(), String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(87) {
                return Ok(());
            }
            return Err(format!(
                "failed to open isolated headless worker process: {error}"
            ));
        }

        let status = TerminateProcess(handle, 1);
        let terminate_error = std::io::Error::last_os_error();
        CloseHandle(handle);
        if status == 0 {
            if terminate_error.raw_os_error() == Some(87) {
                Ok(())
            } else {
                Err(format!(
                    "failed to terminate isolated headless worker: {terminate_error}"
                ))
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn terminate_headless_worker_process(_pid: u32) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
struct HeadlessWindowsJob {
    handle: *mut c_void,
}

#[cfg(windows)]
impl Drop for HeadlessWindowsJob {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;

        unsafe {
            if !self.handle.is_null() {
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(windows)]
fn attach_headless_worker_job(
    child: &std::process::Child,
    request: &HeadlessExecutionRequest,
    verbose: bool,
) -> Option<HeadlessWindowsJob> {
    use std::mem;
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
        JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOB_OBJECT_LIMIT_PROCESS_MEMORY, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JobObjectExtendedLimitInformation, SetInformationJobObject,
    };

    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            if verbose {
                eprintln!(
                    "[agamc] warning: failed to create a Windows job object for isolated headless execution: {}",
                    std::io::Error::last_os_error()
                );
            }
            return None;
        }

        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_PROCESS_MEMORY
            | JOB_OBJECT_LIMIT_JOB_MEMORY
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        limits.ProcessMemoryLimit = request.policy.max_memory_bytes.min(usize::MAX as u64) as usize;
        limits.JobMemoryLimit = request.policy.max_memory_bytes.min(usize::MAX as u64) as usize;
        limits.BasicLimitInformation.ActiveProcessLimit =
            if matches!(request.backend, HeadlessExecutionBackend::Jit) {
                4
            } else {
                16
            };

        let set_status = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const c_void,
            mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        if set_status == 0 {
            if verbose {
                eprintln!(
                    "[agamc] warning: failed to configure a Windows job object for isolated headless execution: {}",
                    std::io::Error::last_os_error()
                );
            }
            return Some(HeadlessWindowsJob { handle: job });
        }

        let assign_status = AssignProcessToJobObject(job, child.as_raw_handle() as *mut c_void);
        if assign_status == 0 {
            if verbose {
                eprintln!(
                    "[agamc] warning: failed to attach the isolated headless worker to a Windows job object: {}",
                    std::io::Error::last_os_error()
                );
            }
        }

        Some(HeadlessWindowsJob { handle: job })
    }
}

fn normalize_headless_request(
    request: &HeadlessExecutionRequest,
) -> Result<HeadlessExecutionRequest, String> {
    if request.source.trim().is_empty() {
        return Err("headless execution request source cannot be empty".into());
    }
    if request.opt_level > 3 {
        return Err(format!(
            "headless execution opt_level `{}` must be 0..=3",
            request.opt_level
        ));
    }
    let source_bytes = request.source.as_bytes().len();
    if source_bytes > request.policy.max_source_bytes {
        return Err(format!(
            "headless execution request source is {} bytes, exceeding the policy limit of {} bytes",
            source_bytes, request.policy.max_source_bytes
        ));
    }
    if request.args.len() > request.policy.max_arg_count {
        return Err(format!(
            "headless execution request includes {} arg(s), exceeding the policy limit of {}",
            request.args.len(),
            request.policy.max_arg_count
        ));
    }
    let total_arg_bytes = request
        .args
        .iter()
        .map(|arg| arg.as_bytes().len())
        .fold(0usize, usize::saturating_add);
    if total_arg_bytes > request.policy.max_total_arg_bytes {
        return Err(format!(
            "headless execution request arguments occupy {} bytes, exceeding the policy limit of {} bytes",
            total_arg_bytes, request.policy.max_total_arg_bytes
        ));
    }
    if request.policy.max_runtime_ms == 0 {
        return Err("headless execution policy `max_runtime_ms` must be greater than zero".into());
    }
    if request.policy.max_memory_bytes == 0 {
        return Err(
            "headless execution policy `max_memory_bytes` must be greater than zero".into(),
        );
    }
    if !request.policy.allow_native_backends
        && !matches!(request.backend, HeadlessExecutionBackend::Jit)
    {
        return Err(format!(
            "headless execution policy only allows the `jit` backend; `{}` requires `policy.allow_native_backends=true`",
            render_headless_backend_label(request.backend)
        ));
    }

    let mut normalized = request.clone();
    normalized.filename = sanitize_headless_filename(&normalized.filename);
    Ok(normalized)
}

fn sanitize_headless_filename(filename: &str) -> String {
    let filename = filename.trim();
    let candidate = Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("snippet.agam");
    let mut sanitized = String::new();
    for ch in candidate.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        sanitized = "snippet.agam".into();
    }
    if !sanitized.ends_with(".agam") {
        sanitized.push_str(".agam");
    }
    sanitized
}

fn create_unique_headless_dir(base: &Path, label: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(base).map_err(|error| {
        format!(
            "failed to create headless sandbox root `{}`: {error}",
            base.display()
        )
    })?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("failed to read system time for temp dir: {error}"))?
        .as_nanos();
    for attempt in 0..32u32 {
        let path = base.join(format!(
            "{label}_{}_{}_{}",
            std::process::id(),
            now,
            attempt
        ));
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to create headless temp dir `{}`: {error}",
                    path.display()
                ));
            }
        }
    }
    Err("failed to allocate a unique headless temp directory".into())
}

fn create_headless_sandbox_root() -> Result<PathBuf, String> {
    create_unique_headless_dir(&std::env::temp_dir(), "agam_headless_sandbox")
}

fn create_headless_temp_dir() -> Result<PathBuf, String> {
    let base = env_path(HEADLESS_SANDBOX_ROOT_ENV).unwrap_or_else(std::env::temp_dir);
    create_unique_headless_dir(&base, "agam_headless_run")
}

fn cleanup_headless_temp_dir(path: &Path, verbose: bool) {
    if let Err(error) = std::fs::remove_dir_all(path) {
        if verbose {
            eprintln!(
                "[agamc] warning: failed to remove headless temp dir `{}`: {}",
                path.display(),
                error
            );
        }
    }
}

fn headless_backend_to_backend(backend: HeadlessExecutionBackend) -> Backend {
    match backend {
        HeadlessExecutionBackend::Auto => Backend::Auto,
        HeadlessExecutionBackend::C => Backend::C,
        HeadlessExecutionBackend::Llvm => Backend::Llvm,
        HeadlessExecutionBackend::Jit => Backend::Jit,
    }
}

fn parse_headless_backend_label(value: &str) -> Result<HeadlessExecutionBackend, String> {
    match value {
        "auto" => Ok(HeadlessExecutionBackend::Auto),
        "c" => Ok(HeadlessExecutionBackend::C),
        "llvm" => Ok(HeadlessExecutionBackend::Llvm),
        "jit" => Ok(HeadlessExecutionBackend::Jit),
        _ => Err(format!(
            "unknown backend `{value}`; expected auto, c, llvm, or jit"
        )),
    }
}

fn render_headless_backend_label(backend: HeadlessExecutionBackend) -> &'static str {
    match backend {
        HeadlessExecutionBackend::Auto => "auto",
        HeadlessExecutionBackend::C => "c",
        HeadlessExecutionBackend::Llvm => "llvm",
        HeadlessExecutionBackend::Jit => "jit",
    }
}

fn print_doctor_status(label: &str, status: &str, detail: &str) {
    println!("{label}: {status}");
    println!("  {detail}");
}

fn run_doctor(
    environment: Option<&EnvironmentInspectReport>,
    verbose: bool,
) -> Result<bool, String> {
    let host = current_host_sdk_platform();
    let bundled_root = detect_packaged_llvm_bundle_root();
    let bundled_driver = discover_bundled_llvm_clang();
    let override_driver = configured_llvm_clang_override();
    let native_driver = resolve_native_llvm_command();
    let vs_install = discover_visual_studio_installation_path();
    let vs_driver = discover_visual_studio_llvm_clang();
    let wsl_clang = wsl_command_exists("clang");
    let c_driver = command_exists(default_c_compiler());
    let android_sysroot = resolve_android_sysroot_for_target(None);

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("failed to locate current compiler executable: {}", e))?;

    println!("Agam Doctor");
    println!("host: {host}");
    println!("core compiler: {}", current_exe.display());
    if let Some(environment) = environment {
        println!("environment: {}", environment_selection_label(environment));
        println!(
            "environment manifest: {}",
            environment.manifest_path.display()
        );
    }

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

    let mut healthy = native_driver.is_some();
    if let Some(environment) = environment {
        let resolved = &environment.environment;
        print_doctor_status(
            "env compiler",
            "selected",
            &format!("compiler requirement `{}`", resolved.compiler),
        );
        if let Some(sdk) = resolved.sdk.as_deref() {
            print_doctor_status("env sdk", "selected", &format!("sdk `{sdk}`"));
        } else if verbose {
            print_doctor_status("env sdk", "inherit", "no environment-specific SDK override");
        }
        if let Some(target) = resolved.target.as_deref() {
            let sdk_root = env_path(LLVM_SDKROOT_ENV).or_else(|| env_path("SDKROOT"));
            let (status, detail, target_ok) = match classify_llvm_target_platform(Some(target)) {
                LlvmTargetPlatform::Android => match android_sysroot.as_ref() {
                    Some(path) => (
                        "ok",
                        format!("target `{target}` via sysroot `{}`", path.display()),
                        true,
                    ),
                    None => (
                        "missing",
                        format!(
                            "target `{target}` needs `{LLVM_SYSROOT_ENV}` or `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT`"
                        ),
                        false,
                    ),
                },
                LlvmTargetPlatform::Ios | LlvmTargetPlatform::MacOs => match sdk_root.as_ref() {
                    Some(path) => (
                        "ok",
                        format!("target `{target}` via SDK root `{}`", path.display()),
                        true,
                    ),
                    None => (
                        "missing",
                        format!("target `{target}` needs `{LLVM_SDKROOT_ENV}` or `SDKROOT`"),
                        false,
                    ),
                },
                _ => ("ok", format!("target `{target}`"), true),
            };
            print_doctor_status("env target", status, &detail);
            healthy &= target_ok;
        } else if verbose {
            print_doctor_status("env target", "inherit", "host-native target");
        }
        if let Some(backend) = resolved.preferred_backend {
            let (status, detail, backend_ok) = match backend {
                agam_runtime::contract::RuntimeBackend::Llvm => (
                    if native_driver.is_some() {
                        "ok"
                    } else {
                        "missing"
                    },
                    if native_driver.is_some() {
                        "environment can use the native LLVM backend".to_string()
                    } else {
                        "environment requests LLVM but no native LLVM toolchain was detected"
                            .to_string()
                    },
                    native_driver.is_some(),
                ),
                agam_runtime::contract::RuntimeBackend::C => (
                    if c_driver { "ok" } else { "missing" },
                    if c_driver {
                        format!("environment can use `{}`", default_c_compiler())
                    } else {
                        format!(
                            "environment requests the C backend but `{}` was not detected",
                            default_c_compiler()
                        )
                    },
                    c_driver,
                ),
                agam_runtime::contract::RuntimeBackend::Jit => (
                    "ok",
                    "environment prefers the in-memory JIT backend".to_string(),
                    true,
                ),
                agam_runtime::contract::RuntimeBackend::Auto => (
                    "selected",
                    "environment defers backend choice to normal auto-resolution".to_string(),
                    true,
                ),
            };
            print_doctor_status("env backend", status, &detail);
            healthy &= backend_ok;
        } else if verbose {
            print_doctor_status(
                "env backend",
                "inherit",
                "no environment-specific backend override",
            );
        }
        if let Some(runtime_abi) = resolved.runtime_abi {
            let abi_ok = runtime_abi == agam_runtime::contract::RUNTIME_ABI_VERSION;
            print_doctor_status(
                "env runtime abi",
                if abi_ok { "ok" } else { "mismatch" },
                &format!(
                    "environment expects v{}; host runtime exports v{}",
                    runtime_abi,
                    agam_runtime::contract::RUNTIME_ABI_VERSION
                ),
            );
            healthy &= abi_ok;
        }
        if !resolved.profiles.is_empty() {
            print_doctor_status(
                "env profiles",
                "selected",
                &format!("profiles `{}`", resolved.profiles.join(", ")),
            );
        }
    }

    println!(
        "recommended sdk command: agamc package sdk{} --output {}",
        environment
            .map(|report| format!(" --env {}", report.environment.name))
            .unwrap_or_default(),
        default_sdk_distribution_output_dir().display()
    );

    Ok(healthy)
}

#[derive(Debug)]
struct SdkDistributionOutcome {
    root: PathBuf,
    compiler_binary: PathBuf,
    manifest_path: PathBuf,
    llvm_bundle_root: Option<PathBuf>,
    android_sysroot_root: Option<PathBuf>,
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

fn sdk_supported_targets(
    environment: Option<&EnvironmentInspectReport>,
    packaged_android_sysroot: Option<&str>,
) -> Vec<agam_pkg::SdkTargetProfile> {
    let mut targets = vec![agam_pkg::SdkTargetProfile {
        name: "host-native".into(),
        target_triple: default_host_target_triple(),
        backend: agam_runtime::contract::RuntimeBackend::Llvm,
        sysroot_env: None,
        sdk_env: None,
        packaged_sysroot: None,
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
            packaged_sysroot: packaged_android_sysroot.map(str::to_string),
        });
    }

    if let Some(environment) = environment {
        if let Some(target) = environment.environment.target.as_deref() {
            let platform = classify_llvm_target_platform(Some(target));
            let sysroot_env = match platform {
                LlvmTargetPlatform::Android => Some(LLVM_SYSROOT_ENV.into()),
                _ => None,
            };
            let sdk_env = match platform {
                LlvmTargetPlatform::Ios | LlvmTargetPlatform::MacOs => {
                    Some(LLVM_SDKROOT_ENV.into())
                }
                _ => None,
            };
            let packaged_sysroot = match platform {
                LlvmTargetPlatform::Android => packaged_android_sysroot.map(str::to_string),
                _ => None,
            };
            let backend = match environment.environment.preferred_backend {
                Some(agam_runtime::contract::RuntimeBackend::Auto) | None => {
                    agam_runtime::contract::RuntimeBackend::Llvm
                }
                Some(backend) => backend,
            };

            if let Some(existing) = targets
                .iter_mut()
                .find(|profile| profile.target_triple == target)
            {
                existing.backend = backend;
                if existing.sysroot_env.is_none() {
                    existing.sysroot_env = sysroot_env;
                }
                if existing.sdk_env.is_none() {
                    existing.sdk_env = sdk_env;
                }
                if existing.packaged_sysroot.is_none() {
                    existing.packaged_sysroot = packaged_sysroot;
                }
            } else {
                targets.insert(
                    0,
                    agam_pkg::SdkTargetProfile {
                        name: environment.environment.name.clone(),
                        target_triple: target.to_string(),
                        backend,
                        sysroot_env,
                        sdk_env,
                        packaged_sysroot,
                    },
                );
            }
        }
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

fn validate_android_sysroot_layout(source: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "Android sysroot source `{}` does not exist or is not a directory",
            source.display()
        ));
    }
    if !source.join("usr").is_dir() {
        return Err(format!(
            "Android sysroot `{}` must include a `usr/` directory",
            source.display()
        ));
    }
    Ok(())
}

fn stage_android_sysroot_into_sdk(source: &Path, output_root: &Path) -> Result<PathBuf, String> {
    validate_android_sysroot_layout(source)?;
    let destination = output_root
        .join("target-packs")
        .join("android-arm64")
        .join("sysroot");
    copy_directory_recursive(source, &destination)?;
    Ok(destination)
}

fn package_sdk_distribution(
    output_root: &Path,
    llvm_bundle: Option<&PathBuf>,
    android_sysroot: Option<&PathBuf>,
    environment: Option<&EnvironmentInspectReport>,
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
    let staged_android_sysroot = match resolve_sdk_android_sysroot_source(android_sysroot) {
        Some(source) => {
            let staged = stage_android_sysroot_into_sdk(&source, output_root)?;
            if verbose {
                eprintln!(
                    "[agamc] staged Android sysroot target pack from {}",
                    source.display()
                );
            }
            Some(staged)
        }
        None => None,
    };
    let android_sysroot_relative = staged_android_sysroot
        .as_ref()
        .map(|path| relative_path_string(output_root, path))
        .transpose()?;

    let preferred_llvm_driver = llvm_bundle_root.as_ref().and_then(|root| {
        bundled_llvm_candidate_paths(root)
            .into_iter()
            .find(|path| path.is_file())
    });
    let mut notes = vec![
        "native llvm is the preferred production backend".into(),
        "wsl remains a development-only fallback and is not part of the shipped sdk contract"
            .into(),
    ];
    if let Some(environment) = environment {
        let resolved = &environment.environment;
        let mut note = format!(
            "selected environment `{}` pins compiler `{}`",
            resolved.name, resolved.compiler
        );
        if let Some(sdk) = resolved.sdk.as_deref() {
            note.push_str(&format!(", sdk `{sdk}`"));
        }
        if let Some(target) = resolved.target.as_deref() {
            note.push_str(&format!(", target `{target}`"));
        }
        if let Some(backend) = resolved.preferred_backend {
            note.push_str(&format!(", backend `{}`", runtime_backend_label(backend)));
        }
        if !resolved.profiles.is_empty() {
            note.push_str(&format!(", profiles `{}`", resolved.profiles.join(", ")));
        }
        notes.push(note);
    }
    if let Some(relative) = android_sysroot_relative.as_deref() {
        notes.push(format!(
            "bundled Android target pack `android-arm64` at `{relative}`"
        ));
    }

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
        supported_targets: sdk_supported_targets(environment, android_sysroot_relative.as_deref()),
        notes,
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
        android_sysroot_root: staged_android_sysroot,
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
        && header != "AGAM_LLVM_CALL_CACHE_PROFILE_V5"
        && header != "AGAM_LLVM_CALL_CACHE_PROFILE_V6"
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
                if parts.len() != 6 && parts.len() != 8 {
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
                let (unique_keys, hottest_key_hits) = if parts.len() == 8 {
                    let unique_keys = parts[6].parse::<usize>().map_err(|e| {
                        format!(
                            "invalid LLVM call-cache unique-key count on line {}: {}",
                            line_index + 2,
                            e
                        )
                    })?;
                    let hottest_key_hits = parts[7].parse::<u64>().map_err(|e| {
                        format!(
                            "invalid LLVM call-cache hottest-key hit count on line {}: {}",
                            line_index + 2,
                            e
                        )
                    })?;
                    (unique_keys, hottest_key_hits)
                } else {
                    (entries.max(stores as usize), 0)
                };

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
                        unique_keys,
                        hottest_key_hits,
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
            Some("SC") => {
                if parts.len() != 5 {
                    return Err(format!(
                        "invalid LLVM call-cache specialization-clone line {}: `{}`",
                        line_index + 2,
                        line
                    ));
                }
                let Some(function_index) = function_indexes.get(parts[1]).copied() else {
                    return Err(format!(
                        "LLVM call-cache specialization-clone line {} references unknown function `{}`",
                        line_index + 2,
                        parts[1]
                    ));
                };
                let stable_values = agam_profile::parse_specialization_feedback_signature(parts[2])
                    .map_err(|e| {
                        format!(
                            "invalid LLVM call-cache specialization-clone signature on line {}: {}",
                            line_index + 2,
                            e
                        )
                    })?;
                let guard_hits = parts[3].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache specialization-clone hit count on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                let guard_fallbacks = parts[4].parse::<u64>().map_err(|e| {
                    format!(
                        "invalid LLVM call-cache specialization-clone fallback count on line {}: {}",
                        line_index + 2,
                        e
                    )
                })?;
                if !stable_values.is_empty() && guard_hits.saturating_add(guard_fallbacks) > 0 {
                    functions[function_index]
                        .profile
                        .specialization_profiles
                        .push(agam_profile::CallCacheSpecializationFeedbackProfile {
                            stable_values,
                            guard_hits,
                            guard_fallbacks,
                        });
                }
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
        if !merged.caches_function(&function) {
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
        .filter(|plan| selection.caches_function(&plan.name))
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
    // 1. Try entry-file portable-package prewarm (highest priority)
    if let Some(prewarmed) = load_daemon_prewarmed_entry(path, verbose) {
        let source_features = SourceFeatureFlags {
            call_cache: prewarmed.call_cache,
            experimental_usages: Vec::new(),
        };
        let call_cache = effective_call_cache_selection(features, &source_features);
        return build_prelowered_file(
            path,
            output,
            opt_level,
            backend,
            tuning,
            &prewarmed.package.mir,
            &call_cache,
            &[],
            false,
            verbose,
        );
    }

    // 2. Fallback: try the multi-file warm index for MIR
    if let Some(warm_state) = load_daemon_warm_state_for_file(path, verbose) {
        if warm_state_supports_runnable_reuse(&warm_state) {
            let mir = warm_state.mir.as_ref().expect("checked by helper");
            let call_cache = effective_call_cache_selection(
                features,
                warm_state
                    .source_features
                    .as_ref()
                    .expect("checked by helper"),
            );
            return build_prelowered_file(
                path,
                output,
                opt_level,
                backend,
                tuning,
                mir,
                &call_cache,
                &[],
                false,
                verbose,
            );
        }
        if verbose && warm_state.mir.is_some() {
            eprintln!(
                "[agamc] warm state for `{}` is incomplete for build reuse; rebuilding locally",
                path.display()
            );
        }
    }

    // 3. Full pipeline from source
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
    ensure_build_output_parent_dir(output)?;

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
    let (mir, source_features) = lower_to_optimized_mir(path, verbose)?;
    run_with_llvm_prelowered(
        path,
        args,
        opt_level,
        tuning,
        &mir,
        &source_features,
        verbose,
        features,
    )
}

fn run_with_llvm_prelowered(
    path: &PathBuf,
    args: &[String],
    opt_level: u8,
    tuning: &ReleaseTuning,
    mir: &agam_mir::ir::MirModule,
    source_features: &SourceFeatureFlags,
    verbose: bool,
    features: FeatureFlags,
) -> Result<i32, String> {
    let allow_dev_wsl_llvm = allow_dev_wsl_llvm();
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

    let exe_path = default_build_output_path(path, tuning.target.as_deref())?;
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
    run_with_jit_prelowered(path, args, &mir, &source_features, verbose, features)
}

fn run_with_jit_prelowered(
    path: &PathBuf,
    args: &[String],
    mir: &agam_mir::ir::MirModule,
    source_features: &SourceFeatureFlags,
    verbose: bool,
    features: FeatureFlags,
) -> Result<i32, String> {
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
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::time::Duration;

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

    fn with_clean_agam_registry_env<R>(f: impl FnOnce() -> R) -> R {
        let _guard = registry_env_lock()
            .lock()
            .expect("registry env lock should not be poisoned");
        let default_key = "AGAM_REGISTRY_INDEX";
        let agam_key = agam_pkg::registry_index_env_var("agam");
        let default_restore = RegistryIndexEnvRestore::capture(default_key);
        let agam_restore = RegistryIndexEnvRestore::capture(&agam_key);
        unsafe {
            std::env::remove_var(default_key);
            std::env::remove_var(&agam_key);
        }
        let result = f();
        drop(agam_restore);
        drop(default_restore);
        result
    }

    fn environment_report(
        name: &str,
        target: Option<&str>,
        backend: Option<agam_runtime::contract::RuntimeBackend>,
    ) -> EnvironmentInspectReport {
        EnvironmentInspectReport {
            workspace_root: PathBuf::from("C:/agam/workspace"),
            manifest_path: PathBuf::from("C:/agam/workspace/agam.toml"),
            selected_by_default: false,
            environment: agam_pkg::ResolvedEnvironment {
                name: name.into(),
                compiler: "0.2.0".into(),
                sdk: None,
                target: target.map(str::to_string),
                runtime_abi: Some(agam_runtime::contract::RUNTIME_ABI_VERSION),
                preferred_backend: backend,
                profiles: vec!["release".into()],
                packages: vec!["json@1.4.0".into()],
            },
        }
    }

    fn build_request(file: impl Into<PathBuf>, output: impl Into<PathBuf>) -> BuildRequest {
        BuildRequest {
            file: file.into(),
            output: output.into(),
        }
    }

    fn check_request(file: impl Into<PathBuf>) -> CheckRequest {
        CheckRequest { file: file.into() }
    }

    fn test_request(file: impl Into<PathBuf>) -> TestRequest {
        TestRequest { file: file.into() }
    }

    fn update_maximum(counter: &AtomicUsize, candidate: usize) {
        let mut current = counter.load(AtomicOrdering::SeqCst);
        while candidate > current {
            match counter.compare_exchange(
                current,
                candidate,
                AtomicOrdering::SeqCst,
                AtomicOrdering::SeqCst,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
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
            agam_pkg::read_workspace_manifest_from_path(&project_root.join("agam.toml"))
                .expect("read manifest")
                .project
                .name
                == "hello-app"
        );
        let gitignore =
            fs::read_to_string(project_root.join(".gitignore")).expect("read gitignore");
        assert!(gitignore.contains("dist/"));
        assert!(!gitignore.contains("src/main"));

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
        agam_pkg::write_workspace_manifest_to_path(
            &manifest,
            &agam_pkg::scaffold_workspace_manifest("workspace"),
        )
        .expect("write manifest");
        fs::write(&entry, render_project_entry("workspace")).expect("write entry");
        fs::write(&test_file, render_project_smoke_test()).expect("write test");

        let layout =
            resolve_workspace_layout(Some(root.clone())).expect("workspace layout should resolve");

        assert_eq!(layout.root, root);
        assert_eq!(layout.manifest_path.as_ref(), Some(&manifest));
        assert_eq!(layout.project_name, "workspace");
        assert_eq!(layout.entry_file, entry);
        assert_eq!(layout.test_files, vec![test_file]);

        let _ = fs::remove_dir_all(layout.root);
    }

    #[test]
    fn test_resolve_workspace_layout_uses_manifest_declared_entry_path() {
        let root = temp_dir("workspace_entry");
        let manifest = root.join("agam.toml");
        let entry = root.join("app").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create app");

        let mut workspace_manifest = agam_pkg::scaffold_workspace_manifest("workspace-entry");
        workspace_manifest.project.entry = Some("app/main.agam".into());
        agam_pkg::write_workspace_manifest_to_path(&manifest, &workspace_manifest)
            .expect("write manifest");
        fs::write(&entry, render_project_entry("workspace-entry")).expect("write entry");

        let layout =
            resolve_workspace_layout(Some(root.clone())).expect("workspace layout should resolve");

        assert_eq!(layout.manifest_path.as_ref(), Some(&manifest));
        assert_eq!(layout.project_name, "workspace-entry");
        assert_eq!(layout.entry_file, entry);
        assert_eq!(layout.source_files, vec![layout.entry_file.clone()]);

        let _ = fs::remove_dir_all(layout.root);
    }

    #[test]
    fn test_resolve_workspace_layout_rejects_manifest_entry_outside_workspace() {
        let root = temp_dir("workspace_invalid_entry");
        let manifest = root.join("agam.toml");

        let mut workspace_manifest = agam_pkg::scaffold_workspace_manifest("workspace-invalid");
        workspace_manifest.project.entry = Some("../escape.agam".into());
        agam_pkg::write_workspace_manifest_to_path(&manifest, &workspace_manifest)
            .expect("write manifest");

        let error = resolve_workspace_layout(Some(root.clone())).expect_err("manifest should fail");
        assert!(error.contains("must stay inside the workspace root"));

        let _ = fs::remove_dir_all(root);
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
    fn test_resolve_entry_source_path_uses_workspace_root_entry_file() {
        let root = temp_dir("entry_source_root");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        agam_pkg::write_workspace_manifest_to_path(
            &manifest,
            &agam_pkg::scaffold_workspace_manifest("entry-source-root"),
        )
        .expect("write manifest");
        fs::write(&entry, render_project_entry("entry-source-root")).expect("write entry");

        let resolved =
            resolve_entry_source_path(&root).expect("workspace root should resolve to entry file");
        assert_eq!(resolved, entry);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_default_package_output_path_uses_dist_for_manifest_workspace() {
        let root = temp_dir("package_output_workspace");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        agam_pkg::write_workspace_manifest_to_path(
            &manifest,
            &agam_pkg::scaffold_workspace_manifest("package-output-workspace"),
        )
        .expect("write manifest");
        fs::write(&entry, render_project_entry("package-output-workspace")).expect("write entry");

        let output = default_package_output_path(&root)
            .expect("workspace root should resolve package output");
        assert_eq!(
            output,
            root.join("dist")
                .join("package-output-workspace.agpkg.json")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_default_package_output_path_keeps_single_file_neighbor() {
        let root = temp_dir("package_output_single_file");
        let file = root.join("script.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        let output =
            default_package_output_path(&file).expect("single-file package output should resolve");
        assert_eq!(output, root.join("script.agpkg.json"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_default_build_output_path_uses_dist_for_manifest_workspace() {
        let root = temp_dir("build_output_workspace");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        agam_pkg::write_workspace_manifest_to_path(
            &manifest,
            &agam_pkg::scaffold_workspace_manifest("build-output-workspace"),
        )
        .expect("write manifest");
        fs::write(&entry, render_project_entry("build-output-workspace")).expect("write entry");

        let output = default_build_output_path(&root, Some("x86_64-pc-windows-msvc"))
            .expect("workspace root should resolve build output");
        assert_eq!(output, root.join("dist").join("build-output-workspace.exe"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_default_build_output_path_keeps_single_file_neighbor() {
        let root = temp_dir("build_output_single_file");
        let file = root.join("script.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        let output = default_build_output_path(&file, None)
            .expect("single-file build output should resolve");
        assert_eq!(output, default_native_binary_output_path(&file, None));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_resolve_build_requests_uses_workspace_manifest_entry_output() {
        let root = temp_dir("build_requests_manifest");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        agam_pkg::write_workspace_manifest_to_path(
            &manifest,
            &agam_pkg::scaffold_workspace_manifest("build-requests-manifest"),
        )
        .expect("write manifest");
        fs::write(&entry, render_project_entry("build-requests-manifest")).expect("write entry");

        let requests = resolve_build_requests(
            std::slice::from_ref(&manifest),
            None,
            Some("x86_64-pc-windows-msvc"),
        )
        .expect("manifest input should resolve to entry file");
        assert_eq!(
            requests,
            vec![BuildRequest {
                file: entry.clone(),
                output: entry.with_extension("exe"),
            }]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_compile_file_rejects_undeclared_identifier() {
        let root = temp_dir("compile_semantic_error");
        let file = root.join("broken.agam");
        fs::write(&file, "fn main(): y\n").expect("write source");

        let error = compile_file(&file, false).expect_err("compile should fail");
        assert!(error.contains("semantic error"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_compile_file_accepts_builtin_println() {
        let root = temp_dir("compile_builtin");
        let file = root.join("builtin.agam");
        fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

        compile_file(&file, false).expect("compile should succeed");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_resolve_build_requests_rejects_explicit_output_for_multiple_inputs() {
        let files = vec![PathBuf::from("a.agam"), PathBuf::from("b.agam")];
        let error = resolve_build_requests(&files, Some(PathBuf::from("out.exe")), None)
            .expect_err("multiple inputs should reject one explicit output");
        assert!(error.contains("`--output` only supports a single input file"));
    }

    #[test]
    fn test_resolve_build_requests_uses_default_output_per_file() {
        let root = temp_dir("build_requests_defaults");
        let alpha = root.join("alpha.agam");
        let beta = root.join("beta.agam");
        fs::write(&alpha, "fn main() -> i32 { return 0; }\n").expect("write alpha source");
        fs::write(&beta, "fn main() -> i32 { return 0; }\n").expect("write beta source");

        let files = vec![alpha.clone(), beta.clone()];
        let requests = resolve_build_requests(&files, None, Some("x86_64-pc-windows-msvc"))
            .expect("build requests should resolve");
        assert_eq!(requests.len(), 2);
        assert_eq!(
            requests[0],
            BuildRequest {
                file: alpha.clone(),
                output: alpha.with_extension("exe"),
            }
        );
        assert_eq!(
            requests[1],
            BuildRequest {
                file: beta.clone(),
                output: beta.with_extension("exe"),
            }
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_resolve_build_requests_keeps_explicit_output_for_single_input() {
        let root = temp_dir("build_requests_single_output");
        let file = root.join("main.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        let files = vec![file.clone()];
        let output = root.join("program.exe");
        let requests = resolve_build_requests(&files, Some(output.clone()), None)
            .expect("single input should allow explicit output");
        assert_eq!(requests, vec![BuildRequest { file, output }]);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_resolve_build_requests_resolves_workspace_root_before_explicit_output() {
        let root = temp_dir("build_requests_root_output");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        agam_pkg::write_workspace_manifest_to_path(
            &manifest,
            &agam_pkg::scaffold_workspace_manifest("build-requests-root-output"),
        )
        .expect("write manifest");
        fs::write(&entry, render_project_entry("build-requests-root-output")).expect("write entry");

        let output = root.join("dist").join("program.exe");
        let requests =
            resolve_build_requests(std::slice::from_ref(&root), Some(output.clone()), None)
                .expect("workspace root should resolve to entry before output is applied");
        assert_eq!(
            requests,
            vec![BuildRequest {
                file: entry.clone(),
                output,
            }]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_resolve_build_requests_deduplicates_overlapping_workspace_inputs() {
        let root = temp_dir("build_requests_dedup");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        agam_pkg::write_workspace_manifest_to_path(
            &manifest,
            &agam_pkg::scaffold_workspace_manifest("build-requests-dedup"),
        )
        .expect("write manifest");
        fs::write(&entry, render_project_entry("build-requests-dedup")).expect("write entry");

        let requests = resolve_build_requests(
            &[root.clone(), manifest.clone(), entry.clone()],
            None,
            Some("x86_64-pc-windows-msvc"),
        )
        .expect("overlapping workspace inputs should resolve");
        assert_eq!(
            requests,
            vec![BuildRequest {
                file: entry.clone(),
                output: entry.with_extension("exe"),
            }]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_execute_build_requests_with_runner_preserves_request_order() {
        let requests = vec![
            build_request("alpha.agam", "alpha.exe"),
            build_request("beta.agam", "beta.exe"),
            build_request("gamma.agam", "gamma.exe"),
        ];

        let results = execute_build_requests_with_runner(&requests, 3, |request| {
            let delay_ms = match request.file.file_stem().and_then(|name| name.to_str()) {
                Some("alpha") => 40,
                Some("beta") => 5,
                Some("gamma") => 20,
                _ => 1,
            };
            std::thread::sleep(Duration::from_millis(delay_ms));
            BuildRequestResult {
                request: request.clone(),
                stdout: request.file.to_string_lossy().as_bytes().to_vec(),
                stderr: Vec::new(),
                succeeded: true,
                launch_error: None,
            }
        });

        let result_requests = results
            .iter()
            .map(|result| result.request.clone())
            .collect::<Vec<_>>();
        assert_eq!(result_requests, requests);
    }

    #[test]
    fn test_execute_build_requests_with_runner_respects_parallelism_limit() {
        let requests = vec![
            build_request("one.agam", "one.exe"),
            build_request("two.agam", "two.exe"),
            build_request("three.agam", "three.exe"),
            build_request("four.agam", "four.exe"),
        ];
        let active = AtomicUsize::new(0);
        let observed_max = AtomicUsize::new(0);

        let results = execute_build_requests_with_runner(&requests, 2, |request| {
            let now_active = active.fetch_add(1, AtomicOrdering::SeqCst) + 1;
            update_maximum(&observed_max, now_active);
            std::thread::sleep(Duration::from_millis(20));
            active.fetch_sub(1, AtomicOrdering::SeqCst);

            BuildRequestResult {
                request: request.clone(),
                stdout: Vec::new(),
                stderr: Vec::new(),
                succeeded: true,
                launch_error: None,
            }
        });

        assert_eq!(results.len(), requests.len());
        assert!(observed_max.load(AtomicOrdering::SeqCst) <= 2);
        assert!(observed_max.load(AtomicOrdering::SeqCst) >= 2);
    }

    #[test]
    fn test_execute_check_requests_with_runner_preserves_request_order() {
        let requests = vec![
            check_request("alpha.agam"),
            check_request("beta.agam"),
            check_request("gamma.agam"),
        ];

        let results = execute_check_requests_with_runner(&requests, 3, |request| {
            let delay_ms = match request.file.file_stem().and_then(|name| name.to_str()) {
                Some("alpha") => 40,
                Some("beta") => 5,
                Some("gamma") => 20,
                _ => 1,
            };
            std::thread::sleep(Duration::from_millis(delay_ms));
            CheckRequestResult {
                request: request.clone(),
                stdout: request.file.to_string_lossy().as_bytes().to_vec(),
                stderr: Vec::new(),
                succeeded: true,
                launch_error: None,
            }
        });

        let result_requests = results
            .iter()
            .map(|result| result.request.clone())
            .collect::<Vec<_>>();
        assert_eq!(result_requests, requests);
    }

    #[test]
    fn test_execute_check_requests_with_runner_respects_parallelism_limit() {
        let requests = vec![
            check_request("one.agam"),
            check_request("two.agam"),
            check_request("three.agam"),
            check_request("four.agam"),
        ];
        let active = AtomicUsize::new(0);
        let observed_max = AtomicUsize::new(0);

        let results = execute_check_requests_with_runner(&requests, 2, |request| {
            let now_active = active.fetch_add(1, AtomicOrdering::SeqCst) + 1;
            update_maximum(&observed_max, now_active);
            std::thread::sleep(Duration::from_millis(20));
            active.fetch_sub(1, AtomicOrdering::SeqCst);

            CheckRequestResult {
                request: request.clone(),
                stdout: Vec::new(),
                stderr: Vec::new(),
                succeeded: true,
                launch_error: None,
            }
        });

        assert_eq!(results.len(), requests.len());
        assert!(observed_max.load(AtomicOrdering::SeqCst) <= 2);
        assert!(observed_max.load(AtomicOrdering::SeqCst) >= 2);
    }

    #[test]
    fn test_execute_test_requests_with_runner_preserves_request_order() {
        let requests = vec![
            test_request("alpha.agam"),
            test_request("beta.agam"),
            test_request("gamma.agam"),
        ];

        let results = execute_test_requests_with_runner(&requests, 3, |request| {
            let delay_ms = match request.file.file_stem().and_then(|name| name.to_str()) {
                Some("alpha") => 40,
                Some("beta") => 5,
                Some("gamma") => 20,
                _ => 1,
            };
            std::thread::sleep(Duration::from_millis(delay_ms));
            TestRequestResult {
                request: request.clone(),
                summary: Some(agam_test::FileTestSummary {
                    path: request.file.clone(),
                    summary: agam_test::TestSummary::default(),
                }),
                error: None,
            }
        });

        let result_requests = results
            .iter()
            .map(|result| result.request.clone())
            .collect::<Vec<_>>();
        assert_eq!(result_requests, requests);
    }

    #[test]
    fn test_execute_test_requests_with_runner_respects_parallelism_limit() {
        let requests = vec![
            test_request("one.agam"),
            test_request("two.agam"),
            test_request("three.agam"),
            test_request("four.agam"),
        ];
        let active = AtomicUsize::new(0);
        let observed_max = AtomicUsize::new(0);

        let results = execute_test_requests_with_runner(&requests, 2, |request| {
            let now_active = active.fetch_add(1, AtomicOrdering::SeqCst) + 1;
            update_maximum(&observed_max, now_active);
            std::thread::sleep(Duration::from_millis(20));
            active.fetch_sub(1, AtomicOrdering::SeqCst);

            TestRequestResult {
                request: request.clone(),
                summary: Some(agam_test::FileTestSummary {
                    path: request.file.clone(),
                    summary: agam_test::TestSummary::default(),
                }),
                error: None,
            }
        });

        assert_eq!(results.len(), requests.len());
        assert!(observed_max.load(AtomicOrdering::SeqCst) <= 2);
        assert!(observed_max.load(AtomicOrdering::SeqCst) >= 2);
    }

    #[test]
    fn test_ensure_build_output_parent_dir_creates_missing_directory() {
        let root = temp_dir("build_output_parent_dir");
        let output = root.join("nested").join("program.exe");

        ensure_build_output_parent_dir(&output).expect("missing output parent should be created");

        assert!(output.parent().expect("parent").is_dir());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_compile_dev_source_file_skips_warm_state_when_not_running() {
        let root = temp_dir("compile_dev_no_run");
        let file = root.join("main.agam");
        fs::write(&file, "@lang.advance\nfn main() -> i32 { return 0; }\n").expect("write source");

        let warm = compile_dev_source_file(&file, false, false).expect("dev compile should work");
        assert!(warm.is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_compile_dev_source_file_keeps_warm_state_for_run() {
        let root = temp_dir("compile_dev_run");
        let file = root.join("main.agam");
        fs::write(&file, "@lang.advance\nfn main() -> i32 { return 0; }\n").expect("write source");

        let warm =
            compile_dev_source_file(&file, true, false).expect("warm dev compile should work");
        let warm = warm.expect("warm state should be retained for runnable entry file");
        assert!(warm.source_features.is_some());
        assert_eq!(warm.mir.as_ref().expect("mir").functions.len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_compile_file_with_warm_state_captures_mir_and_source_features() {
        let root = temp_dir("compile_warm_state");
        let file = root.join("warm.agam");
        fs::write(&file, "@lang.advance\nfn main() -> i32 { return 0; }\n").expect("write source");

        let warm = compile_file_with_warm_state(&file, false).expect("warm compile should succeed");
        assert!(warm.source_features.is_some());
        assert_eq!(warm.mir.as_ref().expect("mir").functions.len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_lower_to_optimized_mir_rejects_type_errors() {
        let root = temp_dir("lower_type_error");
        let file = root.join("broken_type.agam");
        fs::write(&file, "fn main(): while 42: let x = 1\n").expect("write source");

        let error = lower_to_optimized_mir(&file, false).expect_err("lowering should fail");
        assert!(error.contains("type error"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_lower_to_optimized_mir_accepts_valid_source() {
        let root = temp_dir("lower_valid");
        let file = root.join("ok.agam");
        fs::write(&file, "fn main(): let x = 42\n").expect("write source");

        let (mir, _) = lower_to_optimized_mir(&file, false).expect("lowering should succeed");
        assert_eq!(mir.functions.len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_incremental_pipeline_clears_all_warm_state_when_manifest_changes() {
        let root = temp_dir("daemon_manifest_invalidation");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        agam_pkg::write_workspace_manifest_to_path(
            &manifest,
            &agam_pkg::scaffold_workspace_manifest("daemon-manifest"),
        )
        .expect("write manifest");
        fs::write(&entry, render_project_entry("daemon-manifest")).expect("write entry");

        let previous = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
        let source_hash = previous.source_files[0].content_hash.clone();
        let mut session = DaemonSession {
            snapshot: Some(previous.clone()),
            cache: BTreeMap::new(),
            last_prewarm: DaemonPrewarmSummary::default(),
        };
        session.cache.entry(entry.clone()).or_default().insert(
            source_hash,
            WarmState {
                source_features: None,
                module: None,
                hir: None,
                mir: None,
            },
        );

        let mut next_manifest = agam_pkg::scaffold_workspace_manifest("daemon-manifest");
        next_manifest.project.version = "0.2.0".into();
        agam_pkg::write_workspace_manifest_to_path(&manifest, &next_manifest)
            .expect("rewrite manifest");

        let next = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
        let diff = agam_pkg::diff_workspace_snapshots(&previous, &next);
        let mut pipeline = IncrementalPipeline::new(&mut session);
        pipeline.apply_diff(next, &diff);

        assert!(session.cache.is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_incremental_pipeline_clears_all_warm_state_when_member_manifest_changes() {
        let root = temp_dir("daemon_member_manifest_invalidation");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        let member_root = root.join("packages").join("core");
        let member_manifest = member_root.join("agam.toml");
        let member_entry = member_root.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create root src");
        fs::create_dir_all(member_entry.parent().expect("member entry parent"))
            .expect("create member src");

        let mut workspace_manifest = agam_pkg::scaffold_workspace_manifest("daemon-manifest");
        workspace_manifest.workspace.members = vec!["packages/core".into()];
        agam_pkg::write_workspace_manifest_to_path(&manifest, &workspace_manifest)
            .expect("write root manifest");
        agam_pkg::write_workspace_manifest_to_path(
            &member_manifest,
            &agam_pkg::scaffold_workspace_manifest("daemon-member"),
        )
        .expect("write member manifest");
        fs::write(&entry, render_project_entry("daemon-manifest")).expect("write root entry");
        fs::write(&member_entry, render_project_entry("daemon-member"))
            .expect("write member entry");

        let previous = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
        let source_hash = previous
            .source_files
            .iter()
            .find(|file| file.path == member_entry)
            .expect("member entry should be tracked")
            .content_hash
            .clone();
        let mut session = DaemonSession {
            snapshot: Some(previous.clone()),
            cache: BTreeMap::new(),
            last_prewarm: DaemonPrewarmSummary::default(),
        };
        session
            .cache
            .entry(member_entry.clone())
            .or_default()
            .insert(
                source_hash,
                WarmState {
                    source_features: None,
                    module: None,
                    hir: None,
                    mir: None,
                },
            );

        let mut next_member_manifest = agam_pkg::scaffold_workspace_manifest("daemon-member");
        next_member_manifest.project.version = "0.2.0".into();
        agam_pkg::write_workspace_manifest_to_path(&member_manifest, &next_member_manifest)
            .expect("rewrite member manifest");

        let next = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
        let diff = agam_pkg::diff_workspace_snapshots(&previous, &next);
        let mut pipeline = IncrementalPipeline::new(&mut session);
        pipeline.apply_diff(next, &diff);

        assert!(session.cache.is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_refresh_daemon_session_reuses_unchanged_files_and_rewarms_changed_ones() {
        let root = temp_dir("daemon_refresh");
        let manifest = root.join("agam.toml");
        let entry = root.join("src").join("main.agam");
        fs::create_dir_all(entry.parent().expect("entry parent")).expect("create src");
        agam_pkg::write_workspace_manifest_to_path(
            &manifest,
            &agam_pkg::scaffold_workspace_manifest("daemon-refresh"),
        )
        .expect("write manifest");
        fs::write(&entry, render_project_entry("daemon-refresh")).expect("write entry");

        let mut session = DaemonSession::default();
        let first_snapshot = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
        let first_hash = first_snapshot.source_files[0].content_hash.clone();
        let (first_warm, first_diff) =
            refresh_daemon_session(&mut session, first_snapshot.clone(), false)
                .expect("warm first snapshot");
        assert_eq!(first_warm.warmed_files, 1);
        assert_eq!(first_warm.reused_files, 0);
        assert_eq!(first_diff.added_files, 2);
        assert!(
            session
                .cache
                .get(&entry)
                .expect("entry cache")
                .contains_key(&first_hash)
        );

        let repeat_snapshot = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
        let (repeat_warm, repeat_diff) =
            refresh_daemon_session(&mut session, repeat_snapshot, false)
                .expect("warm repeated snapshot");
        assert_eq!(repeat_warm.warmed_files, 0);
        assert_eq!(repeat_warm.reused_files, 1);
        assert_eq!(repeat_diff.changed_files, 0);
        assert_eq!(repeat_diff.removed_files, 0);

        fs::write(
            &entry,
            "@lang.advance\n\nfn main() -> i32 {\n    return 1;\n}\n",
        )
        .expect("rewrite entry");
        let changed_snapshot = agam_pkg::snapshot_workspace(Some(root.clone())).expect("snapshot");
        let changed_hash = changed_snapshot.source_files[0].content_hash.clone();
        let (changed_warm, changed_diff) =
            refresh_daemon_session(&mut session, changed_snapshot, false)
                .expect("warm changed snapshot");
        assert_eq!(changed_diff.changed_files, 1);
        assert_eq!(changed_warm.warmed_files, 1);
        assert_eq!(changed_warm.reused_files, 0);
        let entry_versions = session.cache.get(&entry).expect("entry cache");
        assert!(entry_versions.contains_key(&changed_hash));
        assert!(!entry_versions.contains_key(&first_hash));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_active_daemon_status_requires_fresh_heartbeat() {
        let root = temp_dir("daemon_status");
        let mut status = DaemonStatusRecord {
            schema_version: DAEMON_STATUS_SCHEMA_VERSION,
            run_mode: DaemonRunMode::ForegroundLoop,
            workspace_root: root.display().to_string(),
            project_name: "daemon-status".into(),
            pid: process::id(),
            session_started_unix_ms: now_unix_ms(),
            last_heartbeat_unix_ms: now_unix_ms(),
            snapshot_file_count: 2,
            warmed_file_count: 1,
            warmed_version_count: 1,
            ast_decl_count: 1,
            hir_function_count: 1,
            mir_function_count: 1,
            last_error: None,
            prewarm: DaemonPrewarmSummary::default(),
            last_diff: DaemonDiffSummary::default(),
        };
        write_daemon_status(&root, &status).expect("write fresh status");
        assert!(active_daemon_status(&root).expect("read status").is_some());

        status.last_heartbeat_unix_ms = now_unix_ms().saturating_sub(DAEMON_HEARTBEAT_STALE_MS + 1);
        write_daemon_status(&root, &status).expect("write stale status");
        assert!(
            read_daemon_status(&root)
                .expect("read stale status")
                .is_some()
        );
        assert!(
            active_daemon_status(&root)
                .expect("read active status")
                .is_none()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_active_daemon_status_ignores_one_shot_snapshots() {
        let root = temp_dir("daemon_snapshot_status");
        let status = DaemonStatusRecord {
            schema_version: DAEMON_STATUS_SCHEMA_VERSION,
            run_mode: DaemonRunMode::OneShot,
            workspace_root: root.display().to_string(),
            project_name: "daemon-snapshot".into(),
            pid: process::id(),
            session_started_unix_ms: now_unix_ms(),
            last_heartbeat_unix_ms: now_unix_ms(),
            snapshot_file_count: 1,
            warmed_file_count: 1,
            warmed_version_count: 1,
            ast_decl_count: 1,
            hir_function_count: 1,
            mir_function_count: 1,
            last_error: None,
            prewarm: DaemonPrewarmSummary::default(),
            last_diff: DaemonDiffSummary::default(),
        };
        write_daemon_status(&root, &status).expect("write snapshot status");
        assert!(
            active_daemon_status(&root)
                .expect("read active status")
                .is_none()
        );
        assert_eq!(
            daemon_liveness(&status, now_unix_ms()),
            DaemonLiveness::Snapshot
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_active_daemon_status_ignores_error_status() {
        let root = temp_dir("daemon_error_status");
        let status = DaemonStatusRecord {
            schema_version: DAEMON_STATUS_SCHEMA_VERSION,
            run_mode: DaemonRunMode::ForegroundLoop,
            workspace_root: root.display().to_string(),
            project_name: "daemon-error".into(),
            pid: process::id(),
            session_started_unix_ms: now_unix_ms(),
            last_heartbeat_unix_ms: now_unix_ms(),
            snapshot_file_count: 1,
            warmed_file_count: 0,
            warmed_version_count: 0,
            ast_decl_count: 0,
            hir_function_count: 0,
            mir_function_count: 0,
            last_error: Some("parse error".into()),
            prewarm: DaemonPrewarmSummary::default(),
            last_diff: DaemonDiffSummary::default(),
        };
        write_daemon_status(&root, &status).expect("write error status");
        assert!(
            active_daemon_status(&root)
                .expect("read active status")
                .is_none()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_dev_daemon_status_message_reports_snapshot_separately() {
        let root = temp_dir("dev_daemon_snapshot");
        let status = DaemonStatusRecord {
            schema_version: DAEMON_STATUS_SCHEMA_VERSION,
            run_mode: DaemonRunMode::OneShot,
            workspace_root: root.display().to_string(),
            project_name: "dev-daemon-snapshot".into(),
            pid: process::id(),
            session_started_unix_ms: now_unix_ms(),
            last_heartbeat_unix_ms: now_unix_ms(),
            snapshot_file_count: 1,
            warmed_file_count: 1,
            warmed_version_count: 1,
            ast_decl_count: 1,
            hir_function_count: 1,
            mir_function_count: 1,
            last_error: None,
            prewarm: DaemonPrewarmSummary::default(),
            last_diff: DaemonDiffSummary::default(),
        };
        write_daemon_status(&root, &status).expect("write snapshot status");

        let message = dev_daemon_status_message(&root).expect("format dev daemon message");
        assert!(message.contains("snapshot available"));
        assert!(!message.contains("stale"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_dev_daemon_status_message_reports_failure() {
        let root = temp_dir("dev_daemon_error");
        let status = DaemonStatusRecord {
            schema_version: DAEMON_STATUS_SCHEMA_VERSION,
            run_mode: DaemonRunMode::ForegroundLoop,
            workspace_root: root.display().to_string(),
            project_name: "dev-daemon-error".into(),
            pid: process::id(),
            session_started_unix_ms: now_unix_ms(),
            last_heartbeat_unix_ms: now_unix_ms(),
            snapshot_file_count: 1,
            warmed_file_count: 0,
            warmed_version_count: 0,
            ast_decl_count: 0,
            hir_function_count: 0,
            mir_function_count: 0,
            last_error: Some("semantic error".into()),
            prewarm: DaemonPrewarmSummary::default(),
            last_diff: DaemonDiffSummary::default(),
        };
        write_daemon_status(&root, &status).expect("write daemon error status");

        let message = dev_daemon_status_message(&root).expect("format dev daemon message");
        assert!(message.contains("last warm refresh failed"));
        assert!(message.contains("semantic error"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_read_daemon_status_accepts_legacy_status_without_new_fields() {
        let root = temp_dir("daemon_legacy_status");
        ensure_daemon_status_dir(&root).expect("create daemon status dir");
        let path = daemon_status_path(&root);
        fs::write(
            &path,
            format!(
                concat!(
                    "{{",
                    "\"schema_version\":{},",
                    "\"workspace_root\":\"{}\",",
                    "\"project_name\":\"legacy\",",
                    "\"pid\":{},",
                    "\"session_started_unix_ms\":1,",
                    "\"last_heartbeat_unix_ms\":2,",
                    "\"snapshot_file_count\":3,",
                    "\"warmed_file_count\":2,",
                    "\"warmed_version_count\":2,",
                    "\"ast_decl_count\":4,",
                    "\"hir_function_count\":5,",
                    "\"mir_function_count\":6,",
                    "\"last_diff\":{{",
                    "\"added_files\":1,",
                    "\"changed_files\":0,",
                    "\"removed_files\":0,",
                    "\"unchanged_files\":2,",
                    "\"manifest_changed\":false",
                    "}}",
                    "}}"
                ),
                DAEMON_STATUS_SCHEMA_VERSION,
                root.display().to_string().replace('\\', "\\\\"),
                process::id()
            ),
        )
        .expect("write legacy status");

        let status = read_daemon_status(&root)
            .expect("read legacy daemon status")
            .expect("status should parse");
        assert_eq!(status.run_mode, DaemonRunMode::ForegroundLoop);
        assert_eq!(status.last_error, None);
        assert_eq!(status.prewarm, DaemonPrewarmSummary::default());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_run_daemon_foreground_once_persists_status_file() {
        let root = temp_dir("daemon_once_status");
        let file = root.join("main.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        run_daemon_foreground(
            Some(file.clone()),
            true,
            DAEMON_DEFAULT_POLL_MS,
            false,
            false,
        )
        .expect("one-shot daemon run should succeed");

        let status = read_daemon_status(&root)
            .expect("read daemon status")
            .expect("status file should exist after one-shot refresh");
        assert_eq!(status.run_mode, DaemonRunMode::OneShot);
        assert_eq!(status.last_error, None);
        assert_eq!(status.workspace_root, root.display().to_string());
        assert_eq!(status.warmed_file_count, 1);
        assert_eq!(status.snapshot_file_count, 1);
        assert!(status.prewarm.package_ready);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_prewarm_daemon_entry_artifacts_populates_cache() {
        let root = temp_dir("daemon_prewarm_cache");
        let file = root.join("main.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        let snapshot = agam_pkg::snapshot_workspace(Some(file.clone())).expect("snapshot");
        let mut session = DaemonSession::default();
        refresh_daemon_session(&mut session, snapshot.clone(), false).expect("warm snapshot");

        let summary = prewarm_daemon_entry_artifacts(&session, &snapshot, false);
        let cache = agam_runtime::cache::CacheStore::for_path(&root).expect("cache store");
        let status = cache.status(10).expect("cache status");
        let expected_backend = resolve_backend(Backend::Auto, true);

        assert!(summary.package_ready);
        assert_eq!(
            summary.build_backend.as_deref(),
            Some(render_backend_cli_value(expected_backend))
        );
        assert_eq!(summary.last_error, None);
        assert!(
            status.by_kind.iter().any(|kind| {
                kind.kind == agam_runtime::cache::CacheArtifactKind::PortablePackage
            })
        );
        if expected_backend == Backend::Jit {
            assert!(!summary.build_ready);
        } else {
            assert!(summary.build_ready);
            assert!(status.by_kind.iter().any(|kind| {
                matches!(
                    kind.kind,
                    agam_runtime::cache::CacheArtifactKind::NativeBinary
                        | agam_runtime::cache::CacheArtifactKind::LlvmIr
                        | agam_runtime::cache::CacheArtifactKind::CSource
                )
            }));
        }

        // Multi-file warm index should have been written
        assert!(summary.prewarmed_file_count > 0);
        assert_eq!(summary.prewarmed_total_files, 1); // single-file workspace

        let warm_index =
            agam_pkg::read_daemon_warm_index(&root).expect("reading warm index should succeed");
        assert!(
            warm_index.is_some(),
            "warm index should exist after prewarm"
        );
        let warm_index = warm_index.unwrap();
        assert_eq!(warm_index.files.len(), 1);

        // MIR artifacts are persisted in the prewarm directory for cross-process reuse
        let prewarm_dir = daemon_prewarm_stage_dir(&root);
        assert!(prewarm_dir.is_dir(), "prewarm directory should exist");
        let prewarm_entries: Vec<_> = fs::read_dir(&prewarm_dir)
            .expect("read prewarm dir")
            .collect();
        assert!(
            prewarm_entries.iter().any(|e| {
                e.as_ref()
                    .map(|e| e.file_name().to_string_lossy().contains("_mir_"))
                    .unwrap_or(false)
            }),
            "prewarm directory should contain MIR artifact(s)"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_daemon_prewarm_status_message_reports_missing_package_artifact() {
        let root = temp_dir("daemon_prewarm_status_missing_package");
        let missing_package = root.join("missing.agpkg.json");
        let summary = DaemonPrewarmSummary {
            package_ready: true,
            package_artifact_path: Some(missing_package.display().to_string()),
            build_backend: Some("jit".into()),
            ..DaemonPrewarmSummary::default()
        };

        let message =
            daemon_prewarm_status_message(&summary).expect("prewarm status message should exist");
        assert!(message.contains("package stale (artifact missing)"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_load_daemon_prewarmed_entry_reuses_matching_snapshot() {
        let root = temp_dir("daemon_prewarm_reuse");
        let file = root.join("main.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        run_daemon_foreground(
            Some(file.clone()),
            true,
            DAEMON_DEFAULT_POLL_MS,
            false,
            false,
        )
        .expect("one-shot daemon run should succeed");

        let prewarmed =
            load_daemon_prewarmed_entry(&file, false).expect("prewarmed entry should load");
        assert_eq!(prewarmed.package.mir.functions.len(), 1);
        assert_eq!(prewarmed.call_cache, CallCacheSelection::default());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_load_daemon_prewarmed_warm_state_reuses_matching_snapshot() {
        let root = temp_dir("daemon_prewarm_warm_state");
        let file = root.join("main.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        run_daemon_foreground(
            Some(file.clone()),
            true,
            DAEMON_DEFAULT_POLL_MS,
            false,
            false,
        )
        .expect("one-shot daemon run should succeed");

        let warm_state =
            load_daemon_prewarmed_warm_state(&file, false).expect("warm state should load");
        assert!(warm_state.module.is_none());
        assert!(warm_state.hir.is_none());
        assert_eq!(warm_state.mir.as_ref().expect("mir").functions.len(), 1);
        assert!(warm_state.source_features.is_some());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_load_daemon_warm_state_for_file_reuses_disk_call_cache_metadata() {
        let root = temp_dir("daemon_warm_disk_features");
        let file = root.join("main.agam");
        fs::write(
            &file,
            "@lang.advance\n@lang.feat.call_cache\nfn main() -> i32 { return 0; }\n",
        )
        .expect("write source");

        run_daemon_foreground(
            Some(file.clone()),
            true,
            DAEMON_DEFAULT_POLL_MS,
            false,
            false,
        )
        .expect("one-shot daemon run should succeed");

        let warm_state =
            load_daemon_warm_state_for_file(&file, false).expect("warm state should load");
        assert_eq!(warm_state.mir.as_ref().expect("mir").functions.len(), 1);
        let source_features = warm_state
            .source_features
            .as_ref()
            .expect("disk warm state should carry source features");
        assert!(source_features.call_cache.enable_all);
        assert!(!source_features.call_cache.disable_all);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_load_daemon_prewarmed_entry_rejects_hash_mismatch() {
        let root = temp_dir("daemon_prewarm_hash_mismatch");
        let file = root.join("main.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        run_daemon_foreground(
            Some(file.clone()),
            true,
            DAEMON_DEFAULT_POLL_MS,
            false,
            false,
        )
        .expect("one-shot daemon run should succeed");
        fs::write(&file, "fn main() -> i32 { return 1; }\n").expect("rewrite source");

        assert!(load_daemon_prewarmed_entry(&file, false).is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_compile_dev_source_file_prefers_daemon_prewarm_for_run() {
        let root = temp_dir("compile_dev_daemon_prewarm");
        let file = root.join("main.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        run_daemon_foreground(
            Some(file.clone()),
            true,
            DAEMON_DEFAULT_POLL_MS,
            false,
            false,
        )
        .expect("one-shot daemon run should succeed");

        let warm =
            compile_dev_source_file(&file, true, false).expect("warm dev compile should work");
        let warm = warm.expect("warm state should be retained for runnable entry file");
        assert!(warm.module.is_none());
        assert!(warm.hir.is_none());
        assert_eq!(warm.mir.as_ref().expect("mir").functions.len(), 1);
        assert!(warm.source_features.is_some());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_compile_dev_source_file_rebuilds_when_disk_warm_state_lacks_source_features() {
        let root = temp_dir("compile_dev_incomplete_disk_warm_state");
        let file = root.join("main.agam");
        fs::write(
            &file,
            "@lang.advance\n@lang.feat.call_cache\nfn main() -> i32 { return 0; }\n",
        )
        .expect("write source");

        let warm_state =
            compile_file_with_warm_state(&file, false).expect("warm compile should succeed");
        let mir = warm_state.mir.as_ref().expect("mir should exist");
        let content_hash = agam_runtime::cache::hash_bytes(
            &fs::read(&file).expect("read source for content hash"),
        );
        let artifact_path =
            daemon_prewarm_mir_artifact_path(&root, &file).expect("artifact path should resolve");
        if let Some(parent) = artifact_path.parent() {
            fs::create_dir_all(parent).expect("create artifact dir");
        }
        let raw_mir_json = serde_json::to_vec(mir).expect("serialize legacy raw MIR artifact");
        fs::write(&artifact_path, raw_mir_json).expect("write legacy raw MIR artifact");
        agam_pkg::write_daemon_warm_index(
            &root,
            &agam_pkg::DaemonWarmIndex {
                format_version: agam_pkg::DAEMON_WARM_INDEX_FORMAT_VERSION,
                files: BTreeMap::from([(
                    file.display().to_string(),
                    agam_pkg::DaemonWarmFileEntry {
                        content_hash,
                        mir_hash: Some(
                            agam_runtime::cache::hash_serializable(mir)
                                .expect("hash legacy raw MIR artifact"),
                        ),
                        artifact_path: Some(artifact_path.display().to_string()),
                        warm_level: agam_pkg::DaemonWarmLevel::Lowered,
                    },
                )]),
            },
        )
        .expect("write daemon warm index");

        let warm = compile_dev_source_file(&file, true, false).expect("dev compile should succeed");
        let warm = warm.expect("warm state should be rebuilt locally");
        assert!(
            warm.module.is_some(),
            "incomplete disk warm state should not be reused for runnable dev flows"
        );
        assert!(warm.hir.is_some());
        assert!(warm.source_features.is_some());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_run_daemon_foreground_once_persists_error_status_on_failure() {
        let root = temp_dir("daemon_once_error");
        let file = root.join("broken.agam");
        fs::write(&file, "fn main(): missing_name\n").expect("write invalid source");

        let error = run_daemon_foreground(
            Some(file.clone()),
            true,
            DAEMON_DEFAULT_POLL_MS,
            false,
            false,
        )
        .expect_err("one-shot daemon run should fail");
        assert!(error.contains("semantic error"));

        let status = read_daemon_status(&root)
            .expect("read daemon error status")
            .expect("status file should exist after one-shot failure");
        assert_eq!(status.run_mode, DaemonRunMode::OneShot);
        assert_eq!(status.warmed_file_count, 0);
        assert!(
            status
                .last_error
                .as_ref()
                .expect("last error should exist")
                .contains("semantic error")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_run_daemon_cycle_recovers_after_transient_semantic_error() {
        let root = temp_dir("daemon_cycle_recovery");
        let file = root.join("main.agam");
        fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

        let initial_snapshot = agam_pkg::snapshot_workspace(Some(file.clone())).expect("snapshot");
        let workspace = initial_snapshot.session.layout.clone();
        let session_started_unix_ms = now_unix_ms();
        let mut session = DaemonSession::default();

        let first = run_daemon_cycle(
            &mut session,
            &daemon_refresh_snapshot_hint(&workspace),
            &initial_snapshot,
            session_started_unix_ms,
            DaemonRunMode::ForegroundLoop,
            false,
            true,
        )
        .expect("first daemon cycle should succeed");
        let (first_status, first_diff) = match first {
            DaemonCycleOutcome::Success {
                status,
                diff_summary,
                ..
            } => (status, diff_summary),
            DaemonCycleOutcome::Error { error, .. } => {
                panic!("unexpected daemon error on first cycle: {error}")
            }
        };
        assert_eq!(first_status.last_error, None);
        assert_eq!(first_status.warmed_file_count, 1);
        assert_eq!(first_diff.added_files, 1);

        fs::write(&file, "fn main(): missing_name\n").expect("write broken source");
        let second = run_daemon_cycle(
            &mut session,
            &daemon_refresh_snapshot_hint(&workspace),
            &initial_snapshot,
            session_started_unix_ms,
            DaemonRunMode::ForegroundLoop,
            false,
            false,
        )
        .expect("second daemon cycle should return an error status");
        let (second_status, second_error) = match second {
            DaemonCycleOutcome::Error { status, error } => (status, error),
            DaemonCycleOutcome::Success { .. } => {
                panic!("second daemon cycle should have failed");
            }
        };
        assert!(!second_error.is_empty());
        assert_eq!(
            second_status.last_error.as_deref(),
            Some(second_error.as_str())
        );
        assert_eq!(second_status.warmed_file_count, 1);
        assert_eq!(second_status.warmed_version_count, 1);

        fs::write(&file, "fn main(): println(\"recovered\")\n").expect("rewrite fixed source");
        let third = run_daemon_cycle(
            &mut session,
            &daemon_refresh_snapshot_hint(&workspace),
            &initial_snapshot,
            session_started_unix_ms,
            DaemonRunMode::ForegroundLoop,
            false,
            false,
        )
        .expect("third daemon cycle should recover");
        let (third_status, third_diff) = match third {
            DaemonCycleOutcome::Success {
                status,
                diff_summary,
                ..
            } => (status, diff_summary),
            DaemonCycleOutcome::Error { error, .. } => {
                panic!("daemon cycle should have recovered: {error}")
            }
        };
        assert_eq!(third_status.last_error, None);
        assert_eq!(third_status.warmed_file_count, 1);
        assert_eq!(third_diff.changed_files, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_run_daemon_cycle_rewarms_missing_package_artifact() {
        let root = temp_dir("daemon_cycle_missing_prewarm");
        let file = root.join("main.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source");

        let initial_snapshot = agam_pkg::snapshot_workspace(Some(file.clone())).expect("snapshot");
        let workspace = initial_snapshot.session.layout.clone();
        let session_started_unix_ms = now_unix_ms();
        let mut session = DaemonSession::default();

        let first = run_daemon_cycle(
            &mut session,
            &daemon_refresh_snapshot_hint(&workspace),
            &initial_snapshot,
            session_started_unix_ms,
            DaemonRunMode::ForegroundLoop,
            false,
            true,
        )
        .expect("first daemon cycle should succeed");
        let first_status = match first {
            DaemonCycleOutcome::Success { status, .. } => status,
            DaemonCycleOutcome::Error { error, .. } => {
                panic!("unexpected daemon error on first cycle: {error}")
            }
        };
        let package_artifact = PathBuf::from(
            first_status
                .prewarm
                .package_artifact_path
                .clone()
                .expect("package artifact path should exist"),
        );
        assert!(package_artifact.is_file());

        fs::remove_file(&package_artifact).expect("remove daemon prewarm package artifact");
        assert!(!package_artifact.exists());

        let second = run_daemon_cycle(
            &mut session,
            &daemon_refresh_snapshot_hint(&workspace),
            &initial_snapshot,
            session_started_unix_ms,
            DaemonRunMode::ForegroundLoop,
            false,
            false,
        )
        .expect("second daemon cycle should succeed");
        let (second_status, second_diff, prewarm_ran) = match second {
            DaemonCycleOutcome::Success {
                status,
                diff_summary,
                prewarm_ran,
            } => (status, diff_summary, prewarm_ran),
            DaemonCycleOutcome::Error { error, .. } => {
                panic!("daemon cycle should have rewarmed missing package artifact: {error}")
            }
        };
        assert!(prewarm_ran);
        assert_eq!(second_diff.changed_files, 0);
        assert!(second_status.prewarm.package_ready);
        let rerwarmed_artifact = PathBuf::from(
            second_status
                .prewarm
                .package_artifact_path
                .expect("package artifact path should be restored"),
        );
        assert!(rerwarmed_artifact.is_file());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_clear_daemon_status_removes_persisted_status_file() {
        let root = temp_dir("daemon_clear_status");
        let file = root.join("main.agam");
        fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

        run_daemon_foreground(
            Some(file.clone()),
            true,
            DAEMON_DEFAULT_POLL_MS,
            false,
            false,
        )
        .expect("one-shot daemon run should succeed");
        assert!(daemon_status_path(&root).is_file());

        clear_daemon_status(Some(file), false).expect("clear daemon status should succeed");
        assert!(!daemon_status_path(&root).exists());
        assert!(
            read_daemon_status(&root)
                .expect("read cleared daemon status")
                .is_none()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_resolve_daemon_workspace_target_allows_missing_source_hint() {
        let root = temp_dir("daemon_missing_source_hint");
        let file = root.join("main.agam");
        fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

        let layout =
            resolve_workspace_layout(Some(file.clone())).expect("existing source should resolve");
        fs::remove_file(&file).expect("remove source");

        let daemon_target = resolve_daemon_workspace_target(Some(file))
            .expect("daemon target should resolve from missing source parent");
        assert_eq!(daemon_target.root, layout.root);
        assert_eq!(daemon_target.project_name, layout.project_name);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_resolve_daemon_workspace_target_allows_root_dir_with_status_but_no_entry() {
        let root = temp_dir("daemon_root_status_hint");
        let file = root.join("main.agam");
        fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

        run_daemon_foreground(
            Some(file.clone()),
            true,
            DAEMON_DEFAULT_POLL_MS,
            false,
            false,
        )
        .expect("one-shot daemon run should succeed");
        fs::remove_file(&file).expect("remove source");

        let daemon_target = resolve_daemon_workspace_target(Some(root.clone()))
            .expect("daemon target should resolve from root with persisted status");
        assert_eq!(daemon_target.root, root);

        let _ = fs::remove_dir_all(daemon_target.root);
    }

    #[test]
    fn test_resolve_daemon_workspace_target_allows_existing_directory_without_workspace_layout() {
        let root = temp_dir("daemon_existing_dir_hint");

        let daemon_target = resolve_daemon_workspace_target(Some(root.clone()))
            .expect("daemon target should resolve from an existing directory");
        assert_eq!(daemon_target.root, root);

        let _ = fs::remove_dir_all(daemon_target.root);
    }

    #[test]
    fn test_daemon_refresh_snapshot_hint_uses_entry_file_for_single_file_workspace() {
        let root = temp_dir("daemon_refresh_hint_single_file");
        let file = root.join("main.agam");
        fs::write(&file, "fn main(): println(\"hi\")\n").expect("write source");

        let layout = resolve_workspace_layout(Some(file.clone()))
            .expect("single-file workspace should resolve");
        assert_eq!(daemon_refresh_snapshot_hint(&layout), file);

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
    fn test_persisted_profile_builds_specialization_plans_for_cache_enabled_functions() {
        let profile = agam_profile::PersistentCallCacheProfile {
            schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 0,
            total_stores: 2,
            functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
                name: "hot".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 0,
                total_stores: 1,
                last_entries: 1,
                profile: agam_profile::CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 32,
                    avg_reuse_distance: None,
                    max_reuse_distance: None,
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

        let (selection, promoted) =
            apply_persisted_optimize_profile(&CallCacheSelection::default(), Some(&profile));
        let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

        assert!(promoted.is_empty());
        assert!(selection.optimize_functions.is_empty());
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].name, "hot");
        assert_eq!(plans[0].stable_values.len(), 2);
        assert_eq!(plans[1].stable_values.len(), 1);
        assert_eq!(plans[1].stable_values[0].raw_bits, 33);
    }

    #[test]
    fn test_persisted_profile_builds_specialization_plans_for_explicit_basic_selection() {
        let profile = agam_profile::PersistentCallCacheProfile {
            schema_version: agam_profile::CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 0,
            total_stores: 2,
            functions: vec![agam_profile::PersistentCallCacheFunctionProfile {
                name: "hot".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 0,
                total_stores: 1,
                last_entries: 1,
                profile: agam_profile::CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 32,
                    avg_reuse_distance: None,
                    max_reuse_distance: None,
                    stable_values: vec![agam_profile::StableScalarValueProfile {
                        index: 0,
                        raw_bits: 33,
                        matches: 24,
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

        let selection = CallCacheSelection {
            disable_all: true,
            include_functions: ["hot".to_string()].into_iter().collect(),
            ..Default::default()
        };
        let (selection, promoted) = apply_persisted_optimize_profile(&selection, Some(&profile));
        let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

        assert!(promoted.is_empty());
        assert!(selection.optimize_functions.is_empty());
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "hot");
        assert_eq!(plans[0].stable_values[0].raw_bits, 33);
    }

    #[test]
    fn test_persisted_profile_skips_specialization_plans_when_cache_disabled() {
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

        let selection = CallCacheSelection {
            disable_all: true,
            ..Default::default()
        };
        let plans = apply_persisted_specialization_profile(&selection, Some(&profile));

        assert!(plans.is_empty());
    }

    #[test]
    fn test_parse_llvm_call_cache_run_profile() {
        let profile = parse_llvm_call_cache_run_profile(
            "AGAM_LLVM_CALL_CACHE_PROFILE_V6\nFN\thot\t32\t24\t2\t1\t3\t24\nSP\thot\t12\t4\nSC\thot\t0=33\t12\t0\nSC\thot\t0=33,1=7\t0\t4\nSV\thot\t0\t33\t24\nRD\thot\t1\t3\t24\nFN\twarm\t8\t0\t0\t0\t0\t0\nSP\twarm\t0\t0\nSV\twarm\t0\t7\t0\nRD\twarm\t0\t0\t0\n",
        )
        .expect("profile should parse");

        assert_eq!(profile.backend, "llvm");
        assert_eq!(profile.total_calls, 40);
        assert_eq!(profile.total_hits, 24);
        assert_eq!(profile.total_stores, 2);
        assert_eq!(profile.functions.len(), 2);
        assert_eq!(profile.functions[0].name, "hot");
        assert_eq!(profile.functions[0].entries, 1);
        assert_eq!(profile.functions[0].profile.unique_keys, 3);
        assert_eq!(profile.functions[0].profile.hottest_key_hits, 24);
        assert_eq!(profile.functions[0].profile.stable_values.len(), 1);
        assert_eq!(profile.functions[0].profile.stable_values[0].raw_bits, 33);
        assert_eq!(profile.functions[0].profile.avg_reuse_distance, Some(1));
        assert_eq!(profile.functions[0].profile.max_reuse_distance, Some(3));
        assert_eq!(profile.functions[0].profile.specialization_guard_hits, 12);
        assert_eq!(
            profile.functions[0].profile.specialization_guard_fallbacks,
            4
        );
        assert_eq!(
            profile.functions[0].profile.specialization_profiles.len(),
            2
        );
        assert_eq!(
            profile.functions[0].profile.specialization_profiles[0].stable_values[0].index,
            0
        );
        assert_eq!(
            profile.functions[0].profile.specialization_profiles[0].stable_values[0].raw_bits,
            33
        );
    }

    #[test]
    fn test_parse_llvm_call_cache_run_profile_v4_compatibility() {
        let profile = parse_llvm_call_cache_run_profile(
            "AGAM_LLVM_CALL_CACHE_PROFILE_V4\nFN\thot\t32\t24\t2\t1\nSP\thot\t12\t4\nSV\thot\t0\t33\t24\nRD\thot\t1\t3\t24\n",
        )
        .expect("legacy profile should parse");

        assert_eq!(profile.functions.len(), 1);
        assert_eq!(profile.functions[0].profile.unique_keys, 2);
        assert_eq!(profile.functions[0].profile.hottest_key_hits, 0);
        assert_eq!(profile.functions[0].profile.avg_reuse_distance, Some(1));
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
                    specialization_profiles: Vec::new(),
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
                    specialization_profiles: Vec::new(),
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
                    specialization_profiles: Vec::new(),
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
        let targets = sdk_supported_targets(None, None);
        assert!(!targets.is_empty());
        assert_eq!(targets[0].name, "host-native");
        assert_eq!(
            targets[0].backend,
            agam_runtime::contract::RuntimeBackend::Llvm
        );
    }

    #[test]
    fn test_sdk_supported_targets_record_packaged_android_sysroot() {
        let targets = sdk_supported_targets(None, Some("target-packs/android-arm64/sysroot"));
        let android = targets
            .iter()
            .find(|target| target.target_triple == "aarch64-linux-android21")
            .expect("default SDK target list should include android");
        assert_eq!(
            android.packaged_sysroot.as_deref(),
            Some("target-packs/android-arm64/sysroot")
        );
    }

    #[test]
    fn test_requested_backend_for_command_uses_llvm_when_target_is_selected() {
        let requested = requested_backend_for_command(
            Backend::Auto,
            None,
            false,
            Some("aarch64-linux-android21"),
        );
        assert_eq!(requested, Backend::Llvm);
    }

    #[test]
    fn test_requested_backend_from_environment_ignores_jit_for_build() {
        let environment = environment_report(
            "dev",
            None,
            Some(agam_runtime::contract::RuntimeBackend::Jit),
        );
        assert_eq!(
            requested_backend_from_environment(&environment.environment, false),
            None
        );
        assert_eq!(
            requested_backend_from_environment(&environment.environment, true),
            Some(Backend::Jit)
        );
    }

    #[test]
    fn test_sdk_supported_targets_include_selected_environment_target() {
        let environment = environment_report(
            "release-linux",
            Some("x86_64-unknown-linux-musl"),
            Some(agam_runtime::contract::RuntimeBackend::Llvm),
        );

        let targets = sdk_supported_targets(Some(&environment), None);
        assert!(targets.iter().any(|target| {
            target.name == "release-linux"
                && target.target_triple == "x86_64-unknown-linux-musl"
                && target.backend == agam_runtime::contract::RuntimeBackend::Llvm
        }));
    }

    #[test]
    fn test_package_sdk_distribution_records_selected_environment_metadata() {
        let root = temp_dir("sdk_env_metadata");
        let output = root.join("dist");
        let environment = environment_report(
            "release-linux",
            Some("x86_64-unknown-linux-musl"),
            Some(agam_runtime::contract::RuntimeBackend::Llvm),
        );

        let outcome = package_sdk_distribution(&output, None, None, Some(&environment), false)
            .expect("package sdk should succeed");
        let manifest = agam_pkg::read_sdk_distribution_manifest_from_path(&outcome.manifest_path)
            .expect("read sdk manifest");

        assert!(manifest.notes.iter().any(|note| {
            note.contains("selected environment `release-linux`")
                && note.contains("target `x86_64-unknown-linux-musl`")
                && note.contains("backend `llvm`")
        }));
        assert!(manifest.supported_targets.iter().any(|target| {
            target.name == "release-linux"
                && target.target_triple == "x86_64-unknown-linux-musl"
                && target.backend == agam_runtime::contract::RuntimeBackend::Llvm
        }));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_package_sdk_distribution_stages_android_target_pack() {
        let root = temp_dir("sdk_android_target_pack");
        let output = root.join("dist");
        let sysroot = root.join("android-sysroot");
        fs::create_dir_all(sysroot.join("usr").join("include"))
            .expect("create synthetic android sysroot");

        let outcome = package_sdk_distribution(&output, None, Some(&sysroot), None, false)
            .expect("package sdk should accept an explicit android sysroot");
        let manifest = agam_pkg::read_sdk_distribution_manifest_from_path(&outcome.manifest_path)
            .expect("read sdk manifest");
        let expected_sysroot = output
            .join("target-packs")
            .join("android-arm64")
            .join("sysroot");
        let android = manifest
            .supported_targets
            .iter()
            .find(|target| target.target_triple == "aarch64-linux-android21")
            .expect("manifest should include android target support");
        assert_eq!(
            android.packaged_sysroot.as_deref(),
            Some("target-packs/android-arm64/sysroot")
        );
        assert!(
            expected_sysroot.join("usr").is_dir(),
            "staged SDK should include the Android sysroot target pack"
        );
        assert_eq!(
            outcome.android_sysroot_root.as_deref(),
            Some(expected_sysroot.as_path())
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_sanitize_headless_filename_keeps_single_file_name() {
        assert_eq!(
            sanitize_headless_filename("../tmp\\demo script"),
            "demo_script.agam"
        );
        assert_eq!(sanitize_headless_filename("session.agam"), "session.agam");
    }

    #[test]
    fn test_repl_execution_cache_reuses_warm_state_for_unchanged_source() {
        let request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            filename: "demo.agam".into(),
            backend: HeadlessExecutionBackend::Jit,
            ..HeadlessExecutionRequest::default()
        };
        let mut cache = ReplExecutionCache::new(&request.filename)
            .expect("repl execution cache should initialize");

        let first_ptr = {
            cache
                .materialize_request(&request)
                .expect("request materialization should succeed");
            let warm = cache
                .ensure_materialized_warm_state(false)
                .expect("first warm state build should succeed");
            warm as *const WarmState as usize
        };
        let second_ptr = {
            cache
                .materialize_request(&request)
                .expect("second request materialization should succeed");
            let warm = cache
                .ensure_materialized_warm_state(false)
                .expect("second warm state lookup should succeed");
            warm as *const WarmState as usize
        };

        assert_eq!(
            first_ptr, second_ptr,
            "unchanged REPL buffers should reuse the cached warm state"
        );
    }

    #[test]
    fn test_repl_execution_cache_invalidates_warm_state_when_source_changes() {
        let mut request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            filename: "demo.agam".into(),
            backend: HeadlessExecutionBackend::Jit,
            ..HeadlessExecutionRequest::default()
        };
        let mut cache = ReplExecutionCache::new(&request.filename)
            .expect("repl execution cache should initialize");

        cache
            .materialize_request(&request)
            .expect("first request materialization should succeed");
        let first_hash = cache
            .source_hash
            .clone()
            .expect("materialized request should record a source hash");
        cache
            .ensure_materialized_warm_state(false)
            .expect("first warm state build should succeed");
        assert!(
            cache
                .daemon_session
                .cache
                .get(cache.source_path())
                .expect("daemon cache for REPL entry")
                .contains_key(&first_hash)
        );

        request.source = "fn main() -> i32 { return 1; }\n".into();

        cache
            .materialize_request(&request)
            .expect("changed request materialization should succeed");
        let second_hash = cache
            .source_hash
            .clone()
            .expect("updated request should record a source hash");

        assert_ne!(
            first_hash, second_hash,
            "changed REPL buffers should update the cached source hash"
        );
        assert_eq!(
            std::fs::read_to_string(cache.source_path()).expect("read materialized REPL source"),
            request.source
        );
        cache
            .ensure_materialized_warm_state(false)
            .expect("changed warm state build should succeed");
        let versions = cache
            .daemon_session
            .cache
            .get(cache.source_path())
            .expect("daemon cache for changed REPL entry");
        assert!(
            versions.contains_key(&second_hash),
            "changed REPL buffers should warm the new source hash"
        );
        assert!(
            !versions.contains_key(&first_hash),
            "changed REPL buffers should invalidate the previous daemon warm state version"
        );
    }

    #[test]
    fn test_repl_execution_cache_updates_manifest_when_filename_changes() {
        let mut cache =
            ReplExecutionCache::new("demo.agam").expect("repl execution cache should initialize");
        let previous_source_path = cache.source_path().clone();

        let request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            filename: "renamed.agam".into(),
            backend: HeadlessExecutionBackend::Jit,
            ..HeadlessExecutionRequest::default()
        };

        cache
            .materialize_request(&request)
            .expect("renamed request materialization should succeed");

        let manifest = agam_pkg::read_workspace_manifest_from_path(&cache.manifest_path)
            .expect("read REPL workspace manifest");
        assert_eq!(
            manifest.project.entry.as_deref(),
            Some("src/renamed.agam"),
            "REPL manifest should track the current buffer filename"
        );
        assert_eq!(
            cache.source_path(),
            &cache.root.join("src").join("renamed.agam")
        );
        assert!(
            !previous_source_path.exists(),
            "renaming the REPL buffer should remove the stale source path"
        );
    }

    #[test]
    fn test_execute_repl_request_runs_in_process() {
        let request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            filename: "demo.agam".into(),
            backend: HeadlessExecutionBackend::Jit,
            ..HeadlessExecutionRequest::default()
        };
        let mut cache = ReplExecutionCache::new(&request.filename)
            .expect("repl execution cache should initialize");

        let exit_code =
            execute_repl_request(&request, &mut cache, false).expect("REPL request should run");
        assert_eq!(exit_code, 0);
    }

    #[test]
    fn test_execute_headless_request_runs_jit_in_process_and_captures_stdout() {
        let request = HeadlessExecutionRequest {
            source: "fn main(): println(\"hi\")\n".into(),
            filename: "snippet.agam".into(),
            backend: HeadlessExecutionBackend::Jit,
            ..HeadlessExecutionRequest::default()
        };

        let response = execute_headless_request(&request, false);
        assert!(
            response.success,
            "expected successful headless response: {response:?}"
        );
        assert_eq!(response.exit_code, Some(0));
        assert_eq!(response.stdout, "hi\n");
        assert!(response.stderr.is_empty());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_execute_headless_request_buffers_jit_parse_errors_into_stderr() {
        let request = HeadlessExecutionRequest {
            source: "fn main(".into(),
            filename: "broken.agam".into(),
            backend: HeadlessExecutionBackend::Jit,
            ..HeadlessExecutionRequest::default()
        };

        let response = execute_headless_request(&request, false);
        assert!(!response.success);
        assert!(response.exit_code.is_none());
        assert!(response.stdout.is_empty());
        assert!(
            response.stderr.contains("error"),
            "expected rendered parse diagnostics in stderr: {response:?}"
        );
        assert!(response.error.is_some());
    }

    #[test]
    fn test_execute_headless_request_runs_available_non_jit_backend_in_process() {
        let backend = if resolve_llvm_run_toolchain().is_some() {
            HeadlessExecutionBackend::Llvm
        } else if command_exists(default_c_compiler()) {
            HeadlessExecutionBackend::C
        } else {
            return;
        };
        let request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            filename: "native.agam".into(),
            backend,
            policy: HeadlessExecutionPolicy {
                allow_native_backends: true,
                ..HeadlessExecutionPolicy::default()
            },
            ..HeadlessExecutionRequest::default()
        };

        let response = execute_headless_request(&request, false);
        assert!(
            response.success,
            "expected successful non-JIT headless response: {response:?}"
        );
        assert_eq!(response.exit_code, Some(0));
        assert!(response.stdout.is_empty());
        assert!(response.stderr.is_empty());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_build_exec_request_from_inline_source_uses_cli_options() {
        let request = build_exec_request(
            None,
            Some("fn main() -> i32 { return 0; }\n".into()),
            Some("agent.agam".into()),
            Backend::Llvm,
            3,
            true,
            vec!["hello".into(), "world".into()],
            "process".into(),
            false,
            false,
        )
        .expect("inline exec request should build");

        assert_eq!(request.filename, "agent.agam");
        assert_eq!(request.source, "fn main() -> i32 { return 0; }\n");
        assert_eq!(request.backend, HeadlessExecutionBackend::Llvm);
        assert_eq!(request.opt_level, 3);
        assert!(request.fast);
        assert_eq!(request.args, vec!["hello".to_string(), "world".to_string()]);
        assert!(request.policy.allow_native_backends);
    }

    #[test]
    fn test_build_exec_request_from_file_reads_source_and_defaults_filename() {
        let root = temp_dir("exec_request_file");
        let file = root.join("demo.agam");
        fs::write(&file, "fn main() -> i32 { return 0; }\n").expect("write source file");

        let request = build_exec_request(
            Some(file.clone()),
            None,
            None,
            Backend::Jit,
            2,
            false,
            Vec::new(),
            "process".into(),
            false,
            false,
        )
        .expect("file exec request should build");

        assert_eq!(request.filename, "demo.agam");
        assert_eq!(request.source, "fn main() -> i32 { return 0; }\n");
        assert_eq!(request.backend, HeadlessExecutionBackend::Jit);
        assert!(!request.policy.allow_native_backends);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_normalize_headless_request_rejects_source_over_policy_limit() {
        let request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            policy: HeadlessExecutionPolicy {
                max_source_bytes: 8,
                ..HeadlessExecutionPolicy::default()
            },
            ..HeadlessExecutionRequest::default()
        };

        let error =
            normalize_headless_request(&request).expect_err("oversized source should be rejected");
        assert!(error.contains("exceeding the policy limit"));
    }

    #[test]
    fn test_normalize_headless_request_rejects_too_many_args() {
        let request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            args: vec!["alpha".into(), "beta".into()],
            policy: HeadlessExecutionPolicy {
                max_arg_count: 1,
                ..HeadlessExecutionPolicy::default()
            },
            ..HeadlessExecutionRequest::default()
        };

        let error = normalize_headless_request(&request)
            .expect_err("requests exceeding the arg-count policy should be rejected");
        assert!(error.contains("exceeding the policy limit"));
    }

    #[test]
    fn test_normalize_headless_request_rejects_native_backend_without_policy_opt_in() {
        let request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            backend: HeadlessExecutionBackend::Llvm,
            ..HeadlessExecutionRequest::default()
        };

        let error = normalize_headless_request(&request)
            .expect_err("native backend should require explicit policy opt-in");
        assert!(error.contains("policy.allow_native_backends=true"));
    }

    #[test]
    fn test_normalize_headless_request_rejects_zero_runtime_limit() {
        let request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            policy: HeadlessExecutionPolicy {
                max_runtime_ms: 0,
                ..HeadlessExecutionPolicy::default()
            },
            ..HeadlessExecutionRequest::default()
        };

        let error = normalize_headless_request(&request)
            .expect_err("zero runtime limit should be rejected");
        assert!(error.contains("max_runtime_ms"));
    }

    #[test]
    fn test_normalize_headless_request_rejects_zero_memory_limit() {
        let request = HeadlessExecutionRequest {
            source: "fn main() -> i32 { return 0; }\n".into(),
            policy: HeadlessExecutionPolicy {
                max_memory_bytes: 0,
                ..HeadlessExecutionPolicy::default()
            },
            ..HeadlessExecutionRequest::default()
        };

        let error =
            normalize_headless_request(&request).expect_err("zero memory limit should be rejected");
        assert!(error.contains("max_memory_bytes"));
    }

    #[test]
    fn test_cli_parses_exec_command_with_inline_source() {
        let cli = Cli::try_parse_from([
            "agamc",
            "exec",
            "--source",
            "fn main() -> i32 { return 0; }",
            "--backend",
            "jit",
            "--arg",
            "alpha",
        ])
        .expect("exec command should parse");

        match cli.command {
            Command::Exec {
                json,
                pretty,
                source,
                backend,
                args,
                ..
            } => {
                assert!(!json);
                assert!(!pretty);
                assert_eq!(source.as_deref(), Some("fn main() -> i32 { return 0; }"));
                assert_eq!(backend, Backend::Jit);
                assert_eq!(args, vec!["alpha".to_string()]);
            }
            other => panic!("expected exec command, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_repl_command_understands_backend_and_fast_commands() {
        assert_eq!(
            parse_repl_command(":backend llvm").expect("parse backend"),
            Some(ReplCommandKind::Backend(HeadlessExecutionBackend::Llvm))
        );
        assert_eq!(
            parse_repl_command(":fast on").expect("parse fast"),
            Some(ReplCommandKind::Fast(true))
        );
        assert_eq!(
            parse_repl_command(":opt 2").expect("parse opt"),
            Some(ReplCommandKind::Opt(2))
        );
    }

    #[test]
    fn test_parse_repl_command_rejects_unknown_commands() {
        let error = parse_repl_command(":wat").expect_err("unknown repl commands should fail");
        assert!(error.contains("unknown repl command"));
    }

    #[test]
    fn test_optional_workspace_environment_allows_missing_path_without_env() {
        let resolved = maybe_resolve_optional_workspace_environment(None, None)
            .expect("missing workspace path should be allowed when no env was requested");
        assert!(resolved.is_none());
    }

    #[test]
    fn test_optional_workspace_environment_allows_non_workspace_path_without_env() {
        let root = temp_dir("sdk_optional_env_free");
        let resolved = maybe_resolve_optional_workspace_environment(Some(root.clone()), None)
            .expect("non-workspace path should be allowed when no env was requested");
        assert!(resolved.is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_optional_workspace_environment_requires_workspace_when_env_is_requested() {
        let error = maybe_resolve_optional_workspace_environment(None, Some("release"))
            .expect_err("selecting an environment should require a workspace");
        assert!(error.contains("`--env` requires a workspace"));
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

    #[test]
    fn test_publish_workspace_to_registry_writes_local_index_entry() {
        let root = temp_dir("publish_workspace");
        let workspace = root.join("workspace");
        let src = workspace.join("src");
        let entry = src.join("main.agam");
        fs::create_dir_all(&src).expect("create source directory");
        fs::write(&entry, render_project_entry("publish-demo")).expect("write entry source");

        let mut manifest = agam_pkg::scaffold_workspace_manifest("publish-demo");
        manifest.project.keywords = vec!["math".into(), "ml".into()];
        manifest.dependencies.insert(
            "core".into(),
            agam_pkg::DependencySpec {
                version: Some("^1.0".into()),
                optional: true,
                features: vec!["simd".into()],
                ..agam_pkg::DependencySpec::default()
            },
        );
        agam_pkg::write_workspace_manifest_to_path(
            &agam_pkg::default_manifest_path(&workspace),
            &manifest,
        )
        .expect("write manifest");

        let index_root = root.join("registry-index");
        let report = publish_workspace_to_registry(
            Some(workspace.clone()),
            &index_root,
            &["alice".into(), " bob ".into(), "alice".into()],
            Some(&"Sample package".to_string()),
            Some(&"https://example.com/publish-demo".to_string()),
            Some(&"https://github.com/agam-lang/publish-demo".to_string()),
            Some(&"https://cdn.example.com/publish-demo-0.1.0.agam-src.tar.gz".to_string()),
            false,
            false,
            false,
        )
        .expect("publish should succeed");

        assert!(!report.dry_run);
        assert!(!report.official);
        assert!(report.bootstrapped_config);
        assert_eq!(report.owners, vec!["alice".to_string(), "bob".to_string()]);
        assert!(report.receipt.is_some());
        assert!(index_root.join("config.json").is_file());

        let config = agam_pkg::read_registry_config(&index_root).expect("read registry config");
        assert_eq!(
            config.format_version,
            agam_pkg::REGISTRY_INDEX_FORMAT_VERSION
        );

        let entry =
            agam_pkg::read_registry_package_entry(&index_root, "publish-demo").expect("read entry");
        assert_eq!(entry.owners, vec!["alice".to_string(), "bob".to_string()]);
        assert_eq!(entry.description.as_deref(), Some("Sample package"));
        assert_eq!(
            entry.homepage.as_deref(),
            Some("https://example.com/publish-demo")
        );
        assert_eq!(
            entry.repository.as_deref(),
            Some("https://github.com/agam-lang/publish-demo")
        );
        assert_eq!(entry.keywords, vec!["math".to_string(), "ml".to_string()]);
        assert_eq!(entry.releases.len(), 1);
        assert_eq!(entry.releases[0].dependencies.len(), 1);
        assert_eq!(entry.releases[0].dependencies[0].name, "core");
        assert_eq!(entry.releases[0].dependencies[0].version_req, "^1.0");
        assert!(entry.releases[0].dependencies[0].optional);
        assert_eq!(
            entry.releases[0].dependencies[0].features,
            vec!["simd".to_string()]
        );
        assert_eq!(
            entry.releases[0].download_url.as_deref(),
            Some("https://cdn.example.com/publish-demo-0.1.0.agam-src.tar.gz")
        );
        let provenance = entry.releases[0]
            .provenance
            .as_ref()
            .expect("publish should record provenance");
        assert_eq!(provenance.published_by.as_deref(), Some("alice"));
        assert_eq!(
            provenance.source_repository.as_deref(),
            Some("https://github.com/agam-lang/publish-demo")
        );
        assert_eq!(provenance.source_checksum, report.manifest.checksum);
        assert_eq!(
            provenance.manifest_checksum,
            report.manifest.manifest_checksum
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_publish_workspace_to_registry_dry_run_keeps_index_clean() {
        let root = temp_dir("publish_dry_run");
        let workspace = root.join("workspace");
        let src = workspace.join("src");
        let entry = src.join("main.agam");
        fs::create_dir_all(&src).expect("create source directory");
        fs::write(&entry, render_project_entry("dry-run-demo")).expect("write entry source");
        agam_pkg::write_workspace_manifest_to_path(
            &agam_pkg::default_manifest_path(&workspace),
            &agam_pkg::scaffold_workspace_manifest("dry-run-demo"),
        )
        .expect("write manifest");

        let index_root = root.join("registry-index");
        let report = publish_workspace_to_registry(
            Some(workspace.clone()),
            &index_root,
            &[],
            None,
            None,
            None,
            None,
            false,
            true,
            false,
        )
        .expect("dry run should succeed");

        assert!(report.dry_run);
        assert!(report.receipt.is_none());
        assert!(!report.bootstrapped_config);
        assert_eq!(
            report.index_path,
            agam_pkg::registry_index_path(&report.manifest.name)
        );
        assert!(!index_root.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_publish_workspace_to_registry_supports_official_packages() {
        let root = temp_dir("publish_official_workspace");
        let workspace = root.join("workspace");
        let src = workspace.join("src");
        let entry = src.join("main.agam");
        fs::create_dir_all(&src).expect("create source directory");
        fs::write(&entry, render_project_entry("official-demo")).expect("write entry source");

        let mut manifest = agam_pkg::scaffold_workspace_manifest("official-demo");
        manifest.project.name = "agam-std".into();
        agam_pkg::write_workspace_manifest_to_path(
            &agam_pkg::default_manifest_path(&workspace),
            &manifest,
        )
        .expect("write manifest");

        let index_root = root.join("registry-index");
        agam_pkg::write_registry_config(
            &index_root,
            &agam_pkg::RegistryConfig {
                format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: None,
                name: Some("agam".into()),
            },
        )
        .expect("write registry config");

        let report = publish_workspace_to_registry(
            Some(workspace.clone()),
            &index_root,
            &["agam-lang".into()],
            None,
            None,
            Some(&"https://github.com/agam-lang/agam-std".to_string()),
            None,
            true,
            false,
            false,
        )
        .expect("official publish should succeed");

        assert!(report.official);
        let entry =
            agam_pkg::read_registry_package_entry(&index_root, "agam-std").expect("read entry");
        assert_eq!(entry.owners, vec!["agam-lang".to_string()]);
        assert_eq!(entry.releases.len(), 1);
        assert_eq!(entry.releases[0].version, manifest.project.version);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_inspect_registry_package_reads_entry_metadata() {
        let root = temp_dir("registry_inspect");
        let index_root = root.join("registry-index");
        agam_pkg::write_registry_config(
            &index_root,
            &agam_pkg::RegistryConfig {
                format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: None,
                name: Some("agam".into()),
            },
        )
        .expect("write registry config");

        agam_pkg::publish_to_registry_index(
            &index_root,
            &agam_pkg::PublishManifest {
                name: "json".into(),
                version: "1.2.0".into(),
                agam_version: "0.1".into(),
                checksum: "sha256-json".into(),
                manifest_checksum: "manifest-json".into(),
                description: Some("JSON support".into()),
                keywords: vec!["json".into(), "parser".into()],
                homepage: Some("https://example.com/json".into()),
                repository: Some("https://github.com/agam-lang/json".into()),
                download_url: None,
                dependencies: vec![agam_pkg::RegistryReleaseDependency {
                    name: "core".into(),
                    version_req: "^1.0".into(),
                    registry: None,
                    optional: false,
                    features: vec![],
                }],
                features: vec!["simd".into()],
            },
            &["alice".into()],
            "2026-04-10T12:00:00Z",
        )
        .expect("publish package");

        let report = inspect_registry_package(&index_root, "json").expect("inspect package");
        assert_eq!(report.index_name, "agam");
        assert_eq!(report.index_path, "js/on/json");
        assert_eq!(report.entry.name, "json");
        assert_eq!(report.entry.owners, vec!["alice".to_string()]);
        assert_eq!(report.entry.description.as_deref(), Some("JSON support"));
        assert_eq!(
            report.entry.repository.as_deref(),
            Some("https://github.com/agam-lang/json")
        );
        assert_eq!(report.entry.releases.len(), 1);
        assert_eq!(report.entry.releases[0].version, "1.2.0");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_audit_registry_index_package_reports_release_history() {
        let root = temp_dir("registry_audit");
        let index_root = root.join("registry-index");
        agam_pkg::write_registry_config(
            &index_root,
            &agam_pkg::RegistryConfig {
                format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: None,
                name: Some("agam".into()),
            },
        )
        .expect("write registry config");

        agam_pkg::append_release_to_index(
            &index_root,
            "tensor",
            &agam_pkg::RegistryRelease {
                version: "0.3.0".into(),
                checksum: "sha256-tensor".into(),
                agam_version: "0.1".into(),
                dependencies: vec![agam_pkg::RegistryReleaseDependency {
                    name: "core".into(),
                    version_req: "^1.0".into(),
                    registry: None,
                    optional: false,
                    features: vec!["simd".into()],
                }],
                features: vec!["cuda".into()],
                download_url: None,
                provenance: None,
                published_at: "2026-04-10T12:30:00Z".into(),
                yanked: false,
            },
        )
        .expect("append release");

        let report =
            audit_registry_index_package(&index_root, "tensor").expect("audit package history");
        assert_eq!(report.index_name, "agam");
        assert_eq!(report.index_path, "te/ns/tensor");
        assert!(
            report
                .lines
                .iter()
                .any(|line| line.contains("package: tensor"))
        );
        assert!(report.lines.iter().any(|line| line.contains("releases: 1")));
        assert!(
            report
                .lines
                .iter()
                .any(|line| line.contains("sha256-tensor"))
        );
        assert!(
            report
                .lines
                .iter()
                .any(|line| line.contains("dep: core ^1.0"))
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_install_registry_dependency_pins_latest_release_and_refreshes_lockfile() {
        let root = temp_dir("registry_install");
        let workspace = root.join("workspace");
        let src = workspace.join("src");
        let entry = src.join("main.agam");
        fs::create_dir_all(&src).expect("create source directory");
        fs::write(&entry, render_project_entry("install-demo")).expect("write entry source");
        agam_pkg::write_workspace_manifest_to_path(
            &agam_pkg::default_manifest_path(&workspace),
            &agam_pkg::scaffold_workspace_manifest("install-demo"),
        )
        .expect("write manifest");

        let index_root = root.join("mirror-index");
        agam_pkg::write_registry_config(
            &index_root,
            &agam_pkg::RegistryConfig {
                format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: None,
                name: Some("mirror".into()),
            },
        )
        .expect("write registry config");
        agam_pkg::append_release_to_index(
            &index_root,
            "json",
            &agam_pkg::RegistryRelease {
                version: "1.0.0".into(),
                checksum: "sha256-json-100".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-04-10T10:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("append 1.0.0");
        agam_pkg::append_release_to_index(
            &index_root,
            "json",
            &agam_pkg::RegistryRelease {
                version: "1.2.0".into(),
                checksum: "sha256-json-120".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-04-10T11:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("append 1.2.0");

        let report = install_registry_dependency(
            Some(workspace.clone()),
            &index_root,
            DependencyTable::Main,
            "json",
            None,
            false,
        )
        .expect("install dependency");

        assert_eq!(report.index_name, "mirror");
        assert_eq!(report.selected_version, "1.2.0");
        assert!(report.added_new_entry);
        assert!(report.changed_manifest);

        let manifest = agam_pkg::read_workspace_manifest_from_path(&workspace.join("agam.toml"))
            .expect("read updated manifest");
        let spec = manifest.dependencies.get("json").expect("json dependency");
        assert_eq!(spec.version.as_deref(), Some("1.2.0"));
        assert_eq!(spec.registry.as_deref(), Some("mirror"));

        let lockfile = agam_pkg::read_lockfile_from_path(&workspace.join("agam.lock"))
            .expect("read refreshed lockfile");
        assert_eq!(lockfile.packages.len(), 1);
        assert_eq!(lockfile.packages[0].name, "json");
        assert_eq!(lockfile.packages[0].version, "1.2.0");
        assert_eq!(lockfile.packages[0].content_hash, "sha256-json-120");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_install_registry_profile_pins_curated_packages_and_refreshes_lockfile() {
        let root = temp_dir("registry_profile_install");
        let workspace = root.join("workspace");
        let src = workspace.join("src");
        let entry = src.join("main.agam");
        fs::create_dir_all(&src).expect("create source directory");
        fs::write(&entry, render_project_entry("profile-demo")).expect("write entry source");
        agam_pkg::write_workspace_manifest_to_path(
            &agam_pkg::default_manifest_path(&workspace),
            &agam_pkg::scaffold_workspace_manifest("profile-demo"),
        )
        .expect("write manifest");

        let index_root = root.join("registry-index");
        agam_pkg::write_registry_config(
            &index_root,
            &agam_pkg::RegistryConfig {
                format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: None,
                name: Some("agam".into()),
            },
        )
        .expect("write registry config");
        agam_pkg::append_release_to_index(
            &index_root,
            "agam-std",
            &agam_pkg::RegistryRelease {
                version: "0.1.0".into(),
                checksum: "sha256-agam-std-010".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-04-10T10:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("append agam-std");
        agam_pkg::append_release_to_index(
            &index_root,
            "agam-test",
            &agam_pkg::RegistryRelease {
                version: "0.1.3".into(),
                checksum: "sha256-agam-test-013".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-04-10T11:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("append agam-test");

        let report = install_registry_profile(
            Some(workspace.clone()),
            &index_root,
            DependencyTable::Main,
            "base",
            false,
        )
        .expect("install curated profile");

        assert_eq!(report.profile.name, "base");
        assert_eq!(report.items.len(), 2);
        assert!(report.items.iter().all(|item| item.added_new_entry));

        let manifest = agam_pkg::read_workspace_manifest_from_path(&workspace.join("agam.toml"))
            .expect("read updated manifest");
        assert_eq!(
            manifest
                .dependencies
                .get("agam-std")
                .and_then(|spec| spec.version.as_deref()),
            Some("0.1.0")
        );
        assert_eq!(
            manifest
                .dependencies
                .get("agam-test")
                .and_then(|spec| spec.version.as_deref()),
            Some("0.1.3")
        );

        let lockfile = agam_pkg::read_lockfile_from_path(&workspace.join("agam.lock"))
            .expect("read refreshed lockfile");
        assert_eq!(lockfile.packages.len(), 2);
        assert!(
            lockfile
                .packages
                .iter()
                .any(|package| package.name == "agam-std")
        );
        assert!(
            lockfile
                .packages
                .iter()
                .any(|package| package.name == "agam-test")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_update_registry_dependencies_advances_matching_manifest_entries() {
        let root = temp_dir("registry_update");
        let workspace = root.join("workspace");
        let src = workspace.join("src");
        let entry = src.join("main.agam");
        fs::create_dir_all(&src).expect("create source directory");
        fs::write(&entry, render_project_entry("update-demo")).expect("write entry source");

        let mut manifest = agam_pkg::scaffold_workspace_manifest("update-demo");
        manifest.dependencies.insert(
            "json".into(),
            agam_pkg::DependencySpec {
                version: Some("1.0.0".into()),
                features: vec!["simd".into()],
                optional: true,
                ..agam_pkg::DependencySpec::default()
            },
        );
        agam_pkg::write_workspace_manifest_to_path(&workspace.join("agam.toml"), &manifest)
            .expect("write manifest");

        let index_root = root.join("registry-index");
        agam_pkg::write_registry_config(
            &index_root,
            &agam_pkg::RegistryConfig {
                format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: None,
                name: Some("agam".into()),
            },
        )
        .expect("write registry config");
        agam_pkg::append_release_to_index(
            &index_root,
            "json",
            &agam_pkg::RegistryRelease {
                version: "1.0.0".into(),
                checksum: "sha256-json-100".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-04-10T10:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("append 1.0.0");
        agam_pkg::append_release_to_index(
            &index_root,
            "json",
            &agam_pkg::RegistryRelease {
                version: "1.4.0".into(),
                checksum: "sha256-json-140".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: None,
                provenance: None,
                published_at: "2026-04-10T12:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("append 1.4.0");

        let report = update_registry_dependencies(
            Some(workspace.clone()),
            &index_root,
            DependencyTable::Main,
            &[],
            false,
        )
        .expect("update dependency");

        assert_eq!(report.index_name, "agam");
        assert_eq!(report.items.len(), 1);
        assert!(report.items[0].updated);
        assert_eq!(report.items[0].previous_version.as_deref(), Some("1.0.0"));
        assert_eq!(report.items[0].selected_version, "1.4.0");

        let manifest = agam_pkg::read_workspace_manifest_from_path(&workspace.join("agam.toml"))
            .expect("read updated manifest");
        let spec = manifest.dependencies.get("json").expect("json dependency");
        assert_eq!(spec.version.as_deref(), Some("1.4.0"));
        assert_eq!(spec.registry, None);
        assert_eq!(spec.features, vec!["simd".to_string()]);
        assert!(spec.optional);

        let lockfile = agam_pkg::read_lockfile_from_path(&workspace.join("agam.lock"))
            .expect("read refreshed lockfile");
        assert_eq!(lockfile.packages.len(), 1);
        assert_eq!(lockfile.packages[0].name, "json");
        assert_eq!(lockfile.packages[0].version, "1.4.0");
        assert_eq!(lockfile.packages[0].content_hash, "sha256-json-140");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_yank_registry_release_marks_release_unavailable() {
        let root = temp_dir("registry_yank");
        let index_root = root.join("registry-index");
        agam_pkg::write_registry_config(
            &index_root,
            &agam_pkg::RegistryConfig {
                format_version: agam_pkg::REGISTRY_INDEX_FORMAT_VERSION,
                api_url: None,
                download_url: Some("https://registry.example.com/dl".into()),
                name: Some("agam".into()),
            },
        )
        .expect("write registry config");
        agam_pkg::append_release_to_index(
            &index_root,
            "json",
            &agam_pkg::RegistryRelease {
                version: "1.0.0".into(),
                checksum: "sha256-json-100".into(),
                agam_version: "0.1".into(),
                dependencies: vec![],
                features: vec![],
                download_url: Some(
                    "https://registry.example.com/dl/json/1.0.0/json-1.0.0.agam-src.tar.gz".into(),
                ),
                provenance: Some(agam_pkg::RegistryReleaseProvenance {
                    source_checksum: "sha256-json-100".into(),
                    manifest_checksum: "manifest-json-100".into(),
                    published_by: Some("alice".into()),
                    source_repository: Some("https://github.com/agam-lang/json".into()),
                }),
                published_at: "2026-04-10T12:00:00Z".into(),
                yanked: false,
            },
        )
        .expect("append release");

        let report =
            yank_registry_release(&index_root, "json", "1.0.0", false).expect("yank release");
        assert_eq!(report.index_name, "agam");
        assert!(report.yanked);

        let entry =
            agam_pkg::read_registry_package_entry(&index_root, "json").expect("read package");
        assert!(entry.releases[0].yanked);

        let unyank =
            yank_registry_release(&index_root, "json", "1.0.0", true).expect("unyank release");
        assert!(!unyank.yanked);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_list_workspace_environments_reports_default_dev_environment() {
        let root = temp_dir("env_list");
        let workspace = root.join("workspace");
        let src = workspace.join("src");
        let entry = src.join("main.agam");
        fs::create_dir_all(&src).expect("create source directory");
        fs::write(&entry, render_project_entry("env-list-demo")).expect("write entry source");

        let mut manifest = agam_pkg::scaffold_workspace_manifest("env-list-demo");
        manifest.toolchain = Some(agam_pkg::ToolchainRequirement {
            agam: "0.2.0".into(),
            sdk: Some("host-native".into()),
            target: Some("x86_64-pc-windows-msvc".into()),
            runtime_abi: Some(agam_runtime::contract::RUNTIME_ABI_VERSION),
            preferred_backend: Some(agam_runtime::contract::RuntimeBackend::Llvm),
        });
        manifest.environments.insert(
            "dev".into(),
            agam_pkg::EnvironmentSpec {
                preferred_backend: Some(agam_runtime::contract::RuntimeBackend::Jit),
                profiles: vec!["debug".into()],
                ..agam_pkg::EnvironmentSpec::default()
            },
        );
        manifest.environments.insert(
            "release".into(),
            agam_pkg::EnvironmentSpec {
                target: Some("x86_64-unknown-linux-gnu".into()),
                profiles: vec!["release".into()],
                ..agam_pkg::EnvironmentSpec::default()
            },
        );
        agam_pkg::write_workspace_manifest_to_path(&workspace.join("agam.toml"), &manifest)
            .expect("write manifest");

        let report =
            list_workspace_environments(Some(workspace.clone())).expect("list environments");
        assert_eq!(report.default_environment.as_deref(), Some("dev"));
        assert_eq!(report.environments.len(), 2);
        assert!(report.environments.iter().any(|env| env.name == "dev"));
        assert!(report.environments.iter().any(|env| env.name == "release"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_inspect_workspace_environment_uses_selection_rules() {
        with_clean_agam_registry_env(|| {
            let root = temp_dir("env_inspect");
            let workspace = root.join("workspace");
            let src = workspace.join("src");
            let entry = src.join("main.agam");
            fs::create_dir_all(&src).expect("create source directory");
            fs::write(&entry, render_project_entry("env-inspect-demo"))
                .expect("write entry source");

            let mut manifest = agam_pkg::scaffold_workspace_manifest("env-inspect-demo");
            manifest.toolchain = Some(agam_pkg::ToolchainRequirement {
                agam: "0.2.0".into(),
                sdk: Some("host-native".into()),
                target: Some("x86_64-pc-windows-msvc".into()),
                runtime_abi: Some(agam_runtime::contract::RUNTIME_ABI_VERSION),
                preferred_backend: Some(agam_runtime::contract::RuntimeBackend::Llvm),
            });
            manifest.dependencies.insert(
                "json".into(),
                agam_pkg::DependencySpec {
                    version: Some("1.4.0".into()),
                    ..agam_pkg::DependencySpec::default()
                },
            );
            manifest.environments.insert(
                "dev".into(),
                agam_pkg::EnvironmentSpec {
                    preferred_backend: Some(agam_runtime::contract::RuntimeBackend::Jit),
                    profiles: vec!["debug".into()],
                    ..agam_pkg::EnvironmentSpec::default()
                },
            );
            agam_pkg::write_workspace_manifest_to_path(&workspace.join("agam.toml"), &manifest)
                .expect("write manifest");

            let report =
                inspect_workspace_environment(Some(workspace.clone()), None).expect("inspect env");
            assert!(report.selected_by_default);
            assert_eq!(report.environment.name, "dev");
            assert_eq!(report.environment.compiler, "0.2.0");
            assert_eq!(report.environment.sdk.as_deref(), Some("host-native"));
            assert_eq!(
                report.environment.target.as_deref(),
                Some("x86_64-pc-windows-msvc")
            );
            assert_eq!(
                report.environment.preferred_backend,
                Some(agam_runtime::contract::RuntimeBackend::Jit)
            );
            assert_eq!(report.environment.profiles, vec!["debug".to_string()]);
            assert_eq!(report.environment.packages, vec!["json@1.4.0".to_string()]);

            let _ = fs::remove_dir_all(root);
        });
    }
}
