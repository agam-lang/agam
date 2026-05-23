# Agam Info

This file is now the short index for the core engineering docs instead of a second full project
brief.

## Current Shape

- The Rust workspace is organized by layer under `crates/core`, `crates/middle`,
  `crates/backends`, `crates/runtime`, `crates/tooling`, and `crates/experiments`.
- External integration packages live outside the Rust workspace under `integrations/`.
  The Python-facing `agam-ffi` package now lives in `integrations/python`.
- DevOps automation is canonical under `devops/`, with root `scripts/` kept only as
  compatibility shims and the root `justfile` as the operator-friendly task surface.
- The local development baseline is pinned through `rust-toolchain.toml`, `.python-version`,
  `.editorconfig`, and `.gitattributes` so Windows workstations and CI stay closer to the same contract.
- Root clutter and generated smoke artifacts now live under `fixtures/c-backend-smoke`.
- Local-only experiments and temporary output now belong under `scratch/`.

## Canonical Documents

- [`README.md`](./README.md)
  - public overview, workflows, benchmark story, and user-facing compiler context
- [`CONTRIBUTING.md`](./CONTRIBUTING.md)
  - contributor workflow, verification, and change hygiene
- [`ROADMAP.md`](./ROADMAP.md)
  - public project priorities and near-term direction
- [`GOVERNANCE.md`](./GOVERNANCE.md)
  - maintainer-led decision model for project changes
- [`SECURITY.md`](./SECURITY.md) and [`SUPPORT.md`](./SUPPORT.md)
  - security reporting and support routing
- [`docs/architecture/project-brief.md`](./docs/architecture/project-brief.md)
  - canonical engineering brief for the compiler, repo layout, and platform/toolchain strategy
- [`docs/architecture/decisions/`](./docs/architecture/decisions/)
  - architecture decision records for cross-cutting technical choices
- [`docs/README.md`](./docs/README.md)
  - docs ownership map
- [`devops/README.md`](./devops/README.md)
  - canonical automation, CI, and runbook entrypoint
- [`devops/runbooks/releases/release-readiness.md`](./devops/runbooks/releases/release-readiness.md)
  - release checklist for SDKs, docs, and packaging changes
- [`AGENTS.md`](./AGENTS.md) and [`CLAUDE.md`](./CLAUDE.md)
  - agent entrypoints for repo-local engineering work

## Immediate Direction

- Native LLVM remains the primary production backend for Windows, Linux, and Android.
- Visual Studio Community 2026 remains the canonical Windows host toolchain inventory.
- Current near-term work stays focused on the remaining GPU/NPU gaps (rich kernel
  memory types and broader NPU breadth) plus ecosystem integration
  after the completed packaging, daemon, REPL, runtime-hardening, and host-device
  GPU transfer slices.
