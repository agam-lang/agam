//! GPU kernel configuration, builtin resolution, and validation.
//!
//! Resolves `@gpu(threads=N)` annotations into `GpuKernelConfig`, maps
//! source-level GPU builtins onto compiler-known operations, and validates
//! that kernel functions comply with GPU execution constraints (no heap,
//! no effects, no recursion, scalar returns only).

use agam_ast::{
    Path,
    decl::{Annotation, FunctionDecl},
    expr::{Expr, ExprKind, FStringPart},
    stmt::{ElseBranch, Stmt, StmtKind},
    types::{TypeExpr, TypeExprKind},
};
use serde::{Deserialize, Serialize};

use crate::symbol::TypeId;
use crate::types::TypeStore;

/// GPU kernel launch configuration extracted from `@gpu(...)` annotations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuKernelConfig {
    /// Threads per block (default 256).
    pub threads_per_block: u32,
    /// Shared memory bytes per block (default 0).
    pub shared_memory_bytes: u32,
    /// Optional explicit grid dimensions (blocks_x, blocks_y, blocks_z).
    pub grid_dim: Option<(u32, u32, u32)>,
}

impl Default for GpuKernelConfig {
    fn default() -> Self {
        Self {
            threads_per_block: 256,
            shared_memory_bytes: 0,
            grid_dim: None,
        }
    }
}

/// GPU kernel ABI hint for parameter lowering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GpuKernelParamAbi {
    Scalar(GpuKernelScalarAbi),
    Pointer {
        scalar: GpuKernelScalarAbi,
        depth: u8,
    },
    #[default]
    OpaquePtr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuKernelScalarAbi {
    I1,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    I512,
    F32,
    F64,
}

impl GpuKernelScalarAbi {
    pub fn llvm_ir(self) -> &'static str {
        match self {
            GpuKernelScalarAbi::I1 => "i1",
            GpuKernelScalarAbi::I8 => "i8",
            GpuKernelScalarAbi::I16 => "i16",
            GpuKernelScalarAbi::I32 => "i32",
            GpuKernelScalarAbi::I64 => "i64",
            GpuKernelScalarAbi::I128 => "i128",
            GpuKernelScalarAbi::I256 => "i256",
            GpuKernelScalarAbi::I512 => "i512",
            GpuKernelScalarAbi::F32 => "float",
            GpuKernelScalarAbi::F64 => "double",
        }
    }
}

impl GpuKernelParamAbi {
    pub fn llvm_ir(self) -> String {
        match self {
            GpuKernelParamAbi::Scalar(scalar) => scalar.llvm_ir().into(),
            GpuKernelParamAbi::Pointer { scalar, depth } => {
                format!("{}{}", scalar.llvm_ir(), "*".repeat(depth as usize))
            }
            GpuKernelParamAbi::OpaquePtr => "i8*".into(),
        }
    }

    pub fn scalar_from_name(name: &str) -> Option<Self> {
        match name {
            "bool" => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I1)),
            "i8" | "u8" => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I8)),
            "i16" | "u16" => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I16)),
            "i32" | "u32" | "char" => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I32)),
            "i64" | "u64" | "isize" | "usize" => {
                Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I64))
            }
            "i128" | "u128" => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I128)),
            "i256" | "u256" => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I256)),
            "i512" | "u512" => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I512)),
            "f32" => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::F32)),
            "f64" => Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::F64)),
            _ => None,
        }
    }

    pub fn pointer_to(self) -> Self {
        match self {
            GpuKernelParamAbi::Scalar(scalar) => GpuKernelParamAbi::Pointer { scalar, depth: 1 },
            GpuKernelParamAbi::Pointer { scalar, depth } => {
                if depth == u8::MAX {
                    GpuKernelParamAbi::OpaquePtr
                } else {
                    GpuKernelParamAbi::Pointer {
                        scalar,
                        depth: depth + 1,
                    }
                }
            }
            GpuKernelParamAbi::OpaquePtr => GpuKernelParamAbi::OpaquePtr,
        }
    }

    pub fn pointee_llvm_ir(self) -> String {
        match self {
            GpuKernelParamAbi::Scalar(scalar) => scalar.llvm_ir().into(),
            GpuKernelParamAbi::Pointer { scalar, depth } => {
                let pointee_depth = depth.saturating_sub(1);
                format!("{}{}", scalar.llvm_ir(), "*".repeat(pointee_depth as usize))
            }
            GpuKernelParamAbi::OpaquePtr => "i8".into(),
        }
    }

    pub fn shared_ptr_llvm_ir(self) -> String {
        format!("{} addrspace(3)*", self.llvm_ir())
    }
}

