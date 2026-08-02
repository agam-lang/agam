# Phase T1-error-messages — Nyāya 4-Part Proof Diagnostic Engine & Hankel Root Solvers

## Phase Focus

Upgrading parser error recovery, visual source-span highlights, and constraint diagnostics into formal 4-part Nyāya proofs (*Fact, Reason, Fix, Law*) in `agam_errors`, powered by Hankel moment matrix root solvers.

## Key Capabilities

1. **Nyāya 4-Part Proof Diagnostic Schema**:
   - **Fact (Pratijñā)**: Precise multi-span code locus and observed error condition.
   - **Reason (Hetu)**: Formal type mismatch, effect violation, or borrow check failure.
   - **Fix (Udāharaṇa)**: Actionable, compiler-suggested code modification.
   - **Law (Nigamana)**: Governing language rule or specification constraint.

2. **Hankel Moment Matrix Root Solvers (`agam_errors::hankel`)**:
   - `solve_hankel_determinant(moments: &[FieldElement]) -> Option<Vec<TypeConstraint>>`: Uses Hankel matrix determinants ($\Delta_h(X) = \det(\mu_{i+j}(X))$) and Reed–Solomon moment constraints to uniquely reconstruct missing type constraints and generate exact mathematical proofs.

3. **Single-Pass Multi-Error Recovery**:
   - Resilient Pratt parser error recovery that collects multiple diagnostic proofs per compilation pass without cascading cascades.

## Verification Plan

- Diagnostic snapshot tests verifying 4-part Nyāya proof rendering on type, syntax, and effect errors.
- Hankel matrix solver unit tests for type inference error reconstruction.
