//! Hostile edge-case integration tests for escape analysis, stack promotion, and MirVerifier safety.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;

use agam_mir::ir::{
    BasicBlock, BlockId, Instruction, MirFunction, MirModule, MirParam, Op, Terminator, ValueId,
};
use agam_mir::opt::escape::{self, CalleePurityInfo, EscapeState};
use agam_mir::verifier::MirVerifier;
use agam_sema::symbol::TypeId;

#[test]
fn test_hostile_recursive_return_escape() {
    // Recursive function allocating a local struct and returning it:
    // MUST classify as GlobalEscape and DECLINE stack promotion.
    let b0 = BlockId(0);
    let b_base = BlockId(1);
    let b_rec = BlockId(2);

    let v_n = ValueId(0);
    let v_cond = ValueId(1);
    let v_local = ValueId(2);
    let v_rec_call = ValueId(3);

    let mut module = MirModule {
        functions: vec![MirFunction {
            name: "recursive_alloc".into(),
            generics: vec![],
            params: vec![MirParam {
                name: "n".into(),
                value: v_n,
                ty: TypeId(1),
                gpu_abi: Default::default(),
                memory_type: None,
            }],
            return_ty: TypeId(2),
            entry: b0,
            blocks: vec![
                BasicBlock {
                    id: b0,
                    instructions: vec![
                        Instruction {
                            result: v_cond,
                            ty: TypeId(0),
                            op: Op::BinOp {
                                op: agam_mir::ir::MirBinOp::Eq,
                                left: v_n,
                                right: v_n,
                            },
                        },
                        Instruction {
                            result: v_local,
                            ty: TypeId(2),
                            op: Op::StructConstruct {
                                name: "Node".into(),
                                fields: vec![("val".into(), v_n)],
                            },
                        },
                    ],
                    terminator: Terminator::Branch {
                        condition: v_cond,
                        then_block: b_base,
                        else_block: b_rec,
                    },
                },
                BasicBlock {
                    id: b_base,
                    instructions: vec![],
                    terminator: Terminator::Return(v_local), // ESCAPES through return
                },
                BasicBlock {
                    id: b_rec,
                    instructions: vec![Instruction {
                        result: v_rec_call,
                        ty: TypeId(2),
                        op: Op::Call {
                            callee: "recursive_alloc".into(),
                            args: vec![v_n],
                        },
                    }],
                    terminator: Terminator::Return(v_rec_call),
                },
            ],
            target: Default::default(),
            gpu_config: None,
        }],
        enum_layouts: HashMap::new(),
        struct_layouts: HashMap::new(),
    };

    let (escape_res, promo_res) =
        escape::run_escape_and_promote(&mut module, &CalleePurityInfo::default());

    // Invariant: Must NOT promote v_local because it escapes through return in b_base
    assert_eq!(
        promo_res.total_promoted, 0,
        "Escaping recursive allocation must decline promotion"
    );
    let fn_summary_opt = escape_res.functions.get("recursive_alloc");
    assert!(fn_summary_opt.is_some(), "missing summary");
    if let Some(fn_summary) = fn_summary_opt {
        assert_eq!(
            fn_summary.value_escapes.get(&v_local),
            Some(&EscapeState::GlobalEscape)
        );
    }

    // Verifier must pass without errors
    assert!(MirVerifier::verify_module(&module).is_ok());
}

