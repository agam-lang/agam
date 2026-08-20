//! Universal GPU Target Adapter Interface.
//!
//! Provides the abstract `GpuTargetAdapter` trait and concrete implementations
//! for NVPTX (Nvidia CUDA), AMDGPU (AMD ROCm/HIP), SPIR-V (Vulkan/oneAPI), and
//! Metal (Apple Silicon).

use agam_mir::ir::GpuIntrinsicKind;
use std::fmt::Display;

/// Supported GPU Hardware and Compilation Targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GpuTargetKind {
    /// Nvidia CUDA NVPTX64 backend.
    Nvptx,
    /// AMD ROCm / HIP GCN/RDNA backend.
    Amdgpu,
    /// Vulkan / oneAPI / OpenCL SPIR-V backend.
    Spirv,
    /// Apple Silicon Metal Shading Language / AIR backend.
    Metal,
}

impl Display for GpuTargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nvptx => write!(f, "nvptx64 (Nvidia CUDA)"),
            Self::Amdgpu => write!(f, "amdgcn (AMD ROCm/HIP)"),
            Self::Spirv => write!(f, "spirv64 (Vulkan/oneAPI)"),
            Self::Metal => write!(f, "air64 (Apple Metal)"),
        }
    }
}

/// Abstract adapter trait decoupling GPU MIR lowering from hardware assembly generation.
pub trait GpuTargetAdapter: Send + Sync {
    /// The target classification kind.
    fn target_kind(&self) -> GpuTargetKind;

    /// LLVM target triple string.
    fn target_triple(&self) -> &'static str;

    /// LLVM target datalayout string.
    fn target_datalayout(&self) -> &'static str;

    /// Calling convention keyword for GPU kernel entry points.
    fn kernel_calling_convention(&self) -> &'static str;

    /// Address space index for threadgroup/workgroup shared memory.
    fn shared_memory_addrspace(&self) -> u32;

    /// Emit target-specific header and intrinsic function declarations.
    fn emit_intrinsics_header(&self, buf: &mut String);

    /// Map a high-level GPU intrinsic to the target-specific intrinsic symbol.
    fn map_intrinsic_symbol(&self, kind: GpuIntrinsicKind) -> &'static str;

    /// Emit an intra-block thread execution barrier.
    fn emit_barrier(&self, buf: &mut String);

    /// Emit a warp/wavefront shuffle down operation.
    fn emit_warp_shuffle_down(
        &self,
        result_reg: &str,
        var: &str,
        delta: u32,
        width: u32,
        buf: &mut String,
    );

    /// Emit host-side runtime driver API declarations into host LLVM module.
    fn emit_host_runtime_declarations(&self, buf: &mut String);

    /// Target-specific linker flags required for the host executable.
    fn linker_flags(&self) -> Vec<String>;
}

// ══════════════════════════════════════════════════════════════════════
// 1. Nvidia NVPTX64 (CUDA) Adapter
// ══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, Default)]
pub struct NvptxAdapter;

impl GpuTargetAdapter for NvptxAdapter {
    fn target_kind(&self) -> GpuTargetKind {
        GpuTargetKind::Nvptx
    }

