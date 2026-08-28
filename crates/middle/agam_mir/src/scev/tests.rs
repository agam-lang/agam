use super::*;
use crate::ir::{BasicBlock, BlockId, Instruction, MirBinOp, MirFunction, Op, Terminator, ValueId};
use crate::opt::polyhedral::AffineExpr;
use agam_sema::symbol::TypeId;

fn create_mock_loop_function(
    preheader_insts: Vec<Instruction>,
    body_insts: Vec<Instruction>,
    cond_cmp_op: MirBinOp,
    cond_bound_val: ValueId,
) -> MirFunction {
    let b_entry = BlockId(0);
    let b_header = BlockId(1);
    let b_body = BlockId(2);
    let b_exit = BlockId(3);

    let cond_val = ValueId(100);

    let header_insts = vec![
        Instruction {
            result: ValueId(10),
            ty: TypeId(0),
            op: Op::LoadLocal("i".into()),
        },
        Instruction {
            result: cond_val,
            ty: TypeId(0),
            op: Op::BinOp {
                op: cond_cmp_op,
                left: ValueId(10),
                right: cond_bound_val,
            },
        },
    ];

    MirFunction {
        name: "test_loop".into(),
        generics: vec![],
        params: vec![],
        return_ty: TypeId(0),
        entry: b_entry,
        blocks: vec![
            BasicBlock {
                id: b_entry,
                instructions: preheader_insts,
                terminator: Terminator::Jump(b_header),
            },
            BasicBlock {
                id: b_header,
                instructions: header_insts,
                terminator: Terminator::Branch {
                    condition: cond_val,
                    then_block: b_body,
                    else_block: b_exit,
                },
            },
            BasicBlock {
                id: b_body,
                instructions: body_insts,
                terminator: Terminator::Jump(b_header),
            },
            BasicBlock {
                id: b_exit,
                instructions: vec![],
                terminator: Terminator::ReturnVoid,
            },
        ],
        target: Default::default(),
        gpu_config: None,
    }
}

#[test]
fn test_scev_expr_canonical_normalization() {
    let l0 = BlockId(1);
    let e1 = ScevExpr::add_rec(ScevExpr::constant(0), ScevExpr::constant(1), l0);
    let e2 = ScevExpr::constant(10);

    // Sum should sort stably
    let sum1 = ScevExpr::add(vec![e1.clone(), e2.clone()]);
    let sum2 = ScevExpr::add(vec![e2.clone(), e1.clone()]);
    assert_eq!(sum1, sum2);

    // Constant addition folding: 10 + 5 = 15
    let sum_const = ScevExpr::add(vec![ScevExpr::constant(10), ScevExpr::constant(5)]);
    assert_eq!(sum_const, ScevExpr::constant(15));
}

#[test]
fn test_canonical_1d_counted_loop_scev_and_trip_count() {
    let _b_entry = BlockId(0);
    let b_header = BlockId(1);

    // Preheader: alloca i, store i = 0, const bound = 100
    let preheader = vec![
        Instruction {
            result: ValueId(1),
            ty: TypeId(0),
            op: Op::Alloca {
                name: "i".into(),
                ty: TypeId(0),
            },
        },
        Instruction {
            result: ValueId(2),
            ty: TypeId(0),
            op: Op::ConstInt(0),
        },
        Instruction {
            result: ValueId(3),
            ty: TypeId(0),
            op: Op::StoreLocal {
                name: "i".into(),
                value: ValueId(2),
            },
        },
        Instruction {
            result: ValueId(4),
            ty: TypeId(0),
            op: Op::ConstInt(100),
        },
    ];

    // Body: load i, const 1, i = i + 1
    let body = vec![
        Instruction {
            result: ValueId(20),
            ty: TypeId(0),
            op: Op::LoadLocal("i".into()),
        },
        Instruction {
            result: ValueId(21),
            ty: TypeId(0),
            op: Op::ConstInt(1),
        },
        Instruction {
            result: ValueId(22),
            ty: TypeId(0),
            op: Op::BinOp {
                op: MirBinOp::Add,
                left: ValueId(20),
                right: ValueId(21),
            },
        },
        Instruction {
            result: ValueId(23),
            ty: TypeId(0),
            op: Op::StoreLocal {
                name: "i".into(),
                value: ValueId(22),
            },
        },
    ];

    let func = create_mock_loop_function(preheader, body, MirBinOp::Lt, ValueId(4));
    let nest = LoopNest::build(&func);
    let solver = ScevSolver::new(&func, &nest);

    // Induction variable should be { 0, +, 1 }_b_header
    let ind_opt = solver.analyze_induction_variable(b_header, "i");
    assert!(ind_opt.is_some(), "induction analysis failed");
    let Some(ind_scev) = ind_opt else {
        return;
    };
    assert_eq!(
        ind_scev,
        ScevExpr::AddRec {
            base: Box::new(ScevExpr::constant(0)),
            step: Box::new(ScevExpr::constant(1)),
            loop_id: b_header,
        }
    );

    // Trip count must be exactly 100
    let trip = solver.compute_trip_count(b_header);
    assert_eq!(trip, TripCount::Constant(100));

    // Lower to affine expression in 1D nest
    let affine_opt = lower_to_affine(&ind_scev, &[b_header]);
    assert!(affine_opt.is_some(), "should lower to affine");
    let Some(affine) = affine_opt else {
        return;
    };
    assert_eq!(
        affine,
        AffineExpr {
            constant: 0,
            coeffs: vec![1],
        }
    );
}

