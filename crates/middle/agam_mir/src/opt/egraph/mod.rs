//! E-Graph Equality Saturation & Algebraic Tensor Superoptimization Engine.
//!
//! Implements congruence closure, algebraic equivalence exploration, and
//! square-zero tensor kernel fusion with strict SSA dominance preservation.

pub mod rules;

use std::collections::HashMap;

use agam_sema::symbol::TypeId;
use serde::{Deserialize, Serialize};

use crate::ir::{Instruction, MirBinOp, MirFunction, MirModule, MirUnOp, Op, ValueId};

/// Unique identifier for an Equivalence Class (E-Class).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EClassId(pub u32);

/// An expression node in the E-Graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ENode {
    ConstInt(i64),
    ConstFloat(u64), // f64::to_bits() for Eq/Hash
    ConstBool(bool),
    ConstString(String),
    Unit,
    Var(ValueId),
    BinOp {
        op: MirBinOp,
        left: EClassId,
        right: EClassId,
    },
    UnOp {
        op: MirUnOp,
        operand: EClassId,
    },
    Call {
        callee: String,
        args: Vec<EClassId>,
    },
    Cast {
        value: EClassId,
        target_ty: u32,
    },

    // ── Nilpotent Algebraic Terms (S = C[z1, ..., zr]/(zi^2)) ──
    NilpotentTerm {
        var: EClassId,
        degree: u32,
    },

    // ── High-Level & Fused Tensor Operations ──
    TensorMatMul {
        a: EClassId,
        b: EClassId,
        trans_a: bool,
        trans_b: bool,
    },
    TensorConv2d {
        input: EClassId,
        kernel: EClassId,
        stride: (u32, u32),
        padding: (u32, u32),
    },
    FusedMatmulAdd {
        a: EClassId,
        b: EClassId,
        bias: EClassId,
        trans_a: bool,
        trans_b: bool,
    },
    FusedConv2dRelu {
        input: EClassId,
        kernel: EClassId,
        bias: Option<EClassId>,
        stride: (u32, u32),
        padding: (u32, u32),
    },
    FusedAttention {
        q: EClassId,
        k: EClassId,
        v: EClassId,
    },
}

impl ENode {
    /// Return all child EClassIds for this node.
    pub fn children(&self) -> Vec<EClassId> {
        match self {
            ENode::ConstInt(_)
            | ENode::ConstFloat(_)
            | ENode::ConstBool(_)
            | ENode::ConstString(_)
            | ENode::Unit
            | ENode::Var(_) => Vec::new(),
            ENode::BinOp { left, right, .. } => vec![*left, *right],
            ENode::UnOp { operand, .. } => vec![*operand],
            ENode::Call { args, .. } => args.clone(),
            ENode::Cast { value, .. } => vec![*value],
            ENode::NilpotentTerm { var, .. } => vec![*var],
            ENode::TensorMatMul { a, b, .. } => vec![*a, *b],
            ENode::TensorConv2d { input, kernel, .. } => vec![*input, *kernel],
            ENode::FusedMatmulAdd { a, b, bias, .. } => vec![*a, *b, *bias],
            ENode::FusedConv2dRelu {
                input,
                kernel,
                bias,
                ..
            } => {
                let mut v = vec![*input, *kernel];
                if let Some(b) = bias {
                    v.push(*b);
                }
                v
            }
            ENode::FusedAttention { q, k, v } => vec![*q, *k, *v],
        }
    }

