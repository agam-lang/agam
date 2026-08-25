//! Dynamic memory buffer and array/slice indexing integration tests.

#[cfg(test)]
mod tests {
    use agam_codegen::c_emitter::emit_c;
    use agam_codegen::llvm_emitter::{LlvmEmitOptions, emit_llvm_with_options};
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;
    use agam_mir::lower::MirLowering;

    fn parse_and_lower_mir(source: &str) -> agam_mir::ir::MirModule {
        let source_id = SourceId(0);
        let mut lexer = Lexer::new(source, source_id);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == agam_lexer::TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        let mut parser = agam_parser::Parser::new(tokens);
        let module = parser.parse_module(source_id).expect("parsing failed");
        let mut hir_lower = HirLowering::new();
        let hir = hir_lower.lower_module(&module);
        let mut mir_lower = MirLowering::new();
        mir_lower.lower_module(&hir)
    }

    #[test]
    fn test_llvm_array_allocation_and_indexing() {
        let src = r#"
fn array_ops() -> i32:
    let mut buf = [100, 200, 300, 400]
    buf[2] = 999
    return buf[2]

fn main() -> i32:
    return array_ops()
"#;
        let mir = parse_and_lower_mir(src);
        let llvm =
            emit_llvm_with_options(&mir, LlvmEmitOptions::default()).expect("LLVM emission failed");
        assert!(
            llvm.contains("getelementptr inbounds"),
            "must emit LLVM GEP for buffer indexing"
        );
        assert!(
            llvm.contains("store "),
            "must emit LLVM store for buffer mutation"
        );
        assert!(
            llvm.contains("load "),
            "must emit LLVM load for buffer read"
        );
    }

    #[test]
    fn test_c_array_allocation_and_indexing() {
        let src = r#"
fn array_ops() -> i32:
    let mut buf = [10, 20, 30]
    buf[1] = 77
    return buf[1]
"#;
        let mir = parse_and_lower_mir(src);
        let c_code = emit_c(&mir);
        assert!(
            c_code.contains("((agam_int*)__v"),
            "must emit C pointer indexed store/load"
        );
    }

    #[test]
    fn test_2d_matrix_buffer_indexing() {
        let src = r#"
fn matrix_convolution() -> i32:
    let mut grid = [1, 2, 3, 4, 5, 6, 7, 8, 9]
    let mut sum: i32 = 0
    let mut i: i32 = 0
    while i < 9:
        sum = sum + grid[i]
        i = i + 1
    return sum

fn main() -> i32:
    return matrix_convolution()
"#;
        let mir = parse_and_lower_mir(src);
        let llvm =
            emit_llvm_with_options(&mir, LlvmEmitOptions::default()).expect("LLVM emission failed");
        assert!(
            llvm.contains("getelementptr inbounds"),
            "must emit GEP inside convolution loop"
        );
        assert!(
            llvm.contains("load "),
            "must emit load inside convolution loop"
        );
    }
}
