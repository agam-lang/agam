//! Direct MIR Compile-Time Interpreter and `@comptime` Evaluation Engine.
//!
//! Evaluates pure functions, constant expressions, type assertions, and
//! control-flow branches during compilation with deterministic step bounding.

use std::collections::HashMap;

use crate::ir::{BlockId, MirBinOp, MirFunction, MirModule, MirUnOp, Op, Terminator, ValueId};
use serde::{Deserialize, Serialize};

/// Values representable and manipulable during compile-time evaluation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConstValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
    Enum {
        tag: u32,
        payload: Vec<ConstValue>,
    },
    Struct {
        name: String,
        fields: HashMap<String, ConstValue>,
    },
}

impl ConstValue {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// Errors surfaced during compile-time evaluation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComptimeError {
    DivisionByZero,
    StepLimitExceeded { limit: usize },
    AssertionFailed(String),
    UnassignedValue(ValueId),
    UndefinedFunction(String),
    InvalidBranchCondition,
    InvalidOperandType(String),
    UnsupportedOperation(String),
}

impl std::fmt::Display for ComptimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DivisionByZero => write!(f, "compile-time error: division by zero"),
            Self::StepLimitExceeded { limit } => {
                write!(
                    f,
                    "compile-time step limit of {limit} instructions exceeded"
                )
            }
            Self::AssertionFailed(msg) => write!(f, "comptime assertion failed: {msg}"),
            Self::UnassignedValue(id) => write!(f, "unassigned SSA value {id:?}"),
            Self::UndefinedFunction(name) => write!(f, "undefined function '{name}' in comptime"),
            Self::InvalidBranchCondition => write!(f, "branch condition is not a boolean"),
            Self::InvalidOperandType(op) => write!(f, "invalid operand types for '{op}'"),
            Self::UnsupportedOperation(op) => write!(f, "unsupported comptime operation '{op}'"),
        }
    }
}

impl std::error::Error for ComptimeError {}

/// High-performance compile-time interpreter.
pub struct ComptimeInterpreter<'a> {
    module: Option<&'a MirModule>,
    max_steps: usize,
    steps_taken: usize,
}

impl<'a> ComptimeInterpreter<'a> {
    pub fn new() -> Self {
        Self {
            module: None,
            max_steps: 100_000,
            steps_taken: 0,
        }
    }

    pub fn with_module(mut self, module: &'a MirModule) -> Self {
        self.module = Some(module);
        self
    }

    pub fn with_max_steps(mut self, limit: usize) -> Self {
        self.max_steps = limit;
        self
    }