#[test]
fn test_hostile_escaping_object_store() {
    // Local allocation stored into an object that escapes through return
    let b0 = BlockId(0);
    let v_container = ValueId(0);
    let v_inner = ValueId(1);
    let v_idx = ValueId(2);

    let mut module = MirModule {
        functions: vec![MirFunction {
            name: "store_and_return".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(2),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_container,
                        ty: TypeId(2),
                        op: Op::StructConstruct {
                            name: "Container".into(),
                            fields: vec![],
                        },
                    },
                    Instruction {
                        result: v_inner,
                        ty: TypeId(3),
                        op: Op::StructConstruct {
                            name: "Payload".into(),
                            fields: vec![],
                        },
                    },
                    Instruction {
                        result: v_idx,
                        ty: TypeId(1),
                        op: Op::ConstInt(0),
                    },
                    Instruction {
                        result: ValueId(4),
                        ty: TypeId(0),
                        op: Op::StoreIndex {
                            object: v_container,
                            index: v_idx,
                            value: v_inner,
                        },
                    },
                ],
                terminator: Terminator::Return(v_container), // Container escapes, so v_inner must also escape!
            }],
            target: Default::default(),
            gpu_config: None,
        }],
        enum_layouts: HashMap::new(),
        struct_layouts: HashMap::new(),
    };

    let (escape_res, promo_res) =
        escape::run_escape_and_promote(&mut module, &CalleePurityInfo::default());

    // Both container and inner payload must decline promotion
    assert_eq!(promo_res.total_promoted, 0);
    let fn_summary_opt = escape_res.functions.get("store_and_return");
    assert!(fn_summary_opt.is_some(), "missing summary");
    if let Some(fn_summary) = fn_summary_opt {
        assert_eq!(
            fn_summary.value_escapes.get(&v_container),
            Some(&EscapeState::GlobalEscape)
        );
        assert_eq!(
            fn_summary.value_escapes.get(&v_inner),
            Some(&EscapeState::GlobalEscape)
        );
    }

    assert!(MirVerifier::verify_module(&module).is_ok());
}

#[test]
fn test_hostile_effect_boundary_crossing() {
    // Local allocation passed into an effect handler operation
    let b0 = BlockId(0);
    let v_alloc = ValueId(0);
    let v_effect_res = ValueId(1);

    let mut module = MirModule {
        functions: vec![MirFunction {
            name: "perform_effect".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_alloc,
                        ty: TypeId(2),
                        op: Op::StructConstruct {
                            name: "EffectPayload".into(),
                            fields: vec![],
                        },
                    },
                    Instruction {
                        result: v_effect_res,
                        ty: TypeId(0),
                        op: Op::EffectPerform {
                            effect: "IO".into(),
                            operation: "print".into(),
                            args: vec![v_alloc],
                        },
                    },
                ],
                terminator: Terminator::ReturnVoid,
            }],
            target: Default::default(),
            gpu_config: None,
        }],
        enum_layouts: HashMap::new(),
        struct_layouts: HashMap::new(),
    };

    let (escape_res, promo_res) =
        escape::run_escape_and_promote(&mut module, &CalleePurityInfo::default());

    // Effect boundary escapes globally to the runtime handler
    assert_eq!(promo_res.total_promoted, 0);
    let fn_summary_opt = escape_res.functions.get("perform_effect");
    assert!(fn_summary_opt.is_some(), "missing summary");
    if let Some(fn_summary) = fn_summary_opt {
        assert_eq!(
            fn_summary.value_escapes.get(&v_alloc),
            Some(&EscapeState::GlobalEscape)
        );
    }

    assert!(MirVerifier::verify_module(&module).is_ok());
}

