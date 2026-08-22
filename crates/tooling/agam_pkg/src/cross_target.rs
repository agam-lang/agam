//! Zero-Configuration Cross-Compilation Matrix and Sysroot Engine.
//!
//! Provides Zig-style zero-config cross-compilation across Windows, Linux, macOS,
//! Android, WASM, and RISC-V targets with automated sysroot management and QEMU/Wasmtime emulation planning.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// Target architecture classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TargetArch {
    X86_64,
    Aarch64,
    Wasm32,
    Riscv64,
    Riscv32,
}

impl fmt::Display for TargetArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetArch::X86_64 => write!(f, "x86_64"),
            TargetArch::Aarch64 => write!(f, "aarch64"),
            TargetArch::Wasm32 => write!(f, "wasm32"),
            TargetArch::Riscv64 => write!(f, "riscv64"),
            TargetArch::Riscv32 => write!(f, "riscv32"),
        }
    }
}

/// Target operating system classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TargetOs {
    Windows,
    Linux,
    Darwin,
    Ios,
    Android,
    Wasi,
    None,
}

impl fmt::Display for TargetOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetOs::Windows => write!(f, "windows"),
            TargetOs::Linux => write!(f, "linux"),
            TargetOs::Darwin => write!(f, "darwin"),
            TargetOs::Ios => write!(f, "ios"),
            TargetOs::Android => write!(f, "android"),
            TargetOs::Wasi => write!(f, "wasip2"),
            TargetOs::None => write!(f, "none"),
        }
    }
}

/// Target ABI / C-runtime environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TargetAbi {
    Gnu,
    Msvc,
    Musl,
    Eabi,
    Android,
    Unknown,
}

impl fmt::Display for TargetAbi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TargetAbi::Gnu => write!(f, "gnu"),
            TargetAbi::Msvc => write!(f, "msvc"),
            TargetAbi::Musl => write!(f, "musl"),
            TargetAbi::Eabi => write!(f, "eabi"),
            TargetAbi::Android => write!(f, "android"),
            TargetAbi::Unknown => write!(f, "unknown"),
        }
    }
}

/// Structured Target Triple for the Agam compiler cross-compilation pipeline.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TargetTriple {
    pub raw: String,
    pub arch: TargetArch,
    pub os: TargetOs,
    pub abi: TargetAbi,
}

impl TargetTriple {
    /// Parse a target triple string (e.g. `x86_64-linux-gnu`, `riscv64-linux-gnu`, `wasm32-wasip2`).
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.split('-').collect();
        let arch = match parts.first().copied() {
            Some("x86_64") => TargetArch::X86_64,
            Some("aarch64") => TargetArch::Aarch64,
            Some("wasm32") => TargetArch::Wasm32,
            Some("riscv64") => TargetArch::Riscv64,
            Some("riscv32") => TargetArch::Riscv32,
            other => return Err(format!("Unsupported target architecture: {:?}", other)),
        };

        let mut os = TargetOs::None;
        let mut abi = TargetAbi::Unknown;

        if s.contains("windows") {
            os = TargetOs::Windows;
            abi = TargetAbi::Msvc;
        } else if s.contains("android") {
            os = TargetOs::Android;
            abi = TargetAbi::Android;
        } else if s.contains("darwin") || s.contains("apple") {
            os = TargetOs::Darwin;
        } else if s.contains("ios") {
            os = TargetOs::Ios;
        } else if s.contains("wasi") {
            os = TargetOs::Wasi;
        } else if s.contains("linux") {
            os = TargetOs::Linux;
            if s.contains("musl") {
                abi = TargetAbi::Musl;
            } else {
                abi = TargetAbi::Gnu;
            }
        } else if s.contains("eabi") || s.contains("none") {
            os = TargetOs::None;
            abi = TargetAbi::Eabi;
        }

        Ok(Self {
            raw: s.to_string(),
            arch,
            os,
            abi,
        })
    }

    pub fn host() -> Self {
        #[cfg(target_os = "windows")]
        return Self::parse("x86_64-windows-msvc").unwrap();
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return Self::parse("x86_64-linux-gnu").unwrap();
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        return Self::parse("aarch64-linux-gnu").unwrap();
        #[cfg(target_os = "macos")]
        return Self::parse("aarch64-apple-darwin").unwrap();
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        return Self::parse("x86_64-linux-gnu").unwrap();
    }

    pub fn pointer_width_bits(&self) -> u32 {
        match self.arch {
            TargetArch::X86_64 | TargetArch::Aarch64 | TargetArch::Riscv64 => 64,
            TargetArch::Wasm32 | TargetArch::Riscv32 => 32,
        }
    }

    pub fn is_bare_metal(&self) -> bool {
        self.os == TargetOs::None
    }

    pub fn is_wasm(&self) -> bool {
        self.arch == TargetArch::Wasm32
    }

    /// Recommended host emulator for running target binaries locally (e.g. QEMU / Wasmtime).
    pub fn recommended_emulator(&self) -> Option<&'static str> {
        match self.arch {
            TargetArch::Aarch64 => Some("qemu-aarch64"),
            TargetArch::Riscv64 => Some("qemu-riscv64"),
            TargetArch::Riscv32 => Some("qemu-riscv32"),
            TargetArch::Wasm32 => Some("wasmtime"),
            TargetArch::X86_64 => None,
        }
    }
}

