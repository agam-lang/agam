# Stream 0: Continuous Assurance & Debug Stream

## Overview

Stream 0 defines the mandatory verification, diagnostic testing, and git synchronization protocol triggered after **EVERY feature implementation** across Technical Tiers T0–T6.

## Post-Feature Execution Protocol

Every time a technical feature or fix is written in Stream 1:

1. **Compilation Check:**
   - Execute `cargo check --manifest-path agam/Cargo.toml` across all 27 workspace crates. Must pass with zero errors.

2. **Unit & Integration Verification:**
   - Execute test suites. Verify that all HIR, MIR, sema, parser, and codegen tests pass cleanly.

3. **Multi-Backend Equivalence Check:**
   - Ensure the feature behaves consistently across LLVM IR, C emitter, and JIT engine where applicable.

4. **Diagnostic Integrity:**
   - Verify that invalid inputs emit proper error diagnostics (`E0xxx`–`E4xxx`) rather than compiler panics.

5. **Artifact & Task Update:**
   - Update `task.md` (ticking completed items) and `walkthrough.md`.

6. **Git Commit & Push:**
   - Commit with structured message (`feat(...)`, `fix(...)`, `docs(...)`) and execute `git push origin main`.
