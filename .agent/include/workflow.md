# Workflow

## Core Operating Rules

- Work in the smallest responsible crate first. Avoid unnecessary workspace-wide edits.
- Keep Git staging and commits on Windows. Use WSL Ubuntu 24.04 LTS for Linux and LLVM-adjacent validation.
- Before finalizing backend or codegen work, run scoped `cargo fmt --check` and `cargo check` from WSL when possible.
- Route production failures through `agam_errors`; avoid `.unwrap()` and `.expect()` in compiler passes.
- Preserve `SourceId`, `Span`, and debug metadata through lowering and optimization.
- Run unsafe, FFI, executable-memory, and JIT validation in isolated subprocesses when practical.
- Optimization work is not complete until `agam_profile` or an equivalent localized benchmark records the delta.
- If compile time regresses materially or runtime gets slower, reject the change and rewrite it.

## Documentation Contract

- If you change CLI workflow, packaging, platform support, or agent workflow, update the relevant docs in `README.md`, `info.md`, and `.agent/`.
- Keep agent-facing guidance synchronized through `.agent/` instead of adding divergent tool-specific copies.
- Before closing a local implementation slice, record the shipped change in the relevant `.agent/phases/` files and create a scoped local commit that includes only the files for that slice.

## Delivery Style

- Agam is not a Python wrapper and not a Rust clone. Treat it as its own language with its own semantics.
- Prefer concise, structured writeups with concrete file ownership, verification notes, and remaining risks.
