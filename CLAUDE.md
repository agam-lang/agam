# Agam Claude Instructions

Treat `.agent/` as the canonical shared source of truth for this repo.
Use `AGENTS.md` for the concise universal entrypoint, then fall through to the structured docs below.

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

## Key Rules

- Agam is not Python and not Rust. Use `.agent/test/*.agam` and `benchmarks/benchmarks/**/*.agam` as syntax reality checks.
- Native LLVM parity on Windows, Linux, and Android is the main active program goal.
- Keep WSL as a development and verification environment, not the shipped backend path.
- Prefer the smallest responsible crate and preserve spans, diagnostics, and typed lowering.
- Benchmark optimization work before calling it complete.

## Tool-Specific Folders

- Project slash commands: `.claude/commands/`
- Project subagents: `.claude/agents/`
- Project skills: `.claude/skills/`
- Shared reusable guidance: `.agent/commands/`, `.agent/agents/`, `.agent/rules/`, `.agent/skills/`
