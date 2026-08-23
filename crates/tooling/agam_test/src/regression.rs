//! Regression and edge-case hardening test suite.
//!
//! Validates compiler robustness against obscure syntax patterns, boundary conditions,
//! empty constructs, and stress workloads.

#[cfg(test)]
mod tests {
    use crate::run_source;
    use agam_errors::span::SourceId;
    use agam_parser::Parser;
    use agam_sema::checker::TypeChecker;
    use agam_sema::resolver::Resolver;

    #[test]
    fn test_regression_fstring_empty_and_interpolation() {
        let src = r#"
fn format_values(x: i32) -> i32:
    let empty_str = f""
    let single_interp = f"{x}"
    let compound = f"val={x}, next={x + 1}"
    return x * 2

@test
fn test_fstrings() -> bool:
    return format_values(21) == 42
"#;
        let summary = run_source(src, "memory://test_fstring.agam").expect("run source");
        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn test_regression_deeply_nested_arithmetic_precedence() {
        let src = r#"
fn deep_math(a: i32, b: i32, c: i32) -> i32:
    return ((a + b) * (c - a) + (b * c) / 2) % 17

@test
fn test_deep_math() -> bool:
    let r = deep_math(3, 5, 8)
    return r == ((8 * 5 + 20) % 17)
"#;
        let summary = run_source(src, "memory://test_deep_math.agam").expect("run source");
        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn test_regression_multi_branch_chained_conditions() {
        let src = r#"
fn classify(n: i32) -> i32:
    if n < 0:
        return -1
    else if n == 0:
        return 0
    else if n < 10:
        return 1
    else if n < 100:
        return 2
    else:
        return 3

@test
fn test_classify_negative() -> bool:
    return classify(-5) == -1

@test
fn test_classify_zero() -> bool:
    return classify(0) == 0

@test
fn test_classify_small() -> bool:
    return classify(7) == 1

@test
fn test_classify_medium() -> bool:
    return classify(42) == 2

@test
fn test_classify_large() -> bool:
    return classify(999) == 3
"#;
        let summary = run_source(src, "memory://test_classify.agam").expect("run source");
        assert_eq!(summary.total(), 5);
        assert_eq!(summary.passed(), 5);
    }

    #[test]
    fn test_regression_struct_with_multiple_method_invocations() {
        let src = r#"
struct Accumulator:
    val: i32

impl Accumulator:
    fn add(self, delta: i32) -> i32:
        return self.val + delta

    fn scale_and_add(self, factor: i32, delta: i32) -> i32:
        return (self.val * factor) + delta

@test
fn test_accumulator_methods() -> bool:
    let acc = Accumulator { val: 10 }
    let a = acc.add(5)
    let b = acc.scale_and_add(3, 2)
    return a == 15 && b == 32
"#;
        let summary = run_source(src, "memory://test_struct_methods.agam").expect("run source");
        assert_eq!(summary.total(), 1);
        assert_eq!(summary.passed(), 1);
    }

    #[test]
    fn test_regression_graceful_lexer_error_recovery() {
        // Unterminated string or invalid character should not panic
        let src = "let x = @invalid_syntax_symbol_here_###";
        let tokens = agam_lexer::tokenize(src, SourceId(0));
        assert!(
            !tokens.is_empty(),
            "lexer should emit tokens/error tokens without crashing"
        );
    }

    #[test]
    fn test_regression_graceful_parser_error_recovery() {
        // Incomplete function declaration should return Err(ParseError), not panic
        let src = "fn incomplete_function(a: i32";
        let tokens = agam_lexer::tokenize(src, SourceId(0));
        let mut parser = Parser::new(tokens);
        let result = parser.parse_module(SourceId(0));
        assert!(
            result.is_err(),
            "parser should gracefully return error on malformed tokens"
        );
    }

    #[test]
    fn test_regression_graceful_sema_type_mismatch() {
        // Assigning string to int should produce a structured diagnostic without crashing
        let src = "fn bad_types() -> i32 {\nlet x: i32 = \"not an int\";\nreturn x;\n}";
        let tokens = agam_lexer::tokenize(src, SourceId(0));
        let mut parser = Parser::new(tokens);
        if let Ok(module) = parser.parse_module(SourceId(0)) {
            let mut resolver = Resolver::new();
            resolver.resolve_module(&module);
            let mut checker = TypeChecker::from_resolver(resolver);
            checker.check_module(&module);
            assert!(
                !checker.errors.is_empty(),
                "type checker should record type mismatch error"
            );
        }
    }
}
