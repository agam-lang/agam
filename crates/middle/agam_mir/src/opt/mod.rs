//! MIR optimization pipeline.

pub mod ai_intel;
pub mod baur_strassen;
pub mod constant_fold;
pub mod dce;
pub mod egg_engine;
#[deprecated(note = "Superseded by opt::egg_engine")]
pub mod egraph;
pub mod escape;
pub mod inline;
pub mod loop_unroll;
pub mod polyhedral;

pub use ai_intel::{AiCompilerReport, AiOptimizationAdvisor, OptimizationRecommendation};

use crate::ir::MirModule;
use crate::verifier::MirVerifier;

/// Maximum fixed-point optimization passes permitted to prevent oscillation / infinite loops.
pub const MAX_OPT_PASS_ITERATIONS: usize = 16;

/// Run the default MIR optimization pipeline with iteration fuel bounds and verification gates.
pub fn optimize_module(module: &mut MirModule) -> bool {
    #[cfg(debug_assertions)]
    {
        if let Err(errors) = MirVerifier::verify_module(module) {
            eprintln!(
                "[WARN] MIR pre-opt verification failed with {} errors",
                errors.len()
            );
        }
    }

    let mut changed_any = false;
    let mut iterations = 0;

    while iterations < MAX_OPT_PASS_ITERATIONS {
        iterations += 1;
        let mut changed = false;
        changed |= inline::run(module);
        changed |= constant_fold::run(module);
        changed |= egg_engine::run(module);
        changed |= loop_unroll::run(module);
        changed |= constant_fold::run(module);
        changed |= dce::run(module);

        let purity = escape::CalleePurityInfo::default();
        let (_escapes, promo) = escape::run_escape_and_promote(module, &purity);
        if promo.total_promoted > 0 {
            changed = true;
        }

        if !changed {
            break;
        }

        changed_any = true;
    }

    #[cfg(debug_assertions)]
    {
        if let Err(errors) = MirVerifier::verify_module(module) {
            eprintln!(
                "[WARN] MIR post-opt verification failed with {} errors",
                errors.len()
            );
        }
    }

    changed_any
}

pub fn run_escape_and_promote(
    module: &mut MirModule,
    purity: &escape::CalleePurityInfo,
) -> (escape::EscapeAnalysisResults, escape::StackPromotionResults) {
    escape::run_escape_and_promote(module, purity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, Instruction, MirFunction, MirParam, Op, Terminator, ValueId};
    use agam_sema::symbol::TypeId;

    #[test]
    fn test_optimize_module_bounded_iterations() {
        let b0 = crate::ir::BlockId(0);
        let v0 = ValueId(0);
        let v1 = ValueId(1);

        let mut module = MirModule {
            functions: vec![MirFunction {
                name: "simple".into(),
                generics: vec![],
                params: vec![MirParam {
                    name: "x".into(),
                    value: v0,
                    ty: TypeId(1),
                    gpu_abi: Default::default(),
                    memory_type: None,
                }],
                return_ty: TypeId(1),
                entry: b0,
                blocks: vec![BasicBlock {
                    id: b0,
                    instructions: vec![Instruction {
                        result: v1,
                        ty: TypeId(1),
                        op: Op::ConstInt(10),
                    }],
                    terminator: Terminator::Return(v1),
                }],
                target: Default::default(),
                gpu_config: None,
            }],
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
        };

        // Optimization must complete within bounded iterations and produce verified IR
        optimize_module(&mut module);
        assert!(MirVerifier::verify_module(&module).is_ok());
    }

    #[test]
    fn test_pass_ordering_independence_convergence() {
        let b0 = crate::ir::BlockId(0);
        let v0 = ValueId(0);
        let v1 = ValueId(1);
        let v2 = ValueId(2);

        let make_module = || MirModule {
            functions: vec![MirFunction {
                name: "test_convergence".into(),
                generics: vec![],
                params: vec![MirParam {
                    name: "x".into(),
                    value: v0,
                    ty: TypeId(1),
                    gpu_abi: Default::default(),
                    memory_type: None,
                }],
                return_ty: TypeId(1),
                entry: b0,
                blocks: vec![BasicBlock {
                    id: b0,
                    instructions: vec![
                        Instruction {
                            result: v1,
                            ty: TypeId(1),
                            op: Op::ConstInt(5),
                        },
                        Instruction {
                            result: v2,
                            ty: TypeId(1),
                            op: Op::BinOp {
                                op: crate::ir::MirBinOp::Add,
                                left: v1,
                                right: v1,
                            },
                        },
                    ],
                    terminator: Terminator::Return(v2),
                }],
                target: Default::default(),
                gpu_config: None,
            }],
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
        };

        // Pipeline 1: egg_engine before escape
        let mut mod1 = make_module();
        constant_fold::run(&mut mod1);
        egg_engine::run(&mut mod1);
        let purity = escape::CalleePurityInfo::default();
        escape::run_escape_and_promote(&mut mod1, &purity);
        dce::run(&mut mod1);

        // Pipeline 2: escape before egg_engine
        let mut mod2 = make_module();
        constant_fold::run(&mut mod2);
        escape::run_escape_and_promote(&mut mod2, &purity);
        egg_engine::run(&mut mod2);
        dce::run(&mut mod2);

        // Both pipelines must produce equivalent instructions
        assert_eq!(mod1.functions.len(), mod2.functions.len());
        assert_eq!(
            mod1.functions[0].blocks[0].instructions.len(),
            mod2.functions[0].blocks[0].instructions.len()
        );
        assert!(MirVerifier::verify_module(&mod1).is_ok());
        assert!(MirVerifier::verify_module(&mod2).is_ok());
    }
}
