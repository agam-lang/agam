# DevOps Base

This directory is the canonical home for Agam's operational automation and runbooks.

## Structure

- `scripts/`
  - executable automation entrypoints owned by DevOps concerns
- `runbooks/`
  - human-facing operating guides for toolchains, release flows, and platform setup
- `ci/`
  - maps repo workflows to the scripts and contracts they depend on

## Canonical Entry Points

- `justfile`
  - human-friendly front door for the common local DevOps and CI-adjacent tasks
- `devops/scripts/package_sdk.py`
  - build, package, archive, and validate an SDK layout
- `devops/scripts/validate_sdk_e2e.py`
  - local end-to-end validation mirror for the SDK distribution workflow
- `devops/scripts/vs2026-dev.ps1`
  - Visual Studio Community 2026 bootstrap, validation, and local inner-loop tasks
- `devops/scripts/invoke-python.ps1`
  - resolves the pinned Python baseline across `python`, `py`, and local uv-managed installs

## Local Contracts

- `rust-toolchain.toml`
  - pinned Rust baseline for local and CI-side compiler work
- `.python-version`
  - pinned Python baseline for packaging, benchmark, and release scripts
- `.editorconfig` and `.gitattributes`
  - normalized whitespace and line-ending behavior across Windows and CI
- `scratch/`
  - local-only workspace for experiments and temporary output that should not live beside source

## Compatibility Policy

The root `scripts/` directory now contains thin wrappers for backward compatibility with existing
docs, local muscle memory, and GitHub workflows. New docs and new automation should point at the
`devops/` paths first.

GitHub workflow files remain under `.github/workflows/` because GitHub Actions requires that
location, but their operational documentation belongs under `devops/ci/`.
