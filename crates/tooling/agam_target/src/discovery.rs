//! Host and target toolchain discovery for MSVC, LLVM/Clang, and Android NDK.

#![deny(clippy::unwrap_used)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const LLVM_CLANG_ENV: &str = "AGAM_LLVM_CLANG";
pub const LLVM_BUNDLE_DIR_ENV: &str = "AGAM_LLVM_BUNDLE_DIR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlvmTargetPlatform {
    Windows,
    Linux,
    MacOs,
    Android,
    Ios,
    Unknown,
}

impl LlvmTargetPlatform {
    pub fn is_windows(&self) -> bool {
        matches!(self, Self::Windows)
    }

    pub fn is_unix(&self) -> bool {
        matches!(self, Self::Linux | Self::MacOs | Self::Android | Self::Ios)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClangToolchain {
    pub clang_path: PathBuf,
    pub version: String,
    pub is_bundled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MsvcToolchain {
    pub installation_path: PathBuf,
    pub cl_path: PathBuf,
    pub link_path: PathBuf,
    pub lib_paths: Vec<PathBuf>,
    pub include_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainDiscoveryError {
    ClangNotFound {
        searched_paths: Vec<PathBuf>,
        remediation_hint: String,
    },
    MsvcNotFound {
        remediation_hint: String,
    },
    AndroidNdkNotFound,
}

impl std::fmt::Display for ToolchainDiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClangNotFound {
                searched_paths,
                remediation_hint,
            } => {
                writeln!(
                    f,
                    "error: Native Clang/LLVM toolchain was not found on PATH or Visual Studio."
                )?;
                if !searched_paths.is_empty() {
                    writeln!(f, "Searched locations:")?;
                    for p in searched_paths {
                        writeln!(f, "  - {}", p.display())?;
                    }
                }
                write!(f, "Action: {remediation_hint}")
            }
            Self::MsvcNotFound { remediation_hint } => {
                write!(
                    f,
                    "error: MSVC C++ Build Tools not detected. {remediation_hint}"
                )
            }
            Self::AndroidNdkNotFound => {
                write!(
                    f,
                    "error: Android NDK not found. Set ANDROID_NDK_HOME or ANDROID_NDK_ROOT."
                )
            }
        }
    }
}

impl std::error::Error for ToolchainDiscoveryError {}

pub fn env_path(var_name: &str) -> Option<PathBuf> {
    std::env::var_os(var_name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

pub fn command_exists(command: &str) -> bool {
    let check = if cfg!(windows) { "where" } else { "which" };
    Command::new(check)
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn classify_llvm_target_platform(target: Option<&str>) -> LlvmTargetPlatform {
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

pub fn host_llvm_target_platform() -> LlvmTargetPlatform {
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

pub fn configured_llvm_clang_override() -> Option<String> {
    std::env::var(LLVM_CLANG_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

pub fn llvm_driver_file_names() -> &'static [&'static str] {
    if cfg!(windows) {
        &["clang.exe", "clang++.exe"]
    } else {
        &["clang", "clang++"]
    }
}

pub fn bundled_llvm_platform_dir() -> &'static str {
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

pub fn bundled_llvm_candidate_paths(root: &Path) -> Vec<PathBuf> {
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

pub fn discover_bundled_llvm_clang() -> Option<String> {
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

pub fn windows_vswhere_path() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    env_path("ProgramFiles(x86)").map(|root| {
        root.join("Microsoft Visual Studio")
            .join("Installer")
            .join("vswhere.exe")
    })
}

pub fn discover_visual_studio_installation_path() -> Option<PathBuf> {
    let vswhere = windows_vswhere_path()?;
    if !vswhere.is_file() {
        return None;
    }
    let output = Command::new(vswhere)
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

pub fn visual_studio_llvm_candidate_paths(install_root: &Path) -> Vec<PathBuf> {
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

pub fn standalone_windows_llvm_install_roots() -> Vec<PathBuf> {
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

pub fn standalone_windows_llvm_candidate_paths(install_root: &Path) -> Vec<PathBuf> {
    llvm_driver_file_names()
        .iter()
        .map(|driver| install_root.join("bin").join(driver))
        .collect()
}

pub fn discover_standalone_windows_llvm_clang() -> Option<String> {
    if !cfg!(windows) {
        return None;
    }

    standalone_windows_llvm_install_roots()
        .into_iter()
        .flat_map(|root| standalone_windows_llvm_candidate_paths(&root))
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

pub fn discover_visual_studio_llvm_clang() -> Option<String> {
    let install_root = discover_visual_studio_installation_path()?;
    visual_studio_llvm_candidate_paths(&install_root)
        .into_iter()
        .find(|path| path.is_file())
        .map(|path| path.to_string_lossy().into_owned())
}

pub fn native_llvm_clang_candidates() -> Vec<String> {
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

pub fn resolve_native_llvm_command() -> Option<String> {
    native_llvm_clang_candidates()
        .into_iter()
        .find(|candidate| command_exists(candidate))
}

pub fn configured_llvm_clang() -> String {
    resolve_native_llvm_command().unwrap_or_else(|| {
        if cfg!(windows) {
            "clang.exe".into()
        } else {
            "clang".into()
        }
    })
}

/// Discover native or bundled Clang toolchain on host with detailed diagnostic information.
pub fn find_native_clang() -> Result<ClangToolchain, ToolchainDiscoveryError> {
    if let Some(cmd) = resolve_native_llvm_command() {
        let path = PathBuf::from(&cmd);
        let version = query_clang_version(&path).unwrap_or_else(|| "18+".into());
        return Ok(ClangToolchain {
            clang_path: path,
            version,
            is_bundled: cmd.contains("toolchains") || cmd.contains("llvm"),
        });
    }

    Err(ToolchainDiscoveryError::ClangNotFound {
        searched_paths: native_llvm_clang_candidates().into_iter().map(PathBuf::from).collect(),
        remediation_hint: "Install LLVM (https://releases.llvm.org) or Visual Studio Clang tools, or run with `--backend jit` for in-process execution without external toolchains.".into(),
    })
}

/// Query clang version string safely.
fn query_clang_version(clang_path: &Path) -> Option<String> {
    let output = Command::new(clang_path).arg("--version").output().ok()?;
    if output.status.success() {
        let first_line = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()?
            .trim()
            .to_string();
        Some(first_line)
    } else {
        None
    }
}

/// Discover MSVC toolchain using vswhere on Windows.
pub fn find_msvc_toolchain() -> Result<MsvcToolchain, ToolchainDiscoveryError> {
    #[cfg(windows)]
    {
        if let Some(install_path) = discover_visual_studio_installation_path() {
            let vc_tools_root = install_path.join(r"VC\Tools\MSVC");
            if vc_tools_root.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&vc_tools_root) {
                    let mut versions: Vec<PathBuf> = entries
                        .filter_map(Result::ok)
                        .map(|e| e.path())
                        .filter(|p| p.is_dir())
                        .collect();
                    versions.sort();
                    if let Some(latest_version) = versions.last() {
                        let cl = latest_version.join(r"bin\Hostx64\x64\cl.exe");
                        let link = latest_version.join(r"bin\Hostx64\x64\link.exe");
                        let lib = latest_version.join(r"lib\x64");
                        let inc = latest_version.join("include");

                        return Ok(MsvcToolchain {
                            installation_path: install_path,
                            cl_path: cl,
                            link_path: link,
                            lib_paths: vec![lib],
                            include_paths: vec![inc],
                        });
                    }
                }
            }
        }
    }

    Err(ToolchainDiscoveryError::MsvcNotFound {
        remediation_hint:
            "Install Visual Studio Build Tools with the 'Desktop development with C++' workload."
                .into(),
    })
}

/// Discover Android NDK root from environment variables.
pub fn find_android_ndk() -> Result<PathBuf, ToolchainDiscoveryError> {
    for var in &["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "NDK_HOME"] {
        if let Ok(val) = std::env::var(var) {
            let p = PathBuf::from(val);
            if p.is_dir() {
                return Ok(p);
            }
        }
    }
    Err(ToolchainDiscoveryError::AndroidNdkNotFound)
}
