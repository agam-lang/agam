# Agam-Lang Organization — Agent Board

> This is the **organization-level** workspace. The core compiler lives in `agam/`.
> For the full project briefing, read **`agam/CLAUDE.md`**.

## Organization Structure

| Repository | Purpose |
|------------|---------|
| `agam/` | **Core compiler**, runtime, tooling, packaging — primary roadmap |
| `std/` | Standard library |
| `agamlab/` | Scientific computing platform |
| `agam-vscode/`, `agam-intellij/` | Editor integrations |
| `benchmarks/` | Standalone benchmarks |
| `governance/`, `rfcs/`, `registry-index/`, `sdk-packs/` | Ecosystem infrastructure |
| `examples/`, `playground/`, `agam-lang.github.io/` | Product surfaces |

## What Lives Here (`.agent/`)

| Path | Purpose |
|------|---------|
| `phases/current.md` | Active workstreams (mirrors `agam/.agent/phases/current.md`) |
| `phases/next.md` | Build priority order |
| `phases/details/` | Per-phase implementation checklists |
| `policy/` | Package ecosystem architecture |
| `rules/` | Shared guardrails |
| `skills/` | `caveman`, `caveman-compress`, `graphify`, `benchmark-guard`, `language-guard` |
| `include/` | Organization context and workflow |

## Rules

- Organization-wide work follows the compiler's active implementation order
- Shared guidance lives here; repo-specific guidance lives in each repo's `.agent/`
- When both exist, keep them in sync
- For repo-specific work, switch to that repo's `AGENTS.md` and `.agent/`
