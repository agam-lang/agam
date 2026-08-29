//! Nyāya-grounded structured diagnostic and error models for the Agam GUI engine.

use agam_errors::{Diagnostic, NyayaProof};

/// A specialized result type for Agam GUI operations.
pub type GuiResult<T> = Result<T, GuiError>;

/// A formal Nyāya-grounded error produced by the Agam GUI engine.
///
/// Under the Agam facade boundary invariant, no raw third-party errors
/// (e.g. from `winit`, `wgpu`, `vello`, `cosmic-text`) are ever exposed directly to scripts.
/// Every boundary failure is converted into a 4-part Nyāya proof:
/// 1. `fact` (Pratijñā): What empirical failure occurred at the GUI boundary.
/// 2. `reason` (Hetu): Why this violates GUI runtime invariants.
/// 3. `fix` (Udāharaṇa): Actionable resolution for the application developer.
/// 4. `law` (Nigamana): The governing architectural invariant or spec rule.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("GUI Error: {fact}\n  Reason: {reason}\n  Fix: {fix:?}\n  Law: {law}")]
pub struct GuiError {
    /// Fact / Pratijñā: Empirical condition observed at the GUI boundary.
    pub fact: String,
    /// Reason / Hetu: Formal invariant violation.
    pub reason: String,
    /// Fix / Udāharaṇa: Actionable remedy or configuration fix.
    pub fix: Option<String>,
    /// Law / Nigamana: Governing language/runtime invariant.
    pub law: String,
}

impl GuiError {
    /// Construct a new structured `GuiError`.
    pub fn new(
        fact: impl Into<String>,
        reason: impl Into<String>,
        fix: Option<impl Into<String>>,
        law: impl Into<String>,
    ) -> Self {
        Self {
            fact: fact.into(),
            reason: reason.into(),
            fix: fix.map(Into::into),
            law: law.into(),
        }
    }

    /// Convert into a formal `NyayaProof`.
    pub fn to_proof(&self) -> NyayaProof {
        NyayaProof::new(
            self.fact.clone(),
            self.reason.clone(),
            self.fix.clone(),
            self.law.clone(),
        )
    }

    /// Convert into an `agam_errors::Diagnostic`.
    pub fn to_diagnostic(&self, code: &'static str) -> Diagnostic {
        let mut diag = Diagnostic::error(code, &self.fact).with_proof(self.to_proof());
        if let Some(ref fix) = self.fix {
            diag = diag.with_help(fix);
        }
        diag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gui_error_to_proof_and_diagnostic() {
        let err = GuiError::new(
            "Failed to acquire GPU surface texture",
            "Surface was lost during window resize transaction",
            Some("Re-create surface texture on next frame request"),
            "RFC-gui-engine §1: GPU device-loss must trigger automatic surface re-creation",
        );

        let proof = err.to_proof();
        assert_eq!(proof.fact, "Failed to acquire GPU surface texture");
        assert_eq!(
            proof.law,
            "RFC-gui-engine §1: GPU device-loss must trigger automatic surface re-creation"
        );

        let diag = err.to_diagnostic("E_GUI_SURFACE_LOST");
        assert!(diag.is_error());
        assert_eq!(diag.proof.as_ref().unwrap().fact, proof.fact);
    }
}
