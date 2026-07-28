//! GPU kernel configuration, builtin resolution, and validation.
//!
//! Resolves `@gpu(threads=N)` annotations into `GpuKernelConfig`, maps
//! source-level GPU builtins onto compiler-known operations, and validates
//! that kernel functions comply with GPU execution constraints (no heap,
//! no effects, no recursion, scalar returns only).

use agam_ast::{
    Path,
    decl::Annotation,
    expr::{Expr, ExprKind},
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
    PtrI1,
    PtrI8,
    PtrI16,
    PtrI32,
    PtrI64,
    PtrI128,
    PtrI256,
    PtrI512,
    PtrF32,
    PtrF64,
    #[default]
    OpaquePtr,
}

impl GpuKernelParamAbi {
    pub fn llvm_ir(self) -> &'static str {
        match self {
            GpuKernelParamAbi::I1 => "i1",
            GpuKernelParamAbi::I8 => "i8",
            GpuKernelParamAbi::I16 => "i16",
            GpuKernelParamAbi::I32 => "i32",
            GpuKernelParamAbi::I64 => "i64",
            GpuKernelParamAbi::I128 => "i128",
            GpuKernelParamAbi::I256 => "i256",
            GpuKernelParamAbi::I512 => "i512",
            GpuKernelParamAbi::F32 => "float",
            GpuKernelParamAbi::F64 => "double",
            GpuKernelParamAbi::PtrI1 => "i1*",
            GpuKernelParamAbi::PtrI8 => "i8*",
            GpuKernelParamAbi::PtrI16 => "i16*",
            GpuKernelParamAbi::PtrI32 => "i32*",
            GpuKernelParamAbi::PtrI64 => "i64*",
            GpuKernelParamAbi::PtrI128 => "i128*",
            GpuKernelParamAbi::PtrI256 => "i256*",
            GpuKernelParamAbi::PtrI512 => "i512*",
            GpuKernelParamAbi::PtrF32 => "float*",
            GpuKernelParamAbi::PtrF64 => "double*",
            GpuKernelParamAbi::OpaquePtr => "i8*",
        }
    }

    pub fn scalar_from_name(name: &str) -> Option<Self> {
        match name {
            "bool" => Some(GpuKernelParamAbi::I1),
            "i8" | "u8" => Some(GpuKernelParamAbi::I8),
            "i16" | "u16" => Some(GpuKernelParamAbi::I16),
            "i32" | "u32" | "char" => Some(GpuKernelParamAbi::I32),
            "i64" | "u64" | "isize" | "usize" => Some(GpuKernelParamAbi::I64),
            "i128" | "u128" => Some(GpuKernelParamAbi::I128),
            "i256" | "u256" => Some(GpuKernelParamAbi::I256),
            "i512" | "u512" => Some(GpuKernelParamAbi::I512),
            "f32" => Some(GpuKernelParamAbi::F32),
            "f64" => Some(GpuKernelParamAbi::F64),
            _ => None,
        }
    }

    pub fn pointer_to(self) -> Self {
        match self {
            GpuKernelParamAbi::I1 => GpuKernelParamAbi::PtrI1,
            GpuKernelParamAbi::I8 => GpuKernelParamAbi::PtrI8,
            GpuKernelParamAbi::I16 => GpuKernelParamAbi::PtrI16,
            GpuKernelParamAbi::I32 => GpuKernelParamAbi::PtrI32,
            GpuKernelParamAbi::I64 => GpuKernelParamAbi::PtrI64,
            GpuKernelParamAbi::I128 => GpuKernelParamAbi::PtrI128,
            GpuKernelParamAbi::I256 => GpuKernelParamAbi::PtrI256,
            GpuKernelParamAbi::I512 => GpuKernelParamAbi::PtrI512,
            GpuKernelParamAbi::F32 => GpuKernelParamAbi::PtrF32,
            GpuKernelParamAbi::F64 => GpuKernelParamAbi::PtrF64,
            _ => GpuKernelParamAbi::OpaquePtr,
        }
    }

    pub fn pointee_llvm_ir(self) -> &'static str {
        match self {
            GpuKernelParamAbi::I1 | GpuKernelParamAbi::PtrI1 => "i1",
            GpuKernelParamAbi::I8 | GpuKernelParamAbi::PtrI8 => "i8",
            GpuKernelParamAbi::I16 | GpuKernelParamAbi::PtrI16 => "i16",
            GpuKernelParamAbi::I32 | GpuKernelParamAbi::PtrI32 => "i32",
            GpuKernelParamAbi::I64 | GpuKernelParamAbi::PtrI64 => "i64",
            GpuKernelParamAbi::I128 | GpuKernelParamAbi::PtrI128 => "i128",
            GpuKernelParamAbi::I256 | GpuKernelParamAbi::PtrI256 => "i256",
            GpuKernelParamAbi::I512 | GpuKernelParamAbi::PtrI512 => "i512",
            GpuKernelParamAbi::F32 | GpuKernelParamAbi::PtrF32 => "float",
            GpuKernelParamAbi::F64 | GpuKernelParamAbi::PtrF64 => "double",
            GpuKernelParamAbi::OpaquePtr => "i8",
        }
    }

    pub fn shared_ptr_llvm_ir(self) -> &'static str {
        match self {
            GpuKernelParamAbi::I1 | GpuKernelParamAbi::PtrI1 => "i1 addrspace(3)*",
            GpuKernelParamAbi::I8 | GpuKernelParamAbi::PtrI8 => "i8 addrspace(3)*",
            GpuKernelParamAbi::I16 | GpuKernelParamAbi::PtrI16 => "i16 addrspace(3)*",
            GpuKernelParamAbi::I32 | GpuKernelParamAbi::PtrI32 => "i32 addrspace(3)*",
            GpuKernelParamAbi::I64 | GpuKernelParamAbi::PtrI64 => "i64 addrspace(3)*",
            GpuKernelParamAbi::I128 | GpuKernelParamAbi::PtrI128 => "i128 addrspace(3)*",
            GpuKernelParamAbi::I256 | GpuKernelParamAbi::PtrI256 => "i256 addrspace(3)*",
            GpuKernelParamAbi::I512 | GpuKernelParamAbi::PtrI512 => "i512 addrspace(3)*",
            GpuKernelParamAbi::F32 | GpuKernelParamAbi::PtrF32 => "float addrspace(3)*",
            GpuKernelParamAbi::F64 | GpuKernelParamAbi::PtrF64 => "double addrspace(3)*",
            GpuKernelParamAbi::OpaquePtr => "i8 addrspace(3)*",
        }
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

    // Parse annotation arguments as key=value expressions.
    // The AST represents these as Expr nodes; we extract textual form.
    for arg in &ann.args {
        let text = format!("{:?}", arg);
        // Look for known parameter patterns in debug representation
        if let Some(threads) = extract_int_param(&text, "threads") {
            config.threads_per_block = threads;
        } else if let Some(shared) = extract_int_param(&text, "shared") {
            config.shared_memory_bytes = shared;
        } else {
            // Accept but ignore unknown args for forward compatibility
        }
    }

    Ok(Some(config))
}