    fn target_triple(&self) -> &'static str {
        "nvptx64-nvidia-cuda"
    }

    fn target_datalayout(&self) -> &'static str {
        "e-p:64:64:64-i1:8:8-i8:8:8-i16:16:16-i32:32:32-i64:64:64-f32:32:32-f64:64:64-v16:16:16-v32:32:32-v64:64:64-v128:128:128-n16:32:64"
    }

    fn kernel_calling_convention(&self) -> &'static str {
        "ptx_kernel"
    }

    fn shared_memory_addrspace(&self) -> u32 {
        3
    }

    fn emit_intrinsics_header(&self, buf: &mut String) {
        buf.push_str(
            "\
; ── NVVM Thread Intrinsics ──
declare i32 @llvm.nvvm.read.ptx.sreg.tid.x() nounwind readnone
declare i32 @llvm.nvvm.read.ptx.sreg.tid.y() nounwind readnone
declare i32 @llvm.nvvm.read.ptx.sreg.tid.z() nounwind readnone
declare i32 @llvm.nvvm.read.ptx.sreg.ntid.x() nounwind readnone
declare i32 @llvm.nvvm.read.ptx.sreg.ntid.y() nounwind readnone
declare i32 @llvm.nvvm.read.ptx.sreg.ntid.z() nounwind readnone
declare i32 @llvm.nvvm.read.ptx.sreg.ctaid.x() nounwind readnone
declare i32 @llvm.nvvm.read.ptx.sreg.ctaid.y() nounwind readnone
declare i32 @llvm.nvvm.read.ptx.sreg.ctaid.z() nounwind readnone
declare i32 @llvm.nvvm.read.ptx.sreg.nctaid.x() nounwind readnone
declare void @llvm.nvvm.barrier0() nounwind
declare i32 @llvm.nvvm.shfl.sync.down.i32(i32, i32, i32, i32) nounwind readnone

; ── NVVM Math Builtins ──
declare float @llvm.sin.f32(float) nounwind readnone
declare float @llvm.cos.f32(float) nounwind readnone
declare float @llvm.sqrt.f32(float) nounwind readnone
declare float @llvm.exp.f32(float) nounwind readnone
declare double @llvm.sin.f64(double) nounwind readnone
declare double @llvm.cos.f64(double) nounwind readnone
declare double @llvm.sqrt.f64(double) nounwind readnone
declare double @llvm.exp.f64(double) nounwind readnone
",
        );
    }

    fn map_intrinsic_symbol(&self, kind: GpuIntrinsicKind) -> &'static str {
        match kind {
            GpuIntrinsicKind::ThreadIdX => "@llvm.nvvm.read.ptx.sreg.tid.x()",
            GpuIntrinsicKind::ThreadIdY => "@llvm.nvvm.read.ptx.sreg.tid.y()",
            GpuIntrinsicKind::ThreadIdZ => "@llvm.nvvm.read.ptx.sreg.tid.z()",
            GpuIntrinsicKind::BlockDimX => "@llvm.nvvm.read.ptx.sreg.ntid.x()",
            GpuIntrinsicKind::BlockDimY => "@llvm.nvvm.read.ptx.sreg.ntid.y()",
            GpuIntrinsicKind::BlockDimZ => "@llvm.nvvm.read.ptx.sreg.ntid.z()",
            GpuIntrinsicKind::BlockIdX => "@llvm.nvvm.read.ptx.sreg.ctaid.x()",
            GpuIntrinsicKind::BlockIdY => "@llvm.nvvm.read.ptx.sreg.ctaid.y()",
            GpuIntrinsicKind::BlockIdZ => "@llvm.nvvm.read.ptx.sreg.ctaid.z()",
            GpuIntrinsicKind::Barrier => "@llvm.nvvm.barrier0()",
            GpuIntrinsicKind::WarpShuffleDown => "@llvm.nvvm.shfl.sync.down.i32",
            GpuIntrinsicKind::WarpReduceAdd => "@llvm.nvvm.shfl.sync.down.i32",
            GpuIntrinsicKind::BallotSync => "@llvm.nvvm.vote.ballot.sync",
            GpuIntrinsicKind::NvvmSin => "@llvm.sin.f32",
            GpuIntrinsicKind::NvvmCos => "@llvm.cos.f32",
            GpuIntrinsicKind::NvvmSqrt => "@llvm.sqrt.f32",
            GpuIntrinsicKind::NvvmExp => "@llvm.exp.f32",
            GpuIntrinsicKind::CooperativeMatrixLoad => "@llvm.nvvm.wmma.load",
            GpuIntrinsicKind::CooperativeMatrixStore => "@llvm.nvvm.wmma.store",
            GpuIntrinsicKind::CooperativeMatrixMulAdd => "@llvm.nvvm.wmma.mma",
            GpuIntrinsicKind::CooperativeMatrixLength => "@llvm.nvvm.wmma.length",
        }
    }

    fn emit_barrier(&self, buf: &mut String) {
        buf.push_str("  call void @llvm.nvvm.barrier0()\n");
    }

    fn emit_warp_shuffle_down(
        &self,
        result_reg: &str,
        var: &str,
        delta: u32,
        width: u32,
        buf: &mut String,
    ) {
        let mask = if width >= 32 {
            0xFFFFFFFFu32
        } else {
            (1u32 << width) - 1
        };
        buf.push_str(&format!(
            "  {result_reg} = call i32 @llvm.nvvm.shfl.sync.down.i32(i32 {mask}, i32 {var}, i32 {delta}, i32 {width})\n"
        ));
    }

    fn emit_host_runtime_declarations(&self, buf: &mut String) {
        buf.push_str(
            "\
; ── CUDA Host Runtime API Declarations ──
declare i32 @cudaMalloc(ptr, i64)
declare i32 @cudaFree(ptr)
declare i32 @cudaMemcpy(ptr, ptr, i64, i32)
declare i32 @cudaLaunchKernel(ptr, i64, i32, i64, i32, ptr, i64, ptr)
declare i32 @cudaDeviceSynchronize()
",
        );
    }

    fn linker_flags(&self) -> Vec<String> {
        vec![
            "-lcudart".into(),
            "-lnvvm".into(),
            "-lnvptxcompiler_static".into(),
        ]
    }
}