/// Source-level GPU builtin recognized by the compiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBuiltin {
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
    SharedAlloc,
    WarpShuffleDown,
    WarpReduceAdd,
    BallotSync,
    Sin,
    Cos,
    Sqrt,
    Exp,
}

impl GpuBuiltin {
    /// Return the compiler type produced by this GPU builtin.
    pub fn return_type(self, types: &mut TypeStore) -> TypeId {
        match self {
            GpuBuiltin::Barrier => types.unit(),
            GpuBuiltin::SharedAlloc => types.fresh_var(),
            GpuBuiltin::Sin | GpuBuiltin::Cos | GpuBuiltin::Sqrt | GpuBuiltin::Exp => types.f32(),
            GpuBuiltin::WarpShuffleDown | GpuBuiltin::WarpReduceAdd | GpuBuiltin::BallotSync => {
                types.i32()
            }
            GpuBuiltin::ThreadIdX
            | GpuBuiltin::ThreadIdY
            | GpuBuiltin::ThreadIdZ
            | GpuBuiltin::BlockIdX
            | GpuBuiltin::BlockIdY
            | GpuBuiltin::BlockIdZ
            | GpuBuiltin::BlockDimX
            | GpuBuiltin::BlockDimY
            | GpuBuiltin::BlockDimZ => types.i32(),
        }
    }

    /// Return the expected argument types for this GPU builtin.
    pub fn arg_types(self, types: &TypeStore) -> Vec<TypeId> {
        match self {
            GpuBuiltin::ThreadIdX
            | GpuBuiltin::ThreadIdY
            | GpuBuiltin::ThreadIdZ
            | GpuBuiltin::BlockIdX
            | GpuBuiltin::BlockIdY
            | GpuBuiltin::BlockIdZ
            | GpuBuiltin::BlockDimX
            | GpuBuiltin::BlockDimY
            | GpuBuiltin::BlockDimZ
            | GpuBuiltin::Barrier => Vec::new(),
            GpuBuiltin::SharedAlloc => vec![types.i32()],
            GpuBuiltin::WarpShuffleDown => {
                vec![types.i32(), types.i32(), types.i32(), types.i32()]
            }
            GpuBuiltin::WarpReduceAdd => vec![types.i32()],
            GpuBuiltin::BallotSync => vec![types.i32(), types.bool()],
            GpuBuiltin::Sin | GpuBuiltin::Cos | GpuBuiltin::Sqrt | GpuBuiltin::Exp => {
                vec![types.f32()]
            }
        }
    }
}

/// Resolve a fully qualified GPU builtin path.
pub fn resolve_gpu_builtin(path: &str) -> Option<GpuBuiltin> {
    match path {
        "agam::gpu::thread_id_x" | "gpu::thread_id_x" => Some(GpuBuiltin::ThreadIdX),
        "agam::gpu::thread_id_y" | "gpu::thread_id_y" => Some(GpuBuiltin::ThreadIdY),
        "agam::gpu::thread_id_z" | "gpu::thread_id_z" => Some(GpuBuiltin::ThreadIdZ),
        "agam::gpu::block_id_x" | "gpu::block_id_x" => Some(GpuBuiltin::BlockIdX),
        "agam::gpu::block_id_y" | "gpu::block_id_y" => Some(GpuBuiltin::BlockIdY),
        "agam::gpu::block_id_z" | "gpu::block_id_z" => Some(GpuBuiltin::BlockIdZ),
        "agam::gpu::block_dim_x" | "gpu::block_dim_x" => Some(GpuBuiltin::BlockDimX),
        "agam::gpu::block_dim_y" | "gpu::block_dim_y" => Some(GpuBuiltin::BlockDimY),
        "agam::gpu::block_dim_z" | "gpu::block_dim_z" => Some(GpuBuiltin::BlockDimZ),
        "agam::gpu::barrier" | "gpu::barrier" => Some(GpuBuiltin::Barrier),
        "agam::gpu::shared_alloc" | "gpu::shared_alloc" => Some(GpuBuiltin::SharedAlloc),
        "agam::gpu::warp_shuffle_down" | "gpu::warp_shuffle_down" => {
            Some(GpuBuiltin::WarpShuffleDown)
        }
        "agam::gpu::warp_reduce_add" | "gpu::warp_reduce_add" => Some(GpuBuiltin::WarpReduceAdd),
        "agam::gpu::ballot_sync" | "gpu::ballot_sync" => Some(GpuBuiltin::BallotSync),
        "agam::gpu::sin" | "gpu::sin" => Some(GpuBuiltin::Sin),
        "agam::gpu::cos" | "gpu::cos" => Some(GpuBuiltin::Cos),
        "agam::gpu::sqrt" | "gpu::sqrt" => Some(GpuBuiltin::Sqrt),
        "agam::gpu::exp" | "gpu::exp" => Some(GpuBuiltin::Exp),
        _ => None,
    }
}

