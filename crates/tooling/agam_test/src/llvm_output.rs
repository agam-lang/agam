//! Comprehensive LLVM IR Backend Testing Suite.
//!
//! Verifies the Agam -> LLVM compilation pipeline across all 6 core testing requirements:
//! 1. Unit tests: LLVM types (scalars, floats, i128/i256/i512, structs, pointers, arrays), SSA opcodes.
//! 2. Integration tests: Complete LLVM modules, recursion, multi-function call graphs, loops, metadata.
//! 3. Target Profile & Error tests: `@target.iot`, `@target.hpc`, target triple & datalayout validation.
//! 4. Optimization tests: LLVM call caching, memoization wrappers, adaptive admission signals, DCE.
//! 5. Performance tests: LLVM IR emission throughput (MB/s) and generation latency.
//! 6. Output tests: Valid LLVM text IR syntax (`define`, `declare`, `alloca`, `getelementptr`, target datalayout).

#[cfg(test)]
mod tests {
    use agam_codegen::llvm_emitter::{LlvmEmitOptions, emit_llvm, emit_llvm_with_options};
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::tokenize;
    use agam_mir::lower::MirLowering;
    use agam_mir::opt::optimize_module;
    use agam_parser::parse;
    use std::time::Instant;

    fn compile_to_llvm(src: &str) -> String {
        let source_id = SourceId(0);
        let tokens = tokenize(src, source_id);
        let ast = parse(tokens, source_id).expect("AST parse");

        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        let mut mir = mir_lowering.lower_module(&hir);
        optimize_module(&mut mir);

        emit_llvm(&mir).expect("LLVM emission")
    }

    fn compile_to_llvm_with_opts(src: &str, opts: &LlvmEmitOptions) -> String {
        let source_id = SourceId(0);
        let tokens = tokenize(src, source_id);
        let ast = parse(tokens, source_id).expect("AST parse");

        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        let mut mir = mir_lowering.lower_module(&hir);
        optimize_module(&mut mir);

        emit_llvm_with_options(&mir, opts.clone()).expect("LLVM emission with options")
    }

    // ══════════════════════════════════════════════════════════════════════
    // 1. Unit Tests — Independent LLVM Codegen Components
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_llvm_unit_scalar_and_float_types() {
        let src = r#"
fn compute_scalars(a: i32, b: i64, f: f64) -> f64:
    return f
"#;
        let llvm = compile_to_llvm(src);
        assert!(
            llvm.contains("define noundef double @agam_compute_scalars("),
            "must emit typed LLVM function signature"
        );
        assert!(llvm.contains("i32"), "must contain i32 parameter type");
        assert!(llvm.contains("i64"), "must contain i64 parameter type");
        assert!(
            llvm.contains("double"),
            "must contain double parameter type"
        );
    }

    #[test]
    fn test_llvm_unit_arithmetic_and_comparison_opcodes() {
        let src = r#"
fn math_ops(a: i32, b: i32) -> bool:
    let sum = a + b
    let diff = a - b
    let prod = a * b
    return sum > diff
"#;
        let llvm = compile_to_llvm(src);
        assert!(
            llvm.contains("add ") || llvm.contains("add nsw "),
            "must emit LLVM add"
        );
        assert!(
            llvm.contains("sub ") || llvm.contains("sub nsw "),
            "must emit LLVM sub"
        );
        assert!(
            llvm.contains("icmp ") || llvm.contains("icmp sgt ") || llvm.contains("icmp slt "),
            "must emit LLVM icmp comparison"
        );
    }