#[test]
fn test_happy_path_temporary_promotion() {
    // Pure computation with local temporary allocations that NEVER escape
    let b0 = BlockId(0);
    let v_c10 = ValueId(0);
    let v_c20 = ValueId(1);
    let v_tmp1 = ValueId(2);
    let v_tmp2 = ValueId(3);
    let v_field = ValueId(4);

    let mut module = MirModule {
        functions: vec![MirFunction {
            name: "pure_calc".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(1),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_c10,
                        ty: TypeId(1),
                        op: Op::ConstInt(10),
                    },
                    Instruction {
                        result: v_c20,
                        ty: TypeId(1),
                        op: Op::ConstInt(20),
                    },
                    Instruction {
                        result: v_tmp1,
                        ty: TypeId(2),
                        op: Op::StructConstruct {
                            name: "Vec2".into(),
                            fields: vec![("x".into(), v_c10)],
                        },
                    },
                    Instruction {
                        result: v_tmp2,
                        ty: TypeId(2),
                        op: Op::StructConstruct {
                            name: "Vec2".into(),
                            fields: vec![("x".into(), v_c20)],
                        },
                    },
                    Instruction {
                        result: v_field,
                        ty: TypeId(1),
                        op: Op::GetField {
                            object: v_tmp1,
                            field: "x".into(),
                        },
                    },
                ],
                terminator: Terminator::Return(v_field), // Returns primitive int, NOT the structs
            }],
            target: Default::default(),
            gpu_config: None,
        }],
        enum_layouts: HashMap::new(),
        struct_layouts: HashMap::new(),
    };

    let (escape_res, promo_res) =
        escape::run_escape_and_promote(&mut module, &CalleePurityInfo::default());

    // Both temporary structs are non-escaping and promoted to stack!
    assert_eq!(promo_res.total_promoted, 2);
    let fn_summary_opt = escape_res.functions.get("pure_calc");
    assert!(fn_summary_opt.is_some(), "missing summary");
    if let Some(fn_summary) = fn_summary_opt {
        assert_eq!(
            fn_summary.value_escapes.get(&v_tmp1),
            Some(&EscapeState::NoEscape)
        );
        assert_eq!(
            fn_summary.value_escapes.get(&v_tmp2),
            Some(&EscapeState::NoEscape)
        );
    }

    assert!(MirVerifier::verify_module(&module).is_ok());
}

#[test]
fn test_ast_rewrite_arc_alloc_promoted_to_alloca_with_copy_and_release_removed() {
    // Trivially droppable scalar primitive:
    // ArcAlloc -> Alloca
    // ArcRetain -> Copy
    // ArcRelease -> removed entirely (no StackDrop needed)
    let b0 = BlockId(0);
    let v_alloc = ValueId(0);
    let v_alias = ValueId(1);
    let v_rel = ValueId(2);

    let mut func = MirFunction {
        name: "test_scalar_promote".into(),
        generics: vec![],
        params: vec![],
        return_ty: TypeId(0), // Unit
        entry: b0,
        blocks: vec![BasicBlock {
            id: b0,
            instructions: vec![
                Instruction {
                    result: v_alloc,
                    ty: TypeId(1), // Int
                    op: Op::ArcAlloc {
                        name: "num".into(),
                        ty: TypeId(1),
                    },
                },
                Instruction {
                    result: v_alias,
                    ty: TypeId(1),
                    op: Op::ArcRetain { value: v_alloc },
                },
                Instruction {
                    result: v_rel,
                    ty: TypeId(0),
                    op: Op::ArcRelease { value: v_alias },
                },
            ],
            terminator: Terminator::ReturnVoid,
        }],
        target: Default::default(),
        gpu_config: None,
    };

    let changed = escape::rewrite_escape_and_promote(&mut func, &CalleePurityInfo::default());
    assert!(changed, "Expected function to be transformed by rewrite_escape_and_promote");

    // Check AST instruction sequence
    let block = &func.blocks[0];
    let mut found_alloca = false;
    let mut found_copy = false;
    let mut found_release = false;
    let mut found_stack_drop = false;

    for instr in &block.instructions {
        match &instr.op {
            Op::Alloca { name, ty } => {
                assert_eq!(instr.result, v_alloc);
                assert_eq!(name, "num");
                assert_eq!(*ty, TypeId(1));
                found_alloca = true;
            }
            Op::Copy(src) => {
                assert_eq!(instr.result, v_alias);
                assert_eq!(*src, v_alloc);
                found_copy = true;
            }
            Op::ArcRelease { .. } => {
                found_release = true;
            }
            Op::StackDrop { .. } => {
                found_stack_drop = true;
            }
            _ => {}
        }
    }

    assert!(found_alloca, "ArcAlloc must be replaced with Alloca in AST");
    assert!(found_copy, "ArcRetain must be replaced with Copy in AST");
    assert!(!found_release, "ArcRelease must be removed from AST");
    assert!(
        !found_stack_drop,
        "Primitive scalar must NOT emit StackDrop in AST"
    );

    assert!(
        MirVerifier::verify_function(&func).is_ok(),
        "Promoted function must satisfy MirVerifier"
    );
}

