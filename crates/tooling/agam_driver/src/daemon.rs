//! Incremental daemon, warm-state management, and IPC.

use super::*;

pub(crate) const DAEMON_STATUS_SCHEMA_VERSION: u32 = 1;

pub(crate) const DAEMON_HEARTBEAT_STALE_MS: u128 = 5_000;

pub(crate) const NESTED_BUILD_REQUEST_ENV: &str = "AGAM_NESTED_BUILD_REQUEST";

pub(crate) const NESTED_CHECK_REQUEST_ENV: &str = "AGAM_NESTED_CHECK_REQUEST";

pub(crate) const HEADLESS_EXEC_WORKER_ENV: &str = "AGAM_HEADLESS_EXEC_WORKER";

pub(crate) const HEADLESS_SANDBOX_ROOT_ENV: &str = "AGAM_HEADLESS_SANDBOX_ROOT";

#[derive(Debug)]
pub(crate) struct DaemonPrewarmedEntry {
    pub package: agam_pkg::PortablePackage,
    pub call_cache: CallCacheSelection,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DaemonDiffSummary {
    pub added_files: usize,
    pub changed_files: usize,
    pub removed_files: usize,
    pub unchanged_files: usize,
    pub manifest_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DaemonRunMode {
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
pub(crate) struct DaemonStatusRecord {
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
pub(crate) enum DaemonIpcRequest {
    Status,
    GetWarmMir {
        file_path: String,
        content_hash: String,
    },
    Stop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum DaemonIpcResponse {
    Status(DaemonStatusRecord),
    WarmMir {
        found: bool,
        mir_json: Option<String>,
        call_cache_json: Option<String>,
    },
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DaemonLiveness {
    Running,
    Snapshot,
    Stale,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WarmSummary {
    pub warmed_files: usize,
    pub reused_files: usize,
    pub warmed_version_count: usize,
    pub ast_decl_count: usize,
    pub hir_function_count: usize,
    pub mir_function_count: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WarmCacheSummary {
    pub file_count: usize,
    pub version_count: usize,
    pub ast_decl_count: usize,
    pub hir_function_count: usize,
    pub mir_function_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DaemonPrewarmSummary {
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

pub(crate) enum DaemonCycleOutcome {
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

#[derive(Debug, Serialize)]
pub(crate) struct DaemonWarmArtifact<'a> {
    pub mir: &'a agam_mir::ir::MirModule,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_cache: Option<&'a CallCacheSelection>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DaemonWarmArtifactOwned {
    pub mir: agam_mir::ir::MirModule,
    #[serde(default)]
    pub call_cache: Option<CallCacheSelection>,
}

/// Daemon pipeline and state owner.
#[derive(Debug, Default)]
pub(crate) struct DaemonSession {
    pub snapshot: Option<agam_pkg::WorkspaceSnapshot>,
    pub cache: BTreeMap<PathBuf, BTreeMap<String, WarmState>>,
    pub last_prewarm: DaemonPrewarmSummary,
}

/// Pipeline that takes a diff and reuses warm state where possible.
pub(crate) struct IncrementalPipeline<'a> {
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

pub(crate) struct DaemonWorkspaceTarget {
    pub root: PathBuf,
    pub project_name: String,
}

pub(crate) fn daemon_workspace_target_from_layout(
    layout: WorkspaceLayout,
) -> DaemonWorkspaceTarget {
    DaemonWorkspaceTarget {
        root: layout.root,
        project_name: layout.project_name,
    }
}

pub(crate) fn daemon_workspace_target_from_root(root: PathBuf) -> DaemonWorkspaceTarget {
    let project_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("agam-workspace")
        .to_string();
    DaemonWorkspaceTarget { root, project_name }
}

pub(crate) fn daemon_refresh_snapshot_hint(workspace: &WorkspaceLayout) -> PathBuf {
    if workspace.manifest_path.is_none() {
        workspace.entry_file.clone()
    } else {
        workspace.root.clone()
    }
}

pub(crate) fn resolve_daemon_workspace_target(
    path: Option<PathBuf>,
) -> Result<DaemonWorkspaceTarget, String> {
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

pub(crate) fn manifest_entry_path(
    root: &Path,
    manifest: &agam_pkg::WorkspaceManifest,
) -> Result<PathBuf, String> {
    let entry = manifest.project.entry.as_deref().unwrap_or("src/main.agam");
    workspace_relative_path(root, entry, "`project.entry`")
}

pub(crate) fn workspace_relative_path(
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

pub(crate) fn warm_state_mir<'a>(
    file: &Path,
    warm_state: &'a WarmState,
) -> Result<&'a agam_mir::ir::MirModule, String> {
    warm_state
        .mir
        .as_ref()
        .ok_or_else(|| format!("warm MIR state missing for `{}`", file.display()))
}

pub(crate) fn warm_state_source_features<'a>(
    file: &Path,
    warm_state: &'a WarmState,
) -> Result<&'a SourceFeatureFlags, String> {
    warm_state
        .source_features
        .as_ref()
        .ok_or_else(|| format!("warm source features missing for `{}`", file.display()))
}

pub(crate) fn source_features_from_call_cache(
    call_cache: CallCacheSelection,
) -> SourceFeatureFlags {
    SourceFeatureFlags {
        call_cache,
        experimental_usages: Vec::new(),
    }
}

pub(crate) fn warm_state_supports_runnable_reuse(warm_state: &WarmState) -> bool {
    warm_state.mir.is_some() && warm_state.source_features.is_some()
}

pub(crate) fn warm_state_module<'a>(
    file: &Path,
    warm_state: &'a WarmState,
) -> Result<&'a agam_ast::Module, String> {
    warm_state
        .module
        .as_ref()
        .ok_or_else(|| format!("warm AST module missing for `{}`", file.display()))
}

pub(crate) fn load_daemon_prewarmed_entry(
    path: &PathBuf,
    verbose: bool,
) -> Option<DaemonPrewarmedEntry> {
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

pub(crate) fn warm_state_from_daemon_prewarmed_entry(prewarmed: DaemonPrewarmedEntry) -> WarmState {
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

pub(crate) fn load_daemon_prewarmed_warm_state(path: &PathBuf, verbose: bool) -> Option<WarmState> {
    load_daemon_prewarmed_entry(path, verbose).map(warm_state_from_daemon_prewarmed_entry)
}

pub(crate) fn print_cache_status(
    path: Option<PathBuf>,
    recent: usize,
    verbose: bool,
) -> Result<(), String> {
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

pub(crate) fn dev_daemon_status_message(root: &Path) -> Result<String, String> {
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

pub(crate) fn run_dev_workflow(
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
        eprintln!("\x1b[1;32mâœ“\x1b[0m Formatted {} file(s).", changed.len());
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
    eprintln!("\x1b[1;32mâœ“\x1b[0m Type checks passed.");

    if !no_tests && !workspace.test_files.is_empty() {
        let totals = run_agam_tests(&workspace.test_files, verbose)?;
        if totals.failed > 0 {
            return Err(format!(
                "Agam tests failed: {} passed; {} failed",
                totals.passed, totals.failed
            ));
        }
        eprintln!("\x1b[1;32mâœ“\x1b[0m Agam tests passed: {}", totals.passed);
    }

    if no_run {
        eprintln!("\x1b[1;32mâœ“\x1b[0m Dev checks completed.");
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

    eprintln!("\x1b[1;32mâœ“\x1b[0m Dev run completed.");
    Ok(())
}

pub(crate) fn human_bytes(bytes: u64) -> String {
    pub(crate) const KIB: f64 = 1024.0;
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

pub(crate) fn relative_age(delta_ms: u128) -> String {
    pub(crate) const SECOND: u128 = 1000;
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

pub(crate) fn now_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

pub(crate) fn daemon_status_path(root: &Path) -> PathBuf {
    root.join(".agam_cache").join("daemon").join("status.json")
}

pub(crate) fn daemon_pid_path(root: &Path) -> PathBuf {
    root.join(".agam_cache").join("daemon").join("daemon.pid")
}

pub(crate) fn daemon_shutdown_path(root: &Path) -> PathBuf {
    root.join(".agam_cache")
        .join("daemon")
        .join("shutdown_requested")
}

pub(crate) fn daemon_port_path(root: &Path) -> PathBuf {
    root.join(".agam_cache").join("daemon").join("daemon.port")
}

pub(crate) fn ensure_daemon_status_dir(root: &Path) -> Result<PathBuf, String> {
    let dir = root.join(".agam_cache").join("daemon");
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "failed to create daemon status directory `{}`: {e}",
            dir.display()
        )
    })?;
    Ok(dir)
}

pub(crate) fn read_daemon_status(root: &Path) -> Result<Option<DaemonStatusRecord>, String> {
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

pub(crate) fn write_daemon_status(root: &Path, status: &DaemonStatusRecord) -> Result<(), String> {
    ensure_daemon_status_dir(root)?;
    let path = daemon_status_path(root);
    let json = serde_json::to_vec_pretty(status)
        .map_err(|e| format!("failed to serialize daemon status: {e}"))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("failed to write daemon status `{}`: {e}", path.display()))
}

pub(crate) fn clear_daemon_status(path: Option<PathBuf>, verbose: bool) -> Result<(), String> {
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

pub(crate) fn daemon_liveness(status: &DaemonStatusRecord, now: u128) -> DaemonLiveness {
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
pub(crate) fn active_daemon_status(root: &Path) -> Result<Option<DaemonStatusRecord>, String> {
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

pub(crate) fn tracked_snapshot_file_count(snapshot: &agam_pkg::WorkspaceSnapshot) -> usize {
    snapshot.manifests.len() + snapshot.source_files.len() + snapshot.test_files.len()
}

pub(crate) fn workspace_diff_is_empty(diff: &agam_pkg::WorkspaceSnapshotDiff) -> bool {
    diff.added_files.is_empty() && diff.changed_files.is_empty() && diff.removed_files.is_empty()
}

pub(crate) fn snapshot_diff_touches_manifest(
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

pub(crate) fn summarize_workspace_diff(
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

pub(crate) fn daemon_diff_has_changes(summary: &DaemonDiffSummary) -> bool {
    summary.manifest_changed
        || summary.added_files > 0
        || summary.changed_files > 0
        || summary.removed_files > 0
}

pub(crate) fn daemon_entry_snapshot<'a>(
    snapshot: &'a agam_pkg::WorkspaceSnapshot,
) -> Option<&'a agam_pkg::WorkspaceFileSnapshot> {
    snapshot
        .source_files
        .iter()
        .chain(&snapshot.test_files)
        .find(|file| file.path == snapshot.session.layout.entry_file)
}

pub(crate) fn warm_state_for_snapshot_file<'a>(
    session: &'a DaemonSession,
    file: &agam_pkg::WorkspaceFileSnapshot,
) -> Option<&'a WarmState> {
    session
        .cache
        .get(&file.path)
        .and_then(|versions| versions.get(&file.content_hash))
}

pub(crate) fn record_prewarm_error(summary: &mut DaemonPrewarmSummary, message: String) {
    match summary.last_error.as_mut() {
        Some(existing) => {
            existing.push_str(" | ");
            existing.push_str(&message);
        }
        None => summary.last_error = Some(message),
    }
}

pub(crate) fn daemon_prewarm_stage_dir(root: &Path) -> PathBuf {
    root.join(".agam_cache").join("daemon").join("prewarm")
}

pub(crate) fn ensure_daemon_prewarm_stage_dir(root: &Path) -> Result<PathBuf, String> {
    let dir = daemon_prewarm_stage_dir(root);
    std::fs::create_dir_all(&dir).map_err(|e| {
        format!(
            "failed to create daemon prewarm directory `{}`: {e}",
            dir.display()
        )
    })?;
    Ok(dir)
}

pub(crate) fn daemon_prewarm_stage_prefix(
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

pub(crate) fn daemon_prewarm_package_output(
    root: &Path,
    entry_file: &Path,
) -> Result<PathBuf, String> {
    Ok(daemon_prewarm_stage_prefix(root, entry_file, "package")?.with_extension("agpkg.json"))
}

pub(crate) fn daemon_prewarm_build_output(
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

pub(crate) fn clean_prewarm_output(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

pub(crate) fn build_outcome_artifact_kind_label(
    backend: Backend,
    outcome: &BuildOutcome,
) -> &'static str {
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

pub(crate) fn daemon_prewarm_package_artifact_missing(prewarm: &DaemonPrewarmSummary) -> bool {
    if !prewarm.package_ready {
        return false;
    }
    prewarm
        .package_artifact_path
        .as_deref()
        .map(|path| !Path::new(path).is_file())
        .unwrap_or(true)
}

pub(crate) fn daemon_prewarm_needs_refresh(prewarm: &DaemonPrewarmSummary) -> bool {
    daemon_prewarm_package_artifact_missing(prewarm)
}

pub(crate) fn daemon_prewarm_status_message(prewarm: &DaemonPrewarmSummary) -> Option<String> {
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

pub(crate) fn prewarm_daemon_entry_artifacts(
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
            // File was parsed/checked but not lowered â€” record at a lower warm level
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

pub(crate) fn daemon_prewarm_mir_artifact_path(
    root: &Path,
    file: &Path,
) -> Result<PathBuf, String> {
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

pub(crate) fn write_warm_artifact(
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

pub(crate) fn read_warm_artifact(path: &Path) -> Result<WarmState, String> {
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
pub(crate) fn load_daemon_warm_state_for_file(path: &Path, verbose: bool) -> Option<WarmState> {
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

pub(crate) fn build_daemon_status(
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

pub(crate) fn build_daemon_error_status(
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

pub(crate) fn print_daemon_status(path: Option<PathBuf>, verbose: bool) -> Result<(), String> {
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

pub(crate) fn refresh_daemon_session(
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

pub(crate) fn run_daemon_cycle(
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

pub(crate) fn spawn_ipc_server(
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

pub(crate) fn send_daemon_ipc_request(
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

pub(crate) fn run_daemon_foreground(
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
pub(crate) fn start_daemon_background(
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
        // Stale PID file â€” remove it
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
pub(crate) fn stop_daemon_background(path: Option<PathBuf>, verbose: bool) -> Result<(), String> {
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

pub(crate) fn summarize_warm_cache(
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

pub(crate) fn warm_workspace_session(
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