    /// Canonicalize the children of this ENode using the given union-find.
    pub fn canonicalize(&mut self, uf: &mut UnionFind) {
        match self {
            ENode::ConstInt(_)
            | ENode::ConstFloat(_)
            | ENode::ConstBool(_)
            | ENode::ConstString(_)
            | ENode::Unit
            | ENode::Var(_) => {}
            ENode::BinOp { left, right, .. } => {
                *left = uf.find(*left);
                *right = uf.find(*right);
            }
            ENode::UnOp { operand, .. } => {
                *operand = uf.find(*operand);
            }
            ENode::Call { args, .. } => {
                for arg in args {
                    *arg = uf.find(*arg);
                }
            }
            ENode::Cast { value, .. } => {
                *value = uf.find(*value);
            }
            ENode::NilpotentTerm { var, .. } => {
                *var = uf.find(*var);
            }
            ENode::TensorMatMul { a, b, .. } => {
                *a = uf.find(*a);
                *b = uf.find(*b);
            }
            ENode::TensorConv2d { input, kernel, .. } => {
                *input = uf.find(*input);
                *kernel = uf.find(*kernel);
            }
            ENode::FusedMatmulAdd { a, b, bias, .. } => {
                *a = uf.find(*a);
                *b = uf.find(*b);
                *bias = uf.find(*bias);
            }
            ENode::FusedConv2dRelu {
                input,
                kernel,
                bias,
                ..
            } => {
                *input = uf.find(*input);
                *kernel = uf.find(*kernel);
                if let Some(b) = bias {
                    *b = uf.find(*b);
                }
            }
            ENode::FusedAttention { q, k, v } => {
                *q = uf.find(*q);
                *k = uf.find(*k);
                *v = uf.find(*v);
            }
        }
    }
}

/// An Equivalence Class containing equivalent ENodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EClass {
    pub id: EClassId,
    pub nodes: Vec<ENode>,
    pub parents: Vec<(ENode, EClassId)>,
}

/// Union-Find data structure with path compression.
#[derive(Debug, Clone, Default)]
pub struct UnionFind {
    parents: Vec<EClassId>,
}

impl UnionFind {
    pub fn make_set(&mut self) -> EClassId {
        let id = EClassId(self.parents.len() as u32);
        self.parents.push(id);
        id
    }

    pub fn find(&mut self, mut id: EClassId) -> EClassId {
        let mut root = id;
        while root != self.parents[root.0 as usize] {
            root = self.parents[root.0 as usize];
        }
        // Path compression
        while id != root {
            let next = self.parents[id.0 as usize];
            self.parents[id.0 as usize] = root;
            id = next;
        }
        root
    }

    pub fn union(&mut self, a: EClassId, b: EClassId) -> EClassId {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a != root_b {
            self.parents[root_b.0 as usize] = root_a;
        }
        root_a
    }
}

/// The main E-Graph structure for Equality Saturation.
#[derive(Debug, Clone, Default)]
pub struct EGraph {
    pub union_find: UnionFind,
    classes: HashMap<EClassId, EClass>,
    memo: HashMap<ENode, EClassId>,
    worklist: Vec<EClassId>,
}

