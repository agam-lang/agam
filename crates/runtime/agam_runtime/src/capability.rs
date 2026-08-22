//! Fine-Grained Capability & Permission System (Deno-style & WASI-style).
//!
//! Provides language-level permission enforcement, unauthorized access auditing,
//! capability attenuation, and multi-tier isolation modeling.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Isolation tier for executing untrusted workloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IsolationTier {
    /// In-process execution with no OS sandboxing.
    None,
    /// Process-level sandboxing (Win32 Job Object, Linux prctl/setrlimit).
    #[default]
    Process,
    /// OCI Container namespace isolation.
    Container,
    /// Hardware-enforced Firecracker MicroVM virtualization.
    MicroVm,
}

impl IsolationTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Process => "process",
            Self::Container => "container",
            Self::MicroVm => "microvm",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "none" => Some(Self::None),
            "process" => Some(Self::Process),
            "container" => Some(Self::Container),
            "microvm" | "vm" => Some(Self::MicroVm),
            _ => None,
        }
    }
}

/// Discrete capabilities that code can request or require.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Read access to the filesystem (optionally constrained to prefix).
    FsRead(Option<PathBuf>),
    /// Write access to the filesystem (optionally constrained to prefix).
    FsWrite(Option<PathBuf>),
    /// Outbound network connection (optionally constrained to host:port pattern).
    NetConnect(Option<String>),
    /// Inbound network listener (optionally constrained to bind address).
    NetListen(Option<String>),
    /// Read environment variables (optionally constrained to specific var name).
    EnvRead(Option<String>),
    /// Spawn external child processes.
    ProcessSpawn(Option<String>),
    /// Access GPU compute hardware.
    GpuAccess,
    /// All permissions granted (unrestricted/ambient authority).
    All,
}

/// Error returned when a required capability has not been granted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionDeniedError {
    pub requested: Capability,
    pub reason: String,
}

impl std::fmt::Display for PermissionDeniedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PermissionDenied: Unauthorized access for capability `{:?}`: {}",
            self.requested, self.reason
        )
    }
}

impl std::error::Error for PermissionDeniedError {}

/// Capability permission set and runtime checker.
#[derive(Debug, Clone, Default)]
pub struct CapabilitySet {
    granted: HashSet<Capability>,
    strict_deny_all: bool,
}

impl CapabilitySet {
    /// Create an empty (strict) capability set granting no permissions.
    pub fn empty() -> Self {
        Self {
            granted: HashSet::new(),
            strict_deny_all: false,
        }
    }

    /// Create an unrestricted capability set granting ambient authority.
    pub fn unrestricted() -> Self {
        let mut set = Self::empty();
        set.grant(Capability::All);
        set
    }

    /// Create a strict deny-all sandbox configuration.
    pub fn deny_all() -> Self {
        Self {
            granted: HashSet::new(),
            strict_deny_all: true,
        }
    }

    /// Grant a capability.
    pub fn grant(&mut self, cap: Capability) {
        if !self.strict_deny_all {
            self.granted.insert(cap);
        }
    }

    /// Check if filesystem read is allowed for `path`.
    pub fn check_fs_read(&self, path: &Path) -> Result<(), PermissionDeniedError> {
        if self.granted.contains(&Capability::All) {
            return Ok(());
        }

        for cap in &self.granted {
            if let Capability::FsRead(allowed_prefix) = cap {
                match allowed_prefix {
                    None => return Ok(()),
                    Some(prefix) if path.starts_with(prefix) => return Ok(()),
                    _ => {}
                }
            }
        }

        Err(PermissionDeniedError {
            requested: Capability::FsRead(Some(path.to_path_buf())),
            reason: format!(
                "Read access to `{}` is denied. Run with `--allow-fs-read={}` to grant permission.",
                path.display(),
                path.display()
            ),
        })
    }

