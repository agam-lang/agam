//! # agam_test
//!
//! Test framework, property testing, and fuzzing.

pub mod async_concurrency;
pub mod bench;
pub mod c_output;
pub mod compiler_fuzz;
pub mod coverage;
pub mod doctest;
pub mod error_reporting;
pub mod fuzz;
pub mod gpu_output;
pub mod llvm_output;
pub mod opt_semantics;
pub mod perf_speed;
pub mod pipeline_integration;
pub mod property;
pub mod regression;
pub mod snapshot;
pub mod toolchain_output;
pub mod unit_passes;

pub use bench::{BenchConfig, BenchResult, BenchmarkHarness};
pub use compiler_fuzz::{
    AstMutationEngine, CompilerFuzzReport, CompilerPipelineFuzzer, PipelineFuzzOutcome,
};
pub use coverage::{CoverageReport, FileCoverage, LineStatus};
pub use doctest::{DocTestCase, DocTestExtractor};
pub use fuzz::{FuzzRunner, MutationStrategy};
pub use property::{PropertyResult, PropertyRunner, TestRng};
pub use snapshot::{SnapshotError, SnapshotManager};

use std::fs;
use std::path::{Path, PathBuf};

use agam_ast::Module;
use agam_ast::decl::DeclKind;
use agam_errors::{SourceFile, SourceId};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestCase {
    pub name: String,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestResult {
    pub case: TestCase,
    pub passed: bool,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TestSummary {
    pub results: Vec<TestResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTestSummary {
    pub path: PathBuf,
    pub summary: TestSummary,
}

impl TestSummary {
    pub fn total(&self) -> usize {
        self.results.len()
    }

    pub fn passed(&self) -> usize {
        self.results.iter().filter(|result| result.passed).count()
    }

    pub fn failed(&self) -> usize {
        self.results.iter().filter(|result| !result.passed).count()
    }
}

pub fn run_file(path: &Path) -> Result<TestSummary, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("failed to read Agam test file `{}`: {e}", path.display()))?;
    run_source(&source, &path.to_string_lossy())
}

pub fn run_paths(paths: &[PathBuf]) -> Result<Vec<FileTestSummary>, String> {
    paths
        .iter()
        .map(|path| {
            run_file(path).map(|summary| FileTestSummary {
                path: path.clone(),
                summary,
            })
        })
        .collect()
}

pub fn run_inputs(inputs: Vec<PathBuf>) -> Result<Vec<FileTestSummary>, String> {
    let paths = agam_pkg::expand_agam_inputs(inputs)?;
    run_paths(&paths)
}

fn run_source(source: &str, label: &str) -> Result<TestSummary, String> {
    let source_id = SourceId(0);
    let tokens = agam_lexer::tokenize(source, source_id);
    let module = agam_parser::parse(tokens, source_id).map_err(|errors| {
        errors
            .iter()
            .map(|error| error.message.clone())
            .collect::<Vec<_>>()
            .join("; ")
    })?;
    let source_file = SourceFile::new(source_id, label.to_string(), source.to_string());
    let test_cases = collect_test_cases(&module, &source_file);
    if test_cases.is_empty() {
        return Ok(TestSummary::default());
    }

    let mut hir_lowering = agam_hir::lower::HirLowering::new();
    let hir = hir_lowering.lower_module(&module);
    let mut mir_lowering = agam_mir::lower::MirLowering::new();
    let mut mir = mir_lowering.lower_module(&hir);
    let _ = agam_mir::opt::optimize_module(&mut mir);

    let compiled = agam_jit::CompiledJitModule::compile(&mir, agam_jit::JitOptions::default())?;
    let results = test_cases
        .into_iter()
        .map(|case| {
            let evaluation = compiled.run_function(&case.name, &[]);
            match evaluation {
                Ok(value) => {
                    let (passed, message) = evaluate_test_value(value);
                    TestResult {
                        case,
                        passed,
                        message,
                    }
                }
                Err(error) => TestResult {
                    case,
                    passed: false,
                    message: Some(error),
                },
            }
        })
        .collect();

    Ok(TestSummary { results })
}

fn collect_test_cases(module: &Module, source_file: &SourceFile) -> Vec<TestCase> {
    module
        .declarations
        .iter()
        .filter_map(|decl| match &decl.kind {
            DeclKind::Function(function)
                if function
                    .annotations
                    .iter()
                    .any(|annotation| annotation.name.name == "test") =>
            {
                let (line, column) =
                    source_file.offset_to_line_col(function.name.span.start as usize);
                Some(TestCase {
                    name: function.name.name.clone(),
                    line: line + 1,
                    column: column + 1,
                })
            }
            _ => None,
        })
        .collect()
}

fn evaluate_test_value(value: agam_jit::JitValue) -> (bool, Option<String>) {
    match value {
        agam_jit::JitValue::Unit => (true, None),
        agam_jit::JitValue::Bool(true) => (true, None),
        agam_jit::JitValue::Bool(false) => (false, Some("returned false".into())),
        agam_jit::JitValue::Int(0) => (true, None),
        agam_jit::JitValue::Int(value) => (false, Some(format!("returned {value}"))),
        agam_jit::JitValue::UInt(0) => (true, None),
        agam_jit::JitValue::UInt(value) => (false, Some(format!("returned {value}"))),
        agam_jit::JitValue::Float32(0.0) => (true, None),
        agam_jit::JitValue::Float32(value) => (false, Some(format!("returned {value}"))),
        agam_jit::JitValue::Float64(0.0) => (true, None),
        agam_jit::JitValue::Float64(value) => (false, Some(format!("returned {value}"))),
        agam_jit::JitValue::Pointer(0) => (true, None),
        agam_jit::JitValue::Pointer(value) => (false, Some(format!("returned {value}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_source_executes_test_functions() {
        let summary = run_source(
            r#"
@test
fn passes() -> bool:
    return true

@test
fn fails() -> bool:
    return false
"#,
            "memory://tests.agam",
        )
        .expect("run source tests");

        assert_eq!(summary.total(), 2);
        assert_eq!(summary.passed(), 1);
        assert_eq!(summary.failed(), 1);
        assert_eq!(summary.results[0].case.name, "passes");
        assert!(summary.results[0].passed);
        assert_eq!(
            summary.results[1].message.as_deref(),
            Some("returned false")
        );
    }

    #[test]
    fn run_source_returns_empty_summary_without_test_annotations() {
        let summary = run_source(
            r#"
fn helper() -> bool:
    return true
"#,
            "memory://helpers.agam",
        )
        .expect("run source without tests");

        assert_eq!(summary.total(), 0);
    }

    #[test]
    fn run_paths_preserves_file_paths() {
        let dir = std::env::temp_dir().join(format!(
            "agam_test_paths_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should move forward")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let file = dir.join("smoke.agam");
        std::fs::write(&file, "@test\nfn smoke() -> bool:\n    return true\n")
            .expect("write test file");

        let summaries = run_paths(std::slice::from_ref(&file)).expect("run test paths");

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].path, file);
        assert_eq!(summaries[0].summary.passed(), 1);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn run_source_ranges_and_counting_loops() {
        let summary = run_source(
            r#"
@test
fn test_range_exclusive() -> bool:
    let mut total: i64 = 0
    for i in 1..5:
        total = total + i
    return total == 10

@test
fn test_range_inclusive() -> bool:
    let mut total: i64 = 0
    for i in 1..=5:
        total = total + i
    return total == 15
"#,
            "memory://ranges.agam",
        )
        .expect("run range and loop tests");

        assert_eq!(summary.total(), 2);
        assert_eq!(summary.passed(), 2);
        assert_eq!(summary.failed(), 0);
    }

    #[test]
    fn run_source_pattern_matching_complex() {
        let summary = run_source(
            r#"
fn classify(x: i64) -> i64:
    return match x:
        1 | 2 | 3 => 10,
        4..10 => 20,
        _ => 30

@test
fn test_or_and_range_patterns() -> bool:
    let r1 = classify(2)
    let r2 = classify(7)
    let r3 = classify(99)
    return r1 == 10 && r2 == 20 && r3 == 30
"#,
            "memory://patterns.agam",
        )
        .expect("run pattern tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_lambda_and_closures() {
        let summary = run_source(
            r#"
@test
fn test_closure_invocation() -> bool:
    let f = |x: i64, y: i64| -> i64 { x * y + 10 }
    let res = f(3, 4)
    return res == 22
"#,
            "memory://lambdas.agam",
        )
        .expect("run lambda tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_struct_and_methods() {
        let summary = run_source(
            r#"
struct Counter:
    val: i64

impl Counter:
    fn new(v: i64) -> Counter:
        return Counter { val: v }

    fn add(self, x: i64) -> i64:
        return self.val + x

@test
fn test_struct_methods() -> bool:
    let c = Counter::new(42)
    let res = Counter::add(c, 8)
    return res == 50
"#,
            "memory://struct_methods.agam",
        )
        .expect("run struct method tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_multi_function_recursion() {
        let summary = run_source(
            r#"
fn fib(n: i64) -> i64:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

@test
fn test_fibonacci() -> bool:
    let f0 = fib(0)
    let f1 = fib(1)
    let f7 = fib(7)
    let f10 = fib(10)
    return f0 == 0 && f1 == 1 && f7 == 13 && f10 == 55
"#,
            "memory://recursion.agam",
        )
        .expect("run recursion tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_while_and_break_continue() {
        let summary = run_source(
            r#"
@test
fn test_while_accumulation() -> bool:
    let mut i: i64 = 0
    let mut sum: i64 = 0
    while i < 10:
        i = i + 1
        sum = sum + i
    return sum == 55
"#,
            "memory://while_loops.agam",
        )
        .expect("run while loop tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_multi_field_struct() {
        let summary = run_source(
            r#"
struct Point:
    x: i32
    y: i32

fn sum_point(p: Point) -> i32:
    return p.x + p.y

@test
fn test_multi_field_struct() -> bool:
    let p = Point { x: 15, y: 25 }
    let s = sum_point(p)
    return p.x == 15 && p.y == 25 && s == 40
"#,
            "memory://point_struct.agam",
        )
        .expect("run struct tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_enum_payloads_and_matching() {
        let summary = run_source(
            r#"
enum OptionInt:
    Some(i32),
    None

fn unwrap_or(opt: OptionInt, default_val: i32) -> i32:
    return match opt:
        OptionInt::Some(v) => v,
        OptionInt::None => default_val

@test
fn test_enum_some() -> bool:
    let s = OptionInt::Some(42)
    let r1 = unwrap_or(s, 0)
    return r1 == 42

@test
fn test_enum_none() -> bool:
    let n = OptionInt::None
    let r2 = unwrap_or(n, 99)
    return r2 == 99
"#,
            "memory://enums.agam",
        )
        .expect("run enum tests");

        assert_eq!(summary.total(), 2);
        assert_eq!(summary.passed(), 2);
    }

    #[test]
    fn run_source_boolean_logic_and_short_circuit() {
        let summary = run_source(
            r#"
fn check_bool(a: bool, b: bool, c: bool) -> bool:
    return (a && b) || (!a && c)

@test
fn test_complex_booleans() -> bool:
    let r1 = check_bool(true, true, false)
    let r2 = check_bool(true, false, true)
    let r3 = check_bool(false, false, true)
    let r4 = check_bool(false, true, false)
    return r1 == true && r2 == false && r3 == true && r4 == false
"#,
            "memory://booleans.agam",
        )
        .expect("run boolean tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_nested_function_calls_and_arithmetic() {
        let summary = run_source(
            r#"
fn square(x: i64) -> i64:
    return x * x

fn double_val(x: i64) -> i64:
    return x * 2

fn compute(a: i64, b: i64) -> i64:
    return double_val(square(a)) + square(double_val(b))

@test
fn test_nested_arithmetic() -> bool:
    let res = compute(3, 4)
    return res == 82
"#,
            "memory://nested_arithmetic.agam",
        )
        .expect("run nested arithmetic tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_if_else_expressions() {
        let summary = run_source(
            r#"
fn pick_value(flag: bool) -> i64:
    return if flag: 100 else: 200

@test
fn test_if_else_expr() -> bool:
    let v1 = pick_value(true)
    let v2 = pick_value(false)
    return v1 == 100 && v2 == 200
"#,
            "memory://if_else_expr.agam",
        )
        .expect("run if-else expression tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_compound_assignments() {
        let summary = run_source(
            r#"
@test
fn test_compound_ops() -> bool:
    let mut x: i64 = 10
    x += 5
    x -= 3
    x *= 4
    x /= 2
    return x == 24
"#,
            "memory://compound_assign.agam",
        )
        .expect("run compound assign tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_early_returns_in_loops() {
        let summary = run_source(
            r#"
fn find_first_divisible(limit: i64, divisor: i64) -> i64:
    let mut i: i64 = 1
    while i <= limit:
        if i % divisor == 0:
            return i
        i += 1
    return 0

@test
fn test_early_return() -> bool:
    let r1 = find_first_divisible(20, 7)
    let r2 = find_first_divisible(10, 13)
    return r1 == 7 && r2 == 0
"#,
            "memory://early_returns.agam",
        )
        .expect("run early return tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_trait_definition_and_implementation() {
        let summary = run_source(
            r#"
trait Describable:
    fn describe(self) -> i64

struct Item:
    id: i64

impl Describable for Item:
    fn describe(self) -> i64:
        return self.id * 10

@test
fn test_trait_impl() -> bool:
    let item = Item { id: 7 }
    let desc = Describable::describe(item)
    return desc == 70
"#,
            "memory://traits.agam",
        )
        .expect("run trait tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_generics_multi_type_params() {
        let summary = run_source(
            r#"
fn pick_first<T, U>(a: T, b: U) -> T:
    return a

fn pick_second<T, U>(a: T, b: U) -> U:
    return b

@test
fn test_generic_picks() -> bool:
    let r1 = pick_first(42, true)
    let r2 = pick_second(100, 999)
    return r1 == 42 && r2 == 999
"#,
            "memory://generics.agam",
        )
        .expect("run generics tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_chained_if_else_expressions() {
        let summary = run_source(
            r#"
fn categorize(score: i64) -> i64:
    return if score >= 90: 1 else if score >= 70: 2 else: 3

@test
fn test_chained_if_else() -> bool:
    let c1 = categorize(95)
    let c2 = categorize(80)
    let c3 = categorize(50)
    return c1 == 1 && c2 == 2 && c3 == 3
"#,
            "memory://chained_if.agam",
        )
        .expect("run chained if tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_nested_loops_grid_sum() {
        let summary = run_source(
            r#"
fn grid_sum(rows: i64, cols: i64) -> i64:
    let mut total: i64 = 0
    let mut r: i64 = 0
    while r < rows:
        let mut c: i64 = 0
        while c < cols:
            total += r * cols + c
            c += 1
        r += 1
    return total

@test
fn test_grid_sum() -> bool:
    let sum = grid_sum(3, 4)
    # Sum of 0..11 = 66
    return sum == 66
"#,
            "memory://grid_sum.agam",
        )
        .expect("run grid sum tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_bitwise_operations_and_shifts() {
        let summary = run_source(
            r#"
@test
fn test_bitwise_ops() -> bool:
    let a: i64 = 12   # 0b1100
    let b: i64 = 10   # 0b1010
    let and_res = a & b   # 0b1000 = 8
    let or_res = a | b    # 0b1110 = 14
    let xor_res = a ^ b   # 0b0110 = 6
    let shl_res = a << 2  # 48
    let shr_res = a >> 1  # 6
    return and_res == 8 && or_res == 14 && xor_res == 6 && shl_res == 48 && shr_res == 6
"#,
            "memory://bitwise.agam",
        )
        .expect("run bitwise tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_collatz_conjecture_recursion() {
        let summary = run_source(
            r#"
fn collatz_steps(n: i64) -> i64:
    if n == 1:
        return 0
    if n % 2 == 0:
        return 1 + collatz_steps(n / 2)
    return 1 + collatz_steps(3 * n + 1)

@test
fn test_collatz() -> bool:
    let s1 = collatz_steps(1)
    let s2 = collatz_steps(6)   # 6 -> 3 -> 10 -> 5 -> 16 -> 8 -> 4 -> 2 -> 1 (8 steps)
    let s3 = collatz_steps(12)  # 12 -> 6 (1 + 8 = 9 steps)
    return s1 == 0 && s2 == 8 && s3 == 9
"#,
            "memory://collatz.agam",
        )
        .expect("run collatz tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_higher_order_functions_and_composition() {
        let summary = run_source(
            r#"
fn apply_twice(x: i64, f: |i64| -> i64) -> i64:
    return f(f(x))

@test
fn test_higher_order() -> bool:
    let res = apply_twice(5, |v| v * 2 + 1)
    # v = 5 -> 11 -> 23
    return res == 23
"#,
            "memory://higher_order.agam",
        )
        .expect("run higher order tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_floating_point_math_and_conversions() {
        let summary = run_source(
            r#"
fn compute_area(radius: f64) -> f64:
    let pi: f64 = 3.14159
    return pi * radius * radius

@test
fn test_float_math() -> bool:
    let area = compute_area(2.0)
    # 3.14159 * 4.0 = 12.56636
    return area > 12.56 && area < 12.57
"#,
            "memory://floats.agam",
        )
        .expect("run float tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_gcd_euclidean_algorithm() {
        let summary = run_source(
            r#"
fn gcd(a: i64, b: i64) -> i64:
    if b == 0:
        return a
    return gcd(b, a % b)

@test
fn test_gcd() -> bool:
    let g1 = gcd(48, 18)  # 6
    let g2 = gcd(101, 10) # 1
    let g3 = gcd(54, 24)  # 6
    return g1 == 6 && g2 == 1 && g3 == 6
"#,
            "memory://gcd.agam",
        )
        .expect("run gcd tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn run_source_multi_return_and_conditional_branching() {
        let summary = run_source(
            r#"
fn sign_check(n: i64) -> i64:
    if n > 0:
        return 1
    if n < 0:
        return -1
    return 0

@test
fn test_sign_check() -> bool:
    let pos = sign_check(42)
    let neg = sign_check(-99)
    let zero = sign_check(0)
    return pos == 1 && neg == -1 && zero == 0
"#,
            "memory://sign_check.agam",
        )
        .expect("run sign check tests");

        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }
}