impl EGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new ENode to the E-Graph, returning its canonical EClassId.
    pub fn add(&mut self, mut node: ENode) -> EClassId {
        node.canonicalize(&mut self.union_find);

        if let Some(&id) = self.memo.get(&node) {
            return self.union_find.find(id);
        }

        let id = self.union_find.make_set();
        for child in node.children() {
            let canonical_child = self.union_find.find(child);
            if let Some(class) = self.classes.get_mut(&canonical_child) {
                class.parents.push((node.clone(), id));
            }
        }

        self.memo.insert(node.clone(), id);
        self.classes.insert(
            id,
            EClass {
                id,
                nodes: vec![node],
                parents: Vec::new(),
            },
        );

        id
    }

    /// Merge two EClasses, asserting their mathematical equivalence.
    pub fn union(&mut self, a: EClassId, b: EClassId) -> EClassId {
        let root_a = self.union_find.find(a);
        let root_b = self.union_find.find(b);
        if root_a == root_b {
            return root_a;
        }

        let new_root = self.union_find.union(root_a, root_b);
        let old_root = if new_root == root_a { root_b } else { root_a };

        if let Some(mut old_class) = self.classes.remove(&old_root) {
            if let Some(new_class) = self.classes.get_mut(&new_root) {
                new_class.nodes.append(&mut old_class.nodes);
                new_class.parents.append(&mut old_class.parents);
            }
        }

        self.worklist.push(new_root);
        new_root
    }

    /// Restore congruence invariants across the E-Graph.
    pub fn rebuild(&mut self) {
        while let Some(class_id) = self.worklist.pop() {
            let canonical_class = self.union_find.find(class_id);
            if let Some(class) = self.classes.get(&canonical_class) {
                let parents = class.parents.clone();
                for (mut node, parent_id) in parents {
                    self.memo.remove(&node);
                    node.canonicalize(&mut self.union_find);
                    self.memo.insert(node, self.union_find.find(parent_id));
                }
            }
        }
    }

    /// Find canonical root EClassId.
    pub fn find(&self, id: EClassId) -> EClassId {
        let mut curr = id;
        while curr != self.union_find.parents[curr.0 as usize] {
            curr = self.union_find.parents[curr.0 as usize];
        }
        curr
    }

    pub fn classes(&self) -> impl Iterator<Item = &EClass> {
        self.classes.values()
    }

    pub fn get_class_nodes(&self, id: EClassId) -> Vec<ENode> {
        let root = self.find(id);
        self.classes
            .get(&root)
            .map(|c| c.nodes.clone())
            .unwrap_or_default()
    }

    pub fn get_canonical_node(&self, id: EClassId) -> ENode {
        let nodes = self.get_class_nodes(id);
        nodes.first().cloned().unwrap_or(ENode::Unit)
    }

    /// Run equality saturation up to `max_iters` iterations.
    pub fn saturate(&mut self, max_iters: usize) -> usize {
        let mut total_matches = 0;
        for _ in 0..max_iters {
            let matches = rules::apply_rules(self);
            if matches == 0 {
                break;
            }
            total_matches += matches;
            self.rebuild();
        }
        total_matches
    }
}

/// Cost model for extracting the mathematically optimal AST from the E-Graph.
#[derive(Debug, Clone, Default)]
pub struct CostModel;

impl CostModel {
    pub fn node_cost(&self, node: &ENode) -> u32 {
        match node {
            ENode::ConstInt(_)
            | ENode::ConstFloat(_)
            | ENode::ConstBool(_)
            | ENode::ConstString(_)
            | ENode::Unit
            | ENode::Var(_) => 1,
            ENode::UnOp { .. } => 2,
            ENode::BinOp { .. } => 3,
            ENode::Cast { .. } => 2,
            ENode::NilpotentTerm { .. } => 1,
            ENode::Call { args, .. } => 5 + args.len() as u32,
            ENode::TensorMatMul { .. } => 10,
            ENode::TensorConv2d { .. } => 20,
            ENode::FusedMatmulAdd { .. } => 4, // 4 vs 10 + 3 = 13 (69% cost savings)
            ENode::FusedConv2dRelu { .. } => 6, // 6 vs 20 + 3 + 2 = 25 (76% cost savings)
            ENode::FusedAttention { .. } => 15, // 15 vs 33 (55% cost savings)
        }
    }
}

/// Extractor to extract the lowest-cost term for an EClassId.
pub struct Extractor<'a> {
    egraph: &'a EGraph,
    cost_model: CostModel,
    costs: HashMap<EClassId, (u32, ENode)>,
}

impl<'a> Extractor<'a> {
    pub fn new(egraph: &'a EGraph, cost_model: CostModel) -> Self {
        let mut extractor = Self {
            egraph,
            cost_model,
            costs: HashMap::new(),
        };
        extractor.calculate_costs();
        extractor
    }

