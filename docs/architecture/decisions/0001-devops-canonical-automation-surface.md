# ADR 0001: DevOps Is the Canonical Automation Surface

- Status: accepted
- Date: 2026-04-24
- Supersedes:
- Superseded by:

## Context

The repository had multiple overlapping command surfaces, root-level script clutter, and workstation
setup knowledge spread across ad hoc docs. That makes onboarding slower, increases drift between
contributors, and makes CI/release automation harder to reason about.

## Decision

Agam standardizes on:

- `devops/` as the canonical automation and runbook surface
- root `scripts/` as compatibility shims only
- `justfile` as the human-oriented local task entrypoint
- checked-in toolchain/editor contract files at repo root

## Consequences

- new automation should be added under `devops/`, not as new root scripts
- docs should point contributors toward `just` recipes and `devops/runbooks/`
- compatibility wrappers can remain temporarily, but they should not become the primary surface
- workstation drift is reduced because Rust, Python, line-ending, and editor expectations are now
  explicit repo contracts

## Notes

See `README.md`, `devops/README.md`, `devops/runbooks/windows/visual-studio-2026.md`, and
`justfile`.
