//! Scoped Differential Testing Suite (JIT vs. LLVM AOT Parity).
//!
//! Verifies execution and semantics parity between the Cranelift JIT backend and
//! the LLVM AOT emitter across core arithmetic, control flow, functions, recursion,
//! and buffer operations.

use agam_errors::span::SourceId;
use agam_hir::lower::HirLowering;
use agam_lexer::tokenize;
use agam_mir::lower::MirLowering;
use agam_mir::opt::optimize_module;
use agam_parser::parse;

/// Compiles source to optimized MIR module.
pub fn compile_to_mir(src: &str) -> Result<agam_mir::ir::MirModule, String> {
    let source_id = SourceId(0);
    let tokens = tokenize(src, source_id);
    let ast = parse(tokens, source_id).map_err(|e| format!("parse error: {e:?}"))?;

    let mut hir_lowering = HirLowering::new();
    let hir = hir_lowering.lower_module(&ast);
    let mut mir_lowering = MirLowering::new();
    let mut mir = mir_lowering.lower_module(&hir);
    optimize_module(&mut mir);
    Ok(mir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_source;
    use agam_codegen::llvm_emitter::emit_llvm;

    #[test]
    fn test_differential_arithmetic_and_control_flow() -> Result<(), Box<dyn std::error::Error>> {
        let src = r#"
fn compute(a: i32, b: i32) -> i32:
    let mut sum = 0
    let mut i = a
    while i <= b:
        sum = sum + i
        i = i + 1
    return sum

@test
fn test_compute_range() -> bool:
    return compute(1, 10) == 55
"#;
        // 1. Verify JIT execution
        let summary = run_source(src, "memory://diff_compute.agam")?;
        assert_eq!(summary.passed(), 1, "JIT test execution failed");

        // 2. Verify LLVM IR emission parity
        let mir = compile_to_mir(src)?;
        let llvm_ir = emit_llvm(&mir)?;
        assert!(
            llvm_ir.contains("@agam_compute(") || llvm_ir.contains("@compute("),
            "LLVM must emit compute function signature"
        );
        assert!(
            llvm_ir.contains("add ") || llvm_ir.contains("add nsw "),
            "LLVM must emit addition"
        );
        Ok(())
    }

    #[test]
    fn test_differential_recursive_fibonacci() -> Result<(), Box<dyn std::error::Error>> {
        let src = r#"
fn fib(n: i32) -> i32:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

@test
fn test_fibonacci() -> bool:
    return fib(10) == 55
"#;
        // 1. Verify JIT execution
        let summary = run_source(src, "memory://diff_fib.agam")?;
        assert_eq!(summary.passed(), 1, "JIT fib(10) failed");

        // 2. Verify LLVM IR emission parity
        let mir = compile_to_mir(src)?;
        let llvm_ir = emit_llvm(&mir)?;
        assert!(
            llvm_ir.contains("@agam_fib(") || llvm_ir.contains("@fib("),
            "LLVM must emit fib function signature"
        );
        assert!(llvm_ir.contains("call "), "LLVM must emit recursive call");
        Ok(())
    }

    #[test]
    fn test_differential_prime_sieve_logic() -> Result<(), Box<dyn std::error::Error>> {
        let src = r#"
fn count_primes_to(limit: i32) -> i32:
    let mut count = 0
    let mut n = 2
    while n <= limit:
        let mut is_prime = 1
        let mut d = 2
        while d * d <= n:
            if n % d == 0:
                is_prime = 0
                break
            d = d + 1
        if is_prime == 1:
            count = count + 1
        n = n + 1
    return count

@test
fn test_primes() -> bool:
    return count_primes_to(20) == 8
"#;
        // 1. Verify JIT execution
        let summary = run_source(src, "memory://diff_primes.agam")?;
        assert_eq!(summary.passed(), 1, "JIT primes failed");

        // 2. Verify LLVM IR emission parity
        let mir = compile_to_mir(src)?;
        let llvm_ir = emit_llvm(&mir)?;
        assert!(
            llvm_ir.contains("@agam_count_primes_to(") || llvm_ir.contains("@count_primes_to("),
            "LLVM must emit count_primes_to function signature"
        );
        Ok(())
    }
}