// ══════════════════════════════════════════════════════════════════════
// 2. AMD AMDGPU (ROCm / HIP) Adapter
// ══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, Default)]
pub struct AmdgpuAdapter;

impl GpuTargetAdapter for AmdgpuAdapter {
    fn target_kind(&self) -> GpuTargetKind {
        GpuTargetKind::Amdgpu
    }

    fn target_triple(&self) -> &'static str {
        "amdgcn-amd-amdhsa"
    }

    fn target_datalayout(&self) -> &'static str {
        "e-p:64:64-p1:64:64-p2:32:32-p3:32:32-p4:64:64-p5:32:32-p6:32:32-i64:64-v16:16-v24:32-v32:32-v48:64-v64:64-v96:128-v128:128-v192:256-v256:256-v512:512-v1024:1024-v2048:2048-n32:64-S32-A5-G1-ni:7:8:9"
    }

    fn kernel_calling_convention(&self) -> &'static str {
        "amdgpu_kernel"
    }

    fn shared_memory_addrspace(&self) -> u32 {
        3 // AMD LDS (Local Data Share)
    }

    fn emit_intrinsics_header(&self, buf: &mut String) {
        buf.push_str(
            "\
; ── AMDGPU Workitem Intrinsics ──
declare i32 @llvm.amdgcn.workitem.id.x() nounwind readnone
declare i32 @llvm.amdgcn.workitem.id.y() nounwind readnone
declare i32 @llvm.amdgcn.workitem.id.z() nounwind readnone
declare i32 @llvm.amdgcn.workgroup.id.x() nounwind readnone
declare i32 @llvm.amdgcn.workgroup.id.y() nounwind readnone
declare i32 @llvm.amdgcn.workgroup.id.z() nounwind readnone
declare void @llvm.amdgcn.s.barrier() nounwind
declare i32 @llvm.amdgcn.ds.bpermute(i32, i32) nounwind readnone

; ── AMDGPU Math Builtins ──
declare float @llvm.sin.f32(float) nounwind readnone
declare float @llvm.cos.f32(float) nounwind readnone
declare float @llvm.sqrt.f32(float) nounwind readnone
declare float @llvm.exp.f32(float) nounwind readnone
declare double @llvm.sin.f64(double) nounwind readnone
declare double @llvm.cos.f64(double) nounwind readnone
declare double @llvm.sqrt.f64(double) nounwind readnone
declare double @llvm.exp.f64(double) nounwind readnone
",
        );
    }

    fn map_intrinsic_symbol(&self, kind: GpuIntrinsicKind) -> &'static str {
        match kind {
            GpuIntrinsicKind::ThreadIdX => "@llvm.amdgcn.workitem.id.x()",
            GpuIntrinsicKind::ThreadIdY => "@llvm.amdgcn.workitem.id.y()",
            GpuIntrinsicKind::ThreadIdZ => "@llvm.amdgcn.workitem.id.z()",
            GpuIntrinsicKind::BlockDimX => "@llvm.amdgcn.workitem.id.x()",
            GpuIntrinsicKind::BlockDimY => "@llvm.amdgcn.workitem.id.y()",
            GpuIntrinsicKind::BlockDimZ => "@llvm.amdgcn.workitem.id.z()",
            GpuIntrinsicKind::BlockIdX => "@llvm.amdgcn.workgroup.id.x()",
            GpuIntrinsicKind::BlockIdY => "@llvm.amdgcn.workgroup.id.y()",
            GpuIntrinsicKind::BlockIdZ => "@llvm.amdgcn.workgroup.id.z()",
            GpuIntrinsicKind::Barrier => "@llvm.amdgcn.s.barrier()",
            GpuIntrinsicKind::WarpShuffleDown => "@llvm.amdgcn.ds.bpermute",
            GpuIntrinsicKind::WarpReduceAdd => "@llvm.amdgcn.ds.bpermute",
            GpuIntrinsicKind::BallotSync => "@llvm.amdgcn.ballot.i64",
            GpuIntrinsicKind::NvvmSin => "@llvm.sin.f32",
            GpuIntrinsicKind::NvvmCos => "@llvm.cos.f32",
            GpuIntrinsicKind::NvvmSqrt => "@llvm.sqrt.f32",
            GpuIntrinsicKind::NvvmExp => "@llvm.exp.f32",
            GpuIntrinsicKind::CooperativeMatrixLoad => "@llvm.amdgcn.mfma.load",
            GpuIntrinsicKind::CooperativeMatrixStore => "@llvm.amdgcn.mfma.store",
            GpuIntrinsicKind::CooperativeMatrixMulAdd => "@llvm.amdgcn.mfma.f32.16x16x16f16",
            GpuIntrinsicKind::CooperativeMatrixLength => "@llvm.amdgcn.mfma.length",
        }
    }

    fn emit_barrier(&self, buf: &mut String) {
        buf.push_str("  call void @llvm.amdgcn.s.barrier()\n");
    }

    fn emit_warp_shuffle_down(
        &self,
        result_reg: &str,
        var: &str,
        delta: u32,
        _width: u32,
        buf: &mut String,
    ) {
        let byte_offset = delta * 4;
        buf.push_str(&format!(
            "  {result_reg} = call i32 @llvm.amdgcn.ds.bpermute(i32 {byte_offset}, i32 {var})\n"
        ));
    }

    fn emit_host_runtime_declarations(&self, buf: &mut String) {
        buf.push_str(
            "\
; ── HIP / ROCm Host Runtime API Declarations ──
declare i32 @hipMalloc(ptr, i64)
declare i32 @hipFree(ptr)
declare i32 @hipMemcpy(ptr, ptr, i64, i32)
declare i32 @hipLaunchKernel(ptr, i64, i32, i64, i32, ptr, i64, ptr)
declare i32 @hipDeviceSynchronize()
",
        );
    }

    fn linker_flags(&self) -> Vec<String> {
        vec!["-lamdhip64".into(), "-lhsa-runtime64".into()]
    }
}

