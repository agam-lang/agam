//! Target architecture classification and host toolchain discovery for Agam.

#![deny(clippy::unwrap_used)]

pub mod discovery;
pub mod triple;

pub use discovery::{
    bundled_llvm_candidate_paths, bundled_llvm_platform_dir, classify_llvm_target_platform,
    command_exists, configured_llvm_clang, configured_llvm_clang_override,
    discover_bundled_llvm_clang, discover_standalone_windows_llvm_clang,
    discover_visual_studio_installation_path, discover_visual_studio_llvm_clang, env_path,
    find_android_ndk, find_msvc_toolchain, find_native_clang, host_llvm_target_platform,
    llvm_driver_file_names, native_llvm_clang_candidates, resolve_native_llvm_command,
    standalone_windows_llvm_candidate_paths, standalone_windows_llvm_install_roots,
    visual_studio_llvm_candidate_paths, windows_vswhere_path, ClangToolchain, LlvmTargetPlatform,
    MsvcToolchain, ToolchainDiscoveryError, LLVM_BUNDLE_DIR_ENV, LLVM_CLANG_ENV,
};
pub use triple::{Architecture, Environment, Os, TargetTriple};
