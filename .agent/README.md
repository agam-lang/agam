# Agam Agent Board

This directory is the shared source of truth for agent-facing project guidance.
Use it as the canonical layer behind `AGENTS.md`, `CLAUDE.md`, and `GEMINI.md`.

## Layout

- `include/`: compact shared context that all agents should read first.
- `rules/`: repo guardrails for structure, style, testing, toolchains, and language semantics.
- `commands/`: reusable task briefs for common agent workflows.
- `agents/`: role briefs for multi-agent delegation.
- `policy/`: structured operating rules, project overview, and language-design notes.
- `phases/`: compact phase-status board for what is active, complete, and next.
- `skills/`: repeatable Agam-specific workflows.
- `research/`: supporting research notes that informed the board structure.
- `phases/index.json`: machine-readable phase index generated from the structured phase files.
- `skills/index.json`: machine-readable skill index generated from the canonical skill shelf.
- `test/`: localized benchmark sources and generated inspection artifacts for active phase work.
- `tests/`: lightweight policy and documentation validation notes.
- root `benchmarks/`: organized benchmark suites, harnesses, CI helpers, methodology, and generated result roots.

## Start Order

1. Read the root tool entrypoint for your client: `AGENTS.md`, `CLAUDE.md`, or `GEMINI.md`.
2. Read `include/project-context.md` and `include/workflow.md`.
3. Read `include/toolchain-inventory.md` when platform, SDK, LLVM, game, or graphics work is involved.
4. Read `phases/current.md` and `phases/next.md` when deciding what to build next.
5. Read `phases/catalog.md` and `phases/details/` when you need full structured roadmap detail.
6. Read `policy/` for structured operating rules and project context.
7. Read the relevant files under `rules/`.
8. Use `commands/`, `agents/`, and `skills/` when routing work across multiple agents.
9. Fall back to `README.md` and `info.md` for public-facing context.

## Sync Workflow

- Canonical structured sources:
  - `policy/`
  - `phases/`
  - `skills/`
- Regenerate indexes and mirrored skill shelves with:
  - `python scripts/sync_agent_structure.py`
- Verify there is no drift with:
  - `python scripts/sync_agent_structure.py --check`
- `phases/index.json` now exposes status notes, motive, goal, shipped work, planned work, next-build guidance, dependencies, and agent-interpretation notes.
- `python scripts/sync_agent_structure.py --check` also validates that phase detail files keep the required structured sections.

## Current Program Focus

- Native LLVM is the first-class backend direction on Windows, Linux, and Android.
- WSL is a development and verification environment, not the shipped product path.
- Benchmark-driven optimization is mandatory. Reject regressions instead of rationalizing them.
- Preserve Agam semantics through the full pipeline instead of chasing C/C++ undefined-behavior shortcuts.

## Skill Shelves

- Antigravity-native project skills: `.agent/skills/`
- Claude-native mirrors: `.claude/skills/`
- Gemini-native mirrors: `.gemini/skills/`
- Codex local-home mirrors: `.codex/skills/`