#[test]
fn test_gemm_2d_nested_loop_affine_lowering() {
    let l_outer = BlockId(1); // i loop
    let l_inner = BlockId(2); // j loop
    let nest_chain = vec![l_outer, l_inner];

    // Address expression: addr = i * 16 + j
    // i is {0, +, 1}_l_outer
    let i_scev = ScevExpr::add_rec(ScevExpr::constant(0), ScevExpr::constant(1), l_outer);
    // j is {0, +, 1}_l_inner
    let j_scev = ScevExpr::add_rec(ScevExpr::constant(0), ScevExpr::constant(1), l_inner);

    let i_mul_16 = ScevExpr::mul(vec![i_scev, ScevExpr::constant(16)]);
    let addr_scev = ScevExpr::add(vec![i_mul_16, j_scev]);

    let affine_opt = lower_to_affine(&addr_scev, &nest_chain);
    assert!(affine_opt.is_some(), "GEMM address must be affine");
    let Some(affine) = affine_opt else {
        return;
    };
    // Outer dim (i) = index 0 (coeff 16), Inner dim (j) = index 1 (coeff 1)
    assert_eq!(
        affine,
        AffineExpr {
            constant: 0,
            coeffs: vec![16, 1],
        }
    );
}

#[test]
fn test_fails_closed_on_lexical_variable_shadowing() {
    let b_header = BlockId(1);

    // Function has TWO Allocas for "i" (shadowed in sub-scope)
    let preheader = vec![
        Instruction {
            result: ValueId(1),
            ty: TypeId(0),
            op: Op::Alloca {
                name: "i".into(),
                ty: TypeId(0),
            },
        },
        Instruction {
            result: ValueId(2),
            ty: TypeId(0),
            op: Op::Alloca {
                name: "i".into(),
                ty: TypeId(0),
            },
        },
        Instruction {
            result: ValueId(3),
            ty: TypeId(0),
            op: Op::ConstInt(0),
        },
        Instruction {
            result: ValueId(4),
            ty: TypeId(0),
            op: Op::StoreLocal {
                name: "i".into(),
                value: ValueId(3),
            },
        },
        Instruction {
            result: ValueId(5),
            ty: TypeId(0),
            op: Op::ConstInt(10),
        },
    ];

    let body = vec![
        Instruction {
            result: ValueId(20),
            ty: TypeId(0),
            op: Op::LoadLocal("i".into()),
        },
        Instruction {
            result: ValueId(21),
            ty: TypeId(0),
            op: Op::ConstInt(1),
        },
        Instruction {
            result: ValueId(22),
            ty: TypeId(0),
            op: Op::BinOp {
                op: MirBinOp::Add,
                left: ValueId(20),
                right: ValueId(21),
            },
        },
        Instruction {
            result: ValueId(23),
            ty: TypeId(0),
            op: Op::StoreLocal {
                name: "i".into(),
                value: ValueId(22),
            },
        },
    ];

    let func = create_mock_loop_function(preheader, body, MirBinOp::Lt, ValueId(5));
    let nest = LoopNest::build(&func);
    let solver = ScevSolver::new(&func, &nest);

    // Invariant: Multiple Allocas for "i" MUST fail closed!
    assert!(solver.analyze_induction_variable(b_header, "i").is_none());
    assert_eq!(solver.compute_trip_count(b_header), TripCount::Unknown);
}

