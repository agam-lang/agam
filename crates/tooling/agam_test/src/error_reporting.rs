//! Error tests verifying accurate error detection and diagnostic reporting.

#[cfg(test)]
mod tests {
    use agam_errors::span::SourceId;
    use agam_lexer::tokenize;
    use agam_parser::parse;
    use agam_sema::gpu::validate_gpu_kernel_body;
    use agam_sema::resolver::Resolver;

    #[test]
    fn test_error_syntax_missing_delimiter() {
        let src = "fn broken(: i32) -> i32: return 0";
        let source_id = SourceId(0);
        let tokens = tokenize(src, source_id);
        let res = parse(tokens, source_id);
        assert!(
            res.is_err(),
            "parser must report syntax error for missing identifier"
        );
    }

    #[test]
    fn test_error_unresolved_variable() {
        let src = "fn bad_var() -> i32:\n    return undefined_symbol + 1";
        let source_id = SourceId(0);
        let tokens = tokenize(src, source_id);
        let ast = parse(tokens, source_id).unwrap();

        let mut resolver = Resolver::new();
        resolver.resolve_module(&ast);
        assert!(
            !resolver.errors.is_empty(),
            "resolver must detect undefined symbol"
        );
    }

    #[test]
    fn test_error_gpu_kernel_constraint_violations() {
        // Direct recursion in GPU kernel must be rejected
        let errors = validate_gpu_kernel_body(
            false, // no effects
            false, // no strings
            false, // no heap
            true,  // calls self (recursion)
            "my_kernel",
            &[],
        );
        assert!(
            !errors.is_empty(),
            "GPU validation must reject recursive kernel"
        );

        // Heap allocation in GPU kernel must be rejected
        let errors_heap = validate_gpu_kernel_body(
            false,
            false,
            true, // heap alloc
            false,
            "my_kernel",
            &[],
        );
        assert!(
            !errors_heap.is_empty(),
            "GPU validation must reject heap allocation"
        );

        // Effects in GPU kernel must be rejected
        let errors_effects = validate_gpu_kernel_body(
            true, // effects
            false,
            false,
            false,
            "my_kernel",
            &[],
        );
        assert!(
            !errors_effects.is_empty(),
            "GPU validation must reject effects in kernel"
        );
    }
}
