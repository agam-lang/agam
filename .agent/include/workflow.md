# Workflow

> Full rules are now in `CLAUDE.md` § 6 (Rules). This file exists for backward compatibility.

## Core Rules

- Smallest responsible crate first
- Route failures through `agam_errors`; preserve spans and lowering traceability
- No `.unwrap()` / `.expect()` in compiler passes
- Prefer optimal time/space complexity; justify tradeoffs
- Optimization requires measured benchmarks

## Process

- After completing a slice → update `.agent/phases/` records
- If CLI/packaging/platform changes → update `README.md`, `info.md`, `.agent/`
- Scoped commits: only include the files for the completed slice

## Build Verification

```powershell
cargo check --manifest-path agam/Cargo.toml
cargo test --manifest-path agam/Cargo.toml
```