#[test]
fn test_ast_rewrite_non_trivial_aggregate_emits_stack_drop() {
    // Non-trivial aggregate type:
    // ArcAlloc -> Alloca
    // ArcRelease -> removed and exactly one StackDrop { value: root } emitted before return
    let b0 = BlockId(0);
    let v_alloc = ValueId(0);
    let v_rel = ValueId(1);
    let non_trivial_ty = TypeId(25); // User struct / non-primitive

    let mut func = MirFunction {
        name: "test_aggregate_promote".into(),
        generics: vec![],
        params: vec![],
        return_ty: TypeId(0),
        entry: b0,
        blocks: vec![BasicBlock {
            id: b0,
            instructions: vec![
                Instruction {
                    result: v_alloc,
                    ty: non_trivial_ty,
                    op: Op::ArcAlloc {
                        name: "agg".into(),
                        ty: non_trivial_ty,
                    },
                },
                Instruction {
                    result: v_rel,
                    ty: TypeId(0),
                    op: Op::ArcRelease { value: v_alloc },
                },
            ],
            terminator: Terminator::ReturnVoid,
        }],
        target: Default::default(),
        gpu_config: None,
    };

    let changed = escape::rewrite_escape_and_promote(&mut func, &CalleePurityInfo::default());
    assert!(changed, "Expected rewrite_escape_and_promote to succeed");

    let block = &func.blocks[0];
    let mut found_alloca = false;
    let mut found_release = false;
    let mut stack_drops = Vec::new();

    for instr in &block.instructions {
        match &instr.op {
            Op::Alloca { name, ty } => {
                assert_eq!(name, "agg");
                assert_eq!(*ty, non_trivial_ty);
                found_alloca = true;
            }
            Op::ArcRelease { .. } => {
                found_release = true;
            }
            Op::StackDrop { value } => {
                stack_drops.push(*value);
            }
            _ => {}
        }
    }

    assert!(found_alloca, "ArcAlloc must be replaced with Alloca");
    assert!(!found_release, "ArcRelease must be removed from AST");
    assert_eq!(
        stack_drops.len(),
        1,
        "Exactly one StackDrop must be emitted on exit edge"
    );
    assert_eq!(
        stack_drops[0], v_alloc,
        "StackDrop must drop the root allocation"
    );

    assert!(
        MirVerifier::verify_function(&func).is_ok(),
        "Promoted function must satisfy MirVerifier"
    );
}

#[test]
fn test_ast_rewrite_partial_escape_branch_declines_promotion() {
    // Allocation escapes on ONE branch of an if/else:
    // b0 -> branch to b_ret (escapes) and b_drop (releases)
    // Partial escapes MUST NOT partially promote: Op::ArcAlloc and Op::ArcRelease remain untouched.
    let b0 = BlockId(0);
    let b_ret = BlockId(1);
    let b_drop = BlockId(2);

    let v_alloc = ValueId(0);
    let v_cond = ValueId(1);
    let v_rel = ValueId(2);

    let mut func = MirFunction {
        name: "test_partial_escape".into(),
        generics: vec![],
        params: vec![],
        return_ty: TypeId(1),
        entry: b0,
        blocks: vec![
            BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v_alloc,
                        ty: TypeId(1),
                        op: Op::ArcAlloc {
                            name: "part".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: v_cond,
                        ty: TypeId(0),
                        op: Op::ConstInt(1),
                    },
                ],
                terminator: Terminator::Branch {
                    condition: v_cond,
                    then_block: b_ret,
                    else_block: b_drop,
                },
            },
            BasicBlock {
                id: b_ret,
                instructions: vec![],
                terminator: Terminator::Return(v_alloc), // ESCAPES here!
            },
            BasicBlock {
                id: b_drop,
                instructions: vec![Instruction {
                    result: v_rel,
                    ty: TypeId(0),
                    op: Op::ArcRelease { value: v_alloc },
                }],
                terminator: Terminator::ReturnVoid,
            },
        ],
        target: Default::default(),
        gpu_config: None,
    };

    let changed = escape::rewrite_escape_and_promote(&mut func, &CalleePurityInfo::default());
    assert!(!changed, "Partial escape MUST decline promotion");

    // Verify AST remains untouched
    assert!(matches!(
        func.blocks[0].instructions[0].op,
        Op::ArcAlloc { .. }
    ));
    assert!(matches!(
        func.blocks[2].instructions[0].op,
        Op::ArcRelease { .. }
    ));

    assert!(MirVerifier::verify_function(&func).is_ok());
}

