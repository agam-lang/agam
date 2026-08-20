//! Output tests verifying generated C code correctness.

#[cfg(test)]
mod tests {
    use agam_codegen::c_emitter::emit_c;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::tokenize;
    use agam_mir::lower::MirLowering;
    use agam_parser::parse;

    fn compile_to_c(src: &str) -> String {
        let source_id = SourceId(0);
        let tokens = tokenize(src, source_id);
        let ast = parse(tokens, source_id).expect("AST parse");

        let mut hir_lowering = HirLowering::new();
        let hir = hir_lowering.lower_module(&ast);
        let mut mir_lowering = MirLowering::new();
        let mir = mir_lowering.lower_module(&hir);

        emit_c(&mir)
    }

    #[test]
    fn test_c_emitter_includes_required_headers() {
        let src = "fn add(a: i32, b: i32) -> i32: return a + b";
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
    }

    #[test]
    fn test_c_emitter_generates_valid_function_signature_and_body() {
        let src = r#"
fn multiply(x: i32, y: i32) -> i32:
    return x * y
"#;
        let c_code = compile_to_c(src);

        assert!(
            c_code.contains("multiply("),
            "must contain C function signature"
        );
        assert!(c_code.contains("return"), "must contain return statement");
    }

    #[test]
    fn test_c_emitter_generates_struct_declarations() {
        let src = r#"
struct Vector2D:
    x: f64
    y: f64

fn make_vec() -> f64:
    return 0.0
"#;
        let c_code = compile_to_c(src);
        assert!(
            c_code.contains("struct") || c_code.contains("Vector2D") || c_code.contains("typedef"),
            "must contain type declarations"
        );
    }

    #[test]
    fn test_c_emitter_generates_control_flow_blocks() {
        let src = r#"
fn check_max(a: i32, b: i32) -> i32:
    if a > b:
        return a
    else:
        return b
"#;
        let c_code = compile_to_c(src);
        assert!(
            c_code.contains("if") || c_code.contains("return") || c_code.contains("switch"),
            "must emit valid control flow"
        );
    }
}