#[test]
fn test_fails_closed_on_multiple_store_locals_in_latch() {
    let b_header = BlockId(1);

    let preheader = vec![
        Instruction {
            result: ValueId(1),
            ty: TypeId(0),
            op: Op::Alloca {
                name: "i".into(),
                ty: TypeId(0),
            },
        },
        Instruction {
            result: ValueId(2),
            ty: TypeId(0),
            op: Op::ConstInt(0),
        },
        Instruction {
            result: ValueId(3),
            ty: TypeId(0),
            op: Op::StoreLocal {
                name: "i".into(),
                value: ValueId(2),
            },
        },
        Instruction {
            result: ValueId(4),
            ty: TypeId(0),
            op: Op::ConstInt(10),
        },
    ];

    // Body has TWO Stores to "i" in latch (e.g. conditional branches)
    let body = vec![
        Instruction {
            result: ValueId(20),
            ty: TypeId(0),
            op: Op::StoreLocal {
                name: "i".into(),
                value: ValueId(2),
            },
        },
        Instruction {
            result: ValueId(21),
            ty: TypeId(0),
            op: Op::StoreLocal {
                name: "i".into(),
                value: ValueId(4),
            },
        },
    ];

    let func = create_mock_loop_function(preheader, body, MirBinOp::Lt, ValueId(4));
    let nest = LoopNest::build(&func);
    let solver = ScevSolver::new(&func, &nest);

    // Invariant: Multiple StoreLocals to "i" in latch MUST fail closed!
    assert!(solver.analyze_induction_variable(b_header, "i").is_none());
    assert_eq!(solver.compute_trip_count(b_header), TripCount::Unknown);
}

#[test]
fn test_fails_closed_on_non_linear_product() {
    let l0 = BlockId(1);
    let l1 = BlockId(2);
    let nest = vec![l0, l1];

    let i = ScevExpr::add_rec(ScevExpr::constant(0), ScevExpr::constant(1), l0);
    let j = ScevExpr::add_rec(ScevExpr::constant(0), ScevExpr::constant(1), l1);

    // Non-linear product: i * j
    let non_linear = ScevExpr::mul(vec![i, j]);
    assert!(lower_to_affine(&non_linear, &nest).is_none());
}

