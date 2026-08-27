//! Hostile edge-case integration tests for escape analysis, stack promotion, and MirVerifier safety.

#![deny(clippy::unwrap_used)]

use std::collections::HashMap;

use agam_mir::ir::{BasicBlock, BlockId, Instruction, MirFunction, MirModule, MirParam, Op, Terminator, ValueId};
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

    let (escape_res, promo_res) = escape::run_escape_and_promote(&mut module, &CalleePurityInfo::default());

    // Invariant: Must NOT promote v_local because it escapes through return in b_base
    assert_eq!(promo_res.total_promoted, 0, "Escaping recursive allocation must decline promotion");
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

    let (escape_res, promo_res) = escape::run_escape_and_promote(&mut module, &CalleePurityInfo::default());

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

    let (escape_res, promo_res) = escape::run_escape_and_promote(&mut module, &CalleePurityInfo::default());

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

    let (escape_res, promo_res) = escape::run_escape_and_promote(&mut module, &CalleePurityInfo::default());

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
