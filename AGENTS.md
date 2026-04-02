# Agam Agent Instructions

This file is the Codex entrypoint and the universal repo entrypoint for shared agent guidance.
The canonical shared source of truth lives under `.agent/`.

## Read First

- `.agent/README.md`
- `.agent/include/project-context.md`
- `.agent/include/workflow.md`
- `.agent/include/toolchain-inventory.md` when platform or SDK work is involved
- `.agent/phases/current.md`
- `.agent/phases/next.md`
- `.agent/phases/catalog.md` when you need full phase coverage
- `.agent/phases/details/` when you need merged roadmap detail for active phases
- `.agent/rules/language-guardrails.md`
- `.agent/rules/toolchain-and-platform.md`

## Non-Negotiables

- Agam is its own language. Do not treat `.agam` as Python or Rust shorthand.
- Native LLVM on Windows, Linux, and Android is the top active product direction.
- WSL is a development and verification fallback, not the shipped backend story.
- Work in the smallest responsible crate and avoid needless cross-crate churn.
- Route compiler failures through `agam_errors`; preserve spans and lowering traceability.
- Optimization work requires measured validation, not intuition.
- Before closing a local slice, update the relevant `.agent/phases/` record and create a scoped local commit containing only that slice's files.

## Repo Map

- `crates/`: compiler, runtime, tooling, and packaging crates
- `examples/`: runnable source examples
- `benchmarks/`: organized benchmark suites, harnesses, CI helpers, and generated result roots
- `.agent/test/`: localized phase-work benchmark sources and generated inspection artifacts
- `.agent/tests/`: lightweight policy and documentation validation notes
- `.agent/skills/`: Antigravity-facing project skills and the canonical skill shelf
- `.claude/`, `.gemini/`, `.codex/`: tool-specific commands, agents, skill mirrors, and local config helpers

## Multi-Agent Guidance

- Shared routing rules: `.agent/include/multi-agent.md`
- Shared role briefs: `.agent/agents/`
- Shared task briefs: `.agent/commands/`

## Long-Form Sources

- `.agent/policy/`
- `.agent/phases/`
- `README.md`
- `info.md`