/// Resolve a GPU builtin from an AST path.
pub fn resolve_gpu_builtin_path(path: &Path) -> Option<GpuBuiltin> {
    let full = path_name(path);
    resolve_gpu_builtin(&full)
}

/// Resolve a GPU builtin from an AST expression such as `agam.gpu.thread_id_x`.
pub fn resolve_gpu_builtin_expr(expr: &Expr) -> Option<GpuBuiltin> {
    expr_name(expr).and_then(|full| resolve_gpu_builtin(&full))
}

/// Resolve a GPU builtin from a dotted/module-style method call.
pub fn resolve_gpu_builtin_member(object: &Expr, member: &str) -> Option<GpuBuiltin> {
    let mut full = expr_name(object)?;
    full.push_str("::");
    full.push_str(member);
    resolve_gpu_builtin(&full)
}

fn path_name(path: &Path) -> String {
    path.segments
        .iter()
        .map(|segment| segment.name.as_str())
        .collect::<Vec<_>>()
        .join("::")
}

fn expr_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Identifier(ident) => Some(ident.name.clone()),
        ExprKind::PathExpr(path) => Some(path_name(path)),
        ExprKind::FieldAccess { object, field } => {
            let mut full = expr_name(object)?;
            full.push_str("::");
            full.push_str(&field.name);
            Some(full)
        }
        _ => None,
    }
}

#[derive(Default)]
struct GpuKernelBodyFacts {
    has_effects: bool,
    has_strings: bool,
    has_heap_alloc: bool,
    body_calls_self: bool,
    callees: Vec<String>,
}

/// Errors encountered during GPU kernel validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuKernelError {
    /// Heap allocation is not allowed in GPU kernels.
    HeapAllocationProhibited,
    /// Effect perform/handle is not allowed in GPU kernels.
    EffectsProhibited,
    /// Recursion is not allowed in GPU kernels.
    RecursionProhibited { callee: String },
    /// String operations are not allowed in GPU kernels.
    StringOpsProhibited,
    /// Dynamic dispatch is not allowed in GPU kernels.
    DynamicDispatchProhibited,
    /// Return type must be void or a scalar type.
    InvalidReturnType,
    /// Multiple `@gpu` annotations on the same function.
    MultipleGpuAnnotations,
    /// Invalid argument in `@gpu(...)` annotation.
    InvalidAnnotationArg { arg: String },
}

impl std::fmt::Display for GpuKernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuKernelError::HeapAllocationProhibited => {
                write!(f, "heap allocation is not allowed in GPU kernels")
            }
            GpuKernelError::EffectsProhibited => {
                write!(f, "effect perform/handle is not allowed in GPU kernels")
            }
            GpuKernelError::RecursionProhibited { callee } => {
                write!(f, "recursion (`{callee}`) is not allowed in GPU kernels")
            }
            GpuKernelError::StringOpsProhibited => {
                write!(f, "string operations are not allowed in GPU kernels")
            }
            GpuKernelError::DynamicDispatchProhibited => {
                write!(f, "dynamic dispatch is not allowed in GPU kernels")
            }
            GpuKernelError::InvalidReturnType => {
                write!(f, "GPU kernel return type must be void or a scalar")
            }
            GpuKernelError::MultipleGpuAnnotations => {
                write!(f, "multiple @gpu annotations on the same function")
            }
            GpuKernelError::InvalidAnnotationArg { arg } => {
                write!(f, "invalid @gpu annotation argument: `{arg}`")
            }
        }
    }
}

