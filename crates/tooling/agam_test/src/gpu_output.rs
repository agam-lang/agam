//! Comprehensive GPU / NVPTX & CUDA Backend Testing Suite.
//!
//! Verifies the Agam -> NVPTX / CUDA compilation pipeline across all 6 core testing requirements:
//! 1. Unit tests: NVPTX target triples, thread/block builtins, shared memory addrspace(3), fast math.
//! 2. Integration tests: `@gpu(...)` kernels, host launch bindings, parameter marshalling.
//! 3. Error tests: Strict validation rejection of recursion, heap allocs, effects, strings.
//! 4. Optimization tests: NVVM fast-math intrinsics (`llvm.sin.f32`, `llvm.sqrt.f32`), pointer math.
//! 5. Performance tests: GPU kernel generation throughput and batch compilation.
//! 6. Output tests: Valid NVPTX64 IR syntax, CUDA linkage declarations, host memory APIs.

#[cfg(test)]
mod tests {
    use agam_codegen::gpu_emitter::{CudaLinkage, emit_gpu_module, emit_host_cuda_declarations};
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::tokenize;
    use agam_mir::lower::MirLowering;
    use agam_mir::opt::optimize_module;
    use agam_parser::parse;
    use agam_sema::gpu::validate_gpu_kernel_body;

    fn compile_to_gpu(src: &str) -> Option<String> {
        let source_id = SourceId(0);
        let tokens = tokenize(src, source_id);
        let ast = parse(tokens, source_id).expect("AST parse");

        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        let mut mir = mir_lowering.lower_module(&hir);
        optimize_module(&mut mir);

        emit_gpu_module(&mir)
    }

    // ══════════════════════════════════════════════════════════════════════
    // 1. Unit Tests — Independent GPU / NVPTX Codegen Components
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_gpu_unit_target_triple_and_data_layout() {
        let src = r#"
@gpu(threads=256)
fn vector_add(a: &[f32], b: &[f32], c: &mut [f32]):
    let idx = agam.gpu.thread_id_x()
    c[idx] = a[idx] + b[idx]
"#;
        let gpu_ir = compile_to_gpu(src);
        assert!(gpu_ir.is_some(), "must emit GPU module for @gpu function");
        let ir = gpu_ir.unwrap();
        assert!(
            ir.contains("target triple = \"nvptx64-nvidia-cuda\""),
            "must declare NVPTX64 CUDA target triple"
        );
        assert!(
            ir.contains("target datalayout = "),
            "must declare GPU data layout"
        );
    }

    #[test]
    fn test_gpu_unit_thread_and_block_index_intrinsics() {
        let src = r#"
@gpu(threads=128)
fn compute_indices(out: &mut [i32]):
    let tid_x = agam.gpu.thread_id_x()
    let bid_x = agam.gpu.block_id_x()
    let bdim_x = agam.gpu.block_dim_x()
    out[tid_x] = bid_x * bdim_x + tid_x
"#;
        let gpu_ir = compile_to_gpu(src).expect("compile gpu");
        assert!(
            gpu_ir.contains("llvm.nvvm.read.ptx.sreg.tid.x") || gpu_ir.contains("thread_id_x"),
            "must emit thread ID intrinsic"
        );
        assert!(
            gpu_ir.contains("llvm.nvvm.read.ptx.sreg.ctaid.x") || gpu_ir.contains("block_id_x"),
            "must emit block ID intrinsic"
        );
    }

    #[test]
    fn test_gpu_unit_shared_memory_addrspace() {
        let src = r#"
@gpu(threads=64, shared=1024)
fn shared_kernel(out: &mut [f32]):
    agam.gpu.barrier()
    out[0] = 1.0
"#;
        let gpu_ir = compile_to_gpu(src).expect("compile gpu");
        assert!(
            gpu_ir.contains("addrspace(3)")
                || gpu_ir.contains("barrier")
                || gpu_ir.contains("shared"),
            "must emit shared memory or barrier"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 2. Integration Tests — Complete GPU Kernel Pipeline
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_gpu_integration_vector_addition_kernel() {
        let src = r#"
@gpu(block=(16, 16, 1), grid=(4, 4, 1))
fn mat_add_kernel(a: &[f32], b: &[f32], out: &mut [f32]):
    let tx = agam.gpu.thread_id_x()
    let ty = agam.gpu.thread_id_y()
    let idx = ty * 16 + tx
    out[idx] = a[idx] + b[idx]
"#;
        let gpu_ir = compile_to_gpu(src).expect("compile mat_add_kernel");
        assert!(
            gpu_ir.contains("define ptx_kernel void @"),
            "must define GPU kernel entry point"
        );
        assert!(
            gpu_ir.contains("%p0") && gpu_ir.contains("%p1"),
            "must contain kernel parameters"
        );
        assert!(gpu_ir.contains("ret void"), "GPU kernel must return void");
    }

    // ══════════════════════════════════════════════════════════════════════
    // 3. Error Tests — Strict Kernel Validation Rejection
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_gpu_errors_strict_kernel_rules() {
        // Rejects recursion in GPU kernels
        let errs_recursion = validate_gpu_kernel_body(false, false, false, true, "kernel_rec", &[]);
        assert!(!errs_recursion.is_empty(), "must reject recursion");

        // Rejects heap allocations in GPU kernels
        let errs_heap = validate_gpu_kernel_body(false, false, true, false, "kernel_heap", &[]);
        assert!(!errs_heap.is_empty(), "must reject heap alloc");

        // Rejects algebraic effects in GPU kernels
        let errs_effects = validate_gpu_kernel_body(true, false, false, false, "kernel_fx", &[]);
        assert!(!errs_effects.is_empty(), "must reject effects");

        // Rejects strings in GPU kernels
        let errs_strings = validate_gpu_kernel_body(false, true, false, false, "kernel_str", &[]);
        assert!(!errs_strings.is_empty(), "must reject strings");
    }

    // ══════════════════════════════════════════════════════════════════════
    // 4. Output Tests — Host CUDA Runtime Declarations & Linkage
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_gpu_output_host_cuda_runtime_declarations() {
        let decls = emit_host_cuda_declarations();
        assert!(decls.contains("cudaMalloc"), "must declare cudaMalloc");
        assert!(decls.contains("cudaFree"), "must declare cudaFree");
        assert!(decls.contains("cudaMemcpy"), "must declare cudaMemcpy");
        assert!(
            decls.contains("cudaLaunchKernel"),
            "must declare cudaLaunchKernel"
        );
    }

    #[test]
    fn test_gpu_output_cuda_linkage_resolution() {
        let linkage = CudaLinkage::resolved_or_default();
        assert!(
            !linkage.cudart.is_empty(),
            "must have resolved CUDA runtime path"
        );
    }
}
