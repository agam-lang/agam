# Agent Documentation Test Area

## Purpose

1. This folder holds lightweight validation notes and future scripts for `.agent/policy/`, `.agent/phases/`, and `info.md`.
2. Its purpose is to keep agent-policy checks and planning-document checks in one place instead of scattering them across the repo.
3. Benchmark workspace sources, harnesses, CI helpers, and generated result roots now live under `benchmarks/`, while localized phase-work microbenchmarks remain under `.agent/test/`, so this folder can stay focused on lightweight policy/document validation.
4. The structured shared agent board now lives under `.agent/README.md`, `.agent/include/`, `.agent/rules/`, `.agent/commands/`, and `.agent/agents/`.
5. The canonical Antigravity-native skill shelf lives under `.agent/skills/`, with mirrors in `.claude/skills/`, `.gemini/skills/`, and `.codex/skills/`.
6. The compact phase-status board lives under `.agent/phases/`.

## Current Validation Scope

1. Confirm required rule headings and phase blocks exist.
2. Confirm README points automated contributors to the structured `.agent/` board.
3. Confirm `info.md` and `README.md` retain architecture and immediate-next-phase context.
4. Confirm future optimization phases reference `agam_profile` for benchmark validation.
5. Confirm the Native LLVM SDK distribution phase (`15H`) remains represented in planning docs.
6. Confirm the SDK packaging workflow and script remain present.
7. Confirm `agamc doctor` is represented in the project docs and planning docs.
8. Confirm `agamc new`, `agamc dev`, and `agamc cache status` remain represented in the project docs after the first Phase 15G workflow slice lands.
10. Confirm `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `ANTIGRAVITY.md`, `.claude/`, `.gemini/`, and `.codex/` stay aligned with `.agent/`.
11. Confirm the VS 2026 toolchain inventory stays represented in `.agent/include/toolchain-inventory.md`, `.agent/rules/toolchain-and-platform.md`, and `info.md`.
12. Confirm `.agent/phases/current.md`, `.agent/phases/next.md`, `.agent/phases/catalog.md`, and `.agent/phases/details/` stay aligned with `info.md` and the structured roadmap content.
13. Confirm `scripts/sync_agent_structure.py --check` passes, the generated JSON indexes stay current, and required phase-template sections remain present.
14. Confirm `benchmarks/README.md`, `benchmarks/METHODOLOGY.md`, `benchmarks/config/`, `benchmarks/infrastructure/`, `benchmarks/harness/`, `benchmarks/ci/`, and `benchmarks/tests/` remain present.

## Suggested WSL Checks

```bash
cd /mnt/c/Projects/agam
grep -n "Benchmarking Mandate\|Zero-Regression Tolerance" .agent/policy/operating-rules.md info.md
grep -n "AGENTS.md\|CLAUDE.md\|GEMINI.md\|ANTIGRAVITY.md\|\.agent/policy" README.md
test -f AGENTS.md && test -f CLAUDE.md && test -f GEMINI.md && test -f ANTIGRAVITY.md
test -d .claude/commands && test -d .claude/agents && test -d .claude/skills && test -d .gemini/commands && test -d .gemini/agents && test -d .gemini/skills && test -d .codex/prompts && test -d .codex/skills
test -f .agent/include/toolchain-inventory.md
test -f .agent/phases/current.md && test -f .agent/phases/next.md
test -f .agent/phases/catalog.md && test -d .agent/phases/details
test -d .agent/policy
test -f .agent/phases/details/16.md && test -f .agent/phases/details/28.md
python scripts/sync_agent_structure.py --check
grep -n "Phase 15D\|Phase 15G\|Phase 15H" .agent/phases/details/15D.md .agent/phases/details/15G.md .agent/phases/details/15H.md info.md
grep -n "agamc doctor\|Doctor" README.md info.md .agent/phases/details/15G.md .agent/phases/details/15H.md
grep -n "agamc new\|agamc dev\|agamc cache status" README.md info.md .agent/phases/details/15G.md
test -f .github/workflows/sdk-dist.yml && test -f scripts/package_sdk.py
test -f benchmarks/README.md && test -f benchmarks/METHODOLOGY.md
test -f .github/workflows/benchmarks.yml && test -f benchmarks/ci/gh_benchmark_cli.py
```
