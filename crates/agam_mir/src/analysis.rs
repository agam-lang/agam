//! Shared MIR-level call-cache selection analysis.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ir::MirModule;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallCacheRequest {
    pub enable_all: bool,
    pub optimize_all: bool,
    pub include_only: BTreeSet<String>,
    pub optimize_only: BTreeSet<String>,
    pub exclude: BTreeSet<String>,
}

impl CallCacheRequest {
    fn requests_function(&self, function: &str) -> bool {
        if self.exclude.contains(function) {
            return false;
        }

        self.enable_all
            || self.optimize_all
            || self.include_only.contains(function)
            || self.optimize_only.contains(function)
    }

    fn mode_for(&self, function: &str) -> Option<CallCacheMode> {
        if !self.requests_function(function) {
            return None;
        }

        if self.optimize_all || self.optimize_only.contains(function) {
            Some(CallCacheMode::Optimize)
        } else {
            Some(CallCacheMode::Basic)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallCacheMode {
    Basic,
    Optimize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallCacheFunctionAnalysis {
    pub name: String,
    pub requested: bool,
    pub eligible: bool,
    pub mode: Option<CallCacheMode>,
    pub rejection_reasons: Vec<CallCacheRejectReason>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallCacheAnalysis {
    pub functions: Vec<CallCacheFunctionAnalysis>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallCacheRejectReason {
    TooManyArguments {
        actual: usize,
        max_supported: usize,
    },
    UnsupportedReturnType {
        description: String,
    },
    UnsupportedParameterType {
        index: usize,
        description: String,
    },
}

impl fmt::Display for CallCacheRejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallCacheRejectReason::TooManyArguments {
                actual,
                max_supported,
            } => write!(
                f,
                "needs {actual} arguments but the current cache supports at most {max_supported}"
            ),
            CallCacheRejectReason::UnsupportedReturnType { description } => {
                write!(f, "unsupported return type: {description}")
            }
            CallCacheRejectReason::UnsupportedParameterType { index, description } => {
                write!(f, "unsupported parameter {index}: {description}")
            }
        }
    }
}

pub fn analyze_call_cache(
    module: &MirModule,
    request: &CallCacheRequest,
    support_reasons: &BTreeMap<String, Vec<CallCacheRejectReason>>,
) -> CallCacheAnalysis {
    let functions = module
        .functions
        .iter()
        .map(|function| {
            let requested = request.requests_function(&function.name);
            let rejection_reasons = if requested {
                support_reasons
                    .get(&function.name)
                    .cloned()
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            let eligible = requested && rejection_reasons.is_empty();
            let mode = if eligible {
                request.mode_for(&function.name)
            } else {
                None
            };

            CallCacheFunctionAnalysis {
                name: function.name.clone(),
                requested,
                eligible,
                mode,
                rejection_reasons,
            }
        })
        .collect();

    CallCacheAnalysis { functions }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use agam_sema::symbol::TypeId;

    use super::*;
    use crate::ir::{BlockId, MirFunction, MirModule};

    fn module_with_functions(names: &[&str]) -> MirModule {
        MirModule {
            functions: names
                .iter()
                .enumerate()
                .map(|(index, name)| MirFunction {
                    name: (*name).into(),
                    params: Vec::new(),
                    return_ty: TypeId(0),
                    blocks: Vec::new(),
                    entry: BlockId(index as u32),
                })
                .collect(),
        }
    }

    #[test]
    fn global_basic_request_marks_supported_functions_eligible() {
        let module = module_with_functions(&["hot", "cold"]);
        let mut support_reasons = BTreeMap::new();
        support_reasons.insert(
            "cold".into(),
            vec![CallCacheRejectReason::TooManyArguments {
                actual: 5,
                max_supported: 4,
            }],
        );

        let analysis = analyze_call_cache(
            &module,
            &CallCacheRequest {
                enable_all: true,
                ..CallCacheRequest::default()
            },
            &support_reasons,
        );

        assert_eq!(analysis.functions.len(), 2);
        assert_eq!(analysis.functions[0].name, "hot");
        assert!(analysis.functions[0].requested);
        assert!(analysis.functions[0].eligible);
        assert_eq!(analysis.functions[0].mode, Some(CallCacheMode::Basic));

        assert_eq!(analysis.functions[1].name, "cold");
        assert!(analysis.functions[1].requested);
        assert!(!analysis.functions[1].eligible);
        assert_eq!(analysis.functions[1].mode, None);
        assert_eq!(analysis.functions[1].rejection_reasons.len(), 1);
    }

    #[test]
    fn selective_requests_can_mix_basic_and_optimize_modes() {
        let module = module_with_functions(&["hot", "basic", "idle"]);
        let analysis = analyze_call_cache(
            &module,
            &CallCacheRequest {
                include_only: ["basic".into()].into_iter().collect(),
                optimize_only: ["hot".into()].into_iter().collect(),
                ..CallCacheRequest::default()
            },
            &BTreeMap::new(),
        );

        assert_eq!(analysis.functions[0].mode, Some(CallCacheMode::Optimize));
        assert_eq!(analysis.functions[1].mode, Some(CallCacheMode::Basic));
        assert!(!analysis.functions[2].requested);
        assert!(!analysis.functions[2].eligible);
        assert_eq!(analysis.functions[2].mode, None);
    }

    #[test]
    fn excludes_override_global_optimize_mode() {
        let module = module_with_functions(&["hot", "skip"]);
        let analysis = analyze_call_cache(
            &module,
            &CallCacheRequest {
                enable_all: true,
                optimize_all: true,
                exclude: ["skip".into()].into_iter().collect(),
                ..CallCacheRequest::default()
            },
            &BTreeMap::new(),
        );

        assert_eq!(analysis.functions[0].mode, Some(CallCacheMode::Optimize));
        assert!(analysis.functions[0].eligible);

        assert!(!analysis.functions[1].requested);
        assert!(!analysis.functions[1].eligible);
        assert_eq!(analysis.functions[1].mode, None);
    }
}
