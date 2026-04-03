//! Escape-analysis and stack-promotion shims.
//!
//! The current backend pipeline expects these summaries, but the deeper
//! analysis is still staged separately. Keep the API stable and conservative.

use std::collections::BTreeMap;

use crate::ir::MirModule;

#[derive(Clone, Debug, Default)]
pub struct CalleePurityInfo;

#[derive(Clone, Debug, Default)]
pub struct FunctionEscapeSummary;

#[derive(Clone, Debug, Default)]
pub struct EscapeAnalysisResults {
    pub functions: BTreeMap<String, FunctionEscapeSummary>,
}

#[derive(Clone, Debug, Default)]
pub struct FunctionPromotionSummary {
    pub promoted_locals: Vec<String>,
    pub skipped: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct StackPromotionResults {
    pub total_promoted: usize,
    pub total_arc_elided: usize,
    pub functions: BTreeMap<String, FunctionPromotionSummary>,
}

pub fn run_escape_and_promote(
    module: &mut MirModule,
    _purity: &CalleePurityInfo,
) -> (EscapeAnalysisResults, StackPromotionResults) {
    let functions = module
        .functions
        .iter()
        .map(|function| (function.name.clone(), FunctionEscapeSummary))
        .collect();

    (
        EscapeAnalysisResults { functions },
        StackPromotionResults::default(),
    )
}

#[cfg(test)]
mod tests {
    use agam_sema::symbol::TypeId;

    use super::*;
    use crate::ir::{BlockId, MirFunction};

    #[test]
    fn escape_shim_reports_each_function_without_promotions() {
        let mut module = MirModule {
            functions: vec![MirFunction {
                name: "main".into(),
                params: Vec::new(),
                return_ty: TypeId(0),
                blocks: Vec::new(),
                entry: BlockId(0),
            }],
        };

        let (escape, promotion) = run_escape_and_promote(&mut module, &CalleePurityInfo);
        assert_eq!(escape.functions.len(), 1);
        assert_eq!(promotion.total_promoted, 0);
        assert_eq!(promotion.total_arc_elided, 0);
        assert!(promotion.functions.is_empty());
    }
}
