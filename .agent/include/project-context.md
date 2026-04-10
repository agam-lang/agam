# Project Context

> Most of this is now in the root `CLAUDE.md`. This file exists for backward compatibility.

## Architecture

- Frontend: `agam_lexer` → `agam_parser` → `agam_ast`
- Semantics: `agam_sema` (resolver + type checker)
- Lowering: `agam_hir` → `agam_mir` (with optimizer)
- Backends: `agam_codegen` (C/LLVM IR), `agam_jit` (Cranelift)
- Runtime: `agam_runtime` (ABI, cache, host detection)
- Tooling: `agam_driver`, `agam_fmt`, `agam_lsp`, `agam_test`, `agam_profile`
- Packaging: `agam_pkg` (manifest, workspace, snapshot, portable packages, SDK)

## Current Priority

Native LLVM production backend on Windows, Linux, and Android.

## Full Briefing

See `CLAUDE.md` at the repo root.
