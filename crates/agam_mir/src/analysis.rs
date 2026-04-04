//! Shared MIR-level call-cache selection analysis.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ir::{MirFunction, MirModule, Op};

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CallCacheRejectReason {
    TooManyArguments { actual: usize, max_supported: usize },
    UnsupportedReturnType { description: String },
    UnsupportedParameterType { index: usize, description: String },
    CallsImpureBuiltin { builtin: String },
    CallsImpureFunction { callee: String },
    CallsUnknownFunction { callee: String },
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
            CallCacheRejectReason::CallsImpureBuiltin { builtin } => {
                write!(f, "calls impure builtin `{builtin}`")
            }
            CallCacheRejectReason::CallsImpureFunction { callee } => {
                write!(f, "calls impure function `{callee}`")
            }
            CallCacheRejectReason::CallsUnknownFunction { callee } => {
                write!(f, "calls unknown function `{callee}`")
            }
        }
    }
}

pub fn semantic_call_cache_rejection_reasons(
    module: &MirModule,
) -> BTreeMap<String, Vec<CallCacheRejectReason>> {
    let functions_by_name: BTreeMap<&str, &MirFunction> = module
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function))
        .collect();
    let mut memo = BTreeMap::new();
    let mut visiting = BTreeSet::new();

    for function in &module.functions {
        collect_semantic_call_cache_rejection_reasons(
            function.name.as_str(),
            &functions_by_name,
            &mut memo,
            &mut visiting,
        );
    }

    memo
}

fn collect_semantic_call_cache_rejection_reasons(
    function_name: &str,
    functions_by_name: &BTreeMap<&str, &MirFunction>,
    memo: &mut BTreeMap<String, Vec<CallCacheRejectReason>>,
    visiting: &mut BTreeSet<String>,
) -> Vec<CallCacheRejectReason> {
    if let Some(reasons) = memo.get(function_name) {
        return reasons.clone();
    }

    if !visiting.insert(function_name.to_string()) {
        return Vec::new();
    }

    let mut reasons = BTreeSet::new();

    if let Some(function) = functions_by_name.get(function_name) {
        for block in &function.blocks {
            for instr in &block.instructions {
                let Op::Call { callee, .. } = &instr.op else {
                    continue;
                };

                if functions_by_name.contains_key(callee.as_str()) {
                    let callee_reasons = collect_semantic_call_cache_rejection_reasons(
                        callee,
                        functions_by_name,
                        memo,
                        visiting,
                    );
                    if !callee_reasons.is_empty() {
                        reasons.insert(CallCacheRejectReason::CallsImpureFunction {
                            callee: callee.clone(),
                        });
                    }
                    continue;
                }

                match builtin_call_cache_semantics(callee) {
                    BuiltinCallCacheSemantics::Stable => {}
                    BuiltinCallCacheSemantics::Impure => {
                        reasons.insert(CallCacheRejectReason::CallsImpureBuiltin {
                            builtin: callee.clone(),
                        });
                    }
                    BuiltinCallCacheSemantics::Unknown => {
                        reasons.insert(CallCacheRejectReason::CallsUnknownFunction {
                            callee: callee.clone(),
                        });
                    }
                }
            }
        }
    }

    visiting.remove(function_name);
    let reasons: Vec<_> = reasons.into_iter().collect();
    memo.insert(function_name.to_string(), reasons.clone());
    reasons
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BuiltinCallCacheSemantics {
    Stable,
    Impure,
    Unknown,
}

fn builtin_call_cache_semantics(name: &str) -> BuiltinCallCacheSemantics {
    match name {
        "argc"
        | "argv"
        | "parse_int"
        | "adam"
        | "dataframe_mean"
        | "tensor_checksum"
        | "dataframe_build_sin"
        | "dataframe_filter_gt"
        | "dataframe_sort"
        | "dataframe_group_by"
        | "tensor_fill_rand"
        | "dense_layer"
        | "conv2d"
        | "dataframe_free"
        | "tensor_free"
        | "len" => BuiltinCallCacheSemantics::Stable,
        "print" | "println" | "print_int" | "print_str" | "clock" | "has_next" | "next" => {
            BuiltinCallCacheSemantics::Impure
        }
        _ => BuiltinCallCacheSemantics::Unknown,
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
    use crate::ir::{
        BasicBlock, BlockId, Instruction, MirFunction, MirModule, Terminator, ValueId,
    };

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

    fn function_with_calls(name: &str, callees: &[&str]) -> MirFunction {
        MirFunction {
            name: name.into(),
            params: Vec::new(),
            return_ty: TypeId(0),
            blocks: vec![BasicBlock {
                id: BlockId(0),
                instructions: callees
                    .iter()
                    .enumerate()
                    .map(|(index, callee)| Instruction {
                        result: ValueId(index as u32),
                        ty: TypeId(0),
                        op: Op::Call {
                            callee: (*callee).into(),
                            args: Vec::new(),
                        },
                    })
                    .collect(),
                terminator: Terminator::ReturnVoid,
            }],
            entry: BlockId(0),
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

    #[test]
    fn semantic_rejections_mark_clock_based_functions_impure() {
        let module = MirModule {
            functions: vec![function_with_calls("nowish", &["clock"])],
        };

        let reasons = semantic_call_cache_rejection_reasons(&module);

        assert_eq!(
            reasons.get("nowish"),
            Some(&vec![CallCacheRejectReason::CallsImpureBuiltin {
                builtin: "clock".into(),
            }])
        );
    }

    #[test]
    fn semantic_rejections_allow_transitively_stable_calls() {
        let module = MirModule {
            functions: vec![
                function_with_calls("arg_count", &["argc"]),
                function_with_calls("outer", &["arg_count"]),
            ],
        };

        let reasons = semantic_call_cache_rejection_reasons(&module);

        assert_eq!(reasons.get("arg_count"), Some(&Vec::new()));
        assert_eq!(reasons.get("outer"), Some(&Vec::new()));
    }

    #[test]
    fn semantic_rejections_propagate_impurity_through_user_calls() {
        let module = MirModule {
            functions: vec![
                function_with_calls("nowish", &["clock"]),
                function_with_calls("outer", &["nowish"]),
            ],
        };

        let reasons = semantic_call_cache_rejection_reasons(&module);

        assert_eq!(
            reasons.get("outer"),
            Some(&vec![CallCacheRejectReason::CallsImpureFunction {
                callee: "nowish".into(),
            }])
        );
    }
}
