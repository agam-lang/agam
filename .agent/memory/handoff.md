# Multi-AI Handoff Contract

## Current State

- **Last committed T0 subtask:** `60debac` — enum and pattern-match lowering across HIR and MIR.
- **Active technical tier:** `T0-type-system`.
- **Current uncommitted changes:** local bindings now shadow same-named enum variants during HIR lowering; C code generation now emits an `AgamEnum` tagged union for enum construction, tag extraction, and scalar payload slots.
- **Verification:** `cargo check --workspace`, `cargo test -p agam_hir`, `cargo test -p agam_codegen --no-run`, and file-level Rust formatting checks pass. Windows Application Control blocks execution of the newly built `agam_codegen` test binary (`os error 4551`).
- **Environment:** `graphify update .` is unavailable because the `graphify` executable is not installed. Existing benchmark-result files are unrelated untracked artifacts and must be preserved.

## Next Pending Action

1. Implement the matching LLVM aggregate representation for enum values. C now has a concrete tagged-union layout, but LLVM still needs one before it can lower `EnumConstruct`, `EnumTag`, and `EnumPayload` correctly.
2. Lower struct literals into a concrete aggregate MIR operation rather than the current layout-only `Unit` placeholder.
3. Run Stream 0 verification and update the graph once graphify is available; commit only the intentional T0 source and handoff changes.