/// Check if an annotation is a `@gpu` directive.
fn is_gpu_annotation(ann: &Annotation) -> bool {
    ann.name.name == "gpu"
}

/// Resolve a `GpuKernelConfig` from function annotations.
///
/// Returns `None` if no `@gpu` annotation is present.
/// Returns `Some(config)` with extracted parameters if `@gpu` is found.
///
/// Supported annotation forms:
/// - `@gpu` → default config (256 threads)
/// - `@gpu(threads=512)` → custom thread count
/// - `@gpu(threads=512, shared=4096)` → threads + shared memory
/// - `@gpu(threads=256, grid=(8,8,1))` → threads + explicit grid
pub fn resolve_gpu_config(
    annotations: &[Annotation],
) -> Result<Option<GpuKernelConfig>, GpuKernelError> {
    let gpu_annotations: Vec<&Annotation> = annotations
        .iter()
        .filter(|a| is_gpu_annotation(a))
        .collect();

    if gpu_annotations.is_empty() {
        return Ok(None);
    }
    if gpu_annotations.len() > 1 {
        return Err(GpuKernelError::MultipleGpuAnnotations);
    }

    let ann = gpu_annotations[0];
    let mut config = GpuKernelConfig::default();

    for arg in &ann.args {
        let Some((key, value)) = annotation_assignment(arg) else {
            continue;
        };
        match key {
            "threads" => {
                config.threads_per_block =
                    expr_as_u32(value).ok_or_else(|| GpuKernelError::InvalidAnnotationArg {
                        arg: "threads".into(),
                    })?;
            }
            "shared" => {
                config.shared_memory_bytes =
                    expr_as_u32(value).ok_or_else(|| GpuKernelError::InvalidAnnotationArg {
                        arg: "shared".into(),
                    })?;
            }
            "grid" => {
                config.grid_dim =
                    Some(expr_as_grid_dim(value).ok_or_else(|| {
                        GpuKernelError::InvalidAnnotationArg { arg: "grid".into() }
                    })?);
            }
            _ => {
                // Accept but ignore unknown args for forward compatibility
            }
        }
    }

    Ok(Some(config))
}

fn annotation_assignment(arg: &Expr) -> Option<(&str, &Expr)> {
    let ExprKind::Assign { target, value } = &arg.kind else {
        return None;
    };
    match &target.kind {
        ExprKind::Identifier(ident) => Some((ident.name.as_str(), value.as_ref())),
        ExprKind::PathExpr(path) => path
            .segments
            .last()
            .map(|segment| (segment.name.as_str(), value.as_ref())),
        _ => None,
    }
}

fn expr_as_u32(expr: &Expr) -> Option<u32> {
    match expr.kind {
        ExprKind::IntLiteral(value) if value >= 0 => Some(value as u32),
        _ => None,
    }
}

fn expr_as_grid_dim(expr: &Expr) -> Option<(u32, u32, u32)> {
    let values = match &expr.kind {
        ExprKind::TupleLiteral(values) | ExprKind::ArrayLiteral(values) => values,
        _ => return None,
    };
    if values.len() != 3 {
        return None;
    }
    Some((
        expr_as_u32(&values[0])?,
        expr_as_u32(&values[1])?,
        expr_as_u32(&values[2])?,
    ))
}

/// Validate that a function body is compatible with GPU kernel execution.
///
/// This operates on HIR expression kinds to check for prohibited constructs.
pub fn validate_gpu_kernel_body(
    body_has_effects: bool,
    body_has_strings: bool,
    body_has_heap_alloc: bool,
    body_calls_self: bool,
    self_name: &str,
    callees: &[String],
) -> Vec<GpuKernelError> {
    let mut errors = Vec::new();

    if body_has_effects {
        errors.push(GpuKernelError::EffectsProhibited);
    }
    if body_has_strings {
        errors.push(GpuKernelError::StringOpsProhibited);
    }
    if body_has_heap_alloc {
        errors.push(GpuKernelError::HeapAllocationProhibited);
    }
    if body_calls_self {
        errors.push(GpuKernelError::RecursionProhibited {
            callee: self_name.to_string(),
        });
    }
    for callee in callees {
        if callee == self_name {
            errors.push(GpuKernelError::RecursionProhibited {
                callee: callee.clone(),
            });
        }
    }

    errors
}

