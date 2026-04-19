# Project Overview

## What Agam Is

- Agam is a next-generation, multi-paradigm, natively compiled, memory-safe language and toolchain implemented in Rust.
- It aims to combine Python-level readability, Rust-like safety, and native-speed execution.
- AI, tensor, dataframe, autodiff, and numerical workflows are meant to be language-native rather than wrapper-heavy library surfaces.

## Vision

- Build one coherent omni-language that can span systems programming, automation, apps, AI/ML, and scientific computing without becoming a bag of disconnected features.

## Mission

- Cover embedded and bare-metal reliability
- shell and automation immediacy
- OS, driver, and IoT control
- cross-platform app and library development
- Windows, macOS, and iOS quality targets
- web and server productivity
- Android ergonomics
- MATLAB-class numerical and modeling workflows

## Architecture Summary

- Frontend: `agam_lexer`, `agam_parser`, `agam_ast`
- Semantics and lowering: `agam_sema`, `agam_hir`, `agam_mir`
- Backends and execution: `agam_codegen`, `agam_jit`, `agam_runtime`
- Tooling: `agam_driver`, `agam_fmt`, `agam_lsp`, `agam_test`, `agam_profile`
- Packaging and distribution: `agam_pkg`, portable package/runtime metadata, SDK packaging, future source-package and environment contracts, bundled LLVM contract, `agamc doctor`

## Current Product Direction

- Native LLVM on Windows, Linux, and Android is the top product priority.
- WSL is development-only convenience.
- macOS and iOS remain planned targets but not validation-complete product targets yet.

## Key Performance Direction

- Agam should compete with optimized `clang++` on proven native workloads.
- That performance must be earned through proofs, lowering quality, and runtime design, not undefined-behavior shortcuts.
