//! Target triple parsing and architecture classification.

use std::fmt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Architecture {
    X86_64,
    AArch64,
    Riscv64,
    Wasm32,
    Nvptx64,
    Unknown,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::X86_64 => write!(f, "x86_64"),
            Self::AArch64 => write!(f, "aarch64"),
            Self::Riscv64 => write!(f, "riscv64"),
            Self::Wasm32 => write!(f, "wasm32"),
            Self::Nvptx64 => write!(f, "nvptx64"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Os {
    Windows,
    Linux,
    MacOS,
    Android,
    None,
    Unknown,
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows => write!(f, "windows"),
            Self::Linux => write!(f, "linux"),
            Self::MacOS => write!(f, "macos"),
            Self::Android => write!(f, "android"),
            Self::None => write!(f, "none"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Environment {
    Msvc,
    Gnu,
    Musl,
    Cuda,
    None,
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Msvc => write!(f, "msvc"),
            Self::Gnu => write!(f, "gnu"),
            Self::Musl => write!(f, "musl"),
            Self::Cuda => write!(f, "cuda"),
            Self::None => write!(f, "none"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TargetTriple {
    pub raw: String,
    pub arch: Architecture,
    pub vendor: String,
    pub os: Os,
    pub env: Environment,
}

impl TargetTriple {
    pub fn host() -> Self {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            Self::parse("x86_64-pc-windows-msvc")
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            Self::parse("x86_64-unknown-linux-gnu")
        }
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        {
            Self::parse("aarch64-unknown-linux-gnu")
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self::parse("aarch64-apple-darwin")
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            Self::parse("x86_64-apple-darwin")
        }
        #[cfg(not(any(
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
        )))]
        {
            Self::parse("unknown-unknown-unknown")
        }
    }

    pub fn parse(triple: &str) -> Self {
        let parts: Vec<&str> = triple.split('-').collect();
        let arch = match parts.first().copied().unwrap_or("") {
            "x86_64" | "amd64" => Architecture::X86_64,
            "aarch64" | "arm64" => Architecture::AArch64,
            "riscv64" | "riscv64gc" => Architecture::Riscv64,
            "wasm32" => Architecture::Wasm32,
            "nvptx64" => Architecture::Nvptx64,
            _ => Architecture::Unknown,
        };

        let (vendor, os, env) = if parts.len() == 1 {
            ("unknown".to_string(), Os::Unknown, Environment::None)
        } else if parts.len() == 2 {
            (
                "unknown".to_string(),
                Self::parse_os(parts[1]),
                Environment::None,
            )
        } else if parts.len() == 3 {
            let p2_env = Self::parse_env(parts[2]);
            if p2_env != Environment::None {
                (parts[1].to_string(), Os::None, p2_env)
            } else {
                (
                    parts[1].to_string(),
                    Self::parse_os(parts[2]),
                    Environment::None,
                )
            }
        } else {
            (
                parts[1].to_string(),
                Self::parse_os(parts[2]),
                Self::parse_env(parts[3]),
            )
        };

        Self {
            raw: triple.to_string(),
            arch,
            vendor,
            os,
            env,
        }
    }

    fn parse_os(s: &str) -> Os {
        match s.to_ascii_lowercase().as_str() {
            "windows" | "win32" => Os::Windows,
            "linux" => Os::Linux,
            "darwin" | "macos" => Os::MacOS,
            "android" => Os::Android,
            "none" => Os::None,
            _ => Os::Unknown,
        }
    }

    fn parse_env(s: &str) -> Environment {
        match s.to_ascii_lowercase().as_str() {
            "msvc" => Environment::Msvc,
            "gnu" => Environment::Gnu,
            "musl" => Environment::Musl,
            "cuda" => Environment::Cuda,
            _ => Environment::None,
        }
    }

    pub fn is_windows(&self) -> bool {
        self.os == Os::Windows
    }

    pub fn is_linux(&self) -> bool {
        self.os == Os::Linux
    }

    pub fn is_macos(&self) -> bool {
        self.os == Os::MacOS
    }

    pub fn is_gpu(&self) -> bool {
        self.arch == Architecture::Nvptx64
    }
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl FromStr for TargetTriple {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::parse(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host_triples() {
        let win = TargetTriple::parse("x86_64-pc-windows-msvc");
        assert_eq!(win.arch, Architecture::X86_64);
        assert_eq!(win.os, Os::Windows);
        assert_eq!(win.env, Environment::Msvc);

        let linux = TargetTriple::parse("aarch64-unknown-linux-gnu");
        assert_eq!(linux.arch, Architecture::AArch64);
        assert_eq!(linux.os, Os::Linux);
        assert_eq!(linux.env, Environment::Gnu);

        let gpu = TargetTriple::parse("nvptx64-nvidia-cuda");
        assert_eq!(gpu.arch, Architecture::Nvptx64);
        assert_eq!(gpu.env, Environment::Cuda);
    }
}