/// Validate one `@gpu` function declaration against kernel execution rules.
pub fn validate_gpu_kernel_function(function: &FunctionDecl) -> Vec<GpuKernelError> {
    let mut facts = GpuKernelBodyFacts::default();
    if let Some(body) = &function.body {
        collect_gpu_kernel_block_facts(body, &function.name.name, &mut facts);
    }

    let mut errors = validate_gpu_kernel_body(
        facts.has_effects,
        facts.has_strings,
        facts.has_heap_alloc,
        facts.body_calls_self,
        &function.name.name,
        &facts.callees,
    );

    if let Some(return_ty) = &function.return_type
        && !is_gpu_scalar_type_expr(return_ty)
    {
        errors.push(GpuKernelError::InvalidReturnType);
    }

    errors
}

fn collect_gpu_kernel_block_facts(
    block: &agam_ast::expr::Block,
    self_name: &str,
    facts: &mut GpuKernelBodyFacts,
) {
    for stmt in &block.stmts {
        collect_gpu_kernel_stmt_facts(stmt, self_name, facts);
    }
    if let Some(expr) = &block.expr {
        collect_gpu_kernel_expr_facts(expr, self_name, facts);
    }
}

fn collect_gpu_kernel_stmt_facts(stmt: &Stmt, self_name: &str, facts: &mut GpuKernelBodyFacts) {
    match &stmt.kind {
        StmtKind::Let { value, .. } => {
            if let Some(value) = value {
                collect_gpu_kernel_expr_facts(value, self_name, facts);
            }
        }
        StmtKind::Const { value, .. } | StmtKind::Expression(value) | StmtKind::Throw(value) => {
            collect_gpu_kernel_expr_facts(value, self_name, facts)
        }
        StmtKind::Return(value) | StmtKind::Break(value) | StmtKind::Yield(value) => {
            if let Some(value) = value {
                collect_gpu_kernel_expr_facts(value, self_name, facts);
            }
        }
        StmtKind::While { condition, body } => {
            collect_gpu_kernel_expr_facts(condition, self_name, facts);
            collect_gpu_kernel_block_facts(body, self_name, facts);
        }
        StmtKind::Loop { body } => collect_gpu_kernel_block_facts(body, self_name, facts),
        StmtKind::For { iterable, body, .. } => {
            collect_gpu_kernel_expr_facts(iterable, self_name, facts);
            collect_gpu_kernel_block_facts(body, self_name, facts);
        }
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_gpu_kernel_expr_facts(condition, self_name, facts);
            collect_gpu_kernel_block_facts(then_branch, self_name, facts);
            if let Some(else_branch) = else_branch {
                collect_gpu_kernel_else_branch_facts(else_branch, self_name, facts);
            }
        }
        StmtKind::Match { scrutinee, arms } => {
            collect_gpu_kernel_expr_facts(scrutinee, self_name, facts);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_gpu_kernel_expr_facts(guard, self_name, facts);
                }
                collect_gpu_kernel_expr_facts(&arm.body, self_name, facts);
            }
        }
        StmtKind::TryCatch { body, catches } => {
            collect_gpu_kernel_block_facts(body, self_name, facts);
            for catch in catches {
                collect_gpu_kernel_block_facts(&catch.body, self_name, facts);
            }
        }
        StmtKind::Continue | StmtKind::Declaration(_) => {}
    }
}

fn collect_gpu_kernel_else_branch_facts(
    else_branch: &ElseBranch,
    self_name: &str,
    facts: &mut GpuKernelBodyFacts,
) {
    match else_branch {
        ElseBranch::Else(block) => collect_gpu_kernel_block_facts(block, self_name, facts),
        ElseBranch::ElseIf(stmt) => collect_gpu_kernel_stmt_facts(stmt, self_name, facts),
    }
}

