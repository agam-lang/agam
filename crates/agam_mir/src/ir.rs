//! MIR node definitions — SSA-based, CFG-structured IR.

use agam_sema::gpu::GpuKernelConfig;
use agam_sema::symbol::TypeId;
use agam_sema::target::TargetProfile;
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
    /// Target deployment profile from `@target.*` annotations.
    #[serde(default)]
    pub target: TargetProfile,
    /// GPU kernel configuration from `@gpu(...)` annotations.
    #[serde(default)]
    pub gpu_config: Option<GpuKernelConfig>,
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

    /// Perform an algebraic effect operation.
    ///
    /// Dispatches to the handler registered for the named effect and
    /// operation at runtime. Falls back to static dispatch for builtin
    /// effects when the backend supports it.
    EffectPerform {
        effect: String,
        operation: String,
        args: Vec<ValueId>,
    },

    /// Install an effect handler for a scope.
    ///
    /// The handler's body is a callable that receives the performed
    /// operation arguments. This is the MIR representation of
    /// `with <handler> handle <effect> { ... }` blocks.
    HandleWith {
        effect: String,
        handler: String,
        body: BlockId,
    },

    /// Launch a GPU kernel from host code.
    ///
    /// The grid and block arguments specify the launch dimensions.
    /// The kernel function lives in the separate GPU NVPTX module.
    GpuKernelLaunch {
        kernel_name: String,
        grid: ValueId,
        block: ValueId,
        args: Vec<ValueId>,
    },

    /// Hardware-accelerated GPU or NPU intrinsic.
    GpuIntrinsic {
        kind: GpuIntrinsicKind,
        args: Vec<ValueId>,
    },

    /// Inline PTX or assembly.
    InlineAsm {
        asm_string: String,
        constraints: String,
        args: Vec<ValueId>,
    },
}

/// Specialized hardware intrinsics for the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuIntrinsicKind {
    ThreadIdX,
    ThreadIdY,
    ThreadIdZ,
    BlockIdX,
    BlockIdY,
    BlockIdZ,
    BlockDimX,
    BlockDimY,
    BlockDimZ,
    Barrier,
    WarpShuffleDown,
    WarpReduceAdd,
    BallotSync,
    NvvmSin,
    NvvmCos,
    NvvmSqrt,
    NvvmExp,
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
