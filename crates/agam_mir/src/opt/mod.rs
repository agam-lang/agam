//! MIR optimization pipeline.

pub mod constant_fold;
pub mod dce;
pub mod escape;
pub mod inline;
pub mod loop_unroll;

use crate::ir::MirModule;

/// Run the default MIR optimization pipeline in a fixed-point loop.
pub fn optimize_module(module: &mut MirModule) -> bool {
    let mut changed_any = false;

    loop {
        let mut changed = false;
        changed |= inline::run(module);
        changed |= constant_fold::run(module);
        changed |= loop_unroll::run(module);
        changed |= constant_fold::run(module);
        changed |= dce::run(module);

        if !changed {
            break;
        }

        changed_any = true;
    }

    changed_any
}

pub fn run_escape_and_promote(
    module: &mut MirModule,
    purity: &escape::CalleePurityInfo,
) -> (escape::EscapeAnalysisResults, escape::StackPromotionResults) {
    escape::run_escape_and_promote(module, purity)
}