    /// Evaluate an entire function with the given constant arguments.
    pub fn eval_function(
        &mut self,
        func: &MirFunction,
        args: &[ConstValue],
    ) -> Result<ConstValue, ComptimeError> {
        let mut env: HashMap<ValueId, ConstValue> = HashMap::new();
        let mut prev_block: Option<BlockId> = None;
        let mut current_block_id = func.entry;

        // Populate function parameter environment
        for (param, arg) in func.params.iter().zip(args.iter()) {
            env.insert(param.value, arg.clone());
        }

        loop {
            self.steps_taken += 1;
            if self.steps_taken > self.max_steps {
                return Err(ComptimeError::StepLimitExceeded {
                    limit: self.max_steps,
                });
            }

            let block = func
                .blocks
                .iter()
                .find(|b| b.id == current_block_id)
                .ok_or_else(|| {
                    ComptimeError::UnsupportedOperation(format!(
                        "Block {:?} not found",
                        current_block_id
                    ))
                })?;

            for instr in &block.instructions {
                let val = match &instr.op {
                    Op::ConstInt(i) => ConstValue::Int(*i),
                    Op::ConstFloat(f) => ConstValue::Float(*f),
                    Op::ConstBool(b) => ConstValue::Bool(*b),
                    Op::ConstString(s) => ConstValue::String(s.clone()),
                    Op::Unit => ConstValue::Unit,
                    Op::Copy(src) => env
                        .get(src)
                        .cloned()
                        .ok_or(ComptimeError::UnassignedValue(*src))?,
                    Op::BinOp { op, left, right } => {
                        let l_val = env.get(left).ok_or(ComptimeError::UnassignedValue(*left))?;
                        let r_val = env
                            .get(right)
                            .ok_or(ComptimeError::UnassignedValue(*right))?;
                        eval_binop(*op, l_val, r_val)?
                    }
                    Op::UnOp { op, operand } => {
                        let val = env
                            .get(operand)
                            .ok_or(ComptimeError::UnassignedValue(*operand))?;
                        eval_unop(*op, val)?
                    }
                    Op::Phi(incoming) => {
                        let prev = prev_block.ok_or_else(|| {
                            ComptimeError::UnsupportedOperation("Phi without predecessor".into())
                        })?;
                        let (_, v_id) =
                            incoming.iter().find(|(b, _)| *b == prev).ok_or_else(|| {
                                ComptimeError::UnsupportedOperation("Unmatched Phi incoming".into())
                            })?;
                        env.get(v_id)
                            .cloned()
                            .ok_or(ComptimeError::UnassignedValue(*v_id))?
                    }
                    Op::EnumConstruct { tag, payload } => {
                        let mut evaluated_payload = Vec::new();
                        for p in payload {
                            let p_val = env
                                .get(p)
                                .cloned()
                                .ok_or(ComptimeError::UnassignedValue(*p))?;
                            evaluated_payload.push(p_val);
                        }
                        ConstValue::Enum {
                            tag: *tag,
                            payload: evaluated_payload,
                        }
                    }
                    Op::EnumTag(target) => {
                        let enum_val = env
                            .get(target)
                            .ok_or(ComptimeError::UnassignedValue(*target))?;
                        match enum_val {
                            ConstValue::Enum { tag, .. } => ConstValue::Int(*tag as i64),
                            _ => {
                                return Err(ComptimeError::InvalidOperandType(
                                    "EnumTag on non-enum".into(),
                                ));
                            }
                        }
                    }
                    Op::EnumPayload { value, field_index } => {
                        let enum_val = env
                            .get(value)
                            .ok_or(ComptimeError::UnassignedValue(*value))?;
                        match enum_val {
                            ConstValue::Enum { payload, .. } => payload
                                .get(*field_index as usize)
                                .cloned()
                                .unwrap_or(ConstValue::Unit),
                            _ => {
                                return Err(ComptimeError::InvalidOperandType(
                                    "EnumPayload on non-enum".into(),
                                ));
                            }
                        }
                    }
                    Op::StructConstruct { name, fields } => {
                        let mut evaluated_fields = HashMap::new();
                        for (fname, fval_id) in fields {
                            let fval = env
                                .get(fval_id)
                                .cloned()
                                .ok_or(ComptimeError::UnassignedValue(*fval_id))?;
                            evaluated_fields.insert(fname.clone(), fval);
                        }
                        ConstValue::Struct {
                            name: name.clone(),
                            fields: evaluated_fields,
                        }
                    }
                    Op::GetField { object, field } => {
                        let obj_val = env
                            .get(object)
                            .ok_or(ComptimeError::UnassignedValue(*object))?;
                        match obj_val {
                            ConstValue::Struct { fields, .. } => {
                                fields.get(field).cloned().unwrap_or(ConstValue::Unit)
                            }
                            _ => {
                                return Err(ComptimeError::InvalidOperandType(
                                    "GetField on non-struct".into(),
                                ));
                            }
                        }
                    }
                    Op::Call {
                        callee,
                        args: arg_ids,
                    } => {
                        let mod_ref = self.module.ok_or_else(|| {
                            ComptimeError::UnsupportedOperation(
                                "Cannot call nested function without module context".into(),
                            )
                        })?;
                        let target_func = mod_ref
                            .functions
                            .iter()
                            .find(|f| &f.name == callee)
                            .ok_or_else(|| ComptimeError::UndefinedFunction(callee.clone()))?;

                        let mut call_args = Vec::new();
                        for a in arg_ids {
                            let arg_val = env
                                .get(a)
                                .cloned()
                                .ok_or(ComptimeError::UnassignedValue(*a))?;
                            call_args.push(arg_val);
                        }
                        self.eval_function(target_func, &call_args)?
                    }
                    _ => {
                        return Err(ComptimeError::UnsupportedOperation(format!(
                            "{:?}",
                            instr.op
                        )));
                    }
                };

                env.insert(instr.result, val);
            }

            match &block.terminator {
                Terminator::Return(ret_id) => {
                    return env
                        .get(ret_id)
                        .cloned()
                        .ok_or(ComptimeError::UnassignedValue(*ret_id));
                }
                Terminator::ReturnVoid => {
                    return Ok(ConstValue::Unit);
                }
                Terminator::Jump(target) => {
                    prev_block = Some(current_block_id);
                    current_block_id = *target;
                }
                Terminator::Branch {
                    condition,
                    then_block,
                    else_block,
                } => {
                    let cond_val = env
                        .get(condition)
                        .ok_or(ComptimeError::UnassignedValue(*condition))?;
                    let branch_bool = cond_val
                        .as_bool()
                        .ok_or(ComptimeError::InvalidBranchCondition)?;
                    prev_block = Some(current_block_id);
                    current_block_id = if branch_bool {
                        *then_block
                    } else {
                        *else_block
                    };
                }
                Terminator::Switch {
                    discriminant,
                    cases,
                    default,
                } => {
                    let disc_val = env
                        .get(discriminant)
                        .ok_or(ComptimeError::UnassignedValue(*discriminant))?;
                    let disc_int = disc_val.as_int().ok_or_else(|| {
                        ComptimeError::InvalidOperandType("Switch discriminant must be int".into())
                    })?;

                    prev_block = Some(current_block_id);
                    let mut next = *default;
                    for (case_val, target_block) in cases {
                        if *case_val == disc_int {
                            next = *target_block;
                            break;
                        }
                    }
                    current_block_id = next;
                }
                Terminator::Unreachable => {
                    return Err(ComptimeError::AssertionFailed(
                        "Reached unreachable MIR block".into(),
                    ));
                }
            }
        }
    }
}

