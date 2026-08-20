//! End-to-end integration tests validating the complete compilation pipeline.

#[cfg(test)]
mod tests {
    use crate::run_source;

    #[test]
    fn test_pipeline_recursive_fibonacci() {
        let src = r#"
fn fib(n: i32) -> i32:
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

@test
fn test_fib() -> bool:
    return fib(10) == 55
"#;
        let summary = run_source(src, "memory://test_fib.agam").expect("run source");
        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn test_pipeline_nested_loops_and_accumulation() {
        let src = r#"
fn sum_grid(rows: i32, cols: i32) -> i32:
    let mut total: i32 = 0
    let mut r: i32 = 0
    while r < rows:
        let mut c: i32 = 0
        while c < cols:
            total = total + (r * c)
            c = c + 1
        r = r + 1
    return total

@test
fn test_sum_grid() -> bool:
    return sum_grid(4, 5) == 60
"#;
        let summary = run_source(src, "memory://test_sum_grid.agam").expect("run source");
        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn test_pipeline_bitwise_arithmetic() {
        let src = r#"
fn bitwise_ops(a: i32, b: i32) -> i32:
    let and_val = a & b
    let or_val = a | b
    let xor_val = a ^ b
    return and_val + or_val + xor_val

@test
fn test_bitwise() -> bool:
    return bitwise_ops(12, 10) == 28
"#;
        let summary = run_source(src, "memory://test_bitwise.agam").expect("run source");
        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn test_pipeline_conditional_logic() {
        let src = r#"
fn test_logic_func(a: bool, b: bool, c: bool) -> bool:
    if (a && b) || (!c):
        return true
    return false

@test
fn test_logic_case1() -> bool:
    return test_logic_func(true, true, true)

@test
fn test_logic_case2() -> bool:
    return test_logic_func(false, true, false)

@test
fn test_logic_case3() -> bool:
    let r = test_logic_func(false, true, true)
    return r == false
"#;
        let summary = run_source(src, "memory://test_logic.agam").expect("run source");
        assert_eq!(summary.total(), 3);
        assert_eq!(summary.passed(), 3);
    }
}
