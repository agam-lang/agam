//! Shared MIR-level call-cache selection analysis.

pub mod alias;
pub use alias::{AliasOracle, AliasRelation, DisjointnessProof, PointerProvenance};

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
                    generics: Vec::new(),
                    params: Vec::new(),
                    return_ty: TypeId(0),
                    blocks: Vec::new(),
                    entry: BlockId(index as u32),
                    target: Default::default(),
                    gpu_config: None,
                })
                .collect(),
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
        }
    }

    fn function_with_calls(name: &str, callees: &[&str]) -> MirFunction {
        MirFunction {
            name: name.into(),
            generics: Vec::new(),
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
            target: Default::default(),
            gpu_config: None,
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
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
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
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
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
            enum_layouts: std::collections::HashMap::new(),
            struct_layouts: std::collections::HashMap::new(),
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

// ── Control Flow Graph & Dominator Analysis Infrastructure ──

use crate::ir::{BlockId, Terminator};
use std::collections::{HashMap, HashSet, VecDeque};

/// Control flow graph analysis: successors and predecessors of basic blocks.
#[derive(Clone, Debug)]
pub struct ControlFlowGraph {
    pub preds: HashMap<BlockId, Vec<BlockId>>,
    pub succs: HashMap<BlockId, Vec<BlockId>>,
}

impl ControlFlowGraph {
    pub fn build(func: &MirFunction) -> Self {
        let mut preds: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        let mut succs: HashMap<BlockId, Vec<BlockId>> = HashMap::new();

        for block in &func.blocks {
            preds.entry(block.id).or_default();
            let block_succs = match &block.terminator {
                Terminator::Jump(target) => vec![*target],
                Terminator::Branch {
                    then_block,
                    else_block,
                    ..
                } => vec![*then_block, *else_block],
                Terminator::Switch { cases, default, .. } => {
                    let mut targets: Vec<BlockId> =
                        cases.iter().map(|(_, target)| *target).collect();
                    targets.push(*default);
                    targets
                }
                Terminator::Return(_) | Terminator::ReturnVoid | Terminator::Unreachable => {
                    Vec::new()
                }
            };
            for &succ in &block_succs {
                preds.entry(succ).or_default().push(block.id);
            }
            succs.insert(block.id, block_succs);
        }

        Self { preds, succs }
    }

    pub fn successors(&self, block: BlockId) -> &[BlockId] {
        self.succs.get(&block).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn predecessors(&self, block: BlockId) -> &[BlockId] {
        self.preds.get(&block).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// Reverse Postorder (RPO) traversal of reachable basic blocks in a function.
#[derive(Clone, Debug)]
pub struct ReversePostOrder {
    pub order: Vec<BlockId>,
    pub rpo_indices: HashMap<BlockId, usize>,
}

impl ReversePostOrder {
    pub fn build(func: &MirFunction, cfg: &ControlFlowGraph) -> Self {
        let mut visited = HashSet::new();
        let mut post_order = Vec::new();

        fn dfs(
            node: BlockId,
            cfg: &ControlFlowGraph,
            visited: &mut HashSet<BlockId>,
            post_order: &mut Vec<BlockId>,
        ) {
            visited.insert(node);
            for &succ in cfg.successors(node) {
                if !visited.contains(&succ) {
                    dfs(succ, cfg, visited, post_order);
                }
            }
            post_order.push(node);
        }

        dfs(func.entry, cfg, &mut visited, &mut post_order);
        post_order.reverse();

        let mut rpo_indices = HashMap::new();
        for (idx, &block) in post_order.iter().enumerate() {
            rpo_indices.insert(block, idx);
        }

        Self {
            order: post_order,
            rpo_indices,
        }
    }

    pub fn is_reachable(&self, block: BlockId) -> bool {
        self.rpo_indices.contains_key(&block)
    }

    pub fn rpo_index(&self, block: BlockId) -> Option<usize> {
        self.rpo_indices.get(&block).copied()
    }
}

/// Dominator Tree computing immediate dominators (idom) and dominance queries.
#[derive(Clone, Debug)]
pub struct DominatorTree {
    pub entry: BlockId,
    pub idoms: HashMap<BlockId, BlockId>,
    pub children: HashMap<BlockId, Vec<BlockId>>,
    pub depths: HashMap<BlockId, usize>,
}

impl DominatorTree {
    /// Compute dominators using Cooper-Harvey-Kennedy iterative algorithm on RPO.
    pub fn build(func: &MirFunction, cfg: &ControlFlowGraph, rpo: &ReversePostOrder) -> Self {
        let mut idoms: HashMap<BlockId, BlockId> = HashMap::new();

        if !rpo.order.is_empty() {
            idoms.insert(func.entry, func.entry);
        }

        let mut changed = true;
        while changed {
            changed = false;
            for &block in rpo.order.iter().skip(1) {
                let preds = cfg.predecessors(block);
                let mut new_idom = match preds.iter().find(|&&p| idoms.contains_key(&p)) {
                    Some(&first_pred) => first_pred,
                    None => continue,
                };

                for &other_pred in preds {
                    if other_pred == new_idom || !idoms.contains_key(&other_pred) {
                        continue;
                    }
                    new_idom = Self::intersect(new_idom, other_pred, &idoms, rpo);
                }

                if idoms.get(&block) != Some(&new_idom) {
                    idoms.insert(block, new_idom);
                    changed = true;
                }
            }
        }

        let mut children: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
        for &block in &rpo.order {
            children.entry(block).or_default();
            if block != func.entry
                && let Some(&parent) = idoms.get(&block)
            {
                children.entry(parent).or_default().push(block);
            }
        }

        let mut depths = HashMap::new();
        let mut queue = VecDeque::new();
        depths.insert(func.entry, 0);
        queue.push_back((func.entry, 0));

        while let Some((curr, d)) = queue.pop_front() {
            if let Some(kids) = children.get(&curr) {
                for &kid in kids {
                    depths.insert(kid, d + 1);
                    queue.push_back((kid, d + 1));
                }
            }
        }

        Self {
            entry: func.entry,
            idoms,
            children,
            depths,
        }
    }

    fn intersect(
        mut b1: BlockId,
        mut b2: BlockId,
        idoms: &HashMap<BlockId, BlockId>,
        rpo: &ReversePostOrder,
    ) -> BlockId {
        while b1 != b2 {
            let r1 = rpo.rpo_index(b1).unwrap_or(usize::MAX);
            let r2 = rpo.rpo_index(b2).unwrap_or(usize::MAX);
            if r1 > r2 {
                b1 = idoms.get(&b1).copied().unwrap_or(b1);
            } else {
                b2 = idoms.get(&b2).copied().unwrap_or(b2);
            }
        }
        b1
    }

    /// Returns true if block `a` dominates block `b` (reflexive: a dominates a).
    pub fn dominates(&self, a: BlockId, mut b: BlockId) -> bool {
        if a == b {
            return true;
        }
        while let Some(&parent) = self.idoms.get(&b) {
            if parent == a {
                return true;
            }
            if parent == b {
                break;
            }
            b = parent;
        }
        false
    }

    /// Returns true if block `a` strictly dominates block `b` (a dominates b and a != b).
    pub fn strictly_dominates(&self, a: BlockId, b: BlockId) -> bool {
        a != b && self.dominates(a, b)
    }

    /// Immediate dominator of block.
    pub fn idom(&self, block: BlockId) -> Option<BlockId> {
        if block == self.entry {
            None
        } else {
            self.idoms.get(&block).copied()
        }
    }
}

/// Dominance Frontier for all reachable blocks.
#[derive(Clone, Debug)]
pub struct DominanceFrontier {
    pub frontiers: HashMap<BlockId, HashSet<BlockId>>,
}

impl DominanceFrontier {
    pub fn build(func: &MirFunction, cfg: &ControlFlowGraph, dom_tree: &DominatorTree) -> Self {
        let mut frontiers: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
        for block in &func.blocks {
            frontiers.entry(block.id).or_default();
        }

        for block in &func.blocks {
            let preds = cfg.predecessors(block.id);
            if preds.len() >= 2 {
                for &pred in preds {
                    let mut runner = pred;
                    while runner != block.id && Some(runner) != dom_tree.idom(block.id) {
                        frontiers.entry(runner).or_default().insert(block.id);
                        if let Some(parent) = dom_tree.idom(runner) {
                            runner = parent;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        Self { frontiers }
    }

    pub fn frontier_of(&self, block: BlockId) -> &HashSet<BlockId> {
        static EMPTY: std::sync::LazyLock<HashSet<BlockId>> =
            std::sync::LazyLock::new(HashSet::new);
        self.frontiers.get(&block).unwrap_or(&EMPTY)
    }
}

/// Natural loop analysis.
#[derive(Clone, Debug)]
pub struct LoopForest {
    pub loops: Vec<NaturalLoop>,
}

#[derive(Clone, Debug)]
pub struct NaturalLoop {
    pub header: BlockId,
    pub blocks: HashSet<BlockId>,
    pub back_edges: Vec<(BlockId, BlockId)>,
}

impl LoopForest {
    pub fn build(func: &MirFunction, cfg: &ControlFlowGraph, dom_tree: &DominatorTree) -> Self {
        let mut loops_by_header: HashMap<BlockId, HashSet<BlockId>> = HashMap::new();
        let mut back_edges_by_header: HashMap<BlockId, Vec<(BlockId, BlockId)>> = HashMap::new();

        for block in &func.blocks {
            for &succ in cfg.successors(block.id) {
                // If successor dominates current block, this is a back-edge
                if dom_tree.dominates(succ, block.id) {
                    let header = succ;
                    back_edges_by_header
                        .entry(header)
                        .or_default()
                        .push((block.id, header));
                    let loop_blocks = loops_by_header.entry(header).or_default();
                    loop_blocks.insert(header);
                    loop_blocks.insert(block.id);

                    let mut worklist = vec![block.id];
                    while let Some(node) = worklist.pop() {
                        for &pred in cfg.predecessors(node) {
                            if loop_blocks.insert(pred) {
                                worklist.push(pred);
                            }
                        }
                    }
                }
            }
        }

        let loops = loops_by_header
            .into_iter()
            .map(|(header, blocks)| NaturalLoop {
                header,
                blocks,
                back_edges: back_edges_by_header.remove(&header).unwrap_or_default(),
            })
            .collect();

        Self { loops }
    }
}

#[cfg(test)]
mod dominator_tests {
    use super::*;
    use crate::ir::{BasicBlock, MirFunction, Terminator, ValueId};
    use agam_sema::symbol::TypeId;

    #[test]
    fn test_dominator_tree_diamond_cfg() {
        // CFG: B0 -> B1, B2; B1 -> B3; B2 -> B3
        let b0 = BlockId(0);
        let b1 = BlockId(1);
        let b2 = BlockId(2);
        let b3 = BlockId(3);

        let func = MirFunction {
            name: "diamond".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![
                BasicBlock {
                    id: b0,
                    instructions: vec![],
                    terminator: Terminator::Branch {
                        condition: ValueId(0),
                        then_block: b1,
                        else_block: b2,
                    },
                },
                BasicBlock {
                    id: b1,
                    instructions: vec![],
                    terminator: Terminator::Jump(b3),
                },
                BasicBlock {
                    id: b2,
                    instructions: vec![],
                    terminator: Terminator::Jump(b3),
                },
                BasicBlock {
                    id: b3,
                    instructions: vec![],
                    terminator: Terminator::ReturnVoid,
                },
            ],
            target: Default::default(),
            gpu_config: None,
        };

        let cfg = ControlFlowGraph::build(&func);
        let rpo = ReversePostOrder::build(&func, &cfg);
        let dom_tree = DominatorTree::build(&func, &cfg, &rpo);

        assert!(dom_tree.dominates(b0, b0));
        assert!(dom_tree.dominates(b0, b1));
        assert!(dom_tree.dominates(b0, b2));
        assert!(dom_tree.dominates(b0, b3));
        assert!(!dom_tree.dominates(b1, b3));
        assert!(!dom_tree.dominates(b2, b3));
        assert_eq!(dom_tree.idom(b3), Some(b0));

        let df = DominanceFrontier::build(&func, &cfg, &dom_tree);
        assert!(df.frontier_of(b1).contains(&b3));
        assert!(df.frontier_of(b2).contains(&b3));
        assert!(df.frontier_of(b0).is_empty());
    }

    #[test]
    fn test_loop_forest_detection() {
        // Loop: B0 -> B1; B1 -> B1 (back-edge), B1 -> B2 (exit)
        let b0 = BlockId(0);
        let b1 = BlockId(1);
        let b2 = BlockId(2);

        let func = MirFunction {
            name: "loop_fn".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![
                BasicBlock {
                    id: b0,
                    instructions: vec![],
                    terminator: Terminator::Jump(b1),
                },
                BasicBlock {
                    id: b1,
                    instructions: vec![],
                    terminator: Terminator::Branch {
                        condition: ValueId(0),
                        then_block: b1,
                        else_block: b2,
                    },
                },
                BasicBlock {
                    id: b2,
                    instructions: vec![],
                    terminator: Terminator::ReturnVoid,
                },
            ],
            target: Default::default(),
            gpu_config: None,
        };

        let cfg = ControlFlowGraph::build(&func);
        let rpo = ReversePostOrder::build(&func, &cfg);
        let dom_tree = DominatorTree::build(&func, &cfg, &rpo);
        let loops = LoopForest::build(&func, &cfg, &dom_tree);

        assert_eq!(loops.loops.len(), 1);
        assert_eq!(loops.loops[0].header, b1);
        assert!(loops.loops[0].blocks.contains(&b1));
        assert_eq!(loops.loops[0].back_edges, vec![(b1, b1)]);
    }
}
