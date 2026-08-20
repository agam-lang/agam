//! Optimization semantics preservation tests.
//!
//! Asserts that all MIR optimization passes produce identical execution results
//! to unoptimized MIR.

#[cfg(test)]
mod tests {
    use crate::run_source;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_jit::{CompiledJitModule, JitOptions, JitValue};
    use agam_lexer::tokenize;
    use agam_mir::lower::MirLowering;
    use agam_mir::opt::optimize_module;
    use agam_parser::parse;

    fn compile_to_mir(src: &str) -> agam_mir::ir::MirModule {
        let source_id = SourceId(0);
        let tokens = tokenize(src, source_id);
        let ast = parse(tokens, source_id).expect("AST parse");

        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        mir_lowering.lower_module(&hir)
    }

    #[test]
    fn test_opt_constant_folding_preserves_semantics() {
        let src = r#"
fn fold_math(x: i32) -> i32:
    let c = (10 * 20) + (300 / 3) - 50
    return x + c

@test
fn test_fold() -> i32:
    return fold_math(15)
"#;
        let unopt_mir = compile_to_mir(src);
        let mut opt_mir = unopt_mir.clone();

        let changed = optimize_module(&mut opt_mir);
        assert!(changed, "constant fold pass should optimize constants");

        let compiled_unopt = CompiledJitModule::compile(&unopt_mir, JitOptions::default()).unwrap();
        let compiled_opt = CompiledJitModule::compile(&opt_mir, JitOptions::default()).unwrap();

        let res_unopt = compiled_unopt.run_function("test_fold", &[]).unwrap();
        let res_opt = compiled_opt.run_function("test_fold", &[]).unwrap();

        assert_eq!(
            res_unopt, res_opt,
            "optimized result must match unoptimized execution"
        );
        assert_eq!(res_opt, JitValue::Int(15 + 200 + 100 - 50));
    }

    #[test]
    fn test_opt_inlining_and_dce_preserves_semantics() {
        let src = r#"
fn square(n: i32) -> i32:
    return n * n

fn compute_sum(a: i32, b: i32) -> i32:
    let unused = 999 * 888
    let sq_a = square(a)
    let sq_b = square(b)
    return sq_a + sq_b

@test
fn test_inlining_case1() -> bool:
    return compute_sum(3, 4) == 25

@test
fn test_inlining_case2() -> bool:
    return compute_sum(5, 12) == 169
"#;
        let summary = run_source(src, "memory://test_inlining.agam").expect("run source");
        assert_eq!(summary.total(), 2);
        assert_eq!(summary.passed(), 2);
    }

    #[test]
    fn test_opt_loop_unrolling_preserves_semantics() {
        let src = r#"
fn loop_acc(start: i32) -> i32:
    let mut total: i32 = start
    let mut i: i32 = 0
    while i < 4:
        total = total + i
        i = i + 1
    return total

@test
fn test_unroll() -> bool:
    return loop_acc(100) == 106
"#;
        let summary = run_source(src, "memory://test_unroll.agam").expect("run source");
        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }
}
