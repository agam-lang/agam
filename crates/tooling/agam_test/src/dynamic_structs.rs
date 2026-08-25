//! Comprehensive integration test suite for arbitrary-field dynamic structs and enums.

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
    fn test_emit_16_field_struct_in_llvm() {
        let src = r#"
struct Matrix4x4:
    m00: i64
    m01: i64
    m02: i64
    m03: i64
    m10: i64
    m11: i64
    m12: i64
    m13: i64
    m20: i64
    m21: i64
    m22: i64
    m23: i64
    m30: i64
    m31: i64
    m32: i64
    m33: i64

fn trace_matrix(mat: Matrix4x4) -> i64:
    return mat.m00 + mat.m11 + mat.m22 + mat.m33

fn main() -> i64:
    let mat = Matrix4x4 { m00: 1, m01: 0, m02: 0, m03: 0, m10: 0, m11: 2, m12: 0, m13: 0, m20: 0, m21: 0, m22: 3, m23: 0, m30: 0, m31: 0, m32: 0, m33: 4 }
    return trace_matrix(mat)
"#;
        let mir = parse_and_lower_mir(src);
        let llvm =
            emit_llvm_with_options(&mir, LlvmEmitOptions::default()).expect("LLVM emission failed");
        assert!(
            llvm.contains("%AgamStruct = type { [16 x i64] }") || llvm.contains("[16 x i64]"),
            "LLVM struct must dynamically expand to 16 fields, got:\n{llvm}"
        );
        assert!(
            llvm.contains("extractvalue %AgamStruct"),
            "must emit extractvalue for struct field access"
        );
    }

    #[test]
    fn test_emit_16_field_struct_in_c() {
        let src = r#"
struct Vector16:
    v0: i64
    v1: i64
    v2: i64
    v3: i64
    v4: i64
    v5: i64
    v6: i64
    v7: i64
    v8: i64
    v9: i64
    v10: i64
    v11: i64
    v12: i64
    v13: i64
    v14: i64
    v15: i64

fn sum_v16(vec: Vector16) -> i64:
    return vec.v0 + vec.v15

fn main() -> i64:
    let vec = Vector16 { v0: 10, v1: 0, v2: 0, v3: 0, v4: 0, v5: 0, v6: 0, v7: 0, v8: 0, v9: 0, v10: 0, v11: 0, v12: 0, v13: 0, v14: 0, v15: 20 }
    return sum_v16(vec)
"#;
        let mir = parse_and_lower_mir(src);
        let c_code = emit_c(&mir);
        assert!(
            c_code.contains("fields[16]"),
            "C struct must declare at least 16 fields, got:\n{c_code}"
        );
    }

    #[test]
    fn test_emit_32_field_struct_in_llvm_and_c() {
        let src = r#"
struct LargeState32:
    f0: i64
    f1: i64
    f2: i64
    f3: i64
    f4: i64
    f5: i64
    f6: i64
    f7: i64
    f8: i64
    f9: i64
    f10: i64
    f11: i64
    f12: i64
    f13: i64
    f14: i64
    f15: i64
    f16: i64
    f17: i64
    f18: i64
    f19: i64
    f20: i64
    f21: i64
    f22: i64
    f23: i64
    f24: i64
    f25: i64
    f26: i64
    f27: i64
    f28: i64
    f29: i64
    f30: i64
    f31: i64

fn get_last(s: LargeState32) -> i64:
    return s.f31

fn main() -> i64:
    let state = LargeState32 { f0: 0, f1: 0, f2: 0, f3: 0, f4: 0, f5: 0, f6: 0, f7: 0, f8: 0, f9: 0, f10: 0, f11: 0, f12: 0, f13: 0, f14: 0, f15: 0, f16: 0, f17: 0, f18: 0, f19: 0, f20: 0, f21: 0, f22: 0, f23: 0, f24: 0, f25: 0, f26: 0, f27: 0, f28: 0, f29: 0, f30: 0, f31: 999 }
    return get_last(state)
"#;
        let mir = parse_and_lower_mir(src);
        let llvm =
            emit_llvm_with_options(&mir, LlvmEmitOptions::default()).expect("LLVM emission failed");
        assert!(
            llvm.contains("[32 x i64]"),
            "LLVM struct must dynamically expand to 32 fields, got:\n{llvm}"
        );
        let c_code = emit_c(&mir);
        assert!(
            c_code.contains("fields[32]"),
            "C struct must dynamically expand to 32 fields, got:\n{c_code}"
        );
    }

    #[test]
    fn test_emit_12_payload_enum_in_llvm_and_c() {
        use agam_mir::ir::{
            BasicBlock, BlockId, Instruction, MirFunction, MirModule, Op, Terminator, ValueId,
        };
        use agam_sema::symbol::TypeId;
        use std::collections::HashMap;

        let mut instrs = Vec::new();
        let mut payload_ids = Vec::new();
        for i in 0..12 {
            instrs.push(Instruction {
                result: ValueId(i),
                ty: TypeId(4), // i32
                op: Op::ConstInt((i as i64) * 10),
            });
            payload_ids.push(ValueId(i));
        }
        instrs.push(Instruction {
            result: ValueId(12),
            ty: TypeId(7), // Any / Enum
            op: Op::EnumConstruct {
                tag: 3,
                payload: payload_ids,
            },
        });
        instrs.push(Instruction {
            result: ValueId(13),
            ty: TypeId(4),
            op: Op::EnumPayload {
                value: ValueId(12),
                field_index: 11,
            },
        });

        let module = MirModule {
            functions: vec![MirFunction {
                name: "main".into(),
                params: vec![],
                generics: vec![],
                return_ty: TypeId(4),
                blocks: vec![BasicBlock {
                    id: BlockId(0),
                    instructions: instrs,
                    terminator: Terminator::Return(ValueId(13)),
                }],
                entry: BlockId(0),
                target: agam_sema::target::TargetProfile::Default,
                gpu_config: None,
            }],
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
        };

        let llvm = emit_llvm_with_options(&module, LlvmEmitOptions::default())
            .expect("LLVM emission failed");
        assert!(
            llvm.contains("[12 x i64]"),
            "LLVM enum must dynamically expand to 12 payload fields, got:\n{llvm}"
        );
        let c_code = emit_c(&module);
        assert!(
            c_code.contains("payload[12]"),
            "C enum must dynamically expand to 12 payload fields, got:\n{c_code}"
        );
    }
}
