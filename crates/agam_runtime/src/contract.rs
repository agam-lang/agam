//! Shared package/runtime compatibility contract.

use std::env;
use std::mem;

use serde::{Deserialize, Serialize};

pub const RUNTIME_ABI_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuntimeBackend {
    #[default]
    Auto,
    C,
    Llvm,
    Jit,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeAbi {
    pub version: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeHost {
    pub os: String,
    pub arch: String,
    pub pointer_width: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeRequirements {
    pub preferred_backend: RuntimeBackend,
    pub host_native_only: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeManifest {
    pub abi: RuntimeAbi,
    pub build_host: RuntimeHost,
    pub requirements: RuntimeRequirements,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageLoadPlan {
    pub backend: RuntimeBackend,
    pub host: RuntimeHost,
}

pub fn host_runtime() -> RuntimeHost {
    RuntimeHost {
        os: env::consts::OS.into(),
        arch: env::consts::ARCH.into(),
        pointer_width: (mem::size_of::<usize>() * 8) as u8,
    }
}

pub fn portable_runtime_manifest(
    preferred_backend: RuntimeBackend,
    host_native_only: bool,
) -> RuntimeManifest {
    RuntimeManifest {
        abi: RuntimeAbi {
            version: RUNTIME_ABI_VERSION,
        },
        build_host: host_runtime(),
        requirements: RuntimeRequirements {
            preferred_backend,
            host_native_only,
        },
    }
}

pub fn plan_package_load(
    manifest: &RuntimeManifest,
    requested_backend: RuntimeBackend,
    host: &RuntimeHost,
) -> Result<PackageLoadPlan, String> {
    if manifest.abi.version != RUNTIME_ABI_VERSION {
        return Err(format!(
            "runtime ABI mismatch: package uses v{} but host supports v{}",
            manifest.abi.version, RUNTIME_ABI_VERSION
        ));
    }

    if manifest.requirements.host_native_only && manifest.build_host != *host {
        return Err(format!(
            "package requires its original host ({}/{}/{}-bit) but the current host is {}/{}/{}-bit",
            manifest.build_host.os,
            manifest.build_host.arch,
            manifest.build_host.pointer_width,
            host.os,
            host.arch,
            host.pointer_width
        ));
    }

    let backend = match requested_backend {
        RuntimeBackend::Auto => match manifest.requirements.preferred_backend {
            RuntimeBackend::Auto => RuntimeBackend::Jit,
            preferred => preferred,
        },
        explicit => explicit,
    };

    Ok(PackageLoadPlan {
        backend,
        host: host.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_manifest_uses_current_abi_and_host() {
        let manifest = portable_runtime_manifest(RuntimeBackend::Jit, true);
        assert_eq!(manifest.abi.version, RUNTIME_ABI_VERSION);
        assert_eq!(manifest.requirements.preferred_backend, RuntimeBackend::Jit);
        assert_eq!(manifest.build_host, host_runtime());
    }

    #[test]
    fn plan_package_load_uses_manifest_backend_for_auto() {
        let manifest = portable_runtime_manifest(RuntimeBackend::Llvm, false);
        let host = host_runtime();
        let plan = plan_package_load(&manifest, RuntimeBackend::Auto, &host)
            .expect("package load plan should resolve");
        assert_eq!(plan.backend, RuntimeBackend::Llvm);
        assert_eq!(plan.host, host);
    }

    #[test]
    fn plan_package_load_rejects_host_native_mismatch() {
        let manifest = portable_runtime_manifest(RuntimeBackend::Jit, true);
        let mismatched_host = RuntimeHost {
            os: "linux".into(),
            arch: "x86_64".into(),
            pointer_width: 64,
        };

        let error =
            plan_package_load(&manifest, RuntimeBackend::Auto, &mismatched_host).unwrap_err();
        assert!(error.contains("package requires its original host"));
    }
}