fn collect_gpu_kernel_expr_facts(expr: &Expr, self_name: &str, facts: &mut GpuKernelBodyFacts) {
    match &expr.kind {
        ExprKind::StringLiteral(_) => facts.has_strings = true,
        ExprKind::FStringLiteral { parts } => {
            facts.has_strings = true;
            for part in parts {
                if let FStringPart::Expr(expr) = part {
                    collect_gpu_kernel_expr_facts(expr, self_name, facts);
                }
            }
        }
        ExprKind::ArrayLiteral(elements) | ExprKind::TupleLiteral(elements) => {
            for element in elements {
                collect_gpu_kernel_expr_facts(element, self_name, facts);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_gpu_kernel_expr_facts(left, self_name, facts);
            collect_gpu_kernel_expr_facts(right, self_name, facts);
        }
        ExprKind::Unary { operand, .. }
        | ExprKind::Await(operand)
        | ExprKind::Spawn(operand)
        | ExprKind::Try(operand)
        | ExprKind::Backward(operand)
        | ExprKind::Resume(operand) => collect_gpu_kernel_expr_facts(operand, self_name, facts),
        ExprKind::FieldAccess { object, .. } => {
            collect_gpu_kernel_expr_facts(object, self_name, facts);
        }
        ExprKind::Index { object, index } => {
            collect_gpu_kernel_expr_facts(object, self_name, facts);
            collect_gpu_kernel_expr_facts(index, self_name, facts);
        }
        ExprKind::MethodCall {
            object,
            method,
            args,
        } => {
            collect_gpu_kernel_expr_facts(object, self_name, facts);
            for arg in args {
                collect_gpu_kernel_expr_facts(arg, self_name, facts);
            }

            if resolve_gpu_builtin_member(object, &method.name) == Some(GpuBuiltin::SharedAlloc) {
                return;
            }

            if method.name == "gpu_malloc" {
                facts.has_heap_alloc = true;
            }
        }
        ExprKind::Call { callee, args } => {
            if let Some(name) = expr_name(callee) {
                if name == self_name {
                    facts.body_calls_self = true;
                } else {
                    facts.callees.push(name.clone());
                }

                if name == "gpu_malloc" || name.ends_with("::gpu_malloc") {
                    facts.has_heap_alloc = true;
                }
            }

            collect_gpu_kernel_expr_facts(callee, self_name, facts);
            for arg in args {
                collect_gpu_kernel_expr_facts(arg, self_name, facts);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_gpu_kernel_expr_facts(condition, self_name, facts);
            collect_gpu_kernel_expr_facts(then_branch, self_name, facts);
            if let Some(else_branch) = else_branch {
                collect_gpu_kernel_expr_facts(else_branch, self_name, facts);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_gpu_kernel_expr_facts(scrutinee, self_name, facts);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_gpu_kernel_expr_facts(guard, self_name, facts);
                }
                collect_gpu_kernel_expr_facts(&arm.body, self_name, facts);
            }
        }
        ExprKind::Block(block) | ExprKind::BlockExpr(block) => {
            collect_gpu_kernel_block_facts(block, self_name, facts);
        }
        ExprKind::Lambda { body, .. } => collect_gpu_kernel_expr_facts(body, self_name, facts),
        ExprKind::Assign { target, value } | ExprKind::CompoundAssign { target, value, .. } => {
            collect_gpu_kernel_expr_facts(target, self_name, facts);
            collect_gpu_kernel_expr_facts(value, self_name, facts);
        }
        ExprKind::Range { start, end, .. } => {
            if let Some(start) = start {
                collect_gpu_kernel_expr_facts(start, self_name, facts);
            }
            if let Some(end) = end {
                collect_gpu_kernel_expr_facts(end, self_name, facts);
            }
        }
        ExprKind::Cast { expr, .. } => collect_gpu_kernel_expr_facts(expr, self_name, facts),
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                collect_gpu_kernel_expr_facts(&field.value, self_name, facts);
            }
        }
        ExprKind::Grad { func, .. } => collect_gpu_kernel_expr_facts(func, self_name, facts),
        ExprKind::Perform { args, .. } => {
            facts.has_effects = true;
            for arg in args {
                collect_gpu_kernel_expr_facts(arg, self_name, facts);
            }
        }
        ExprKind::HandleWith { body, .. } => {
            facts.has_effects = true;
            collect_gpu_kernel_expr_facts(body, self_name, facts);
        }
        ExprKind::Identifier(_)
        | ExprKind::PathExpr(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_) => {}
    }
}

