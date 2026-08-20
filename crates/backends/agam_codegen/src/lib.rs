//! # agam_codegen
//!
//! Multi-target code generation backend for the Agam language.
//!
//! Provides:
//! - **ANSI C11 Transpiler (`c_emitter`)**
//! - **Universal GPU Target Adapters (`gpu_adapter`)** (NVPTX, AMDGPU, SPIR-V, Metal)
//! - **GPU Kernel Emitter (`gpu_emitter`)**
//! - **Typed LLVM IR Emitter (`llvm_emitter`)**

pub mod c_emitter;
pub mod gpu_adapter;
pub mod gpu_emitter;
pub mod llvm_emitter;

pub use gpu_adapter::{
    AmdgpuAdapter, GpuTargetAdapter, GpuTargetKind, MetalAdapter, NvptxAdapter, SpirvAdapter,
    adapter_for_target, adapter_from_triple,
};