#[test]
fn test_property_trip_count_against_reference_interpreter() {
    // Deterministic LCG for reproducible 10,000-iteration property fuzzing
    let mut rng_state: u64 = 0x9E3779B97F4A7C15;
    let mut next_rand = || -> i64 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (rng_state >> 33) as i64
    };

    // 1. Fixed edge cases
    let edge_cases = [
        (0, 10, 1, MirBinOp::Lt, 10),
        (0, 10, 1, MirBinOp::LtEq, 11),
        (5, 5, 1, MirBinOp::Lt, 0),
        (5, 5, 1, MirBinOp::LtEq, 1),
        (0, 10, 2, MirBinOp::Lt, 5),
        (0, 10, 3, MirBinOp::Lt, 4),
        (10, 0, -1, MirBinOp::Gt, 10),
        (10, 0, -2, MirBinOp::Gt, 5),
        (10, 0, -1, MirBinOp::GtEq, 11),
        (0, 0, 1, MirBinOp::Lt, 0),
        (0, 0, 1, MirBinOp::LtEq, 1),
        (-10, 10, 3, MirBinOp::Lt, 7), // -10, -7, -4, -1, 2, 5, 8 (7 iters)
        (10, -10, -4, MirBinOp::Gt, 5), // 10, 6, 2, -2, -6 (5 iters)
        // i64 extreme boundaries with i128 overflow safety
        (i64::MAX - 10, i64::MAX, 1, MirBinOp::Lt, 10),
        (i64::MIN + 10, i64::MIN, -1, MirBinOp::Gt, 10),
    ];

    for (start, bound, step, cmp_op, expected) in edge_cases {
        let count = super::solver::compute_constant_trip_count(start, bound, step, cmp_op);
        assert_eq!(
            count,
            Some(expected),
            "Edge case mismatch for ({start}, {bound}, {step}, {cmp_op:?})"
        );
    }

    // 2. 10,000-case randomized property fuzz suite
    let ops = [MirBinOp::Lt, MirBinOp::LtEq, MirBinOp::Gt, MirBinOp::GtEq];
    let candidate_steps = [-16, -7, -3, -2, -1, 1, 2, 3, 7, 16];

    for _ in 0..10_000 {
        let start = (next_rand() % 200) - 100;
        let bound = (next_rand() % 200) - 100;
        let step = candidate_steps[(next_rand().unsigned_abs() as usize) % candidate_steps.len()];
        let cmp_op = ops[(next_rand().unsigned_abs() as usize) % ops.len()];

        // Ground truth via reference interpreter
        let mut curr = start;
        let mut sim_count = 0usize;
        let mut terminated = false;

        for _ in 0..10_000 {
            let active = match cmp_op {
                MirBinOp::Lt => curr < bound,
                MirBinOp::LtEq => curr <= bound,
                MirBinOp::Gt => curr > bound,
                MirBinOp::GtEq => curr >= bound,
                _ => false,
            };
            if !active {
                terminated = true;
                break;
            }
            sim_count += 1;
            curr = curr.saturating_add(step);
        }

        let computed = super::solver::compute_constant_trip_count(start, bound, step, cmp_op);
        if terminated {
            assert_eq!(
                computed,
                Some(sim_count),
                "Fuzz mismatch for start={start}, bound={bound}, step={step}, op={cmp_op:?}"
            );
        }
    }
}

// ── End-to-End Benchmark Pipeline Integration Tests ──

fn compile_to_mir(source: &str) -> crate::ir::MirModule {
    use crate::lower::MirLowering;
    use agam_errors::span::SourceId;
    use agam_hir::lower::HirLowering;
    use agam_lexer::Lexer;

    let unindented = source
        .lines()
        .map(|l| l.trim())
        .collect::<Vec<_>>()
        .join("\n");

    let source_id = SourceId(0);
    let mut lexer = Lexer::new(&unindented, source_id);
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
    let parse_res = parser.parse_module(source_id);
    assert!(parse_res.is_ok(), "parse failed");
    let Ok(module) = parse_res else {
        return crate::ir::MirModule {
            functions: vec![],
            struct_layouts: Default::default(),
            enum_layouts: Default::default(),
        };
    };
    let mut hir_lower = HirLowering::new();
    let hir = hir_lower.lower_module(&module);
    let mut mir_lower = MirLowering::new();
    mir_lower.lower_module(&hir)
}

#[test]
fn test_real_benchmark_dot_product_pipeline() {
    let source = r#"
        fn dot_product() -> i32 {
            let mut sum: i32 = 0;
            let mut k: i32 = 0;
            while k < 64 {
                sum = sum + k;
                k = k + 1;
            }
            return sum;
        }
    "#;
    let module = compile_to_mir(source);
    let func = &module.functions[0];
    let nest = LoopNest::build(func);
    assert_eq!(nest.loops_by_header.len(), 1);

    let header_opt = nest.nest_path.first().copied();
    assert!(header_opt.is_some(), "loop must exist");
    let Some(header_id) = header_opt else {
        return;
    };
    let solver = ScevSolver::new(func, &nest);

    let k_opt = solver.analyze_induction_variable(header_id, "k");
    assert!(k_opt.is_some(), "k must be affine induction variable");
    let Some(k_scev) = k_opt else {
        return;
    };
    assert_eq!(
        k_scev,
        ScevExpr::AddRec {
            base: Box::new(ScevExpr::constant(0)),
            step: Box::new(ScevExpr::constant(1)),
            loop_id: header_id,
        }
    );

    let trip_count = solver.compute_trip_count(header_id);
    assert_eq!(trip_count, TripCount::Constant(64));

    let affine_opt = lower_to_affine(&k_scev, &[header_id]);
    assert!(affine_opt.is_some(), "k must lower to affine");
    let Some(affine) = affine_opt else {
        return;
    };
    assert_eq!(
        affine,
        AffineExpr {
            constant: 0,
            coeffs: vec![1]
        }
    );
}

