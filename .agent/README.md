# `.agent/` — Agent Guidance Directory

> **Start at `CLAUDE.md` (repo root).** It contains the full self-contained briefing.
> This directory holds the structured backing material referenced from there.

## Directory Index

| Path | What It Contains | When To Read |
|------|-----------------|--------------|
| `phases/current.md` | Active workstreams and status | Deciding what to work on |
| `phases/next.md` | Prioritized build order | "What should I build next?" |
| `phases/catalog.md` | Full roadmap status (48 phases) | Need historical context |
| `phases/details/` | Per-phase checklists and remaining work | Executing a specific phase |
| `policy/package-ecosystem.md` | Package, registry, lockfile, environment architecture | Package/dependency work |
| `policy/project-overview.md` | Project vision and architecture summary | General orientation |
| `rules/language-guardrails.md` | Agam syntax and language-design rules | Touching `.agam` syntax |
| `rules/project-structure.md` | Crate and file layout conventions | Adding new code |
| `skills/benchmark-guard/` | Skill: benchmark-driven validation | Performance-sensitive changes |
| `skills/language-guard/` | Skill: Agam language correctness | Syntax/parser changes |
| `include/` | Legacy shared context (superseded by `CLAUDE.md`) | Rarely needed |
| `test/` | Localized phase-work benchmark sources | Active microbenchmarks |
| `tests/` | Policy validation notes | Verification |

## Phase Details Quick Reference

Active phases have their own detail files:

- `details/15F.md` — Incremental daemon, prewarm, parallel compilation
- `details/15G.md` — Premium experience layer (tooling unification)
- `details/15H.md` — Native LLVM SDK distribution
- `details/17A.md` — Workspace contract and dependency manifests

Future phases: `details/17B.md` through `details/17F.md` (package ecosystem)