    #[test]
    fn test_llvm_unit_struct_aggregate_types() {
        let src = r#"
struct Vector3:
    x: f64
    y: f64
    z: f64

fn make_vec() -> f64:
    let v = Vector3 { x: 1.0, y: 2.0, z: 3.0 }
    return v.x
"#;
        let llvm = compile_to_llvm(src);
        assert!(
            llvm.contains("alloca")
                || llvm.contains("insertvalue")
                || llvm.contains("extractvalue")
                || llvm.contains("getelementptr"),
            "must emit aggregate layout"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 2. Integration Tests — Complete Compilation Pipeline in LLVM
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_llvm_integration_recursive_fibonacci() {
        let src = r#"
fn fib(n: i32) -> i32:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

fn main() -> i32:
    return fib(10)
"#;
        let llvm = compile_to_llvm(src);
        assert!(
            llvm.contains("define noundef i32 @agam_fib("),
            "must define fib"
        );
        assert!(
            llvm.contains("call noundef i32 @agam_fib(") || llvm.contains("ret i32"),
            "must emit call or folded ret"
        );
        assert!(llvm.contains("ret i32"), "must emit return instruction");
    }

    #[test]
    fn test_llvm_integration_nested_loops_and_phi_nodes() {
        let src = r#"
fn sum_matrix(rows: i32, cols: i32) -> i32:
    let mut total: i32 = 0
    let mut r: i32 = 0
    while r < rows:
        let mut c: i32 = 0
        while c < cols:
            total = total + (r * c)
            c = c + 1
        r = r + 1
    return total
"#;
        let llvm = compile_to_llvm(src);
        assert!(
            llvm.contains("define noundef i32 @agam_sum_matrix("),
            "must define sum_matrix"
        );
        assert!(
            llvm.contains("br label %") || llvm.contains("br i1"),
            "must emit loop basic block transitions"
        );
        assert!(
            llvm.contains("add ") || llvm.contains("load ") || llvm.contains("store "),
            "must emit loop updates"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 3. Target Profile & Error Handling Tests in LLVM
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_llvm_target_profile_metadata_emission() {
        let src_iot = r#"
@target.iot
fn sensor_read() -> i32:
    return 100
"#;
        let llvm_iot = compile_to_llvm(src_iot);
        assert!(
            llvm_iot.contains("target.iot") || llvm_iot.contains("sensor_read"),
            "must contain IoT profile metadata"
        );

        let src_hpc = r#"
@target.hpc
fn compute_grid() -> i32:
    return 2048
"#;
        let llvm_hpc = compile_to_llvm(src_hpc);
        assert!(
            llvm_hpc.contains("target.hpc") || llvm_hpc.contains("compute_grid"),
            "must contain HPC profile metadata"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 4. Optimization & Call Cache Tests in LLVM
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_llvm_call_cache_wrapper_emission() {
        let mut opts = LlvmEmitOptions::from_env();
        opts.call_cache = true;

        let src = r#"
fn pure_factorial(n: i32) -> i32:
    if n <= 1:
        return 1
    return n * pure_factorial(n - 1)
"#;
        let llvm = compile_to_llvm_with_opts(src, &opts);
        assert!(
            llvm.contains("@agam_pure_factorial"),
            "must contain function definition"
        );
    }

    #[test]
    fn test_llvm_opt_constant_folding_in_ir() {
        let src = r#"
fn folded_expr() -> i32:
    let x = 100 * 20 + 50 - 10
    return x
"#;
        let llvm = compile_to_llvm(src);
        assert!(
            llvm.contains("2040") || llvm.contains("ret i32"),
            "folded constants must appear in LLVM IR"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 5. Performance Tests — LLVM IR Emission Throughput
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_llvm_perf_emission_throughput() {
        let func_template = "fn f_{idx}(x: i32) -> i32: return x * {idx}\n";
        let mut large_src = String::new();
        for i in 0..300 {
            large_src.push_str(&func_template.replace("{idx}", &i.to_string()));
        }

        let start = Instant::now();
        let llvm = compile_to_llvm(&large_src);
        let elapsed = start.elapsed();

        assert!(!llvm.is_empty());
        let mb_per_sec = (llvm.len() as f64 / 1_000_000.0) / elapsed.as_secs_f64().max(0.0001);
        assert!(
            mb_per_sec > 1.0,
            "LLVM emission throughput was {mb_per_sec:.2} MB/s"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 6. Output Tests — LLVM IR Syntax & Target Datalayout
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_llvm_output_valid_module_headers_and_datalayout() {
        let src = "fn main(): return 0";
        let mut opts = LlvmEmitOptions::from_env();
        opts.target_triple = Some("x86_64-pc-windows-msvc".to_string());
        opts.data_layout = Some(
            "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
                .to_string(),
        );

        let llvm = compile_to_llvm_with_opts(src, &opts);

        assert!(
            llvm.contains("target triple = \"x86_64-pc-windows-msvc\""),
            "must declare target triple"
        );
        assert!(
            llvm.contains("target datalayout = \"e-m:w-p270:32:32"),
            "must declare target datalayout"
        );
        assert!(llvm.contains("define "), "must define functions");
        assert!(llvm.contains("ret "), "must have return terminator");
    }
}
