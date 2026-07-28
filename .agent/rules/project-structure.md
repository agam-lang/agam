# Project Structure Rules

- Keep compiler logic in the smallest responsible crate under `crates/`.
- Put runnable examples under `examples/`.
- Keep the organized benchmark workspace under `benchmarks/`, not scattered across the workspace root.
- Keep localized phase-work microbenchmarks and generated inspection artifacts under `.agent/test/`.
- Keep lightweight policy and documentation validation material under `.agent/tests/`.
- Keep shared agent guidance under `.agent/`; root `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md` are entrypoints, not competing sources of truth.
- Add new agent commands under `.claude/commands/` and `.gemini/commands/` when they map to a repeatable workflow.
- Add new multi-agent role definitions under `.agent/agents/` first, then mirror them into tool-specific directories only if the tool can consume them directly.
