//! Toolchain discovery for MSVC, LLVM/Clang, and Android NDK.

use std::path::{Path, PathBuf};
use std::process::Command;

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
            Self::ClangNotFound { searched_paths, remediation_hint } => {
                writeln!(f, "error: Native Clang/LLVM toolchain was not found on PATH or Visual Studio.")?;
                if !searched_paths.is_empty() {
                    writeln!(f, "Searched locations:")?;
                    for p in searched_paths {
                        writeln!(f, "  - {}", p.display())?;
                    }
                }
                write!(f, "Action: {remediation_hint}")
            }
            Self::MsvcNotFound { remediation_hint } => {
                write!(f, "error: MSVC C++ Build Tools not detected. {remediation_hint}")
            }
            Self::AndroidNdkNotFound => {
                write!(f, "error: Android NDK not found. Set ANDROID_NDK_HOME or ANDROID_NDK_ROOT.")
            }
        }
    }
}

impl std::error::Error for ToolchainDiscoveryError {}

/// Discover native or bundled Clang toolchain on host.
pub fn find_native_clang() -> Result<ClangToolchain, ToolchainDiscoveryError> {
    let mut searched = Vec::new();

    // 1. Check AGAM_LLVM_CLANG or LLVM_CLANG environment variable
    for env_var in &["AGAM_LLVM_CLANG", "LLVM_CLANG", "CLANG_PATH"] {
        if let Ok(val) = std::env::var(env_var) {
            let path = PathBuf::from(val);
            if path.is_file() {
                let version = query_clang_version(&path).unwrap_or_else(|| "custom".into());
                return Ok(ClangToolchain {
                    clang_path: path,
                    version,
                    is_bundled: false,
                });
            }
            searched.push(path);
        }
    }

    // 2. Check standard system locations on Windows
    #[cfg(windows)]
    {
        let standard_windows_paths = [
            PathBuf::from(r"C:\Program Files\LLVM\bin\clang.exe"),
            PathBuf::from(r"C:\Program Files (x86)\LLVM\bin\clang.exe"),
            PathBuf::from(r"C:\LLVM\bin\clang.exe"),
        ];
        for p in &standard_windows_paths {
            if p.is_file() {
                let version = query_clang_version(p).unwrap_or_else(|| "18+".into());
                return Ok(ClangToolchain {
                    clang_path: p.clone(),
                    version,
                    is_bundled: false,
                });
            }
            searched.push(p.clone());
        }
    }

    // 3. Check bundled LLVM if distributed with compiler
    if let Some(bundled) = find_bundled_llvm_clang() {
        return Ok(bundled);
    }

    // 4. Try PATH lookup via `where clang` or `which clang`
    let candidate = if cfg!(windows) { "clang.exe" } else { "clang" };
    if let Ok(output) = Command::new(candidate).arg("--version").output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("clang")
                .to_string();
            return Ok(ClangToolchain {
                clang_path: PathBuf::from(candidate),
                version,
                is_bundled: false,
            });
        }
    }

    Err(ToolchainDiscoveryError::ClangNotFound {
        searched_paths: searched,
        remediation_hint: "Install LLVM (https://releases.llvm.org) or Visual Studio Clang tools, or run with `--backend jit` for in-process execution without external toolchains.".into(),
    })
}

/// Discover bundled LLVM if placed in ../llvm or ./llvm relative to the compiler binary.
pub fn find_bundled_llvm_clang() -> Option<ClangToolchain> {
    let current_exe = std::env::current_exe().ok()?;
    let exe_dir = current_exe.parent()?;

    let candidates = [
        exe_dir.join("llvm").join("bin").join(if cfg!(windows) { "clang.exe" } else { "clang" }),
        exe_dir.parent()?.join("llvm").join("bin").join(if cfg!(windows) { "clang.exe" } else { "clang" }),
    ];

    for candidate in &candidates {
        if candidate.is_file() {
            let version = query_clang_version(candidate).unwrap_or_else(|| "bundled".into());
            return Some(ClangToolchain {
                clang_path: candidate.clone(),
                version,
                is_bundled: true,
            });
        }
    }

    None
}

/// Resolve LLVM toolchain with fallback checks.
pub fn resolve_llvm_toolchain() -> Result<ClangToolchain, ToolchainDiscoveryError> {
    find_native_clang()
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
        let vswhere = windows_vswhere_path();
        if let Some(vswhere_path) = vswhere {
            if vswhere_path.is_file() {
                if let Ok(output) = Command::new(&vswhere_path)
                    .args(["-latest", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"])
                    .output()
                {
                    if output.status.success() {
                        let install_path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !install_path_str.is_empty() {
                            let install_path = PathBuf::from(&install_path_str);
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
                }
            }
        }
    }

    Err(ToolchainDiscoveryError::MsvcNotFound {
        remediation_hint: "Install Visual Studio Build Tools with the 'Desktop development with C++' workload.".into(),
    })
}

#[cfg(windows)]
fn windows_vswhere_path() -> Option<PathBuf> {
    let program_files = std::env::var("ProgramFiles(x86)").ok()?;
    let path = PathBuf::from(program_files)
        .join(r"Microsoft Visual Studio\Installer\vswhere.exe");
    if path.is_file() {
        Some(path)
    } else {
        None
    }
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