    /// Check if filesystem write is allowed for `path`.
    pub fn check_fs_write(&self, path: &Path) -> Result<(), PermissionDeniedError> {
        if self.granted.contains(&Capability::All) {
            return Ok(());
        }

        for cap in &self.granted {
            if let Capability::FsWrite(allowed_prefix) = cap {
                match allowed_prefix {
                    None => return Ok(()),
                    Some(prefix) if path.starts_with(prefix) => return Ok(()),
                    _ => {}
                }
            }
        }

        Err(PermissionDeniedError {
            requested: Capability::FsWrite(Some(path.to_path_buf())),
            reason: format!(
                "Write access to `{}` is denied. Run with `--allow-fs-write={}` to grant permission.",
                path.display(),
                path.display()
            ),
        })
    }

    /// Check if network connection to `target` (host:port) is allowed.
    pub fn check_net_connect(&self, target: &str) -> Result<(), PermissionDeniedError> {
        if self.granted.contains(&Capability::All) {
            return Ok(());
        }

        for cap in &self.granted {
            if let Capability::NetConnect(allowed_pattern) = cap {
                match allowed_pattern {
                    None => return Ok(()),
                    Some(pattern) if target.contains(pattern) => return Ok(()),
                    _ => {}
                }
            }
        }

        Err(PermissionDeniedError {
            requested: Capability::NetConnect(Some(target.to_string())),
            reason: format!(
                "Outbound network connect to `{target}` is denied. Run with `--allow-net={target}` to grant permission."
            ),
        })
    }

    /// Check if child process execution is allowed.
    pub fn check_process_spawn(&self, cmd: &str) -> Result<(), PermissionDeniedError> {
        if self.granted.contains(&Capability::All) {
            return Ok(());
        }

        for cap in &self.granted {
            if let Capability::ProcessSpawn(allowed_cmd) = cap {
                match allowed_cmd {
                    None => return Ok(()),
                    Some(c) if c == cmd => return Ok(()),
                    _ => {}
                }
            }
        }

        Err(PermissionDeniedError {
            requested: Capability::ProcessSpawn(Some(cmd.to_string())),
            reason: format!(
                "Spawning external process `{cmd}` is denied. Run with `--allow-process={cmd}` to grant permission."
            ),
        })
    }
}

static GLOBAL_CAPABILITIES: OnceLock<Mutex<CapabilitySet>> = OnceLock::new();

/// Access global capability manager instance.
pub fn global_capabilities() -> &'static Mutex<CapabilitySet> {
    GLOBAL_CAPABILITIES.get_or_init(|| Mutex::new(CapabilitySet::unrestricted()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_permissions_allow_and_deny() {
        let mut caps = CapabilitySet::empty();
        caps.grant(Capability::FsRead(Some(PathBuf::from("/tmp"))));
        caps.grant(Capability::NetConnect(Some(
            "api.agam-lang.org".to_string(),
        )));

        // Allowed
        assert!(caps.check_fs_read(Path::new("/tmp/data.txt")).is_ok());
        assert!(caps.check_net_connect("api.agam-lang.org:443").is_ok());

        // Denied
        assert!(caps.check_fs_read(Path::new("/etc/passwd")).is_err());
        assert!(caps.check_fs_write(Path::new("/tmp/data.txt")).is_err());
        assert!(caps.check_net_connect("evil.com:80").is_err());
        assert!(caps.check_process_spawn("sh").is_err());
    }

    #[test]
    fn test_isolation_tier_parsing() {
        assert_eq!(
            IsolationTier::parse("process"),
            Some(IsolationTier::Process)
        );
        assert_eq!(
            IsolationTier::parse("microvm"),
            Some(IsolationTier::MicroVm)
        );
        assert_eq!(
            IsolationTier::parse("container"),
            Some(IsolationTier::Container)
        );
        assert_eq!(IsolationTier::parse("none"), Some(IsolationTier::None));
        assert_eq!(IsolationTier::parse("invalid"), None);
    }
}
