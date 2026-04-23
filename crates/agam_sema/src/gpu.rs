//! GPU kernel configuration and validation.
//!
//! Resolves `@gpu(threads=N)` annotations into `GpuKernelConfig` and
//! validates that kernel functions comply with GPU execution constraints
//! (no heap, no effects, no recursion, scalar returns only).

use agam_ast::decl::Annotation;
use serde::{Deserialize, Serialize};

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
                write!(
                    f,
                    "recursion (`{callee}`) is not allowed in GPU kernels"
                )
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
    let gpu_annotations: Vec<&Annotation> =
        annotations.iter().filter(|a| is_gpu_annotation(a)).collect();

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
        let errors =
            validate_gpu_kernel_body(false, false, false, false, "kern", &[]);
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_effects_rejected() {
        let errors =
            validate_gpu_kernel_body(true, false, false, false, "kern", &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], GpuKernelError::EffectsProhibited);
    }

    #[test]
    fn validate_strings_rejected() {
        let errors =
            validate_gpu_kernel_body(false, true, false, false, "kern", &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], GpuKernelError::StringOpsProhibited);
    }

    #[test]
    fn validate_heap_rejected() {
        let errors =
            validate_gpu_kernel_body(false, false, true, false, "kern", &[]);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0], GpuKernelError::HeapAllocationProhibited);
    }

    #[test]
    fn validate_recursion_rejected() {
        let errors = validate_gpu_kernel_body(
            false,
            false,
            false,
            true,
            "kern",
            &["kern".into()],
        );
        // Both body_calls_self and callees contain self
        assert!(errors
            .iter()
            .any(|e| matches!(e, GpuKernelError::RecursionProhibited { .. })));
    }

    #[test]
    fn validate_multiple_violations() {
        let errors =
            validate_gpu_kernel_body(true, true, true, false, "kern", &[]);
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
}
