//! Scalar Evolution (SCEV) Worklist Solver and Symbolic Loop Trip-Count Engine.

use std::collections::{HashMap, HashSet};

use crate::analysis::{ControlFlowGraph, DominatorTree, LoopForest, ReversePostOrder};
use crate::ir::{BasicBlock, BlockId, Instruction, MirBinOp, MirFunction, Op, Terminator, ValueId};
use crate::scev::expr::ScevExpr;

/// A canonical single-entry, single-latch loop descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopDescriptor {
    /// Preheader block (executes once before loop entry).
    pub preheader: BlockId,
    /// Loop header containing the loop condition branch.
    pub header: BlockId,
    /// Loop body / latch block that branches back to header.
    pub latch: BlockId,
    /// Exit block target when loop condition is false.
    pub exit: BlockId,
    /// All basic blocks belonging to this natural loop.
    pub blocks: HashSet<BlockId>,
}

/// A structured hierarchy of nested loops ordered outermost-to-innermost.
#[derive(Clone, Debug)]
pub struct LoopNest {
    /// Loop identifiers ordered from outermost (dimension 0) to innermost (dimension N-1).
    pub nest_path: Vec<BlockId>,
    /// Map from loop header BlockId to its canonical descriptor.
    pub loops_by_header: HashMap<BlockId, LoopDescriptor>,
}

impl LoopNest {
    /// Build all canonical loop descriptors and nesting hierarchy for a function.
    pub fn build(func: &MirFunction) -> Self {
        let cfg = ControlFlowGraph::build(func);
        let rpo = ReversePostOrder::build(func, &cfg);
        let dom_tree = DominatorTree::build(func, &cfg, &rpo);
        let forest = LoopForest::build(func, &cfg, &dom_tree);

        let blocks_by_id: HashMap<BlockId, &BasicBlock> =
            func.blocks.iter().map(|b| (b.id, b)).collect();

        let mut loops_by_header = HashMap::new();

        for nat_loop in &forest.loops {
            let Some(header_block) = blocks_by_id.get(&nat_loop.header) else {
                continue;
            };

            let Terminator::Branch {
                then_block: _,
                else_block,
                ..
            } = header_block.terminator
            else {
                continue;
            };

            // Identify preheaders (predecessors of header not inside the natural loop)
            let preheaders: Vec<BlockId> = cfg
                .predecessors(nat_loop.header)
                .iter()
                .copied()
                .filter(|p| !nat_loop.blocks.contains(p))
                .collect();

            if preheaders.len() != 1 {
                continue; // Non-canonical multi-preheader loop; fails closed
            }
            let preheader = preheaders[0];
            if nat_loop.back_edges.len() != 1 {
                continue; // Multi-latch or non-canonical loop; fails closed
            }
            let latch = nat_loop.back_edges[0].0;
            let exit = else_block;

            if !nat_loop.blocks.contains(&latch) {
                continue;
            }

            loops_by_header.insert(
                nat_loop.header,
                LoopDescriptor {
                    preheader,
                    header: nat_loop.header,
                    latch,
                    exit,
                    blocks: nat_loop.blocks.clone(),
                },
            );
        }

        // Determine outermost-to-innermost nesting path
        let mut nest_path: Vec<BlockId> = loops_by_header.keys().copied().collect();
        // Sort by block count descending: outermost loop contains more blocks than inner loop
        nest_path.sort_by(|a, b| {
            let len_a = loops_by_header.get(a).map_or(0, |l| l.blocks.len());
            let len_b = loops_by_header.get(b).map_or(0, |l| l.blocks.len());
            len_b.cmp(&len_a)
        });

        Self {
            nest_path,
            loops_by_header,
        }
    }

    /// Retrieve the enclosing loop chain for an inner loop header, ordered outermost-to-innermost.
    pub fn enclosing_nest_chain(&self, inner_header: BlockId) -> Vec<BlockId> {
        let mut chain = Vec::new();
        for &outer_header in &self.nest_path {
            if let Some(outer_desc) = self.loops_by_header.get(&outer_header)
                && outer_desc.blocks.contains(&inner_header)
            {
                chain.push(outer_header);
            }
        }
        chain
    }
}

/// Symbolic loop trip-count solution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TripCount {
    /// Exact compile-time constant iteration count.
    Constant(usize),
    /// Symbolic closed-form trip-count expression.
    Symbolic(ScevExpr),
    /// Non-affine or indeterminate trip count (fails closed).
    Unknown,
}

/// Solver for extracting SCEV recurrences and computing closed-form trip counts.
pub struct ScevSolver<'a> {
    pub func: &'a MirFunction,
    pub loop_nest: &'a LoopNest,
    alloca_counts: HashMap<String, usize>,
    instructions_by_result: HashMap<ValueId, &'a Instruction>,
    blocks_by_id: HashMap<BlockId, &'a BasicBlock>,
}

