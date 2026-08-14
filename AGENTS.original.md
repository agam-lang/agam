# Agam Compiler — Agent Instructions

> Read `CLAUDE.md` for the complete self-contained project briefing.
> This file mirrors the same content for Codex and other agent surfaces.

---

## Quick Orientation

- **What:** Agam is a next-generation compiled language (Rust workspace, 26 crates)
- **Goal:** Native LLVM as first-class production backend for Windows, Linux, Android
- **Active work:** Phase 23 (Elite GPU/NPU Capabilities: Auto-tuning, Inline PTX, Warp Primitives, Device-Local Effects)
- **Build next:** See `.agent/phases/next.md`
- **CLI:** `agamc {build,run,check,lock,new,dev,daemon,fmt,test,lsp,repl,exec,doctor,env,publish,registry,cache status,package {pack,inspect,run,sdk}}`

## Architecture

```text
Source → agam_lexer → agam_parser → agam_ast → agam_sema → agam_hir → agam_mir → agam_codegen/agam_jit
```

Key crates: `agam_driver` (CLI, daemon, REPL/headless execution, `exec` tool), `agam_pkg` (manifest/workspace/packaging), `agam_runtime` (ABI/cache), `agam_errors` (diagnostics)

Physical layout: `crates/{core,middle,backends,runtime,tooling,experiments}/...` plus
`integrations/python` for the external Python package surface.

## Non-Negotiables

- Agam is its own language — not Python, not Rust. Check real `.agam` files for syntax.
- Work in the smallest responsible crate. Preserve spans and diagnostics.
- Route failures through `agam_errors`; no `.unwrap()` in compiler passes.
- Optimization requires measured benchmarks, not intuition.
- When making architectural or syntax decisions, log ergonomics vs. security tradeoffs in `.agent/wiki/ergonomics-and-security.md`. Ensure strict security is always the highest priority while pushing for `@lang.base` usability.
- Focus on closing gaps in ecosystem parity (packages, LSP, garbage collection, and error diagnostics) as tracked in `.agent/wiki/future-ecosystem-and-tooling.md`.
- After major changes, commit locally. After a final milestone or substantial batch of changes, commit and push to GitHub.

## Repo Map

| Path | Purpose |
|------|---------|
| `crates/` | Layered Rust workspace grouped into `core/`, `middle/`, `backends/`, `runtime/`, `tooling/`, and `experiments/` |
| `integrations/` | External integration packages outside the Rust workspace |
| `fixtures/` | Smoke fixtures and generated examples moved out of the repo root |
| `devops/` | Canonical automation, CI mapping, and runbooks |
| `docs/architecture/` | Canonical engineering brief and repo structure notes |
| `justfile` | Root task runner for common local DevOps workflows |
| `examples/` | Runnable `.agam` examples |
| `benchmarks/` | Benchmark suites and harnesses |
| `.agent/phases/` | Active phase status, build order, per-phase checklists |
| `.agent/policy/` | Package ecosystem architecture, project overview |
| `.agent/rules/` | Language guardrails, structure rules |
| `.agent/skills/` | `caveman`, `caveman-compress`, `cargo-lens`, `spec-archiver`, `benchmark-guard`, `language-guard` |
| `.agent/wiki/` | LLM Second Brain for architectural synthesis |
| `.agent/evals/` | Verification templates for Evaluation-Driven Development |
| `CLAUDE.md` | **Full self-contained briefing** (read this) |

## Active Skills

| Skill | Purpose | Trigger |
| --- | --- | --- |
| `caveman` | ~75% output token reduction — **ALWAYS ON** | Auto-active / `/caveman` |
| `caveman-compress` | ~46% input token reduction on context files | `/caveman:compress <file>` |
| `cargo-lens` | Extract compiler errors without dumping context | Auto on build failures |
| `spec-archiver` | Safely archive completed specs | On spec completion |
| `benchmark-guard` | Benchmark-driven validation for perf claims | Auto on optimization work |
| `language-guard` | Prevent treating `.agam` as Python/Rust | Auto on syntax work |

## External Integrations

- `claude-mem` — persistent memory layer. Use progressive disclosure before rereading old notes.
- `graphify-out/GRAPH_REPORT.md` — cheaper than raw-file grep for architecture questions
- `graphify-out/graph.json` and `graphify-out/cache/` — generated artifacts, not durable review surfaces
- Codex uses `$caveman` / `$graphify` syntax (not `/`)

## Deep Dives

- Phase checklists: `.agent/phases/details/{15F,15G,15H,16,17A,17B,...}.md`
- Package/registry architecture: `.agent/policy/package-ecosystem.md`
- Build priority order: `.agent/phases/next.md`
- Full phase catalog: `.agent/phases/catalog.md`
