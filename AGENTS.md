# Agam Compiler — Agent Instructions

> Read `CLAUDE.md` for the complete self-contained project briefing.
> This file mirrors the same content for Codex and other agent surfaces.

---

## Quick Orientation

- **What:** Agam is a next-generation compiled language (Rust workspace, 26 crates)
- **Goal:** Native LLVM as first-class production backend for Windows, Linux, Android
- **Active work:** Phase 15F (incremental daemon), 15G (premium tooling), 15H (SDK distribution), 17A (manifest contract)
- **Build next:** See `.agent/phases/next.md`
- **CLI:** `agamc {build,run,check,new,dev,daemon,fmt,test,lsp,doctor,cache,package,repl}`

## Architecture

```
Source → agam_lexer → agam_parser → agam_ast → agam_sema → agam_hir → agam_mir → agam_codegen/agam_jit
```

Key crates: `agam_driver` (CLI, ~8900 lines), `agam_pkg` (manifest/workspace/packaging), `agam_runtime` (ABI/cache), `agam_errors` (diagnostics)

## Non-Negotiables

- Agam is its own language — not Python, not Rust. Check real `.agam` files for syntax.
- Work in the smallest responsible crate. Preserve spans and diagnostics.
- Route failures through `agam_errors`; no `.unwrap()` in compiler passes.
- Optimization requires measured benchmarks, not intuition.
- After each slice, update `.agent/phases/` and commit only that slice's files.

## Repo Map

| Path | Purpose |
|------|---------|
| `crates/` | All 26 compiler, runtime, and tooling crates |
| `examples/` | Runnable `.agam` examples |
| `benchmarks/` | Benchmark suites and harnesses |
| `.agent/phases/` | Active phase status, build order, per-phase checklists |
| `.agent/policy/` | Package ecosystem architecture, project overview |
| `.agent/rules/` | Language guardrails, structure rules |
| `.agent/skills/` | `benchmark-guard`, `language-guard` |
| `CLAUDE.md` | **Full self-contained briefing** (read this) |

## Deep Dives

- Phase checklists: `.agent/phases/details/{15F,15G,15H,17A,...}.md`
- Package/registry architecture: `.agent/policy/package-ecosystem.md`
- Build priority order: `.agent/phases/next.md`
- Full phase catalog: `.agent/phases/catalog.md`