fn is_gpu_scalar_type_expr(ty: &TypeExpr) -> bool {
    match &ty.kind {
        TypeExprKind::Named(path) => path
            .segments
            .last()
            .and_then(|segment| GpuKernelParamAbi::scalar_from_name(&segment.name))
            .is_some(),
        TypeExprKind::Refined { base, .. } => is_gpu_scalar_type_expr(base),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_ast::Ident;
    use agam_errors::Span;
    use agam_errors::span::SourceId;

    fn make_span() -> Span {
        Span::new(SourceId(0), 0, 0)
    }

    fn gpu_annotation(args: Vec<agam_ast::expr::Expr>) -> Annotation {
        Annotation {
            name: Ident {
                name: "gpu".into(),
                span: make_span(),
            },
            args,
            span: make_span(),
        }
    }

    fn plain_annotation(name: &str) -> Annotation {
        Annotation {
            name: Ident {
                name: name.into(),
                span: make_span(),
            },
            args: vec![],
            span: make_span(),
        }
    }

    fn ident_expr(name: &str) -> Expr {
        Expr {
            id: agam_ast::NodeId(0),
            span: make_span(),
            kind: ExprKind::Identifier(Ident {
                name: name.into(),
                span: make_span(),
            }),
        }
    }

    fn int_expr(value: i64) -> Expr {
        Expr {
            id: agam_ast::NodeId(0),
            span: make_span(),
            kind: ExprKind::IntLiteral(value),
        }
    }

    fn assign_expr(name: &str, value: Expr) -> Expr {
        Expr {
            id: agam_ast::NodeId(0),
            span: make_span(),
            kind: ExprKind::Assign {
                target: Box::new(ident_expr(name)),
                value: Box::new(value),
            },
        }
    }

    #[test]
    fn resolve_no_gpu_annotation() {
        let anns = vec![plain_annotation("test")];
        assert_eq!(resolve_gpu_config(&anns).unwrap(), None);
    }

    #[test]
    fn resolve_empty_annotations() {
        assert_eq!(resolve_gpu_config(&[]).unwrap(), None);
    }

    #[test]
    fn resolve_plain_gpu_annotation_gives_defaults() {
        let anns = vec![gpu_annotation(vec![])];
        let config = resolve_gpu_config(&anns).unwrap().unwrap();
        assert_eq!(config.threads_per_block, 256);
        assert_eq!(config.shared_memory_bytes, 0);
        assert_eq!(config.grid_dim, None);
    }

    #[test]
    fn resolve_gpu_annotation_arguments() {
        let anns = vec![gpu_annotation(vec![
            assign_expr("threads", int_expr(128)),
            assign_expr("shared", int_expr(64)),
            assign_expr(
                "grid",
                Expr {
                    id: agam_ast::NodeId(0),
                    span: make_span(),
                    kind: ExprKind::TupleLiteral(vec![int_expr(8), int_expr(1), int_expr(1)]),
                },
            ),
        ])];
        let config = resolve_gpu_config(&anns).unwrap().unwrap();
        assert_eq!(config.threads_per_block, 128);
        assert_eq!(config.shared_memory_bytes, 64);
        assert_eq!(config.grid_dim, Some((8, 1, 1)));
    }

    #[test]
    fn resolve_multiple_gpu_annotations_is_error() {
        let anns = vec![gpu_annotation(vec![]), gpu_annotation(vec![])];
        assert_eq!(
            resolve_gpu_config(&anns).unwrap_err(),
            GpuKernelError::MultipleGpuAnnotations
        );
    }

    #[test]
    fn validate_clean_kernel_passes() {
        let errors = validate_gpu_kernel_body(false, false, false, false, "kern", &[]);
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_effects_rejected() {
        let errors = validate_gpu_kernel_body(true, false, false, false, "kern", &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], GpuKernelError::EffectsProhibited);
    }

    #[test]
    fn validate_strings_rejected() {
        let errors = validate_gpu_kernel_body(false, true, false, false, "kern", &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], GpuKernelError::StringOpsProhibited);
    }

    #[test]
    fn validate_heap_rejected() {
        let errors = validate_gpu_kernel_body(false, false, true, false, "kern", &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], GpuKernelError::HeapAllocationProhibited);
    }

    #[test]
    fn validate_recursion_rejected() {
        let errors = validate_gpu_kernel_body(false, false, false, true, "kern", &["kern".into()]);
        // Both body_calls_self and callees contain self
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, GpuKernelError::RecursionProhibited { .. }))
        );
    }

    #[test]
    fn validate_multiple_violations() {
        let errors = validate_gpu_kernel_body(true, true, true, false, "kern", &[]);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn gpu_kernel_error_display() {
        let err = GpuKernelError::EffectsProhibited;
        assert!(err.to_string().contains("not allowed in GPU kernels"));
    }

    #[test]
    fn default_config_values() {
        let config = GpuKernelConfig::default();
        assert_eq!(config.threads_per_block, 256);
        assert_eq!(config.shared_memory_bytes, 0);
        assert_eq!(config.grid_dim, None);
    }

    #[test]
    fn resolves_gpu_builtin_paths() {
        assert_eq!(
            resolve_gpu_builtin("agam::gpu::thread_id_x"),
            Some(GpuBuiltin::ThreadIdX)
        );
        assert_eq!(
            resolve_gpu_builtin("gpu::barrier"),
            Some(GpuBuiltin::Barrier)
        );
        assert_eq!(
            resolve_gpu_builtin("agam::gpu::sqrt"),
            Some(GpuBuiltin::Sqrt)
        );
        assert_eq!(
            resolve_gpu_builtin("agam::gpu::shared_alloc"),
            Some(GpuBuiltin::SharedAlloc)
        );
        assert_eq!(
            resolve_gpu_builtin("agam::gpu::warp_shuffle_down"),
            Some(GpuBuiltin::WarpShuffleDown)
        );
        assert_eq!(
            resolve_gpu_builtin("agam::gpu::warp_reduce_add"),
            Some(GpuBuiltin::WarpReduceAdd)
        );
        assert_eq!(
            resolve_gpu_builtin("agam::gpu::ballot_sync"),
            Some(GpuBuiltin::BallotSync)
        );
        assert_eq!(resolve_gpu_builtin("std::math::sqrt"), None);
    }

    #[test]
    fn gpu_builtin_signatures_use_backend_types() {
        let mut types = TypeStore::new();
        assert_eq!(GpuBuiltin::ThreadIdX.return_type(&mut types), types.i32());
        assert_eq!(GpuBuiltin::Barrier.return_type(&mut types), types.unit());
        assert_eq!(GpuBuiltin::Sin.return_type(&mut types), types.f32());
        assert_eq!(
            GpuBuiltin::WarpShuffleDown.return_type(&mut types),
            types.i32()
        );
        assert_eq!(GpuBuiltin::Sqrt.arg_types(&types), vec![types.f32()]);
        assert_eq!(GpuBuiltin::SharedAlloc.arg_types(&types), vec![types.i32()]);
        assert_eq!(
            GpuBuiltin::WarpShuffleDown.arg_types(&types),
            vec![types.i32(), types.i32(), types.i32(), types.i32()]
        );
        assert_eq!(
            GpuBuiltin::WarpReduceAdd.arg_types(&types),
            vec![types.i32()]
        );
        assert_eq!(
            GpuBuiltin::BallotSync.arg_types(&types),
            vec![types.i32(), types.bool()]
        );
    }

    #[test]
    fn gpu_param_abi_supports_256_and_512_bit_integer_names() {
        assert_eq!(
            GpuKernelParamAbi::scalar_from_name("i256"),
            Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I256))
        );
        assert_eq!(
            GpuKernelParamAbi::scalar_from_name("u256"),
            Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I256))
        );
        assert_eq!(
            GpuKernelParamAbi::scalar_from_name("i512"),
            Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I512))
        );
        assert_eq!(
            GpuKernelParamAbi::scalar_from_name("u512"),
            Some(GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I512))
        );
        assert_eq!(
            GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I256).pointer_to(),
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::I256,
                depth: 1,
            }
        );
        assert_eq!(
            GpuKernelParamAbi::Scalar(GpuKernelScalarAbi::I512).pointer_to(),
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::I512,
                depth: 1,
            }
        );
        assert_eq!(
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::F32,
                depth: 1,
            }
            .pointer_to(),
            GpuKernelParamAbi::Pointer {
                scalar: GpuKernelScalarAbi::F32,
                depth: 2,
            }
        );
    }
}
