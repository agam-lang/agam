//! Unit tests verifying each compiler pass in total isolation.

#[cfg(test)]
mod tests {
    use agam_ast::decl::DeclKind;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_jit::{CompiledJitModule, JitOptions, JitValue};
    use agam_lexer::{TokenKind, tokenize};
    use agam_mir::lower::MirLowering;
    use agam_parser::parse;
    use agam_sema::checker::TypeChecker;
    use agam_sema::resolver::Resolver;

    // ── Pass 1: Lexer Isolation ──
    #[test]
    fn test_pass_lexer_tokenizes_primitives_and_keywords() {
        let src = "let mut count: i32 = 42\nfn add(x: f64) -> f64: return x + 3.14";
        let tokens = tokenize(src, SourceId(0));
        let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();

        assert!(kinds.contains(&TokenKind::Let));
        assert!(kinds.contains(&TokenKind::Mut));
        assert!(kinds.contains(&TokenKind::Identifier));
        assert!(kinds.contains(&TokenKind::Colon));
        assert!(kinds.contains(&TokenKind::IntLiteral));
        assert!(kinds.contains(&TokenKind::Fn));
        assert!(kinds.contains(&TokenKind::Arrow));
        assert!(kinds.contains(&TokenKind::FloatLiteral));
        assert!(kinds.contains(&TokenKind::Plus));
    }

    #[test]
    fn test_pass_lexer_preserves_span_accuracy() {
        let src = "let target = 100";
        let tokens = tokenize(src, SourceId(0));
        let target_tok = tokens.iter().find(|t| t.lexeme == "target").unwrap();
        assert_eq!(target_tok.span.start, 4);
        assert_eq!(target_tok.span.end, 10);
    }

    // ── Pass 2: Parser Isolation ──
    #[test]
    fn test_pass_parser_constructs_ast_tree() {
        let src = "fn compute(a: i32, b: i32) -> i32: return a * b + 1";
        let tokens = tokenize(src, SourceId(0));
        let ast = parse(tokens, SourceId(0)).expect("AST parse should succeed");

        assert_eq!(ast.declarations.len(), 1);
        if let DeclKind::Function(f) = &ast.declarations[0].kind {
            assert_eq!(f.name.name, "compute");
            assert_eq!(f.params.len(), 2);
            assert!(f.return_type.is_some());
            assert!(f.body.is_some());
        } else {
            panic!("expected function declaration");
        }
    }

    // ── Pass 3: Semantic Analysis / Type Checker Isolation ──
    #[test]
    fn test_pass_sema_resolution_and_type_check() {
        let src = "fn add(a: i32, b: i32) -> i32: return a + b";
        let tokens = tokenize(src, SourceId(0));
        let ast = parse(tokens, SourceId(0)).expect("AST parse");

        let mut resolver = Resolver::new();
        resolver.resolve_module(&ast);
        assert!(
            resolver.errors.is_empty(),
            "symbol resolution should have 0 errors"
        );

        let mut checker = TypeChecker::from_resolver(resolver);
        checker.check_module(&ast);
        assert!(checker.errors.is_empty(), "type check should have 0 errors");
    }

    // ── Pass 4: HIR Lowering Isolation ──
    #[test]
    fn test_pass_hir_lowering_transforms_ast() {
        let src = "fn identity(x: i32) -> i32: return x";
        let tokens = tokenize(src, SourceId(0));
        let ast = parse(tokens, SourceId(0)).expect("AST parse");

        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        assert_eq!(hir.functions.len(), 1);
        assert_eq!(hir.functions[0].name, "identity");
        assert_eq!(hir.functions[0].params.len(), 1);
    }

    // ── Pass 5: MIR SSA Lowering Isolation ──
    #[test]
    fn test_pass_mir_lowering_constructs_cfg() {
        let src = "fn branch_calc(cond: bool) -> i32:\n    if cond:\n        return 10\n    else:\n        return 20";
        let tokens = tokenize(src, SourceId(0));
        let ast = parse(tokens, SourceId(0)).expect("AST parse");
        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        let mir = mir_lowering.lower_module(&hir);

        assert_eq!(mir.functions.len(), 1);
        let func = &mir.functions[0];
        assert_eq!(func.name, "branch_calc");
        assert!(
            func.blocks.len() >= 2,
            "expected multiple basic blocks for branching"
        );
    }

    // ── Pass 6: JIT and Codegen Backend Isolation ──
    #[test]
    fn test_pass_jit_execution_computes_exact_result() {
        let src = r#"
@test
fn multiply_test() -> i32:
    let x: i32 = 6
    let y: i32 = 7
    return x * y
"#;
        let tokens = tokenize(src, SourceId(0));
        let ast = parse(tokens, SourceId(0)).expect("AST parse");
        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        let mir = mir_lowering.lower_module(&hir);

        let compiled =
            CompiledJitModule::compile(&mir, JitOptions::default()).expect("JIT compile");
        let result = compiled
            .run_function("multiply_test", &[])
            .expect("JIT run");

        assert_eq!(result, JitValue::Int(42));
    }
}
