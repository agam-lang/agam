//! Interactive REPL and headless agent-facing execution.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplSession {
    pub request: HeadlessExecutionRequest,
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
    pub(crate) fn append_line(&mut self, line: &str) {
        self.request.source.push_str(line);
        self.request.source.push('\n');
    }

    pub(crate) fn replace_source(&mut self, filename: String, source: String) {
        self.request.filename = filename;
        self.request.source = source;
    }

    pub(crate) fn clear(&mut self) {
        self.request.source.clear();
    }
}

#[derive(Debug)]
pub(crate) struct ReplExecutionCache {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub source_path: PathBuf,
    pub filename: String,
    pub source_hash: Option<String>,
    pub daemon_session: DaemonSession,
}

impl ReplExecutionCache {
    pub(crate) fn new(filename: &str) -> Result<Self, String> {
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

    pub(crate) fn source_path(&self) -> &PathBuf {
        &self.source_path
    }

    pub(crate) fn materialize_request(
        &mut self,
        request: &HeadlessExecutionRequest,
    ) -> Result<(), String> {
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

    pub(crate) fn ensure_materialized_warm_state(
        &mut self,
        verbose: bool,
    ) -> Result<&WarmState, String> {
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

pub(crate) fn repl_workspace_entry_relative_path(filename: &str) -> String {
    format!("src/{filename}")
}

pub(crate) fn repl_workspace_entry_path(root: &Path, filename: &str) -> PathBuf {
    root.join("src").join(filename)
}

pub(crate) fn write_repl_workspace_manifest(
    manifest_path: &Path,
    filename: &str,
) -> Result<(), String> {
    let mut manifest = agam_pkg::scaffold_workspace_manifest("repl-session");
    manifest.project.entry = Some(repl_workspace_entry_relative_path(filename));
    agam_pkg::write_workspace_manifest_to_path(manifest_path, &manifest)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReplCommandKind {
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

pub(crate) fn run_repl_shell(verbose: bool) -> Result<i32, String> {
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

pub(crate) fn print_repl_help() {
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

pub(crate) fn parse_repl_command(input: &str) -> Result<Option<ReplCommandKind>, String> {
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

pub(crate) fn parse_repl_fast_flag(value: &str) -> Result<bool, String> {
    match value {
        "on" | "true" | "1" => Ok(true),
        "off" | "false" | "0" => Ok(false),
        _ => Err("`:fast` expects `on` or `off`".into()),
    }
}

pub(crate) fn run_exec_tool(
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

pub(crate) fn build_exec_request(
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

pub(crate) fn run_headless_json_request(pretty: bool, verbose: bool) -> Result<i32, String> {
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

pub(crate) fn write_headless_response(
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

pub(crate) fn headless_response_exit_code(response: &HeadlessExecutionResponse) -> i32 {
    if let Some(code) = response.exit_code {
        code
    } else if response.success {
        0
    } else {
        1
    }
}

pub(crate) fn backend_to_headless_backend(backend: Backend) -> HeadlessExecutionBackend {
    match backend {
        Backend::Auto => HeadlessExecutionBackend::Auto,
        Backend::C => HeadlessExecutionBackend::C,
        Backend::Llvm => HeadlessExecutionBackend::Llvm,
        Backend::Jit => HeadlessExecutionBackend::Jit,
    }
}

pub(crate) fn render_headless_parse_errors(errors: &[agam_parser::ParseError]) -> String {
    let mut stderr = String::new();
    for error in errors {
        stderr.push_str("\x1b[1;31merror\x1b[0m: ");
        stderr.push_str(&error.message);
        stderr.push('\n');
    }
    stderr
}

pub(crate) fn build_headless_warm_state(
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

pub(crate) fn run_with_jit_prelowered_captured(
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
pub(crate) struct CapturedExecution {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub(crate) fn capture_command_output(
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

pub(crate) fn run_with_c_prelowered_captured(
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

pub(crate) fn run_with_llvm_prelowered_captured(
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

pub(crate) fn execute_headless_request_in_process(
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

pub(crate) fn should_execute_headless_request_in_process() -> bool {
    std::env::var_os(HEADLESS_EXEC_WORKER_ENV).is_some() || cfg!(test)
}

pub(crate) fn execute_headless_request_in_worker(
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

pub(crate) fn execute_headless_request(
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

pub(crate) fn execute_repl_request(
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

pub(crate) fn build_headless_worker_command(
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

pub(crate) fn configure_headless_worker_environment(
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

pub(crate) fn should_forward_headless_worker_env_var(key: &str) -> bool {
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
pub(crate) fn configure_headless_worker_platform_before_spawn(
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
pub(crate) fn configure_headless_worker_platform_before_spawn(
    _command: &mut std::process::Command,
    _request: &HeadlessExecutionRequest,
) -> Result<(), String> {
    Ok(())
}

pub(crate) fn wait_for_headless_worker_output(
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
pub(crate) fn terminate_headless_worker_process(pid: u32) -> Result<(), String> {
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
pub(crate) fn terminate_headless_worker_process(pid: u32) -> Result<(), String> {
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
pub(crate) fn terminate_headless_worker_process(_pid: u32) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub(crate) struct HeadlessWindowsJob {
    pub handle: *mut c_void,
}

#[cfg(windows)]
pub(crate) fn attach_headless_worker_job(
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

pub(crate) fn normalize_headless_request(
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

pub(crate) fn sanitize_headless_filename(filename: &str) -> String {
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

pub(crate) fn create_unique_headless_dir(base: &Path, label: &str) -> Result<PathBuf, String> {
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

pub(crate) fn create_headless_sandbox_root() -> Result<PathBuf, String> {
    create_unique_headless_dir(&std::env::temp_dir(), "agam_headless_sandbox")
}

pub(crate) fn create_headless_temp_dir() -> Result<PathBuf, String> {
    let base = env_path(HEADLESS_SANDBOX_ROOT_ENV).unwrap_or_else(std::env::temp_dir);
    create_unique_headless_dir(&base, "agam_headless_run")
}

pub(crate) fn cleanup_headless_temp_dir(path: &Path, verbose: bool) {
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

pub(crate) fn headless_backend_to_backend(backend: HeadlessExecutionBackend) -> Backend {
    match backend {
        HeadlessExecutionBackend::Auto => Backend::Auto,
        HeadlessExecutionBackend::C => Backend::C,
        HeadlessExecutionBackend::Llvm => Backend::Llvm,
        HeadlessExecutionBackend::Jit => Backend::Jit,
    }
}

pub(crate) fn parse_headless_backend_label(
    value: &str,
) -> Result<HeadlessExecutionBackend, String> {
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

pub(crate) fn render_headless_backend_label(backend: HeadlessExecutionBackend) -> &'static str {
    match backend {
        HeadlessExecutionBackend::Auto => "auto",
        HeadlessExecutionBackend::C => "c",
        HeadlessExecutionBackend::Llvm => "llvm",
        HeadlessExecutionBackend::Jit => "jit",
    }
}