// ══════════════════════════════════════════════════════════════════════
// 3. SPIR-V (Vulkan / oneAPI / OpenCL) Adapter
// ══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, Default)]
pub struct SpirvAdapter;

impl GpuTargetAdapter for SpirvAdapter {
    fn target_kind(&self) -> GpuTargetKind {
        GpuTargetKind::Spirv
    }

    fn target_triple(&self) -> &'static str {
        "spirv64-unknown-unknown"
    }

    fn target_datalayout(&self) -> &'static str {
        "e-i64:64-v16:16-v24:32-v32:32-v48:64-v64:64-v96:128-v128:128-v192:256-v256:256-v512:512-v1024:1024-n8:16:32:64"
    }

    fn kernel_calling_convention(&self) -> &'static str {
        "spir_kernel"
    }

    fn shared_memory_addrspace(&self) -> u32 {
        3 // Workgroup Local
    }

    fn emit_intrinsics_header(&self, buf: &mut String) {
        buf.push_str(
            "\
; ── SPIR-V BuiltIn Intrinsics ──
declare <3 x i64> @__spirv_BuiltInLocalInvocationId() nounwind readnone
declare <3 x i64> @__spirv_BuiltInWorkgroupId() nounwind readnone
declare <3 x i64> @__spirv_BuiltInWorkgroupSize() nounwind readnone
declare <3 x i64> @__spirv_BuiltInNumWorkgroups() nounwind readnone
declare void @__spirv_ControlBarrier(i32, i32, i32) nounwind

; ── SPIR-V Math Builtins ──
declare float @llvm.sin.f32(float) nounwind readnone
declare float @llvm.cos.f32(float) nounwind readnone
declare float @llvm.sqrt.f32(float) nounwind readnone
declare float @llvm.exp.f32(float) nounwind readnone
declare double @llvm.sin.f64(double) nounwind readnone
declare double @llvm.cos.f64(double) nounwind readnone
declare double @llvm.sqrt.f64(double) nounwind readnone
declare double @llvm.exp.f64(double) nounwind readnone
",
        );
    }

    fn map_intrinsic_symbol(&self, kind: GpuIntrinsicKind) -> &'static str {
        match kind {
            GpuIntrinsicKind::ThreadIdX => "@__spirv_BuiltInLocalInvocationId",
            GpuIntrinsicKind::ThreadIdY => "@__spirv_BuiltInLocalInvocationId",
            GpuIntrinsicKind::ThreadIdZ => "@__spirv_BuiltInLocalInvocationId",
            GpuIntrinsicKind::BlockDimX => "@__spirv_BuiltInWorkgroupSize",
            GpuIntrinsicKind::BlockDimY => "@__spirv_BuiltInWorkgroupSize",
            GpuIntrinsicKind::BlockDimZ => "@__spirv_BuiltInWorkgroupSize",
            GpuIntrinsicKind::BlockIdX => "@__spirv_BuiltInWorkgroupId",
            GpuIntrinsicKind::BlockIdY => "@__spirv_BuiltInWorkgroupId",
            GpuIntrinsicKind::BlockIdZ => "@__spirv_BuiltInWorkgroupId",
            GpuIntrinsicKind::Barrier => "@__spirv_ControlBarrier",
            GpuIntrinsicKind::WarpShuffleDown => "@__spirv_GroupNonUniformShuffleDown",
            GpuIntrinsicKind::WarpReduceAdd => "@__spirv_GroupNonUniformIAdd",
            GpuIntrinsicKind::BallotSync => "@__spirv_GroupNonUniformBallot",
            GpuIntrinsicKind::NvvmSin => "@llvm.sin.f32",
            GpuIntrinsicKind::NvvmCos => "@llvm.cos.f32",
            GpuIntrinsicKind::NvvmSqrt => "@llvm.sqrt.f32",
            GpuIntrinsicKind::NvvmExp => "@llvm.exp.f32",
            GpuIntrinsicKind::CooperativeMatrixLoad => "@spirv.CooperativeMatrixLoadKHR",
            GpuIntrinsicKind::CooperativeMatrixStore => "@spirv.CooperativeMatrixStoreKHR",
            GpuIntrinsicKind::CooperativeMatrixMulAdd => "@spirv.CooperativeMatrixMulAddKHR",
            GpuIntrinsicKind::CooperativeMatrixLength => "@spirv.CooperativeMatrixLengthKHR",
        }
    }

    fn emit_barrier(&self, buf: &mut String) {
        // Scope 2 (Workgroup), Scope 2 (Workgroup), Semantics 0x100 (AcquireRelease | WorkgroupMemory)
        buf.push_str("  call void @__spirv_ControlBarrier(i32 2, i32 2, i32 256)\n");
    }

    fn emit_warp_shuffle_down(
        &self,
        result_reg: &str,
        var: &str,
        delta: u32,
        _width: u32,
        buf: &mut String,
    ) {
        buf.push_str(&format!(
            "  {result_reg} = call i32 @__spirv_GroupNonUniformShuffleDown(i32 3, i32 {var}, i32 {delta})\n"
        ));
    }

    fn emit_host_runtime_declarations(&self, buf: &mut String) {
        buf.push_str(
            "\
; ── Vulkan / SPIR-V Host Runtime API Declarations ──
declare ptr @vkCreateInstance(ptr, ptr, ptr)
declare i32 @vkCreateComputePipelines(ptr, i64, i32, ptr, ptr, ptr)
declare i32 @vkQueueSubmit(ptr, i32, ptr, i64)
declare i32 @vkQueueWaitIdle(ptr)
",
        );
    }

    fn linker_flags(&self) -> Vec<String> {
        vec!["-lSPIRV-Tools".into(), "-lvulkan".into()]
    }
}

