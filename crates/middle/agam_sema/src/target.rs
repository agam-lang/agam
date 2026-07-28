//! Target profile system for omni-targeting directives.
//!
//! Allows Agam programs to declare their deployment target using annotations
//! like `@target.iot`, `@target.enterprise`, or `@target.hpc`. The compiler
//! derives constraints from the profile and enforces them through sema and codegen.

use agam_ast::decl::Annotation;
use serde::{Deserialize, Serialize};

/// Target deployment profiles that control compiler behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TargetProfile {
    /// Default: no restrictions, full stdlib and runtime.
    #[default]
    Default,
    /// IoT/embedded: no heap, no stdlib, sub-10KB binary targets.
    /// Strips ARC memory, refuses heap allocations, targets ESP32/Arduino.
    Iot,
    /// Enterprise: heavier runtime services — GC choice, thread pooling,
    /// capability-sandbox locking.
    Enterprise,
    /// HPC: aggressive vectorization, compute-oriented codegen, SIMD preference.
    Hpc,
}

impl TargetProfile {
    /// Human-readable name for diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Iot => "iot",
            Self::Enterprise => "enterprise",
            Self::Hpc => "hpc",
        }
    }
}

/// Constraints derived from a target profile.
/// Codegen and sema check these to enforce / enable target-specific behavior.
#[derive(Debug, Clone)]
pub struct TargetConstraints {
    /// Whether heap allocation (`malloc`, `String`, etc.) is allowed.
    pub allow_heap: bool,
    /// Whether the standard library (effects, dataframe, tensor) is available.
    pub allow_stdlib: bool,
    /// Whether algebraic effects (`perform`) are allowed.
    pub allow_effects: bool,
    /// Whether the codegen should prefer aggressive loop vectorization.
    pub prefer_vectorization: bool,
    /// Whether thread-pool runtime services should be enabled.
    pub prefer_thread_pool: bool,
    /// Advisory maximum binary size (bytes). `None` = no constraint.
    pub max_binary_size_hint: Option<usize>,
}

impl TargetConstraints {
    /// Derive constraints from a target profile.
    pub fn from_profile(profile: TargetProfile) -> Self {
        match profile {
            TargetProfile::Default => Self {
                allow_heap: true,
                allow_stdlib: true,
                allow_effects: true,
                prefer_vectorization: false,
                prefer_thread_pool: false,
                max_binary_size_hint: None,
            },
            TargetProfile::Iot => Self {
                allow_heap: false,
                allow_stdlib: false,
                allow_effects: false,
                prefer_vectorization: false,
                prefer_thread_pool: false,
                max_binary_size_hint: Some(10 * 1024), // 10 KB
            },
            TargetProfile::Enterprise => Self {
                allow_heap: true,
                allow_stdlib: true,
                allow_effects: true,
                prefer_vectorization: false,
                prefer_thread_pool: true,
                max_binary_size_hint: None,
            },
            TargetProfile::Hpc => Self {
                allow_heap: true,
                allow_stdlib: true,
                allow_effects: true,
                prefer_vectorization: true,
                prefer_thread_pool: true,
                max_binary_size_hint: None,
            },
        }
    }
}

/// Errors that arise from target profile resolution or validation.
#[derive(Debug, Clone)]
pub enum TargetError {
    /// Multiple `@target.*` annotations found.
    MultipleTargets { first: String, second: String },
    /// Unknown target profile name.
    UnknownTarget(String),
    /// A feature was used that is prohibited by the active target.
    ProhibitedFeature {
        profile: TargetProfile,
        feature: String,
        reason: String,
    },
}

impl std::fmt::Display for TargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MultipleTargets { first, second } => {
                write!(
                    f,
                    "conflicting target directives: `@target.{first}` and `@target.{second}`; \
                     only one target profile may be active per function"
                )
            }
            Self::UnknownTarget(name) => {
                write!(
                    f,
                    "unknown target profile `@target.{name}`; \
                     expected `iot`, `enterprise`, or `hpc`"
                )
            }
            Self::ProhibitedFeature {
                profile,
                feature,
                reason,
            } => {
                write!(
                    f,
                    "`{feature}` is not allowed under `@target.{}`: {reason}",
                    profile.label()
                )
            }
        }
    }
}