#[test]
fn test_ast_rewrite_call_escape_declines_promotion() {
    // Allocation passed to an unknown call:
    // MUST decline promotion and leave ArcAlloc untouched
    let b0 = BlockId(0);
    let v_alloc = ValueId(0);
    let v_call_res = ValueId(1);

    let mut func = MirFunction {
        name: "test_call_escape".into(),
        generics: vec![],
        params: vec![],
        return_ty: TypeId(0),
        entry: b0,
        blocks: vec![BasicBlock {
            id: b0,
            instructions: vec![
                Instruction {
                    result: v_alloc,
                    ty: TypeId(1),
                    op: Op::ArcAlloc {
                        name: "called".into(),
                        ty: TypeId(1),
                    },
                },
                Instruction {
                    result: v_call_res,
                    ty: TypeId(0),
                    op: Op::Call {
                        callee: "opaque_external_function".into(),
                        args: vec![v_alloc],
                    },
                },
            ],
            terminator: Terminator::ReturnVoid,
        }],
        target: Default::default(),
        gpu_config: None,
    };

    let changed = escape::rewrite_escape_and_promote(&mut func, &CalleePurityInfo::default());
    assert!(!changed, "Call escape MUST decline promotion");

    assert!(matches!(
        func.blocks[0].instructions[0].op,
        Op::ArcAlloc { .. }
    ));

    assert!(MirVerifier::verify_function(&func).is_ok());
}

#[test]
fn test_ast_rewrite_phi_merge_with_param_declines_promotion() {
    // Phi node merges a local ArcAlloc with an incoming function parameter:
    // MUST decline promotion
    let b_entry = BlockId(0);
    let b_alloc = BlockId(1);
    let b_pass = BlockId(2);
    let b_merge = BlockId(3);

    let v_param = ValueId(0);
    let v_cond = ValueId(1);
    let v_alloc = ValueId(2);
    let v_phi = ValueId(3);

    let mut func = MirFunction {
        name: "test_phi_param_merge".into(),
        generics: vec![],
        params: vec![MirParam {
            name: "input".into(),
            value: v_param,
            ty: TypeId(1),
            gpu_abi: Default::default(),
            memory_type: None,
        }],
        return_ty: TypeId(1),
        entry: b_entry,
        blocks: vec![
            BasicBlock {
                id: b_entry,
                instructions: vec![Instruction {
                    result: v_cond,
                    ty: TypeId(0),
                    op: Op::ConstInt(1),
                }],
                terminator: Terminator::Branch {
                    condition: v_cond,
                    then_block: b_alloc,
                    else_block: b_pass,
                },
            },
            BasicBlock {
                id: b_alloc,
                instructions: vec![Instruction {
                    result: v_alloc,
                    ty: TypeId(1),
                    op: Op::ArcAlloc {
                        name: "phi_local".into(),
                        ty: TypeId(1),
                    },
                }],
                terminator: Terminator::Jump(b_merge),
            },
            BasicBlock {
                id: b_pass,
                instructions: vec![],
                terminator: Terminator::Jump(b_merge),
            },
            BasicBlock {
                id: b_merge,
                instructions: vec![Instruction {
                    result: v_phi,
                    ty: TypeId(1),
                    op: Op::Phi(vec![(b_alloc, v_alloc), (b_pass, v_param)]),
                }],
                terminator: Terminator::Return(v_phi),
            },
        ],
        target: Default::default(),
        gpu_config: None,
    };

    let changed = escape::rewrite_escape_and_promote(&mut func, &CalleePurityInfo::default());
    assert!(!changed, "Phi merge with param MUST decline promotion");

    // In b_alloc, instruction 0 must remain ArcAlloc
    assert!(matches!(
        func.blocks[1].instructions[0].op,
        Op::ArcAlloc { .. }
    ));

    assert!(MirVerifier::verify_function(&func).is_ok());
}

