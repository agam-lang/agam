//! Target architecture classification and host toolchain discovery for Agam.

#![deny(clippy::unwrap_used)]

pub mod discovery;
pub mod triple;

pub use discovery::{
    find_android_ndk, find_bundled_llvm_clang, find_msvc_toolchain, find_native_clang,
    resolve_llvm_toolchain, ClangToolchain, MsvcToolchain, ToolchainDiscoveryError,
};
pub use triple::{Architecture, Environment, Os, TargetTriple};