impl<'a> ScevSolver<'a> {
    pub fn new(func: &'a MirFunction, loop_nest: &'a LoopNest) -> Self {
        let mut alloca_counts = HashMap::new();
        let mut instructions_by_result = HashMap::new();
        let mut blocks_by_id = HashMap::new();

        for block in &func.blocks {
            blocks_by_id.insert(block.id, block);
            for instr in &block.instructions {
                instructions_by_result.insert(instr.result, instr);
                if let Op::Alloca { name, .. } = &instr.op {
                    *alloca_counts.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }

        Self {
            func,
            loop_nest,
            alloca_counts,
            instructions_by_result,
            blocks_by_id,
        }
    }

    /// Analyze an induction variable for a loop and construct its `{ base, +, step }_L` recurrence.
    pub fn analyze_induction_variable(&self, loop_id: BlockId, var_name: &str) -> Option<ScevExpr> {
        // Invariant 1: Local variable must not be shadowed (single Alloca in function)
        if self.alloca_counts.get(var_name).copied().unwrap_or(0) > 1 {
            return None; // Fails closed on lexical shadowing
        }

        let loop_desc = self.loop_nest.loops_by_header.get(&loop_id)?;
        let preheader_block = self.blocks_by_id.get(&loop_desc.preheader)?;
        let latch_block = self.blocks_by_id.get(&loop_desc.latch)?;

        // Invariant 2: Exactly one store to var_name in the preheader
        let base_val = self.find_last_store_value(preheader_block, var_name)?;
        let base_scev = self.eval_value_scev(base_val, loop_id);

        // Invariant 3: Exactly one StoreLocal to var_name in the loop latch
        let latch_stores: Vec<ValueId> = latch_block
            .instructions
            .iter()
            .filter_map(|instr| match &instr.op {
                Op::StoreLocal { name, value } if name == var_name => Some(*value),
                _ => None,
            })
            .collect();

        if latch_stores.len() != 1 {
            return None; // Fails closed on multiple stores or zero stores
        }

        let step_val = latch_stores[0];
        let step_scev = self.extract_step_scev(step_val, var_name, loop_id)?;

        Some(ScevExpr::add_rec(base_scev, step_scev, loop_id))
    }

    /// Solve closed-form trip count for a canonical loop.
    pub fn compute_trip_count(&self, loop_id: BlockId) -> TripCount {
        let Some(loop_desc) = self.loop_nest.loops_by_header.get(&loop_id) else {
            return TripCount::Unknown;
        };
        let Some(header_block) = self.blocks_by_id.get(&loop_desc.header) else {
            return TripCount::Unknown;
        };

        let Terminator::Branch { condition, .. } = header_block.terminator else {
            return TripCount::Unknown;
        };

        let Some(cond_instr) = self.instructions_by_result.get(&condition) else {
            return TripCount::Unknown;
        };

        let Op::BinOp { op, left, right } = cond_instr.op else {
            return TripCount::Unknown;
        };

        let (var_name, cmp_op, bound_val) = match (
            self.resolve_local_name(left),
            self.resolve_local_name(right),
        ) {
            (Some(name), None) => (name, op, right),
            (None, Some(name)) => (name, flip_cmp(op), left),
            _ => return TripCount::Unknown,
        };

        let Some(ind_scev) = self.analyze_induction_variable(loop_id, &var_name) else {
            return TripCount::Unknown;
        };

        let ScevExpr::AddRec { base, step, .. } = ind_scev else {
            return TripCount::Unknown;
        };

        let bound_scev = self.eval_value_scev(bound_val, loop_id);

        // Constant evaluation if both base, bound, and step are constants
        if let (ScevExpr::Constant(start), ScevExpr::Constant(bound), ScevExpr::Constant(step_c)) =
            (&*base, &bound_scev, &*step)
            && let Some(count) = compute_constant_trip_count(*start, *bound, *step_c, cmp_op)
        {
            return TripCount::Constant(count);
        }

        // Symbolic trip-count derivation: ceil((bound - start) / step)
        if let ScevExpr::Constant(step_c) = &*step
            && *step_c == 1
            && cmp_op == MirBinOp::Lt
        {
            let symbolic_diff = ScevExpr::add(vec![
                bound_scev,
                ScevExpr::mul(vec![ScevExpr::Constant(-1), *base]),
            ]);
            return TripCount::Symbolic(symbolic_diff);
        }

        TripCount::Unknown
    }

    fn find_last_store_value(&self, block: &BasicBlock, var_name: &str) -> Option<ValueId> {
        block
            .instructions
            .iter()
            .rev()
            .find_map(|instr| match &instr.op {
                Op::StoreLocal { name, value } if name == var_name => Some(*value),
                _ => None,
            })
    }

    fn extract_step_scev(
        &self,
        step_val: ValueId,
        var_name: &str,
        loop_id: BlockId,
    ) -> Option<ScevExpr> {
        let instr = self.instructions_by_result.get(&step_val)?;
        let Op::BinOp { op, left, right } = instr.op else {
            return None;
        };

        match op {
            MirBinOp::Add => {
                if self.is_load_of(left, var_name) {
                    Some(self.eval_value_scev(right, loop_id))
                } else if self.is_load_of(right, var_name) {
                    Some(self.eval_value_scev(left, loop_id))
                } else {
                    None
                }
            }
            MirBinOp::Sub => {
                if self.is_load_of(left, var_name) {
                    let right_scev = self.eval_value_scev(right, loop_id);
                    Some(ScevExpr::mul(vec![ScevExpr::Constant(-1), right_scev]))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn is_load_of(&self, val: ValueId, var_name: &str) -> bool {
        self.instructions_by_result
            .get(&val)
            .is_some_and(|instr| match &instr.op {
                Op::LoadLocal(name) => name == var_name,
                _ => false,
            })
    }

    fn resolve_local_name(&self, val: ValueId) -> Option<String> {
        self.instructions_by_result
            .get(&val)
            .and_then(|instr| match &instr.op {
                Op::LoadLocal(name) => Some(name.clone()),
                _ => None,
            })
    }

    /// Evaluate an SSA value into a SCEV expression.
    pub fn eval_value_scev(&self, val: ValueId, loop_id: BlockId) -> ScevExpr {
        let Some(instr) = self.instructions_by_result.get(&val) else {
            return ScevExpr::Unknown(val);
        };

        match &instr.op {
            Op::ConstInt(c) => ScevExpr::Constant(*c),
            Op::BinOp { op, left, right } => {
                let l_scev = self.eval_value_scev(*left, loop_id);
                let r_scev = self.eval_value_scev(*right, loop_id);
                match op {
                    MirBinOp::Add => ScevExpr::add(vec![l_scev, r_scev]),
                    MirBinOp::Sub => {
                        let neg_r = ScevExpr::mul(vec![ScevExpr::Constant(-1), r_scev]);
                        ScevExpr::add(vec![l_scev, neg_r])
                    }
                    MirBinOp::Mul => ScevExpr::mul(vec![l_scev, r_scev]),
                    _ => ScevExpr::Unknown(val),
                }
            }
            Op::LoadLocal(name) => {
                // If the load is invariant relative to loop_id, treat as invariant
                if let Some(ind) = self.analyze_induction_variable(loop_id, name) {
                    ind
                } else {
                    ScevExpr::Invariant(val)
                }
            }
            _ => ScevExpr::Invariant(val),
        }
    }
}

fn flip_cmp(op: MirBinOp) -> MirBinOp {
    match op {
        MirBinOp::Lt => MirBinOp::Gt,
        MirBinOp::LtEq => MirBinOp::GtEq,
        MirBinOp::Gt => MirBinOp::Lt,
        MirBinOp::GtEq => MirBinOp::LtEq,
        other => other,
    }
}

pub(crate) fn compute_constant_trip_count(
    start: i64,
    bound: i64,
    step: i64,
    cmp_op: MirBinOp,
) -> Option<usize> {
    if step == 0 {
        return None;
    }
    let start = i128::from(start);
    let bound = i128::from(bound);
    let step = i128::from(step);

    let count = match cmp_op {
        MirBinOp::Lt if step > 0 => {
            if start >= bound {
                0
            } else {
                ((bound - start - 1) / step) + 1
            }
        }
        MirBinOp::Lt if step < 0 => {
            if start >= bound {
                0
            } else {
                return None; // Infinite loop (decrements away from bound)
            }
        }
        MirBinOp::LtEq if step > 0 => {
            if start > bound {
                0
            } else {
                ((bound - start) / step) + 1
            }
        }
        MirBinOp::LtEq if step < 0 => {
            if start > bound {
                0
            } else {
                return None; // Infinite loop
            }
        }
        MirBinOp::Gt if step < 0 => {
            let pos_step = -step;
            if start <= bound {
                0
            } else {
                ((start - bound - 1) / pos_step) + 1
            }
        }
        MirBinOp::Gt if step > 0 => {
            if start <= bound {
                0
            } else {
                return None; // Infinite loop
            }
        }
        MirBinOp::GtEq if step < 0 => {
            let pos_step = -step;
            if start < bound {
                0
            } else {
                ((start - bound) / pos_step) + 1
            }
        }
        MirBinOp::GtEq if step > 0 && start < bound => 0,
        _ => return None,
    };

    usize::try_from(count).ok()
}