#[test]
fn test_verifier_adversarial_evasion_rejected() {
    // Feed adversarial MIR directly to MirVerifier to prove that
    // deep stack provenance checks cannot be evaded.
    let b0 = BlockId(0);
    let v_alloc = ValueId(0);
    let v_alias1 = ValueId(1);
    let v_alias2 = ValueId(2);

    // Evasion attempt 1: Copy-chain alias returned
    let bad_ret_func = MirFunction {
        name: "bad_ret".into(),
        generics: vec![],
        params: vec![],
        return_ty: TypeId(1),
        entry: b0,
        blocks: vec![BasicBlock {
            id: b0,
            instructions: vec![
                Instruction {
                    result: v_alloc,
                    ty: TypeId(1),
                    op: Op::Alloca {
                        name: "s".into(),
                        ty: TypeId(1),
                    },
                },
                Instruction {
                    result: v_alias1,
                    ty: TypeId(1),
                    op: Op::Copy(v_alloc),
                },
                Instruction {
                    result: v_alias2,
                    ty: TypeId(1),
                    op: Op::Copy(v_alias1),
                },
            ],
            terminator: Terminator::Return(v_alias2),
        }],
        target: Default::default(),
        gpu_config: None,
    };

    let err = MirVerifier::verify_function(&bad_ret_func).err();
    assert!(
        err.is_some(),
        "MirVerifier MUST reject returning an aliased stack allocation"
    );
    if let Some(e) = err {
        assert!(
            format!("{:?}", e).contains("EscapingStackAllocation"),
            "Error must be EscapingStackAllocation: {:?}",
            e
        );
    }

    // Evasion attempt 2: Stack allocation smuggled into struct field and returned
    let v_struct = ValueId(3);
    let bad_struct_func = MirFunction {
        name: "bad_struct".into(),
        generics: vec![],
        params: vec![],
        return_ty: TypeId(2),
        entry: b0,
        blocks: vec![BasicBlock {
            id: b0,
            instructions: vec![
                Instruction {
                    result: v_alloc,
                    ty: TypeId(1),
                    op: Op::Alloca {
                        name: "s".into(),
                        ty: TypeId(1),
                    },
                },
                Instruction {
                    result: v_struct,
                    ty: TypeId(2),
                    op: Op::StructConstruct {
                        name: "Wrapper".into(),
                        fields: vec![("f".into(), v_alloc)],
                    },
                },
            ],
            terminator: Terminator::Return(v_struct),
        }],
        target: Default::default(),
        gpu_config: None,
    };

    let err2 = MirVerifier::verify_function(&bad_struct_func).err();
    assert!(
        err2.is_some(),
        "MirVerifier MUST reject returning a struct wrapping a stack allocation"
    );
    if let Some(e) = err2 {
        assert!(
            format!("{:?}", e).contains("EscapingStackAllocation"),
            "Error must be EscapingStackAllocation: {:?}",
            e
        );
    }
}