// ══════════════════════════════════════════════════════════════════════
// 4. Apple Silicon Metal (AIR / MSL) Adapter
// ══════════════════════════════════════════════════════════════════════

#[derive(Clone, Copy, Debug, Default)]
pub struct MetalAdapter;

impl GpuTargetAdapter for MetalAdapter {
    fn target_kind(&self) -> GpuTargetKind {
        GpuTargetKind::Metal
    }

    fn target_triple(&self) -> &'static str {
        "air64-apple-macosx"
    }

    fn target_datalayout(&self) -> &'static str {
        "e-m:o-i64:64-f80:128-n8:16:32:64-S128"
    }

    fn kernel_calling_convention(&self) -> &'static str {
        "metal_kernel"
    }

    fn shared_memory_addrspace(&self) -> u32 {
        3 // Metal threadgroup memory
    }

    fn emit_intrinsics_header(&self, buf: &mut String) {
        buf.push_str(
            "\
; ── Metal AIR Threadgroup Intrinsics ──
declare i32 @air.thread_position_in_threadgroup.x() nounwind readnone
declare i32 @air.thread_position_in_threadgroup.y() nounwind readnone
declare i32 @air.thread_position_in_threadgroup.z() nounwind readnone
declare i32 @air.threads_per_threadgroup.x() nounwind readnone
declare i32 @air.threads_per_threadgroup.y() nounwind readnone
declare i32 @air.threads_per_threadgroup.z() nounwind readnone
declare i32 @air.threadgroup_position_in_grid.x() nounwind readnone
declare i32 @air.threadgroup_position_in_grid.y() nounwind readnone
declare i32 @air.threadgroup_position_in_grid.z() nounwind readnone
declare i32 @air.threadgroups_per_grid.x() nounwind readnone
declare void @air.threadgroup_barrier(i32) nounwind
declare i32 @air.simdgroup_shuffle_down(i32, i32) nounwind readnone

; ── Metal Math Builtins ──
declare float @llvm.sin.f32(float) nounwind readnone
declare float @llvm.cos.f32(float) nounwind readnone
declare float @llvm.sqrt.f32(float) nounwind readnone
declare float @llvm.exp.f32(float) nounwind readnone
declare double @llvm.sin.f64(double) nounwind readnone
declare double @llvm.cos.f64(double) nounwind readnone
declare double @llvm.sqrt.f64(double) nounwind readnone
declare double @llvm.exp.f64(double) nounwind readnone
",
        );
    }

    fn map_intrinsic_symbol(&self, kind: GpuIntrinsicKind) -> &'static str {
        match kind {
            GpuIntrinsicKind::ThreadIdX => "@air.thread_position_in_threadgroup.x()",
            GpuIntrinsicKind::ThreadIdY => "@air.thread_position_in_threadgroup.y()",
            GpuIntrinsicKind::ThreadIdZ => "@air.thread_position_in_threadgroup.z()",
            GpuIntrinsicKind::BlockDimX => "@air.threads_per_threadgroup.x()",
            GpuIntrinsicKind::BlockDimY => "@air.threads_per_threadgroup.y()",
            GpuIntrinsicKind::BlockDimZ => "@air.threads_per_threadgroup.z()",
            GpuIntrinsicKind::BlockIdX => "@air.threadgroup_position_in_grid.x()",
            GpuIntrinsicKind::BlockIdY => "@air.threadgroup_position_in_grid.y()",
            GpuIntrinsicKind::BlockIdZ => "@air.threadgroup_position_in_grid.z()",
            GpuIntrinsicKind::Barrier => "@air.threadgroup_barrier",
            GpuIntrinsicKind::WarpShuffleDown => "@air.simdgroup_shuffle_down",
            GpuIntrinsicKind::WarpReduceAdd => "@air.simdgroup_shuffle_down",
            GpuIntrinsicKind::BallotSync => "@air.simdgroup_ballot",
            GpuIntrinsicKind::NvvmSin => "@llvm.sin.f32",
            GpuIntrinsicKind::NvvmCos => "@llvm.cos.f32",
            GpuIntrinsicKind::NvvmSqrt => "@llvm.sqrt.f32",
            GpuIntrinsicKind::NvvmExp => "@llvm.exp.f32",
            GpuIntrinsicKind::CooperativeMatrixLoad => "@air.simdgroup_matrix.load",
            GpuIntrinsicKind::CooperativeMatrixStore => "@air.simdgroup_matrix.store",
            GpuIntrinsicKind::CooperativeMatrixMulAdd => {
                "@air.simdgroup_matrix.multiply_accumulate"
            }
            GpuIntrinsicKind::CooperativeMatrixLength => "@air.simdgroup_matrix.length",
        }
    }

    fn emit_barrier(&self, buf: &mut String) {
        // Memory flags: 1 = mem_threadgroup
        buf.push_str("  call void @air.threadgroup_barrier(i32 1)\n");
    }

    fn emit_warp_shuffle_down(
        &self,
        result_reg: &str,
        var: &str,
        delta: u32,
        _width: u32,
        buf: &mut String,
    ) {
        buf.push_str(&format!(
            "  {result_reg} = call i32 @air.simdgroup_shuffle_down(i32 {var}, i32 {delta})\n"
        ));
    }

    fn emit_host_runtime_declarations(&self, buf: &mut String) {
        buf.push_str(
            "\
; ── Apple Metal Host Runtime API Declarations ──
declare ptr @MTLCreateSystemDefaultDevice()
declare ptr @objc_msgSend(ptr, ptr)
",
        );
    }

    fn linker_flags(&self) -> Vec<String> {
        vec![
            "-framework".into(),
            "Metal".into(),
            "-framework".into(),
            "Foundation".into(),
        ]
    }
}

/// Factory creating a target adapter for a given target kind.
pub fn adapter_for_target(kind: GpuTargetKind) -> Box<dyn GpuTargetAdapter> {
    match kind {
        GpuTargetKind::Nvptx => Box::new(NvptxAdapter),
        GpuTargetKind::Amdgpu => Box::new(AmdgpuAdapter),
        GpuTargetKind::Spirv => Box::new(SpirvAdapter),
        GpuTargetKind::Metal => Box::new(MetalAdapter),
    }
}

/// Factory resolving target adapter from a target triple string.
pub fn adapter_from_triple(triple: &str) -> Option<Box<dyn GpuTargetAdapter>> {
    if triple.contains("nvptx") || triple.contains("cuda") {
        Some(Box::new(NvptxAdapter))
    } else if triple.contains("amdgcn") || triple.contains("amdhsa") || triple.contains("rocm") {
        Some(Box::new(AmdgpuAdapter))
    } else if triple.contains("spirv") || triple.contains("vulkan") {
        Some(Box::new(SpirvAdapter))
    } else if triple.contains("air64") || triple.contains("metal") {
        Some(Box::new(MetalAdapter))
    } else {
        None
    }
}
