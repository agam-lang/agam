---
name: compiler-harden
description: Automate comprehensive workspace audits, latent bug discovery, integration test expansion, zero-warning linter cleanup, and verified remote synchronization across all 27 crates.
---

# compiler-harden

**Purpose**: Execute end-to-end compiler audit sweeps, identify latent parsing/sema/HIR/MIR/JIT/codegen bugs, create new integration tests in `agam_test`, achieve zero clippy warnings across all crates, and synchronize to remote.

## Trigger Words
- `/harden`
- `/audit-sweep`
- `harden`
- `audit workspace`

## Workflow Protocol

When triggered with `/harden` or `/audit-sweep`, the agent must execute the following 5-step cycle:

1. **Full Workspace Static Analysis**:
   ```powershell
   cargo clippy --all-targets --manifest-path agam\Cargo.toml -- -D warnings
   ```
   Fix all collapsible matches, redundant type conversions, arithmetic divisions lacking `.checked_div()`, and lint warnings across all 27 crates.

2. **Feature Coverage & Edge Case Probing**:
   - Write new end-to-end test cases in `crates/tooling/agam_test/src/lib.rs` targeting real language features (e.g. pattern matching, closures, recursion, structs, enums with payloads, generics, boolean logic).

3. **Compiler Bug Identification & Fix**:
   - Trace any failures through the compiler pipeline:
     - Lexer / Parser (`agam_parser`)
     - Sema / Types (`agam_sema`)
     - HIR Lowering (`agam_hir`)
     - MIR Lowering / CFG (`agam_mir`)
     - JIT Backend / Cranelift DFG (`agam_jit`)
     - Native LLVM Codegen (`agam_codegen`)
   - Apply robust, literature-backed fixes with zero workarounds.

4. **Full Workspace Verification**:
   ```powershell
   cargo test --manifest-path agam\Cargo.toml
   cargo clippy --all-targets --manifest-path agam\Cargo.toml -- -D warnings
   cargo fmt --all --manifest-path agam\Cargo.toml -- --check
   ```

5. **Commit & Remote Sync**:
   - Commit with structured semantic message: `fix(...): ...` or `test(...): ...`.
   - Run `python push_repos.py` to push to `origin/main`.