/// Sysroot status for cross-compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SysrootStatus {
    Installed { path: PathBuf },
    BundledInSdk,
    AutoDownloadPending { url: String },
    Missing,
}

/// Zero-Configuration Sysroot & Toolchain Manager.
pub struct SysrootManager {
    sysroot_dir: PathBuf,
    installed_targets: BTreeMap<String, PathBuf>,
}

impl SysrootManager {
    pub fn new(sysroot_dir: impl Into<PathBuf>) -> Self {
        Self {
            sysroot_dir: sysroot_dir.into(),
            installed_targets: BTreeMap::new(),
        }
    }

    pub fn check_status(&self, target: &TargetTriple) -> SysrootStatus {
        if target == &TargetTriple::host() {
            return SysrootStatus::Installed {
                path: PathBuf::from("host-native"),
            };
        }

        let target_path = self.sysroot_dir.join(&target.raw);
        if target_path.exists() || self.installed_targets.contains_key(&target.raw) {
            SysrootStatus::Installed { path: target_path }
        } else {
            SysrootStatus::AutoDownloadPending {
                url: format!("https://sdk.agam-lang.org/sysroots/{}.tar.xz", target.raw),
            }
        }
    }

    pub fn register_sysroot(&mut self, target_triple: &str, path: impl Into<PathBuf>) {
        self.installed_targets
            .insert(target_triple.to_string(), path.into());
    }
}

/// Cross-Compilation Execution Plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossCompilePlan {
    pub target: TargetTriple,
    pub clang_target_flag: String,
    pub linker_flags: Vec<String>,
    pub sysroot_path: Option<PathBuf>,
    pub emulator: Option<String>,
}

/// Cross-Compilation Planner.
pub struct CrossCompilePlanner;

impl CrossCompilePlanner {
    pub fn plan(target: &TargetTriple, sysroot_manager: &SysrootManager) -> CrossCompilePlan {
        let clang_target_flag = format!("--target={}", target.raw);
        let mut linker_flags = Vec::new();

        let sysroot_path = match sysroot_manager.check_status(target) {
            SysrootStatus::Installed { path } => {
                if path != Path::new("host-native") {
                    linker_flags.push(format!("--sysroot={}", path.display()));
                    Some(path)
                } else {
                    None
                }
            }
            _ => None,
        };

        if target.is_wasm() {
            linker_flags.push("-Wl,--no-entry".to_string());
            linker_flags.push("-Wl,--export-all".to_string());
        } else if target.is_bare_metal() {
            linker_flags.push("-nostdlib".to_string());
            linker_flags.push("-ffreestanding".to_string());
        }

        CrossCompilePlan {
            target: target.clone(),
            clang_target_flag,
            linker_flags,
            sysroot_path,
            emulator: target.recommended_emulator().map(|s| s.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_triples() {
        let x86_win = TargetTriple::parse("x86_64-windows-msvc").unwrap();
        assert_eq!(x86_win.arch, TargetArch::X86_64);
        assert_eq!(x86_win.os, TargetOs::Windows);
        assert_eq!(x86_win.pointer_width_bits(), 64);
        assert!(!x86_win.is_bare_metal());

        let arm_linux = TargetTriple::parse("aarch64-linux-gnu").unwrap();
        assert_eq!(arm_linux.arch, TargetArch::Aarch64);
        assert_eq!(arm_linux.os, TargetOs::Linux);
        assert_eq!(arm_linux.recommended_emulator(), Some("qemu-aarch64"));

        let wasm = TargetTriple::parse("wasm32-wasip2").unwrap();
        assert_eq!(wasm.arch, TargetArch::Wasm32);
        assert_eq!(wasm.pointer_width_bits(), 32);
        assert!(wasm.is_wasm());
        assert_eq!(wasm.recommended_emulator(), Some("wasmtime"));

        let riscv_bare = TargetTriple::parse("riscv32-none-eabi").unwrap();
        assert_eq!(riscv_bare.arch, TargetArch::Riscv32);
        assert!(riscv_bare.is_bare_metal());
    }

    #[test]
    fn test_cross_compile_planner_wasm_and_riscv() {
        let sysroot_mgr = SysrootManager::new("/tmp/agam-sysroots");

        let wasm = TargetTriple::parse("wasm32-wasip2").unwrap();
        let wasm_plan = CrossCompilePlanner::plan(&wasm, &sysroot_mgr);
        assert_eq!(wasm_plan.clang_target_flag, "--target=wasm32-wasip2");
        assert!(
            wasm_plan
                .linker_flags
                .contains(&"-Wl,--export-all".to_string())
        );
        assert_eq!(wasm_plan.emulator, Some("wasmtime".to_string()));

        let riscv = TargetTriple::parse("riscv32-none-eabi").unwrap();
        let riscv_plan = CrossCompilePlanner::plan(&riscv, &sysroot_mgr);
        assert!(riscv_plan.linker_flags.contains(&"-nostdlib".to_string()));
        assert_eq!(riscv_plan.emulator, Some("qemu-riscv32".to_string()));
    }
}
