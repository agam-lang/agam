//! Alias Analysis and Pointer Disjointness Oracle for Agam MIR.
//!
//! Provides formal disjointness proofs (`AliasRelation::NoAlias`) for loop vectorization,
//! safe SIMD code generation, and memory optimization passes.

#![deny(clippy::unwrap_used)]

use crate::ir::{Instruction, MirBinOp, MirFunction, Op, ValueId};
use std::collections::{HashMap, HashSet};

/// Categorization of alias relation between two memory accesses.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum AliasRelation {
    /// Pointers definitely refer to the exact same memory location.
    MustAlias,
    /// Pointers may or may not alias (fails closed).
    MayAlias,
    /// Pointers are proven to refer to disjoint, non-overlapping memory regions.
    NoAlias,
}

/// A formal proof of alias relation between two pointer values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisjointnessProof {
    pub base_a: ValueId,
    pub base_b: ValueId,
    pub relation: AliasRelation,
    pub reason: &'static str,
}

/// Provenance classification of an SSA value used as a pointer or base address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PointerProvenance {
    /// Local stack allocation (`Op::Alloca`).
    StackAlloc { name: String, val_id: ValueId },
    /// Distinct struct field projection from a base pointer.
    StructField { base: ValueId, field: String },
    /// Global or compile-time constant memory buffer.
    ConstantMemory(ValueId),
    /// Function parameter.
    FunctionParam(ValueId),
    /// Derived pointer with an offset relative to a base: `base + offset`.
    Offset { base: ValueId, offset: Option<i64> },
    /// Escaped stack allocation (address passed to foreign/opaque calls).
    EscapedAlloc(ValueId),
    /// Unknown or unresolved provenance.
    Unknown,
}

/// Semantic Alias Oracle for querying memory disjointness.
pub struct AliasOracle<'a> {
    pub func: &'a MirFunction,
    instructions_by_result: HashMap<ValueId, &'a Instruction>,
    escaped_allocas: HashSet<ValueId>,
}