impl<'a> Default for ComptimeInterpreter<'a> {
    fn default() -> Self {
        Self::new()
    }
}

fn eval_binop(
    op: MirBinOp,
    left: &ConstValue,
    right: &ConstValue,
) -> Result<ConstValue, ComptimeError> {
    match (left, right) {
        (ConstValue::Int(l), ConstValue::Int(r)) => match op {
            MirBinOp::Add => Ok(ConstValue::Int(l.wrapping_add(*r))),
            MirBinOp::Sub => Ok(ConstValue::Int(l.wrapping_sub(*r))),
            MirBinOp::Mul => Ok(ConstValue::Int(l.wrapping_mul(*r))),
            MirBinOp::Div => {
                if *r == 0 {
                    Err(ComptimeError::DivisionByZero)
                } else {
                    Ok(ConstValue::Int(l.wrapping_div(*r)))
                }
            }
            MirBinOp::Mod => {
                if *r == 0 {
                    Err(ComptimeError::DivisionByZero)
                } else {
                    Ok(ConstValue::Int(l.wrapping_rem(*r)))
                }
            }
            MirBinOp::Eq => Ok(ConstValue::Bool(l == r)),
            MirBinOp::NotEq => Ok(ConstValue::Bool(l != r)),
            MirBinOp::Lt => Ok(ConstValue::Bool(l < r)),
            MirBinOp::LtEq => Ok(ConstValue::Bool(l <= r)),
            MirBinOp::Gt => Ok(ConstValue::Bool(l > r)),
            MirBinOp::GtEq => Ok(ConstValue::Bool(l >= r)),
            MirBinOp::BitAnd => Ok(ConstValue::Int(l & r)),
            MirBinOp::BitOr => Ok(ConstValue::Int(l | r)),
            MirBinOp::BitXor => Ok(ConstValue::Int(l ^ r)),
            MirBinOp::Shl => Ok(ConstValue::Int(l.wrapping_shl(*r as u32))),
            MirBinOp::Shr => Ok(ConstValue::Int(l.wrapping_shr(*r as u32))),
            _ => Err(ComptimeError::InvalidOperandType(format!("{op:?} on Int"))),
        },
        (ConstValue::Float(l), ConstValue::Float(r)) => match op {
            MirBinOp::Add => Ok(ConstValue::Float(l + r)),
            MirBinOp::Sub => Ok(ConstValue::Float(l - r)),
            MirBinOp::Mul => Ok(ConstValue::Float(l * r)),
            MirBinOp::Div => Ok(ConstValue::Float(l / r)),
            MirBinOp::Eq => Ok(ConstValue::Bool(l == r)),
            MirBinOp::NotEq => Ok(ConstValue::Bool(l != r)),
            MirBinOp::Lt => Ok(ConstValue::Bool(l < r)),
            MirBinOp::LtEq => Ok(ConstValue::Bool(l <= r)),
            MirBinOp::Gt => Ok(ConstValue::Bool(l > r)),
            MirBinOp::GtEq => Ok(ConstValue::Bool(l >= r)),
            _ => Err(ComptimeError::InvalidOperandType(format!(
                "{op:?} on Float"
            ))),
        },
        (ConstValue::Bool(l), ConstValue::Bool(r)) => match op {
            MirBinOp::Eq => Ok(ConstValue::Bool(l == r)),
            MirBinOp::NotEq => Ok(ConstValue::Bool(l != r)),
            MirBinOp::And => Ok(ConstValue::Bool(*l && *r)),
            MirBinOp::Or => Ok(ConstValue::Bool(*l || *r)),
            _ => Err(ComptimeError::InvalidOperandType(format!("{op:?} on Bool"))),
        },
        (ConstValue::String(l), ConstValue::String(r)) => match op {
            MirBinOp::Add => Ok(ConstValue::String(format!("{l}{r}"))),
            MirBinOp::Eq => Ok(ConstValue::Bool(l == r)),
            MirBinOp::NotEq => Ok(ConstValue::Bool(l != r)),
            _ => Err(ComptimeError::InvalidOperandType(format!(
                "{op:?} on String"
            ))),
        },
        _ => Err(ComptimeError::InvalidOperandType(format!(
            "mismatched operands for {op:?}"
        ))),
    }
}

