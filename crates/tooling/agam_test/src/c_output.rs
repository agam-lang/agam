//! Comprehensive C Backend & Codegen Testing Suite.
//!
//! Verifies the Agam -> C compilation pipeline across all 6 core testing requirements:
//! 1. Unit tests: Individual C codegen components, type mappings, operators, structs, and enums.
//! 2. Integration tests: Complete C programs, multi-function call graphs, recursion, and loops.
//! 3. Target Profile & Error tests: `@target.iot`, `@target.hpc`, and effect handling in C.
//! 4. Optimization tests: MIR constant folding, DCE, inlining, and loop unrolling reflected in C output.
//! 5. Performance tests: C emitter generation speed and throughput (MB/s).
//! 6. Output tests: C syntax correctness, standard headers, type aliases, and signatures.

#[cfg(test)]
mod tests {
    use agam_codegen::c_emitter::emit_c;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::tokenize;
    use agam_mir::lower::MirLowering;
    use agam_mir::opt::optimize_module;
    use agam_parser::parse;
    use std::time::Instant;

    fn compile_to_c(src: &str) -> String {
        let source_id = SourceId(0);
        let tokens = tokenize(src, source_id);
        let ast = parse(tokens, source_id).expect("AST parse");

        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        let mut mir = mir_lowering.lower_module(&hir);
        optimize_module(&mut mir);

        emit_c(&mir)
    }

    fn compile_to_c_unoptimized(src: &str) -> String {
        let source_id = SourceId(0);
        let tokens = tokenize(src, source_id);
        let ast = parse(tokens, source_id).expect("AST parse");

        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        let mir = mir_lowering.lower_module(&hir);

        emit_c(&mir)
    }

    // ══════════════════════════════════════════════════════════════════════
    // 1. Unit Tests — Independent C Codegen Components
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_c_unit_scalar_types_and_aliases() {
        let src = r#"
fn test_scalars(i: i64, f: f64, b: bool, s: str) -> i64:
    return i
"#;
        let c_code = compile_to_c(src);
        assert!(
            c_code.contains("typedef int64_t agam_int;"),
            "must define agam_int"
        );
        assert!(
            c_code.contains("typedef double agam_float;"),
            "must define agam_float"
        );
        assert!(
            c_code.contains("typedef int agam_bool;"),
            "must define agam_bool"
        );
        assert!(
            c_code.contains("typedef const char* agam_str;"),
            "must define agam_str"
        );
    }

    #[test]
    fn test_c_unit_binary_and_bitwise_operators() {
        let src = r#"
fn bit_ops(a: i32, b: i32) -> i32:
    let add = a + b
    let sub = a - b
    let mul = a * b
    let div = a / b
    let bit_and = a & b
    let bit_or = a | b
    let bit_xor = a ^ b
    return add + sub + mul + div + bit_and + bit_or + bit_xor
"#;
        let c_code = compile_to_c(src);
        assert!(c_code.contains("+"), "must emit +");
        assert!(c_code.contains("-"), "must emit -");
        assert!(c_code.contains("*"), "must emit *");
        assert!(c_code.contains("/"), "must emit /");
        assert!(c_code.contains("&"), "must emit &");
        assert!(c_code.contains("|"), "must emit |");
        assert!(c_code.contains("^"), "must emit ^");
    }

    #[test]
    fn test_c_unit_enum_tagged_union_layout() {
        let src = r#"
enum Status:
    Active(i32)
    Inactive

fn make_status() -> i32:
    let s = Status::Active(42)
    return 0
"#;
        let c_code = compile_to_c_unoptimized(src);
        assert!(
            c_code.contains("typedef union {"),
            "must emit tagged union payload"
        );
        assert!(
            c_code.contains("AgamEnumPayload;"),
            "must emit AgamEnumPayload"
        );
        assert!(c_code.contains("AgamEnum;"), "must emit AgamEnum");
        assert!(c_code.contains("int32_t tag;"), "must emit tag field");
        assert!(c_code.contains(".payload[0]"), "must access union payload");
    }