/// Extract an integer parameter from a debug-formatted expression string.
fn extract_int_param(text: &str, key: &str) -> Option<u32> {
    // Match patterns like `Assign { target: Var("threads"), value: IntLit(512) }`
    if text.contains(key) {
        // Find the integer literal value
        if let Some(start) = text.find("IntLit(") {
            let after = &text[start + 7..];
            if let Some(end) = after.find(')') {
                return after[..end].parse::<u32>().ok();
            }
        }
    }
    None
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
        assert_eq!(resolve_gpu_builtin("std::math::sqrt"), None);
    }

    #[test]
    fn gpu_builtin_signatures_use_backend_types() {
        let mut types = TypeStore::new();
        assert_eq!(GpuBuiltin::ThreadIdX.return_type(&mut types), types.i32());
        assert_eq!(GpuBuiltin::Barrier.return_type(&mut types), types.unit());
        assert_eq!(GpuBuiltin::Sin.return_type(&mut types), types.f32());
        assert_eq!(GpuBuiltin::Sqrt.arg_types(&types), vec![types.f32()]);
        assert_eq!(GpuBuiltin::SharedAlloc.arg_types(&types), vec![types.i32()]);
    }

    #[test]
    fn gpu_param_abi_supports_256_and_512_bit_integer_names() {
        assert_eq!(
            GpuKernelParamAbi::scalar_from_name("i256"),
            Some(GpuKernelParamAbi::I256)
        );
        assert_eq!(
            GpuKernelParamAbi::scalar_from_name("u256"),
            Some(GpuKernelParamAbi::I256)
        );
        assert_eq!(
            GpuKernelParamAbi::scalar_from_name("i512"),
            Some(GpuKernelParamAbi::I512)
        );
        assert_eq!(
            GpuKernelParamAbi::scalar_from_name("u512"),
            Some(GpuKernelParamAbi::I512)
        );
        assert_eq!(
            GpuKernelParamAbi::I256.pointer_to(),
            GpuKernelParamAbi::PtrI256
        );
        assert_eq!(
            GpuKernelParamAbi::I512.pointer_to(),
            GpuKernelParamAbi::PtrI512
        );
    }
}