fn eval_unop(op: MirUnOp, operand: &ConstValue) -> Result<ConstValue, ComptimeError> {
    match (op, operand) {
        (MirUnOp::Neg, ConstValue::Int(i)) => Ok(ConstValue::Int(-i)),
        (MirUnOp::Neg, ConstValue::Float(f)) => Ok(ConstValue::Float(-f)),
        (MirUnOp::Not, ConstValue::Bool(b)) => Ok(ConstValue::Bool(!b)),
        (MirUnOp::BitNot, ConstValue::Int(i)) => Ok(ConstValue::Int(!i)),
        _ => Err(ComptimeError::InvalidOperandType(format!(
            "{op:?} on {operand:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BasicBlock, Instruction, MirParam};
    use agam_sema::gpu::GpuKernelParamAbi;
    use agam_sema::symbol::TypeId;
    use agam_sema::target::TargetProfile;

    fn build_fibonacci_mir() -> MirFunction {
        // fn fib(n) {
        //   if n <= 1 { return n; }
        //   return fib(n - 1) + fib(n - 2);
        // }
        let entry = BasicBlock {
            id: BlockId(0),
            instructions: vec![
                Instruction {
                    result: ValueId(1),
                    ty: TypeId(0),
                    op: Op::ConstInt(1),
                },
                Instruction {
                    result: ValueId(2),
                    ty: TypeId(0),
                    op: Op::BinOp {
                        op: MirBinOp::LtEq,
                        left: ValueId(0),
                        right: ValueId(1),
                    },
                },
            ],
            terminator: Terminator::Branch {
                condition: ValueId(2),
                then_block: BlockId(1),
                else_block: BlockId(2),
            },
        };

        let base_case = BasicBlock {
            id: BlockId(1),
            instructions: vec![],
            terminator: Terminator::Return(ValueId(0)),
        };

        let recursive_case = BasicBlock {
            id: BlockId(2),
            instructions: vec![
                Instruction {
                    result: ValueId(3),
                    ty: TypeId(0),
                    op: Op::ConstInt(1),
                },
                Instruction {
                    result: ValueId(4),
                    ty: TypeId(0),
                    op: Op::BinOp {
                        op: MirBinOp::Sub,
                        left: ValueId(0),
                        right: ValueId(3),
                    },
                },
                Instruction {
                    result: ValueId(5),
                    ty: TypeId(0),
                    op: Op::Call {
                        callee: "fib".into(),
                        args: vec![ValueId(4)],
                    },
                },
                Instruction {
                    result: ValueId(6),
                    ty: TypeId(0),
                    op: Op::ConstInt(2),
                },
                Instruction {
                    result: ValueId(7),
                    ty: TypeId(0),
                    op: Op::BinOp {
                        op: MirBinOp::Sub,
                        left: ValueId(0),
                        right: ValueId(6),
                    },
                },
                Instruction {
                    result: ValueId(8),
                    ty: TypeId(0),
                    op: Op::Call {
                        callee: "fib".into(),
                        args: vec![ValueId(7)],
                    },
                },
                Instruction {
                    result: ValueId(9),
                    ty: TypeId(0),
                    op: Op::BinOp {
                        op: MirBinOp::Add,
                        left: ValueId(5),
                        right: ValueId(8),
                    },
                },
            ],
            terminator: Terminator::Return(ValueId(9)),
        };

        MirFunction {
            name: "fib".into(),
            generics: vec![],
            params: vec![MirParam {
                name: "n".into(),
                ty: TypeId(0),
                value: ValueId(0),
                gpu_abi: GpuKernelParamAbi::I32,
                memory_type: None,
            }],
            return_ty: TypeId(0),
            blocks: vec![entry, base_case, recursive_case],
            entry: BlockId(0),
            target: TargetProfile::Default,
            gpu_config: None,
        }
    }

    #[test]
    fn test_comptime_arithmetic_evaluation() {
        let block = BasicBlock {
            id: BlockId(0),
            instructions: vec![
                Instruction {
                    result: ValueId(0),
                    ty: TypeId(0),
                    op: Op::ConstInt(20),
                },
                Instruction {
                    result: ValueId(1),
                    ty: TypeId(0),
                    op: Op::ConstInt(22),
                },
                Instruction {
                    result: ValueId(2),
                    ty: TypeId(0),
                    op: Op::BinOp {
                        op: MirBinOp::Add,
                        left: ValueId(0),
                        right: ValueId(1),
                    },
                },
            ],
            terminator: Terminator::Return(ValueId(2)),
        };

        let func = MirFunction {
            name: "add_comptime".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            blocks: vec![block],
            entry: BlockId(0),
            target: TargetProfile::Default,
            gpu_config: None,
        };

        let mut interp = ComptimeInterpreter::new();
        let res = interp.eval_function(&func, &[]).expect("should succeed");
        assert_eq!(res, ConstValue::Int(42));
    }

    #[test]
    fn test_comptime_fibonacci_recursion() {
        let fib_fn = build_fibonacci_mir();
        let module = MirModule {
            functions: vec![fib_fn.clone()],
            enum_layouts: HashMap::new(),
            struct_layouts: HashMap::new(),
        };

        let mut interp = ComptimeInterpreter::new().with_module(&module);
        // fib(7) = 13
        let res = interp
            .eval_function(&fib_fn, &[ConstValue::Int(7)])
            .expect("fib(7)");
        assert_eq!(res, ConstValue::Int(13));
    }

    #[test]
    fn test_comptime_division_by_zero_safety() {
        let block = BasicBlock {
            id: BlockId(0),
            instructions: vec![
                Instruction {
                    result: ValueId(0),
                    ty: TypeId(0),
                    op: Op::ConstInt(10),
                },
                Instruction {
                    result: ValueId(1),
                    ty: TypeId(0),
                    op: Op::ConstInt(0),
                },
                Instruction {
                    result: ValueId(2),
                    ty: TypeId(0),
                    op: Op::BinOp {
                        op: MirBinOp::Div,
                        left: ValueId(0),
                        right: ValueId(1),
                    },
                },
            ],
            terminator: Terminator::Return(ValueId(2)),
        };

        let func = MirFunction {
            name: "div_zero".into(),
            generics: vec![],
            params: vec![],
            return_ty: TypeId(0),
            blocks: vec![block],
            entry: BlockId(0),
            target: TargetProfile::Default,
            gpu_config: None,
        };

        let mut interp = ComptimeInterpreter::new();
        let res = interp.eval_function(&func, &[]);
        assert_eq!(res, Err(ComptimeError::DivisionByZero));
    }
}