#[test]
fn test_real_benchmark_matrix_multiply_gemm_nest_pipeline() {
    let source = r#"
        fn matmul() -> i32 {
            let mut total: i32 = 0;
            let mut i: i32 = 0;
            while i < 16 {
                let mut j: i32 = 0;
                while j < 16 {
                    total = total + i + j;
                    j = j + 1;
                }
                i = i + 1;
            }
            return total;
        }
    "#;
    let module = compile_to_mir(source);
    let func = &module.functions[0];
    let nest = LoopNest::build(func);
    assert_eq!(
        nest.loops_by_header.len(),
        2,
        "Must detect outer and inner loops"
    );

    let outer_header = nest.nest_path[0];
    let inner_header = nest.nest_path[1];
    let nest_chain = nest.enclosing_nest_chain(inner_header);
    assert_eq!(nest_chain, vec![outer_header, inner_header]);

    let solver = ScevSolver::new(func, &nest);

    let i_opt = solver.analyze_induction_variable(outer_header, "i");
    assert!(i_opt.is_some(), "outer i must be affine");
    let Some(i_scev) = i_opt else {
        return;
    };

    let j_opt = solver.analyze_induction_variable(inner_header, "j");
    assert!(j_opt.is_some(), "inner j must be affine");
    let Some(j_scev) = j_opt else {
        return;
    };

    assert_eq!(
        solver.compute_trip_count(outer_header),
        TripCount::Constant(16)
    );
    assert_eq!(
        solver.compute_trip_count(inner_header),
        TripCount::Constant(16)
    );

    // Address calculation: i * 16 + j
    let i_scaled = ScevExpr::mul(vec![i_scev, ScevExpr::constant(16)]);
    let addr_scev = ScevExpr::add(vec![i_scaled, j_scev]);

    let affine_opt = lower_to_affine(&addr_scev, &nest_chain);
    assert!(affine_opt.is_some(), "GEMM address must be affine");
    let Some(affine) = affine_opt else {
        return;
    };
    assert_eq!(
        affine,
        AffineExpr {
            constant: 0,
            coeffs: vec![16, 1], // Dim 0 (outer i) = 16, Dim 1 (inner j) = 1
        }
    );
}

#[test]
fn test_real_benchmark_image_blur_convolution_pipeline() {
    let source = r#"
        fn image_blur() -> i32 {
            let mut sum: i32 = 0;
            let mut y: i32 = 0;
            while y < 32 {
                let mut x: i32 = 0;
                while x < 32 {
                    sum = sum + y + x;
                    x = x + 1;
                }
                y = y + 1;
            }
            return sum;
        }
    "#;
    let module = compile_to_mir(source);
    let func = &module.functions[0];
    let nest = LoopNest::build(func);
    assert_eq!(nest.loops_by_header.len(), 2);

    let outer_y = nest.nest_path[0];
    let inner_x = nest.nest_path[1];
    let nest_chain = nest.enclosing_nest_chain(inner_x);

    let solver = ScevSolver::new(func, &nest);
    let y_opt = solver.analyze_induction_variable(outer_y, "y");
    assert!(y_opt.is_some(), "y affine failed");
    let Some(y_scev) = y_opt else {
        return;
    };

    let x_opt = solver.analyze_induction_variable(inner_x, "x");
    assert!(x_opt.is_some(), "x affine failed");
    let Some(x_scev) = x_opt else {
        return;
    };

    assert_eq!(solver.compute_trip_count(outer_y), TripCount::Constant(32));
    assert_eq!(solver.compute_trip_count(inner_x), TripCount::Constant(32));

    // Stride-32 2D pixel index: y * 32 + x
    let pixel_addr = ScevExpr::add(vec![
        ScevExpr::mul(vec![y_scev, ScevExpr::constant(32)]),
        x_scev,
    ]);

    let affine_opt = lower_to_affine(&pixel_addr, &nest_chain);
    assert!(affine_opt.is_some(), "2D pixel address must be affine");
    let Some(affine) = affine_opt else {
        return;
    };
    assert_eq!(
        affine,
        AffineExpr {
            constant: 0,
            coeffs: vec![32, 1], // Dim 0 (y) = 32, Dim 1 (x) = 1
        }
    );
}

