//! MIR node definitions — SSA-based, CFG-structured IR.

use agam_sema::symbol::TypeId;
use serde::{Deserialize, Serialize};

/// A unique identifier for an SSA value (virtual register).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValueId(pub u32);

/// A unique identifier for a basic block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u32);

/// A complete MIR function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<MirParam>,
    pub return_ty: TypeId,
    pub blocks: Vec<BasicBlock>,
    /// The entry block.
    pub entry: BlockId,
}

/// A function parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirParam {
    pub name: String,
    pub value: ValueId,
    pub ty: TypeId,
}

/// A basic block: a sequence of instructions ending with a terminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

/// An SSA instruction within a basic block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    /// The value this instruction produces.
    pub result: ValueId,
    /// The type of the result.
    pub ty: TypeId,
    /// The operation.
    pub op: Op,
}

/// MIR operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Op {
    /// Integer constant.
    ConstInt(i64),
    /// Float constant.
    ConstFloat(f64),
    /// Boolean constant.
    ConstBool(bool),
    /// String constant.
    ConstString(String),
    /// Unit value.
    Unit,

    /// Binary arithmetic / logic.
    BinOp {
        op: MirBinOp,
        left: ValueId,
        right: ValueId,
    },
    /// Unary operation.
    UnOp { op: MirUnOp, operand: ValueId },

    /// Function call.
    Call { callee: String, args: Vec<ValueId> },
    /// Copy an existing SSA value.
    Copy(ValueId),

    /// Load a local variable.
    LoadLocal(String),
    /// Store to a local variable.
    StoreLocal { name: String, value: ValueId },

    /// Allocate a local variable (stack allocation).
    Alloca { name: String, ty: TypeId },

    /// Field access.
    GetField { object: ValueId, field: String },
    /// Array/tuple index.
    GetIndex { object: ValueId, index: ValueId },

    /// Phi node for SSA join points.
    Phi(Vec<(BlockId, ValueId)>),

    /// Type cast.
    Cast { value: ValueId, target_ty: TypeId },
}

/// MIR binary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MirBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

/// MIR unary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MirUnOp {
    Neg,
    Not,
    BitNot,
}

/// A block terminator — how control leaves a basic block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Terminator {
    /// Return from function with a value.
    Return(ValueId),
    /// Return void.
    ReturnVoid,
    /// Unconditional jump to another block.
    Jump(BlockId),
    /// Conditional branch.
    Branch {
        condition: ValueId,
        then_block: BlockId,
        else_block: BlockId,
    },
    /// Unreachable (after diverging expressions like `panic!`).
    Unreachable,
}

/// A complete MIR module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
}