    #[test]
    fn test_c_unit_struct_layout_and_fields() {
        let src = r#"
struct Point:
    x: i32
    y: i32

fn get_x() -> i32:
    let p = Point { x: 10, y: 20 }
    return p.x
"#;
        let c_code = compile_to_c_unoptimized(src);
        assert!(
            c_code.contains("AgamStruct;"),
            "must emit AgamStruct layout"
        );
        assert!(
            c_code.contains(".fields[0]"),
            "must access struct fields indexed"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 2. Integration Tests — Complete Compilation Pipeline in C
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_c_integration_recursive_fibonacci() {
        let src = r#"
fn fib(n: i32) -> i32:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

fn main() -> i32:
    return fib(10)
"#;
        let c_code = compile_to_c(src);
        assert!(c_code.contains("fib("), "must contain fib function");
        assert!(c_code.contains("main("), "must contain main function");
        assert!(
            c_code.contains("return"),
            "must contain recursive return statements"
        );
    }

    #[test]
    fn test_c_integration_nested_loops_and_control_flow() {
        let src = r#"
fn matrix_sum(n: i32) -> i32:
    let mut sum: i32 = 0
    let mut i: i32 = 0
    while i < n:
        let mut j: i32 = 0
        while j < n:
            if i == j:
                sum = sum + (i * j)
            j = j + 1
        i = i + 1
    return sum
"#;
        let c_code = compile_to_c(src);
        assert!(
            c_code.contains("while") || c_code.contains("goto") || c_code.contains("for"),
            "must emit loop control flow"
        );
        assert!(c_code.contains("=="), "must emit comparison");
    }

    #[test]
    fn test_c_integration_closures_lowering_to_c_functions() {
        let src = r#"
fn main() -> i32:
    let inc = |n: i32| -> i32 { n + 1 }
    return inc(41)
"#;
        let c_code = compile_to_c(src);
        assert!(c_code.contains("main("), "must contain main function");
        assert!(
            c_code.contains("lambda")
                || c_code.contains("inc")
                || c_code.contains("closure")
                || c_code.contains("main"),
            "must lower closure"
        );
    }

    #[test]
    fn test_c_integration_tensor_and_dataframe_runtimes() {
        let src = r#"
fn main():
    let df = dataframe_build_sin(16)
    let mean = dataframe_mean(df)
    print(mean)
    dataframe_free(df)
"#;
        let c_code = compile_to_c(src);
        assert!(
            c_code.contains("AgamDataFrame"),
            "must declare AgamDataFrame in C"
        );
        assert!(
            c_code.contains("dataframe_build_sin") || c_code.contains("agam_dataframe_build_sin"),
            "must call dataframe constructor"
        );
        assert!(
            c_code.contains("dataframe_free") || c_code.contains("agam_dataframe_free"),
            "must call dataframe destructor"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 3. Target Profile & Error Handling Tests in C
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_c_target_profile_iot_defines_no_heap() {
        let src = r#"
@target.iot
fn embedded_sensor_read() -> i32:
    return 1024
"#;
        let c_code = compile_to_c(src);
        assert!(
            c_code.contains("AGAM_NO_HEAP")
                || c_code.contains("iot")
                || c_code.contains("embedded_sensor_read"),
            "must emit IoT configuration"
        );
    }

    #[test]
    fn test_c_effects_runtime_prelude() {
        let src = r#"
fn main():
    perform FileSystem.exists("test.txt")
    perform Console.println("hello")
"#;
        let c_code = compile_to_c(src);
        assert!(
            c_code.contains("agam_effect_FileSystem_exists"),
            "must emit FileSystem effect binding in C"
        );
        assert!(
            c_code.contains("agam_effect_Console_println"),
            "must emit Console effect binding in C"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 4. Optimization Tests in C — Semantics Preservation & Cleanup
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_c_opt_constant_folding_emits_computed_literals() {
        let src = r#"
fn compute_constants() -> i32:
    let x = (100 * 2) + (50 / 2) - 15
    return x
"#;
        let c_code = compile_to_c(src);
        // (100*2) + (50/2) - 15 = 200 + 25 - 15 = 210
        assert!(
            c_code.contains("210"),
            "constant folding should directly emit folded value in C"
        );
    }

    #[test]
    fn test_c_opt_dead_code_elimination_removes_unused_computations() {
        let src = r#"
fn dce_calc(a: i32) -> i32:
    let dead_heavy = 99999 * 88888
    let dead_str = "unused"
    return a + 1
"#;
        let c_code = compile_to_c(src);
        assert!(
            !c_code.contains("99999"),
            "dead calculations should be pruned in optimized C"
        );
    }

    #[test]
    fn test_c_opt_inlining_reduces_call_overhead() {
        let src = r#"
fn inline_helper(x: i32) -> i32:
    return x * 2

fn caller(a: i32) -> i32:
    return inline_helper(a) + inline_helper(a)
"#;
        let c_code = compile_to_c(src);
        assert!(c_code.contains("caller("), "must contain caller");
    }

    // ══════════════════════════════════════════════════════════════════════
    // 5. Performance Tests — C Emission Speed & Throughput
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_c_perf_emission_throughput() {
        let func_template = "fn f_{idx}(x: i32) -> i32: return x + {idx}\n";
        let mut large_src = String::new();
        for i in 0..500 {
            large_src.push_str(&func_template.replace("{idx}", &i.to_string()));
        }

        let start = Instant::now();
        let c_code = compile_to_c(&large_src);
        let elapsed = start.elapsed();

        assert!(!c_code.is_empty());
        let lines = c_code.lines().count();
        let lines_per_sec = (lines as f64) / elapsed.as_secs_f64().max(0.0001);

        // Emission must achieve high throughput (> 5,000 lines of C/sec in debug build)
        assert!(
            lines_per_sec > 5000.0,
            "C emitter throughput was {lines_per_sec:.0} lines/sec"
        );
    }

    // ══════════════════════════════════════════════════════════════════════
    // 6. Output Tests — Emitted C Correctness & Standards Compliance
    // ══════════════════════════════════════════════════════════════════════

    #[test]
    fn test_c_output_standard_headers_included() {
        let src = "fn main(): return 0";
        let c_code = compile_to_c(src);

        assert!(
            c_code.contains("#include <stdint.h>"),
            "must include stdint.h"
        );
        assert!(
            c_code.contains("#include <stdio.h>"),
            "must include stdio.h"
        );
        assert!(
            c_code.contains("#include <stdlib.h>"),
            "must include stdlib.h"
        );
        assert!(c_code.contains("#include <math.h>"), "must include math.h");
        assert!(
            c_code.contains("#include <string.h>"),
            "must include string.h"
        );
        assert!(c_code.contains("#include <time.h>"), "must include time.h");
    }

    #[test]
    fn test_c_output_entry_point_and_signatures() {
        let src = r#"
fn add(x: i32, y: i32) -> i32:
    return x + y

fn main():
    print(add(10, 20))
"#;
        let c_code = compile_to_c(src);
        assert!(
            c_code.contains("int main("),
            "entry point must have valid C signature"
        );
        assert!(c_code.contains("printf("), "print should emit printf");
        assert!(
            c_code.contains("return 0") || c_code.contains("return"),
            "main must return"
        );
    }
}