#[test]
fn test_exact_benchmark_suite_matrix_multiply_file() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir
        .join("../../../benchmarks/suites/02_numerical_computation/matrix_multiply.agam");
    if let Ok(source) = std::fs::read_to_string(&path) {
        let module = compile_to_mir(&source);
        let checksum_opt = module
            .functions
            .iter()
            .find(|f| f.name == "matrix_checksum");
        assert!(checksum_opt.is_some(), "matrix_checksum must exist");
        let Some(checksum_fn) = checksum_opt else {
            return;
        };
        let nest = LoopNest::build(checksum_fn);
        assert_eq!(
            nest.loops_by_header.len(),
            3,
            "Matrix multiply must have a 3-deep loop nest (row, col, inner)"
        );

        // Outermost = row, Middle = col, Innermost = inner
        let row_header = nest.nest_path[0];
        let col_header = nest.nest_path[1];
        let inner_header = nest.nest_path[2];

        let solver = ScevSolver::new(checksum_fn, &nest);
        let row_opt = solver.analyze_induction_variable(row_header, "row");
        assert!(row_opt.is_some(), "row must be affine");
        let Some(row_scev) = row_opt else {
            return;
        };

        let col_opt = solver.analyze_induction_variable(col_header, "col");
        assert!(col_opt.is_some(), "col must be affine");
        let Some(col_scev) = col_opt else {
            return;
        };

        let inner_opt = solver.analyze_induction_variable(inner_header, "inner");
        assert!(inner_opt.is_some(), "inner must be affine");
        let Some(inner_scev) = inner_opt else {
            return;
        };

        assert_eq!(
            row_scev,
            ScevExpr::AddRec {
                base: Box::new(ScevExpr::constant(0)),
                step: Box::new(ScevExpr::constant(1)),
                loop_id: row_header
            }
        );
        assert_eq!(
            col_scev,
            ScevExpr::AddRec {
                base: Box::new(ScevExpr::constant(0)),
                step: Box::new(ScevExpr::constant(1)),
                loop_id: col_header
            }
        );
        assert_eq!(
            inner_scev,
            ScevExpr::AddRec {
                base: Box::new(ScevExpr::constant(0)),
                step: Box::new(ScevExpr::constant(1)),
                loop_id: inner_header
            }
        );
    }
}

#[test]
fn test_exact_benchmark_suite_pixel_filter_file() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path =
        manifest_dir.join("../../../benchmarks/suites/08_media_encoding_kernels/pixel_filter.agam");
    if let Ok(source) = std::fs::read_to_string(&path) {
        let module = compile_to_mir(&source);
        let filter_opt = module
            .functions
            .iter()
            .find(|f| f.name == "pixel_filter_checksum");
        assert!(filter_opt.is_some(), "pixel_filter_checksum must exist");
        let Some(filter_fn) = filter_opt else {
            return;
        };
        let nest = LoopNest::build(filter_fn);
        assert_eq!(
            nest.loops_by_header.len(),
            2,
            "Pixel filter must have a 2-deep loop nest (y, x)"
        );

        let y_header = nest.nest_path[0];
        let x_header = nest.nest_path[1];

        let solver = ScevSolver::new(filter_fn, &nest);
        let y_opt = solver.analyze_induction_variable(y_header, "y");
        assert!(y_opt.is_some(), "y must be affine");
        let Some(y_scev) = y_opt else {
            return;
        };

        let x_opt = solver.analyze_induction_variable(x_header, "x");
        assert!(x_opt.is_some(), "x must be affine");
        let Some(x_scev) = x_opt else {
            return;
        };

        assert_eq!(
            y_scev,
            ScevExpr::AddRec {
                base: Box::new(ScevExpr::constant(1)),
                step: Box::new(ScevExpr::constant(1)),
                loop_id: y_header
            }
        );
        assert_eq!(
            x_scev,
            ScevExpr::AddRec {
                base: Box::new(ScevExpr::constant(1)),
                step: Box::new(ScevExpr::constant(1)),
                loop_id: x_header
            }
        );
    }
}