    fn calculate_costs(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            for class in self.egraph.classes() {
                let class_id = self.egraph.find(class.id);
                for node in &class.nodes {
                    let mut node_cost = self.cost_model.node_cost(node);
                    let mut all_children_known = true;

                    for child in node.children() {
                        let child_id = self.egraph.find(child);
                        if let Some((child_cost, _)) = self.costs.get(&child_id) {
                            node_cost = node_cost.saturating_add(*child_cost);
                        } else {
                            all_children_known = false;
                            break;
                        }
                    }

                    if all_children_known {
                        let current_best = self.costs.get(&class_id).map(|(c, _)| *c);
                        if current_best.is_none() || node_cost < current_best.unwrap() {
                            self.costs.insert(class_id, (node_cost, node.clone()));
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    /// Extract the best ENode representing the given class.
    pub fn find_best(&self, id: EClassId) -> Option<(u32, ENode)> {
        let canonical = self.egraph.find(id);
        self.costs.get(&canonical).cloned()
    }
}

/// Run E-Graph equality saturation and superoptimization across all functions in a module.
pub fn run(module: &mut MirModule) -> bool {
    let mut changed = false;
    for func in &mut module.functions {
        changed |= optimize_function(func);
    }
    changed
}

/// Optimize a single MIR function using E-Graph equality saturation.
pub fn optimize_function(func: &mut MirFunction) -> bool {
    let mut changed = false;

    for block in &mut func.blocks {
        if block.instructions.is_empty() {
            continue;
        }

        let mut egraph = EGraph::new();
        let mut value_to_eclass = HashMap::new();
        let mut eclass_to_value = HashMap::new();

        // 0. Register function parameters in E-Graph as available SSA values
        for param in &func.params {
            let class_id = egraph.add(ENode::Var(param.value));
            value_to_eclass.insert(param.value, class_id);
            eclass_to_value.insert(class_id, param.value);
        }

        // 1. Build E-Graph from basic block instructions
        for inst in &block.instructions {
            let enode = mir_op_to_enode(inst.result, &inst.op, &value_to_eclass, &mut egraph);
            let class_id = egraph.add(enode);
            value_to_eclass.insert(inst.result, class_id);
            // Only insert the earliest value id for this eclass to respect SSA dominance
            eclass_to_value.entry(class_id).or_insert(inst.result);
        }

        // 2. Run equality saturation
        let rewrite_count = egraph.saturate(10);
        if rewrite_count > 0 {
            // 3. Extract the best representations
            let extractor = Extractor::new(&egraph, CostModel);
            let mut new_instructions = Vec::new();

            for inst in &block.instructions {
                let orig_cost = mir_op_cost(&inst.op);

                if let Some(&class_id) = value_to_eclass.get(&inst.result) {
                    if let Some((best_cost, best_node)) = extractor.find_best(class_id) {
                        // Only replace if the new representation has strictly lower cost
                        if best_cost < orig_cost {
                            if let Some(new_op) = enode_to_mir_op(
                                inst.result,
                                &best_node,
                                &inst.op,
                                &eclass_to_value,
                                &egraph,
                            ) {
                                if new_op != inst.op {
                                    changed = true;
                                    new_instructions.push(Instruction {
                                        result: inst.result,
                                        ty: inst.ty,
                                        op: new_op,
                                    });
                                    continue;
                                }
                            }
                        }
                    }
                }
                new_instructions.push(inst.clone());
            }

            block.instructions = new_instructions;
        }
    }

    changed
}

fn mir_op_cost(op: &Op) -> u32 {
    match op {
        Op::ConstInt(_)
        | Op::ConstFloat(_)
        | Op::ConstBool(_)
        | Op::ConstString(_)
        | Op::Unit
        | Op::Copy(_) => 1,
        Op::UnOp { .. } => 2,
        Op::BinOp { .. } => 3,
        Op::Cast { .. } => 2,
        Op::Call { args, .. } => 5 + args.len() as u32,
        // All opaque memory and local ops are atomic base units with cost 1
        _ => 1,
    }
}

fn mir_op_to_enode(
    inst_result: ValueId,
    op: &Op,
    value_to_eclass: &HashMap<ValueId, EClassId>,
    egraph: &mut EGraph,
) -> ENode {
    match op {
        Op::ConstInt(val) => ENode::ConstInt(*val),
        Op::ConstFloat(val) => ENode::ConstFloat(val.to_bits()),
        Op::ConstBool(val) => ENode::ConstBool(*val),
        Op::ConstString(val) => ENode::ConstString(val.clone()),
        Op::Unit => ENode::Unit,
        Op::Copy(v) => {
            if let Some(&id) = value_to_eclass.get(v) {
                egraph.get_canonical_node(id)
            } else {
                ENode::Var(*v)
            }
        }
        Op::BinOp { op, left, right } => {
            let l_id = value_to_eclass
                .get(left)
                .copied()
                .unwrap_or_else(|| egraph.add(ENode::Var(*left)));
            let r_id = value_to_eclass
                .get(right)
                .copied()
                .unwrap_or_else(|| egraph.add(ENode::Var(*right)));
            ENode::BinOp {
                op: *op,
                left: l_id,
                right: r_id,
            }
        }
        Op::UnOp { op, operand } => {
            let o_id = value_to_eclass
                .get(operand)
                .copied()
                .unwrap_or_else(|| egraph.add(ENode::Var(*operand)));
            ENode::UnOp {
                op: *op,
                operand: o_id,
            }
        }
        Op::Call { callee, args } => {
            let arg_ids = args
                .iter()
                .map(|a| {
                    value_to_eclass
                        .get(a)
                        .copied()
                        .unwrap_or_else(|| egraph.add(ENode::Var(*a)))
                })
                .collect();
            ENode::Call {
                callee: callee.clone(),
                args: arg_ids,
            }
        }
        Op::Cast { value, target_ty } => {
            let v_id = value_to_eclass
                .get(value)
                .copied()
                .unwrap_or_else(|| egraph.add(ENode::Var(*value)));
            ENode::Cast {
                value: v_id,
                target_ty: target_ty.0,
            }
        }
        _ => ENode::Var(inst_result),
    }
}

fn enode_to_mir_op(
    inst_result: ValueId,
    node: &ENode,
    original_op: &Op,
    eclass_to_value: &HashMap<EClassId, ValueId>,
    egraph: &EGraph,
) -> Option<Op> {
    match node {
        ENode::ConstInt(val) => Some(Op::ConstInt(*val)),
        ENode::ConstFloat(bits) => Some(Op::ConstFloat(f64::from_bits(*bits))),
        ENode::ConstBool(val) => Some(Op::ConstBool(*val)),
        ENode::ConstString(val) => Some(Op::ConstString(val.clone())),
        ENode::Unit => Some(Op::Unit),
        ENode::Var(v) => {
            // Never copy from self
            if *v == inst_result {
                Some(original_op.clone())
            } else {
                Some(Op::Copy(*v))
            }
        }
        ENode::BinOp { op, left, right } => {
            let l_canonical = egraph.find(*left);
            let r_canonical = egraph.find(*right);
            let l_val = eclass_to_value.get(&l_canonical)?;
            let r_val = eclass_to_value.get(&r_canonical)?;
            Some(Op::BinOp {
                op: *op,
                left: *l_val,
                right: *r_val,
            })
        }
        ENode::UnOp { op, operand } => {
            let o_canonical = egraph.find(*operand);
            let o_val = eclass_to_value.get(&o_canonical)?;
            Some(Op::UnOp {
                op: *op,
                operand: *o_val,
            })
        }
        ENode::Cast { value, target_ty } => {
            let v_canonical = egraph.find(*value);
            let v_val = eclass_to_value.get(&v_canonical)?;
            Some(Op::Cast {
                value: *v_val,
                target_ty: TypeId(*target_ty),
            })
        }
        _ => Some(original_op.clone()),
    }
}
