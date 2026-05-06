# Agam Project Brief

## Summary

Agam is a compiled language and toolchain implemented as a Rust workspace. The active delivery goal
is a supportable native LLVM story for Windows, Linux, and Android, with a Cranelift JIT for fast
local execution and a first-party packaging/runtime/tooling surface around the compiler.

## Compiler Stack

```text
Source -> Lexer -> Parser -> AST -> Sema -> HIR -> MIR -> Codegen/JIT -> Runtime
```

- `agam_errors`
  - span, label, and diagnostic infrastructure
- `agam_lexer`, `agam_parser`, `agam_ast`
  - source tokenization, parsing, and syntax representation
- `agam_sema`, `agam_hir`, `agam_mir`
  - semantic analysis, typed lowering, and optimization handoff
- `agam_codegen`, `agam_jit`
  - LLVM/C code generation and in-memory execution, including the current NVPTX
    kernel pipeline plus host-side GPU buffer transfer lowering
- `agam_runtime`, `agam_std`
  - runtime helpers, sandboxing, ARC/SIMD support, and standard library surfaces
- `agam_driver`, `agam_pkg`, `agam_profile`, `agam_fmt`, `agam_lsp`, `agam_test`
  - CLI, packaging, profiling, formatting, language tooling, and validation

## Repository Layout

```text
crates/
  core/         diagnostics, lexer, parser, AST
  middle/       sema, HIR, MIR
  backends/     codegen and JIT
  runtime/      runtime and stdlib
  tooling/      CLI, packaging, fmt, LSP, tests, profiling
  experiments/  FFI, notebook, macro, SMT, UI, game
integrations/
  python/       external Python package over `agamc exec --json`
devops/         canonical automation, CI mapping, and runbooks
fixtures/       smoke fixtures and generated examples kept out of the root
scratch/        local developer-only experiments and temporary output
docs/           public and engineering docs
scripts/        compatibility shims to canonical devops scripts
justfile        human-friendly task surface for common local workflows
```

## Local Environment Contract

- `rust-toolchain.toml`
  - pins the Rust toolchain baseline plus `clippy` and `rustfmt`
- `.python-version`
  - pins the Python baseline used by packaging and benchmark scripts
- `.editorconfig` and `.gitattributes`
  - keep formatting and line-ending behavior stable across Windows and CI
- `justfile`
  - is the human-facing front door for the normal local inner loop
- `graphify-out/GRAPH_REPORT.md`
  - is the only graph artifact expected to stay reviewable; graph JSON/cache files are generated on demand

## Operations Surface

- `devops/scripts/package_sdk.py`
  - canonical SDK packaging and archive validation entrypoint
- `devops/scripts/validate_sdk_e2e.py`
  - local mirror of the SDK distribution validation flow
- `devops/scripts/vs2026-dev.ps1`
  - Visual Studio Community 2026 bootstrap, toolchain import, and Windows local task surface
- `justfile`
  - common local commands like `doctor`, `vs-status`, `sdk-package`, `sdk-validate`, and
    `ci-local`

## Platform Strategy

- Windows, Linux, and Android are the active native LLVM targets.
- Visual Studio Community 2026 is the canonical Windows host inventory.
- WSL remains a development and verification environment, not the shipped Windows backend story.
- macOS and iOS remain planned targets, not validation-complete product targets.

## Integration and Packaging Notes

- The Rust-side agent/headless execution surface lives in `agam_notebook` and `agam_ffi`.
- The external Python package lives in `integrations/python` and wraps the same
  `agamc exec --json` contract with Python-native request/response helpers and optional
  LangChain/LlamaIndex adapters.
- SDK distribution, release archives, and hosted-runner validation are owned by the `agam_pkg`
  contract plus the `devops/` automation layer.
