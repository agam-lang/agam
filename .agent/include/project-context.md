# Project Context

## What Agam Is

- Agam is a Rust workspace for a next-generation compiled language and toolchain.
- The project goal is an omni-language that combines Python-level readability, Rust-like safety, and native-speed execution.
- AI and numerical workflows are first-class language concerns, not bolt-on library stories.

## Current Highest Priority

- Make the native LLVM backend production-grade on Windows, Linux, and Android.
- Keep `agamc doctor`, SDK packaging, and native toolchain discovery aligned around one supportable contract.
- Keep the performance target pinned to optimized `clang++`-class output on Agam's proven native workloads.
- Use Visual Studio Community 2026 as the canonical Windows-side host toolchain inventory for desktop C++, Android/iOS mobile C++, Linux/Mac cross-workflows, Unity, Unreal, HLSL, and related platform tooling.

## Architecture Map

- Frontend: `agam_lexer`, `agam_parser`, `agam_ast`
- Semantics and lowering: `agam_sema`, `agam_hir`, `agam_mir`
- Execution: `agam_codegen`, `agam_jit`, `agam_runtime`
- Tooling: `agam_driver`, `agam_fmt`, `agam_lsp`, `agam_test`, `agam_profile`
- Packaging and distribution: `agam_pkg`, portable package/runtime metadata, SDK packaging, future source-package and environment contracts, `scripts/package_sdk.py`, `.github/workflows/sdk-dist.yml`

## Long-Form Source Docs

- `README.md`: public-facing project walkthrough and current backend direction.
- `info.md`: condensed architecture, platform goals, and immediate next phases.
- `.agent/policy/operating-rules.md`: structured execution and verification rules.
- `.agent/policy/project-overview.md`: structured project and architecture context.
- `.agent/policy/language-design.md`: language-design and platform-design notes.
- `.agent/policy/package-ecosystem.md`: canonical package, registry, lockfile, environment, and first-party distribution architecture.
- `.agent/include/toolchain-inventory.md`: concise VS 2026 workload and platform inventory.
- `.agent/phases/catalog.md`: compact phase coverage and status.
- `.agent/phases/details/`: structured per-phase implementation detail.
