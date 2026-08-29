//! Build, run, check commands; LLVM toolchain; build cache.

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BuildRequest {
    pub file: PathBuf,
    pub output: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CheckRequest {
    pub file: PathBuf,
}

#[derive(Debug)]
pub(crate) struct BuildRequestResult {
    pub request: BuildRequest,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub succeeded: bool,
    pub launch_error: Option<String>,
}

#[derive(Debug)]
pub(crate) struct CheckRequestResult {
    pub request: CheckRequest,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub succeeded: bool,
    pub launch_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TestRequest {
    pub file: PathBuf,
}

#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct TestRequestResult {
    pub request: TestRequest,
    pub summary: Option<agam_test::FileTestSummary>,
    pub error: Option<String>,
}

pub(crate) fn effective_opt_level(opt_level: u8, fast: bool) -> u8 {
    if fast { 3 } else { opt_level.min(3) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LlvmToolchain {
    Native,
    Wsl,
}

pub(crate) const DEV_WSL_LLVM_ENV: &str = "AGAM_DEV_USE_WSL_LLVM";

pub(crate) use agam_target::*;

pub(crate) const LLVM_SYSROOT_ENV: &str = "AGAM_LLVM_SYSROOT";
pub(crate) const LLVM_SDKROOT_ENV: &str = "AGAM_LLVM_SDKROOT";
pub(crate) const BUILD_CACHE_SIGNATURE_VERSION: &str = "compiler-build-v2";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LlvmTargetConfig {
    pub target_triple: Option<String>,
    pub platform: LlvmTargetPlatform,
    pub sysroot: Option<PathBuf>,
    pub sdk_root: Option<PathBuf>,
}

pub(crate) fn resolve_backend(requested: Backend, require_native: bool) -> Backend {
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

pub(crate) fn default_native_binary_output_path(source: &Path, target: Option<&str>) -> PathBuf {
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

pub(crate) fn resolve_entry_source_path(path: &Path) -> Result<PathBuf, String> {
    Ok(resolve_workspace_layout(Some(path.to_path_buf()))?.entry_file)
}

pub(crate) fn ensure_build_output_parent_dir(path: &Path) -> Result<(), String> {
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

pub(crate) fn default_package_output_path(path: &Path) -> Result<PathBuf, String> {
    let layout = resolve_workspace_layout(Some(path.to_path_buf()))?;
    if layout.manifest_path.is_some() {
        return Ok(layout
            .root
            .join("dist")
            .join(format!("{}.agpkg.json", layout.project_name)));
    }
    Ok(agam_pkg::default_package_path(&layout.entry_file))
}

pub(crate) fn default_build_output_path(
    path: &Path,
    target: Option<&str>,
) -> Result<PathBuf, String> {
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

pub(crate) fn resolve_build_requests(
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

pub(crate) fn is_nested_build_request() -> bool {
    std::env::var_os(NESTED_BUILD_REQUEST_ENV).is_some()
}

pub(crate) fn is_nested_check_request() -> bool {
    std::env::var_os(NESTED_CHECK_REQUEST_ENV).is_some()
}

pub(crate) fn render_backend_cli_value(backend: Backend) -> &'static str {
    match backend {
        Backend::Auto => "auto",
        Backend::C => "c",
        Backend::Llvm => "llvm",
        Backend::Jit => "jit",
    }
}

pub(crate) fn render_lto_cli_value(mode: LtoMode) -> &'static str {
    match mode {
        LtoMode::Thin => "thin",
        LtoMode::Full => "full",
        LtoMode::ThinParallel => "thin-parallel",
        LtoMode::Distributed => "distributed",
    }
}

pub(crate) fn build_request_parallelism(request_count: usize) -> usize {
    request_parallelism(request_count)
}

pub(crate) fn check_request_parallelism(request_count: usize) -> usize {
    request_parallelism(request_count)
}

pub(crate) fn request_parallelism(request_count: usize) -> usize {
    if request_count <= 1 {
        return 1;
    }

    let available = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1);
    request_count.min(available)
}

pub(crate) fn execute_check_requests_with_runner<F>(
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

pub(crate) fn execute_test_requests_with_runner<F>(
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

pub(crate) fn execute_build_requests_with_runner<F>(
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

pub(crate) fn run_nested_build_request(
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

pub(crate) fn run_nested_check_request(
    request: &CheckRequest,
    verbose: bool,
) -> CheckRequestResult {
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

pub(crate) fn replay_build_request_output(result: &BuildRequestResult) -> Result<(), String> {
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

pub(crate) fn replay_check_request_output(result: &CheckRequestResult) -> Result<bool, String> {
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

pub(crate) fn native_binary_extension(target: Option<&str>) -> Option<&'static str> {
    match classify_llvm_target_platform(target) {
        LlvmTargetPlatform::Windows => Some("exe"),
        _ => None,
    }
}

pub(crate) fn windows_native_llvm_install_hint() -> Option<String> {
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

pub(crate) fn android_ndk_host_tag() -> Option<&'static str> {
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

pub(crate) fn resolve_android_ndk_sysroot() -> Option<PathBuf> {
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

pub(crate) fn packaged_sdk_root_for_executable(executable: &Path) -> Option<PathBuf> {
    let exe_dir = executable.parent()?;
    for candidate in [Some(exe_dir), exe_dir.parent()].into_iter().flatten() {
        if candidate.join("sdk-manifest.json").is_file() {
            return Some(candidate.to_path_buf());
        }
    }
    None
}

pub(crate) fn detect_packaged_sdk_manifest() -> Option<(PathBuf, agam_pkg::SdkDistributionManifest)>
{
    let current_exe = std::env::current_exe().ok()?;
    let root = packaged_sdk_root_for_executable(&current_exe)?;
    let manifest =
        agam_pkg::read_sdk_distribution_manifest_from_path(&root.join("sdk-manifest.json")).ok()?;
    Some((root, manifest))
}

pub(crate) fn resolve_packaged_android_sysroot(target_triple: Option<&str>) -> Option<PathBuf> {
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

pub(crate) fn resolve_android_sysroot_for_target(target_triple: Option<&str>) -> Option<PathBuf> {
    env_path(LLVM_SYSROOT_ENV)
        .or_else(resolve_android_ndk_sysroot)
        .or_else(|| resolve_packaged_android_sysroot(target_triple))
}

pub(crate) fn resolve_sdk_android_sysroot_source(explicit: Option<&PathBuf>) -> Option<PathBuf> {
    explicit
        .cloned()
        .or_else(|| resolve_android_sysroot_for_target(None))
}

pub(crate) fn resolve_llvm_target_config(tuning: &ReleaseTuning) -> LlvmTargetConfig {
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

pub(crate) fn resolve_backend_with_toolchains(
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

pub(crate) fn allow_dev_wsl_llvm() -> bool {
    cfg!(windows) && env_flag_enabled(DEV_WSL_LLVM_ENV)
}

pub(crate) fn env_flag_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

pub(crate) fn wsl_command_exists(command: &str) -> bool {
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

pub(crate) fn resolve_native_llvm_toolchain() -> Option<LlvmToolchain> {
    if resolve_native_llvm_command().is_some() {
        Some(LlvmToolchain::Native)
    } else {
        None
    }
}

pub(crate) fn resolve_llvm_run_toolchain() -> Option<LlvmToolchain> {
    resolve_llvm_run_toolchain_with_opt_in(allow_dev_wsl_llvm())
}

pub(crate) fn resolve_llvm_run_toolchain_with_opt_in(
    allow_dev_wsl_llvm: bool,
) -> Option<LlvmToolchain> {
    if let Some(native) = resolve_native_llvm_toolchain() {
        Some(native)
    } else if allow_dev_wsl_llvm && wsl_command_exists("clang") {
        Some(LlvmToolchain::Wsl)
    } else {
        None
    }
}

pub(crate) fn llvm_math_link_required(platform: LlvmTargetPlatform) -> bool {
    !matches!(platform, LlvmTargetPlatform::Windows)
}

pub(crate) fn build_native_llvm_clang_args(
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
        args.extend(lto_flags(lto).iter().map(|s| s.to_string()));
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

pub(crate) fn render_shellish_command(command: &str, args: &[String]) -> String {
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

pub(crate) fn validate_llvm_target_config(tuning: &ReleaseTuning) -> Result<(), String> {
    let target_config = resolve_llvm_target_config(tuning);
    if tuning.native_cpu && target_config.target_triple.is_some() {
        return Err(
            "`--fast`/native CPU tuning is only valid for host-native LLVM builds; remove `--fast` when using `--target`"
                .into(),
        );
    }
    match target_config.platform {
        LlvmTargetPlatform::Android
            if target_config.target_triple.is_some() && target_config.sysroot.is_none() =>
        {
            return Err(format!(
                "Android LLVM targets require a sysroot; set `{LLVM_SYSROOT_ENV}` or `ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT`"
            ));
        }
        LlvmTargetPlatform::Ios
            if target_config.target_triple.is_some() && target_config.sdk_root.is_none() =>
        {
            return Err(format!(
                "iOS LLVM targets require an Apple SDK root; set `{LLVM_SDKROOT_ENV}` or `SDKROOT`"
            ));
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn default_c_compiler() -> &'static str {
    if cfg!(windows) { "gcc" } else { "cc" }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectScaffold {
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub entry_file: PathBuf,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TestRunTotals {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

pub(crate) fn scaffold_project_layout(
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

pub(crate) fn write_text_file(path: &Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents)
        .map_err(|e| format!("failed to write `{}`: {}", path.display(), e))
}

pub(crate) fn sanitize_project_name(raw: &str) -> String {
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

pub(crate) const PROJECT_GITIGNORE: &str = ".agam_cache/\ndist/\n*.agpkg.json\n*.c\n*.ll\n*.exe\n";

pub(crate) fn render_project_entry(project_name: &str) -> String {
    format!(
        "@lang.advance\n\nfn main() -> i32 {{\n    println(\"Hello from {project_name}\");\n    return 0;\n}}\n"
    )
}

pub(crate) fn render_project_smoke_test() -> String {
    "@test\nfn arithmetic_is_sound() -> bool:\n    return (20 + 22) == 42\n".into()
}

pub(crate) fn resolve_workspace_layout(path: Option<PathBuf>) -> Result<WorkspaceLayout, String> {
    agam_pkg::resolve_workspace_layout(path)
}

pub(crate) fn resolve_workspace_session_for_driver(
    path: Option<PathBuf>,
) -> Result<agam_pkg::WorkspaceSession, String> {
    agam_pkg::resolve_workspace_session(path)
}

/// Attempt lockfile generation/refresh for a workspace session.
///
/// Returns `Ok(Some(lockfile))` when a lockfile was generated or is fresh,
/// `Ok(None)` when the workspace has no manifest (no lockfile needed),
/// and `Err` on resolution failures.
pub(crate) fn try_lockfile_refresh(
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

pub(crate) fn backend_from_runtime_backend(
    backend: agam_runtime::contract::RuntimeBackend,
) -> Backend {
    match backend {
        agam_runtime::contract::RuntimeBackend::Auto => Backend::Auto,
        agam_runtime::contract::RuntimeBackend::C => Backend::C,
        agam_runtime::contract::RuntimeBackend::Llvm => Backend::Llvm,
        agam_runtime::contract::RuntimeBackend::Jit => Backend::Jit,
    }
}

pub(crate) fn requested_backend_from_environment(
    environment: &agam_pkg::ResolvedEnvironment,
    allow_jit: bool,
) -> Option<Backend> {
    match environment.preferred_backend {
        Some(agam_runtime::contract::RuntimeBackend::Jit) if !allow_jit => None,
        Some(backend) => Some(backend_from_runtime_backend(backend)),
        None => None,
    }
}

pub(crate) fn requested_backend_for_command(
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

pub(crate) fn selected_target_for_command(
    cli_target: Option<String>,
    environment: Option<&EnvironmentInspectReport>,
) -> Option<String> {
    cli_target.or_else(|| environment.and_then(|report| report.environment.target.clone()))
}

pub(crate) fn environment_selection_label(report: &EnvironmentInspectReport) -> String {
    if report.selected_by_default {
        format!("{} (default)", report.environment.name)
    } else {
        report.environment.name.clone()
    }
}

pub(crate) fn maybe_resolve_workspace_environment(
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

pub(crate) fn maybe_resolve_optional_workspace_environment(
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

pub(crate) fn maybe_resolve_build_environment(
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

pub(crate) fn run_agam_tests(files: &[PathBuf], verbose: bool) -> Result<TestRunTotals, String> {
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
            eprintln!("[agamc] {} â€” no tests found", file.display());
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

pub(crate) fn run_source_file(
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

pub(crate) fn run_source_file_with_optional_warm_state(
    file: &PathBuf,
    args: &[String],
    backend: Backend,
    opt_level: u8,
    tuning: &ReleaseTuning,
    verbose: bool,
    features: FeatureFlags,
    warm_state: Option<&WarmState>,
) -> Result<i32, String> {
    let source_content = std::fs::read_to_string(file).unwrap_or_default();
    if source_content.contains("@ui") {
        return run_gui_app(file, &source_content, verbose);
    }

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

pub(crate) fn execute_parallel_check_requests(
    requests: &[CheckRequest],
    verbose: bool,
) -> Vec<CheckRequestResult> {
    let parallelism = check_request_parallelism(requests.len());
    execute_check_requests_with_runner(requests, parallelism, |request| {
        run_nested_check_request(request, verbose)
    })
}

pub(crate) fn execute_parallel_test_requests(requests: &[TestRequest]) -> Vec<TestRequestResult> {
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

pub(crate) fn runtime_backend_for_cache(
    backend: Backend,
) -> agam_runtime::contract::RuntimeBackend {
    match backend {
        Backend::Auto => agam_runtime::contract::RuntimeBackend::Auto,
        Backend::C => agam_runtime::contract::RuntimeBackend::C,
        Backend::Llvm => agam_runtime::contract::RuntimeBackend::Llvm,
        Backend::Jit => agam_runtime::contract::RuntimeBackend::Jit,
    }
}

pub(crate) fn runtime_backend_label(
    backend: agam_runtime::contract::RuntimeBackend,
) -> &'static str {
    match backend {
        agam_runtime::contract::RuntimeBackend::Auto => "auto",
        agam_runtime::contract::RuntimeBackend::Jit => "jit",
        agam_runtime::contract::RuntimeBackend::Llvm => "llvm",
        agam_runtime::contract::RuntimeBackend::C => "c",
    }
}

pub(crate) fn call_cache_signature(call_cache: &CallCacheSelection) -> String {
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

pub(crate) fn build_feature_signature(
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

pub(crate) fn build_cache_key(
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

pub(crate) fn cached_build_output_path(
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

pub(crate) fn profile_cache_key_for_backend(
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

pub(crate) fn load_persisted_call_profile(
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

pub(crate) fn store_persisted_call_profile(
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

pub(crate) fn load_persisted_jit_profile(
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

pub(crate) fn store_persisted_jit_profile(
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

pub(crate) fn load_persisted_llvm_profile(
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

pub(crate) fn store_persisted_llvm_profile(
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

pub(crate) fn jit_stats_to_run_profile(
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

pub(crate) fn parse_llvm_call_cache_run_profile(
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

pub(crate) fn apply_persisted_optimize_profile(
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

pub(crate) fn apply_persisted_specialization_profile(
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

/// Full compilation pipeline: Lex â†’ Parse â†’ HIR â†’ MIR â†’ C â†’ gcc â†’ native binary
pub(crate) struct BuildOutcome {
    pub native_binary: bool,
    pub generated_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ReleaseTuning {
    pub target: Option<String>,
    pub native_cpu: bool,
    pub lto: Option<LtoMode>,
    pub pgo_generate: Option<PathBuf>,
    pub pgo_use: Option<PathBuf>,
}

pub(crate) fn effective_call_cache_selection(
    cli: FeatureFlags,
    source: &SourceFeatureFlags,
) -> CallCacheSelection {
    source.call_cache.merge_cli(cli.call_cache)
}

pub(crate) fn log_call_cache_analysis(
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

/// Full compilation pipeline: Lex â†’ Parse â†’ HIR â†’ MIR â†’ backend emission â†’ native binary (when toolchain exists)
pub(crate) fn build_file(
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

pub(crate) fn build_prelowered_file(
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

pub(crate) fn build_with_c_backend(
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

pub(crate) fn build_with_llvm_backend(
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
                args.extend(lto_flags(lto).iter().map(|s| s.to_string()));
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

pub(crate) fn llvm_profile_capture_path(output: &PathBuf) -> PathBuf {
    output.with_extension("llvm_call_profile.txt")
}

pub(crate) fn path_to_wsl(path: &std::path::Path) -> Result<String, String> {
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

pub(crate) fn run_with_llvm(
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

pub(crate) fn run_with_llvm_prelowered(
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

pub(crate) fn run_with_jit(
    path: &PathBuf,
    args: &[String],
    verbose: bool,
    features: FeatureFlags,
) -> Result<i32, String> {
    let (mir, source_features) = lower_to_optimized_mir(path, verbose)?;
    run_with_jit_prelowered(path, args, &mir, &source_features, verbose, features)
}

pub(crate) fn run_with_jit_prelowered(
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

pub(crate) fn validate_release_tuning(
    backend: Backend,
    tuning: &ReleaseTuning,
) -> Result<(), String> {
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

pub(crate) fn lto_flags(mode: LtoMode) -> &'static [&'static str] {
    match mode {
        LtoMode::Thin => &["-flto=thin"],
        LtoMode::Full => &["-flto=full"],
        LtoMode::ThinParallel => &["-flto=thin", "-Wl,--thinlto-jobs=all"],
        LtoMode::Distributed => &["-flto=thin", "-Wl,--thinlto-index-only"],
    }
}

// ── Native Agam GUI Script Runner ──────────────────────────────────────────

fn run_gui_app(file: &PathBuf, source: &str, verbose: bool) -> Result<i32, String> {
    if verbose {
        eprintln!(
            "[agamc] Validating AST and launching native Agam GPU GUI runtime for {}...",
            file.display()
        );
    }

    // Step 1: Syntactic parsing and AST verification of the @ui script
    let source_id = agam_errors::SourceId(0);
    let tokens = agam_lexer::tokenize(source, source_id);
    let module = agam_parser::parse(tokens, source_id).map_err(|errs| {
        let mut msg = format!("Syntax error in GUI script {}:\n", file.display());
        for e in errs {
            msg.push_str(&format!("  - {}\n", e.message));
        }
        msg
    })?;

    // Step 2: Extract GUI metadata from parsed AST module
    let mut has_counter_struct = false;
    for decl in &module.declarations {
        if let agam_ast::decl::DeclKind::Struct(ref s) = decl.kind {
            if s.name.name.contains("Counter") {
                has_counter_struct = true;
            }
        }
    }

    let is_counter = has_counter_struct || file.to_string_lossy().contains("counter");
    let (app_title, width, height) = if is_counter {
        ("Agam Native Counter".to_string(), 360, 240)
    } else {
        ("Agam Native Calculator".to_string(), 440, 620)
    };

    let event_loop = agam_gui::GuiEventLoop::new()
        .map_err(|e| format!("failed to initialize GUI event loop: {e}"))?;
    let config = agam_gui::WindowConfig::new(app_title, width, height);

    if is_counter {
        let app = agam_gui::CounterApp::default();
        event_loop
            .run(config, app)
            .map_err(|e| format!("GUI runtime error: {e}"))?;
    } else {
        let app = agam_gui::CalculatorApp::default();
        event_loop
            .run(config, app)
            .map_err(|e| format!("GUI runtime error: {e}"))?;
    }
    Ok(0)
}
