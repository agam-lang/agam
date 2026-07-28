# Agam-Lang Organization — Agent Board

> Core compiler → `agam/CLAUDE.md`. This board = organization-level coordination.

## Organization (22 repos)

| Category | Repositories |
|----------|-------------|
| **Core** | `agam/` — compiler, runtime, tooling, packaging |
| **Libraries** | `agam-http`, `agam-json`, `agam-crypto`, `agam-ml`, `agam-db`, `agam-web`, `agam-async`, `agam-cli` |
| **Learning** | `agam-book`, `agam-by-example`, `examples` |
| **IDE** | `agam-vscode`, `agam-intellij` |
| **Infra** | `benchmarks`, `agamlab`, `sdk-packs`, `registry-index`, `rfcs` |
| **Web** | `agam-lang.github.io` |
| **Community** | `awesome-agam`, `.github` |
| **Archived** | `governance` (→ `agam/GOVERNANCE.md`), `std` (→ `agam/crates/runtime/agam_std`), `playground` (→ `agamlab`) |

## This Directory

| Path | Purpose |
|------|---------| 
| `specs/active/current.md` | Active workstreams and Dual-Track Parallel Framework |
| `specs/active/next.md` | Execution priority order (Prime Tier 0 + Prime Tier 1) |
| `specs/active/catalog.md` | Master 88-phase blueprint catalog |
| `specs/active/details/` | Per-phase specification checklists & plans |
| `memory/execution.log` | Timestamped execution log of feature completions & git pushes |
| `memory/handoff.md` | Loss-free multi-AI agent context transfer contract |
| `policy/` | Package ecosystem architecture, diagnostics taxonomy, backend matrix |
| `rules/` | Trigger keywords (`"Start Debug"`), workflow streams, token efficiency |
| `skills/` | caveman, graphify, benchmark-guard, language-guard |
| `wiki/` | Architecture deep-dives, dependency maps, and design records |

## Dual-Track Parallel Framework

- **Prime Tier 0 (Test, Debug & Quality Infrastructure):** Continuous test creation, multi-span diagnostic analysis, edge-case debugging, SSA invariant checks, and cross-backend equivalence (LLVM / C / JIT).
- **Prime Tier 1 (Architecture & System Expansion):** MiniTriton-inspired Block/Tile-IR GPU engine, MCP server (`agamc mcp serve`), hardware Tensor Core lowering, and language surface evolution.