impl<'a> AliasOracle<'a> {
    /// Build an `AliasOracle` for a MIR function.
    pub fn new(func: &'a MirFunction) -> Self {
        let mut instructions_by_result = HashMap::new();
        let mut escaped_allocas = HashSet::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                instructions_by_result.insert(instr.result, instr);
            }
        }

        // Trace escaped stack allocations:
        // Any Alloca whose value is passed as a Call argument or stored into an external target
        for block in &func.blocks {
            for instr in &block.instructions {
                match &instr.op {
                    Op::Call { args, .. } => {
                        for &arg in args {
                            if let Some(base) = Self::find_alloca_root(arg, &instructions_by_result)
                            {
                                escaped_allocas.insert(base);
                            }
                        }
                    }
                    Op::StoreIndex { value, .. } => {
                        if let Some(base) = Self::find_alloca_root(*value, &instructions_by_result)
                        {
                            escaped_allocas.insert(base);
                        }
                    }
                    _ => {}
                }
            }
        }

        Self {
            func,
            instructions_by_result,
            escaped_allocas,
        }
    }

    fn find_alloca_root(
        val: ValueId,
        instrs: &HashMap<ValueId, &'a Instruction>,
    ) -> Option<ValueId> {
        let mut curr = val;
        let mut visited = HashSet::new();
        while visited.insert(curr) {
            let instr = instrs.get(&curr)?;
            match &instr.op {
                Op::Alloca { .. } => return Some(curr),
                Op::Copy(orig) => curr = *orig,
                Op::GetField { object, .. } => curr = *object,
                _ => return None,
            }
        }
        None
    }

    /// Resolve the pointer provenance of an SSA value.
    pub fn resolve_provenance(&self, val: ValueId) -> PointerProvenance {
        let Some(instr) = self.instructions_by_result.get(&val) else {
            // Check if it is a function parameter
            if self.func.params.iter().any(|p| p.value == val) {
                return PointerProvenance::FunctionParam(val);
            }
            return PointerProvenance::Unknown;
        };

        match &instr.op {
            Op::Alloca { name, .. } => {
                if self.escaped_allocas.contains(&val) {
                    PointerProvenance::EscapedAlloc(val)
                } else {
                    PointerProvenance::StackAlloc {
                        name: name.clone(),
                        val_id: val,
                    }
                }
            }
            Op::GetField { object, field } => PointerProvenance::StructField {
                base: *object,
                field: field.clone(),
            },
            Op::ConstString(_) => PointerProvenance::ConstantMemory(val),
            Op::Copy(orig) => self.resolve_provenance(*orig),
            Op::BinOp {
                op: MirBinOp::Add,
                left,
                right,
            } => {
                let l_const = self.resolve_const_int(*left);
                let r_const = self.resolve_const_int(*right);
                if let Some(offset) = r_const {
                    PointerProvenance::Offset {
                        base: *left,
                        offset: Some(offset),
                    }
                } else if let Some(offset) = l_const {
                    PointerProvenance::Offset {
                        base: *right,
                        offset: Some(offset),
                    }
                } else {
                    PointerProvenance::Offset {
                        base: *left,
                        offset: None,
                    }
                }
            }
            Op::BinOp {
                op: MirBinOp::Sub,
                left,
                right,
            } => {
                if let Some(offset) = self.resolve_const_int(*right) {
                    PointerProvenance::Offset {
                        base: *left,
                        offset: Some(-offset),
                    }
                } else {
                    PointerProvenance::Offset {
                        base: *left,
                        offset: None,
                    }
                }
            }
            _ => PointerProvenance::Unknown,
        }
    }

    /// Helper to resolve a constant integer from an SSA value.
    pub fn resolve_const_int(&self, val: ValueId) -> Option<i64> {
        let instr = self.instructions_by_result.get(&val)?;
        match &instr.op {
            Op::ConstInt(c) => Some(*c),
            Op::Copy(orig) => self.resolve_const_int(*orig),
            _ => None,
        }
    }

    /// Query the alias relation between two memory accesses with explicit access byte widths.
    ///
    /// Implements Invariant B (Access-Width-Aware Disjointness) and Invariant D (Escaped Provenance Degradation).
    pub fn query_alias(
        &self,
        ptr_a: ValueId,
        ptr_b: ValueId,
        size_a: usize,
        size_b: usize,
    ) -> DisjointnessProof {
        // 1. Structural Identity
        if ptr_a == ptr_b {
            return DisjointnessProof {
                base_a: ptr_a,
                base_b: ptr_b,
                relation: AliasRelation::MustAlias,
                reason: "Identical SSA ValueId",
            };
        }

        let prov_a = self.resolve_provenance(ptr_a);
        let prov_b = self.resolve_provenance(ptr_b);

        // 2. Escaped allocations degrade to MayAlias (Invariant D)
        if matches!(prov_a, PointerProvenance::EscapedAlloc(_))
            || matches!(prov_b, PointerProvenance::EscapedAlloc(_))
        {
            return DisjointnessProof {
                base_a: ptr_a,
                base_b: ptr_b,
                relation: AliasRelation::MayAlias,
                reason: "Escaped Pointer Provenance Degradation",
            };
        }

        // 3. Distinct Stack Allocations (Op::Alloca)
        if let (
            PointerProvenance::StackAlloc { val_id: a, .. },
            PointerProvenance::StackAlloc { val_id: b, .. },
        ) = (&prov_a, &prov_b)
            && a != b
        {
            return DisjointnessProof {
                base_a: ptr_a,
                base_b: ptr_b,
                relation: AliasRelation::NoAlias,
                reason: "Distinct Non-Escaping Stack Allocations",
            };
        }

        // 4. Distinct Struct Fields on the same or disjoint objects
        if let (
            PointerProvenance::StructField {
                base: base_a,
                field: field_a,
            },
            PointerProvenance::StructField {
                base: base_b,
                field: field_b,
            },
        ) = (&prov_a, &prov_b)
        {
            if base_a == base_b && field_a != field_b {
                return DisjointnessProof {
                    base_a: ptr_a,
                    base_b: ptr_b,
                    relation: AliasRelation::NoAlias,
                    reason: "Distinct Named Struct Fields on Same Object",
                };
            }
            if base_a != base_b {
                let sub_proof = self.query_alias(*base_a, *base_b, 0, 0);
                if sub_proof.relation == AliasRelation::NoAlias {
                    return DisjointnessProof {
                        base_a: ptr_a,
                        base_b: ptr_b,
                        relation: AliasRelation::NoAlias,
                        reason: "Fields on Disjoint Base Objects",
                    };
                }
            }
        }

        // 5. Offset Arithmetic on the same base pointer (Invariant B: Access-Width Disjointness)
        if let (
            PointerProvenance::Offset {
                base: base_a,
                offset: Some(off_a),
            },
            PointerProvenance::Offset {
                base: base_b,
                offset: Some(off_b),
            },
        ) = (&prov_a, &prov_b)
            && base_a == base_b
        {
            let start_a = i128::from(*off_a);
            let end_a = start_a + (size_a as i128);
            let start_b = i128::from(*off_b);
            let end_b = start_b + (size_b as i128);

            // Disjoint interval test: [start_a, end_a) ∩ [start_b, end_b) = ∅
            if end_a <= start_b || end_b <= start_a {
                return DisjointnessProof {
                    base_a: ptr_a,
                    base_b: ptr_b,
                    relation: AliasRelation::NoAlias,
                    reason: "Disjoint Offset Ranges on Same Base Allocation",
                };
            } else if off_a == off_b && size_a == size_b {
                return DisjointnessProof {
                    base_a: ptr_a,
                    base_b: ptr_b,
                    relation: AliasRelation::MustAlias,
                    reason: "Identical Offset and Size on Same Base",
                };
            } else {
                return DisjointnessProof {
                    base_a: ptr_a,
                    base_b: ptr_b,
                    relation: AliasRelation::MayAlias,
                    reason: "Overlapping Offset Ranges on Same Base",
                };
            }
        }

        // 6. Distinct Constant Memory
        if let (PointerProvenance::ConstantMemory(a), PointerProvenance::ConstantMemory(b)) =
            (&prov_a, &prov_b)
            && a != b
        {
            return DisjointnessProof {
                base_a: ptr_a,
                base_b: ptr_b,
                relation: AliasRelation::NoAlias,
                reason: "Distinct Constant String / Memory Buffers",
            };
        }

        // 7. Fails Closed to MayAlias
        DisjointnessProof {
            base_a: ptr_a,
            base_b: ptr_b,
            relation: AliasRelation::MayAlias,
            reason: "Unknown / Indeterminate Pointer Provenance",
        }
    }

    /// Query whether two pointers are mathematically guaranteed not to alias.
    pub fn is_disjoint(
        &self,
        ptr_a: ValueId,
        ptr_b: ValueId,
        size_a: usize,
        size_b: usize,
    ) -> bool {
        self.query_alias(ptr_a, ptr_b, size_a, size_b).relation == AliasRelation::NoAlias
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, BlockId, Instruction, Terminator};
    use agam_sema::symbol::TypeId;

    #[test]
    fn test_distinct_stack_allocas_no_alias() {
        let b0 = BlockId(0);
        let v1 = ValueId(1);
        let v2 = ValueId(2);

        let func = MirFunction {
            name: "test_allocas".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v1,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "buf_a".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: v2,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "buf_b".into(),
                            ty: TypeId(1),
                        },
                    },
                ],
                terminator: Terminator::ReturnVoid,
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let oracle = AliasOracle::new(&func);
        let proof = oracle.query_alias(v1, v2, 64, 64);
        assert_eq!(proof.relation, AliasRelation::NoAlias);
        assert_eq!(proof.reason, "Distinct Non-Escaping Stack Allocations");
    }

    #[test]
    fn test_identical_ssa_value_must_alias() {
        let b0 = BlockId(0);
        let v1 = ValueId(1);

        let func = MirFunction {
            name: "test_ident".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![Instruction {
                    result: v1,
                    ty: TypeId(1),
                    op: Op::Alloca {
                        name: "buf".into(),
                        ty: TypeId(1),
                    },
                }],
                terminator: Terminator::ReturnVoid,
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let oracle = AliasOracle::new(&func);
        let proof = oracle.query_alias(v1, v1, 32, 32);
        assert_eq!(proof.relation, AliasRelation::MustAlias);
    }

    #[test]
    fn test_distinct_struct_fields_no_alias() {
        let b0 = BlockId(0);
        let obj = ValueId(1);
        let f_x = ValueId(2);
        let f_y = ValueId(3);

        let func = MirFunction {
            name: "test_fields".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: obj,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "point".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: f_x,
                        ty: TypeId(1),
                        op: Op::GetField {
                            object: obj,
                            field: "x".into(),
                        },
                    },
                    Instruction {
                        result: f_y,
                        ty: TypeId(1),
                        op: Op::GetField {
                            object: obj,
                            field: "y".into(),
                        },
                    },
                ],
                terminator: Terminator::ReturnVoid,
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let oracle = AliasOracle::new(&func);
        let proof = oracle.query_alias(f_x, f_y, 8, 8);
        assert_eq!(proof.relation, AliasRelation::NoAlias);
        assert_eq!(proof.reason, "Distinct Named Struct Fields on Same Object");
    }

    #[test]
    fn test_access_width_disjoint_and_overlapping_offsets() {
        let b0 = BlockId(0);
        let base = ValueId(1);
        let c0 = ValueId(2);
        let c4 = ValueId(3);
        let ptr_0 = ValueId(4);
        let ptr_4 = ValueId(5);

        let func = MirFunction {
            name: "test_offsets".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: base,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "arr".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: c0,
                        ty: TypeId(1),
                        op: Op::ConstInt(0),
                    },
                    Instruction {
                        result: c4,
                        ty: TypeId(1),
                        op: Op::ConstInt(4),
                    },
                    Instruction {
                        result: ptr_0,
                        ty: TypeId(1),
                        op: Op::BinOp {
                            op: MirBinOp::Add,
                            left: base,
                            right: c0,
                        },
                    },
                    Instruction {
                        result: ptr_4,
                        ty: TypeId(1),
                        op: Op::BinOp {
                            op: MirBinOp::Add,
                            left: base,
                            right: c4,
                        },
                    },
                ],
                terminator: Terminator::ReturnVoid,
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let oracle = AliasOracle::new(&func);

        // Disjoint: [0, 4) vs [4, 8) -> NoAlias
        let proof_disjoint = oracle.query_alias(ptr_0, ptr_4, 4, 4);
        assert_eq!(proof_disjoint.relation, AliasRelation::NoAlias);

        // Overlapping: [0, 8) vs [4, 12) -> MayAlias
        let proof_overlap = oracle.query_alias(ptr_0, ptr_4, 8, 8);
        assert_eq!(proof_overlap.relation, AliasRelation::MayAlias);
    }

    #[test]
    fn test_escaped_alloca_degrades_to_may_alias() {
        let b0 = BlockId(0);
        let v1 = ValueId(1);
        let v2 = ValueId(2);

        let func = MirFunction {
            name: "test_escaped".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            entry: b0,
            blocks: vec![BasicBlock {
                id: b0,
                instructions: vec![
                    Instruction {
                        result: v1,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "escaped_buf".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: v2,
                        ty: TypeId(1),
                        op: Op::Alloca {
                            name: "normal_buf".into(),
                            ty: TypeId(1),
                        },
                    },
                    Instruction {
                        result: ValueId(3),
                        ty: TypeId(0),
                        op: Op::Call {
                            callee: "foreign_fn".into(),
                            args: vec![v1],
                        },
                    },
                ],
                terminator: Terminator::ReturnVoid,
            }],
            target: Default::default(),
            gpu_config: None,
        };

        let oracle = AliasOracle::new(&func);
        let proof = oracle.query_alias(v1, v2, 32, 32);
        assert_eq!(proof.relation, AliasRelation::MayAlias);
        assert_eq!(proof.reason, "Escaped Pointer Provenance Degradation");
    }

    #[test]
    fn test_20000_case_differential_alias_interval_fuzz() {
        let mut rng_state: u64 = 0xD15EA5E;
        let mut next_rand = || -> i64 {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            (rng_state >> 33) as i64
        };

        for _ in 0..20_000 {
            let off_a = (next_rand() % 1000).abs();
            let off_b = (next_rand() % 1000).abs();
            let size_a = ((next_rand() % 64).abs() + 1) as usize;
            let size_b = ((next_rand() % 64).abs() + 1) as usize;

            let end_a = off_a + size_a as i64;
            let end_b = off_b + size_b as i64;

            let is_disjoint = end_a <= off_b || end_b <= off_a;

            let b0 = BlockId(0);
            let base = ValueId(1);
            let c_a = ValueId(2);
            let c_b = ValueId(3);
            let p_a = ValueId(4);
            let p_b = ValueId(5);

            let func = MirFunction {
                name: "fuzz_func".into(),
                generics: vec![],
                params: vec![],
                return_ty: TypeId(0),
                entry: b0,
                blocks: vec![BasicBlock {
                    id: b0,
                    instructions: vec![
                        Instruction {
                            result: base,
                            ty: TypeId(1),
                            op: Op::Alloca {
                                name: "mem".into(),
                                ty: TypeId(1),
                            },
                        },
                        Instruction {
                            result: c_a,
                            ty: TypeId(1),
                            op: Op::ConstInt(off_a),
                        },
                        Instruction {
                            result: c_b,
                            ty: TypeId(1),
                            op: Op::ConstInt(off_b),
                        },
                        Instruction {
                            result: p_a,
                            ty: TypeId(1),
                            op: Op::BinOp {
                                op: MirBinOp::Add,
                                left: base,
                                right: c_a,
                            },
                        },
                        Instruction {
                            result: p_b,
                            ty: TypeId(1),
                            op: Op::BinOp {
                                op: MirBinOp::Add,
                                left: base,
                                right: c_b,
                            },
                        },
                    ],
                    terminator: Terminator::ReturnVoid,
                }],
                target: Default::default(),
                gpu_config: None,
            };

            let oracle = AliasOracle::new(&func);
            let proof = oracle.query_alias(p_a, p_b, size_a, size_b);

            if is_disjoint {
                assert_eq!(
                    proof.relation,
                    AliasRelation::NoAlias,
                    "Intervals [{off_a}, {end_a}) and [{off_b}, {end_b}) must be NoAlias"
                );
            } else if off_a == off_b && size_a == size_b {
                assert_eq!(proof.relation, AliasRelation::MustAlias);
            } else {
                assert_eq!(proof.relation, AliasRelation::MayAlias);
            }
        }
    }
}
