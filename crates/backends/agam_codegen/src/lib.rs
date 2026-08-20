//! # agam_codegen
//!
//! Multi-target code generation backend for the Agam language.
//!
//! Provides:
//! - **ANSI C11 Transpiler (`c_emitter`)**
//! - **Universal GPU Target Adapters (`gpu_adapter`)** (NVPTX, AMDGPU, SPIR-V, Metal)
//! - **GPU Kernel Emitter (`gpu_emitter`)**
//! - **Direct SPIR-V 1.5 Binary Emitter (`spirv`)**
//! - **Typed LLVM IR Emitter (`llvm_emitter`)**

pub mod c_emitter;
pub mod gpu_adapter;
pub mod gpu_emitter;
pub mod llvm_emitter;
pub mod spirv;
pub mod wasm;

pub use gpu_adapter::{
    AmdgpuAdapter, GpuTargetAdapter, GpuTargetKind, MetalAdapter, NvptxAdapter, SpirvAdapter,
    adapter_for_target, adapter_from_triple,
};
pub use spirv::{
    SPIRV_MAGIC, SPIRV_VERSION_1_5, SpirvModuleBuilder, emit_spirv_binary, emit_spirv_module,
};
pub use wasm::{WASM_MAGIC, WASM_VERSION, WasmModuleBuilder, emit_wasm_binary, emit_wit_interface};