/// Resolve the target profile from a set of annotations.
///
/// Returns `TargetProfile::Default` if no `@target.*` annotation is present.
/// Returns an error if multiple targets are declared or the name is unknown.
pub fn resolve_target_profile(annotations: &[Annotation]) -> Result<TargetProfile, TargetError> {
    let mut resolved: Option<(String, TargetProfile)> = None;

    for ann in annotations {
        let name = &ann.name.name;
        if let Some(target_name) = name.strip_prefix("target.") {
            let profile = match target_name {
                "iot" => TargetProfile::Iot,
                "enterprise" => TargetProfile::Enterprise,
                "hpc" => TargetProfile::Hpc,
                other => return Err(TargetError::UnknownTarget(other.to_string())),
            };

            if let Some((first, _)) = &resolved {
                return Err(TargetError::MultipleTargets {
                    first: first.clone(),
                    second: target_name.to_string(),
                });
            }
            resolved = Some((target_name.to_string(), profile));
        }
    }

    Ok(resolved.map(|(_, p)| p).unwrap_or(TargetProfile::Default))
}

/// Validate that an effect perform is allowed under the given target constraints.
pub fn validate_effect_for_target(
    constraints: &TargetConstraints,
    profile: TargetProfile,
    effect: &str,
    operation: &str,
) -> Result<(), TargetError> {
    if !constraints.allow_effects {
        return Err(TargetError::ProhibitedFeature {
            profile,
            feature: format!("perform {effect}.{operation}"),
            reason: "effects require runtime support not available on this target".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_errors::Span;

    fn ann(name: &str) -> Annotation {
        Annotation {
            name: agam_ast::Ident::new(name, Span::dummy()),
            args: vec![],
            span: Span::dummy(),
        }
    }

    #[test]
    fn default_when_no_target() {
        let result = resolve_target_profile(&[]);
        assert_eq!(result.unwrap(), TargetProfile::Default);
    }

    #[test]
    fn resolves_iot() {
        let result = resolve_target_profile(&[ann("target.iot")]);
        assert_eq!(result.unwrap(), TargetProfile::Iot);
    }

    #[test]
    fn resolves_enterprise() {
        let result = resolve_target_profile(&[ann("target.enterprise")]);
        assert_eq!(result.unwrap(), TargetProfile::Enterprise);
    }

    #[test]
    fn resolves_hpc() {
        let result = resolve_target_profile(&[ann("target.hpc")]);
        assert_eq!(result.unwrap(), TargetProfile::Hpc);
    }

    #[test]
    fn rejects_multiple_targets() {
        let result = resolve_target_profile(&[ann("target.iot"), ann("target.hpc")]);
        assert!(matches!(result, Err(TargetError::MultipleTargets { .. })));
    }

    #[test]
    fn rejects_unknown_target() {
        let result = resolve_target_profile(&[ann("target.potato")]);
        assert!(matches!(result, Err(TargetError::UnknownTarget(_))));
    }

    #[test]
    fn ignores_non_target_annotations() {
        let result = resolve_target_profile(&[ann("safe"), ann("gpu"), ann("target.hpc")]);
        assert_eq!(result.unwrap(), TargetProfile::Hpc);
    }

    #[test]
    fn iot_constraints_deny_heap_and_effects() {
        let c = TargetConstraints::from_profile(TargetProfile::Iot);
        assert!(!c.allow_heap);
        assert!(!c.allow_effects);
        assert!(!c.allow_stdlib);
        assert_eq!(c.max_binary_size_hint, Some(10 * 1024));
    }

    #[test]
    fn hpc_constraints_prefer_vectorization() {
        let c = TargetConstraints::from_profile(TargetProfile::Hpc);
        assert!(c.prefer_vectorization);
        assert!(c.prefer_thread_pool);
        assert!(c.allow_heap);
    }

    #[test]
    fn iot_rejects_effects() {
        let c = TargetConstraints::from_profile(TargetProfile::Iot);
        let err = validate_effect_for_target(&c, TargetProfile::Iot, "Console", "println");
        assert!(err.is_err());
    }

    #[test]
    fn default_allows_effects() {
        let c = TargetConstraints::from_profile(TargetProfile::Default);
        let ok = validate_effect_for_target(&c, TargetProfile::Default, "Console", "println");
        assert!(ok.is_ok());
    }
}
