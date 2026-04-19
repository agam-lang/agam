# Agam-Lang Organization — Agent Instructions

> **For the core compiler**, read `agam/CLAUDE.md` — it's a complete self-contained briefing.
> This root workspace is for organization-level cross-repo coordination.

## 🤖 Unified Multi-AI Workflow

**Whether you are Gemini, Claude, Codex, or any other LLM**, you are participating in a continuous, unified multi-agent development session. The human developer rotates between AI assistants due to usage limits.

**Rules for all AI Agents:**
1. **Seamless Handoff:** You are treating the project exactly where the previous AI left off.
2. **Single Blueprint:** Read the same phase files, track work in the same artifacts (like `task.md`), and follow the same `AGENTS.md` / `CLAUDE.md` rules. Do not invent your own workflow.
3. **Consistent Output:** Produce the same high-quality, verified code and walkthroughs as your peers.

## Quick Start

1. **Compiler work?** → Go to `agam/CLAUDE.md`
2. **Organization-wide planning?** → Read `.agent/README.md`
3. **What to build next?** → See `.agent/phases/next.md`
4. **Repo-specific work?** → Switch to that repo's `AGENTS.md` and `.agent/`

## Active Skills

| Skill | Purpose | Trigger |
|-------|---------|---------|
| `caveman` | ~75% output token reduction via terse communication | Auto-active / `/caveman` |
| `caveman-compress` | ~46% input token reduction on context files | `/caveman:compress <file>` |
| `graphify` | Codebase → knowledge graph for architecture nav | `/graphify [path]` |
| `benchmark-guard` | Benchmark-driven validation for perf claims | Auto on optimization work |
| `language-guard` | Prevent treating `.agam` as Python/Rust | Auto on syntax work |

Skills live in `.agent/skills/`. All agents share the same skills — no agent-specific variants.

## External Integrations

- `claude-mem` is the preferred persistent-memory layer for Claude Code and Gemini CLI
- Use memory progressively: search/index first, fetch full details only for relevant hits
- Keep `graphify-out/GRAPH_REPORT.md` small and fresh; it is cheaper than raw-file grep for architecture questions
- Keep `.graphifyignore` updated so graph builds skip generated/output folders

## Rules

| Rule | File |
|------|------|
| Token efficiency | `.agent/rules/token-efficiency.md` |
| Language guardrails | `.agent/rules/language-guardrails.md` |
| Project structure | `.agent/rules/project-structure.md` |

## Agent-Specific Notes

### Claude Code
- Reads `CLAUDE.md` in each repo
- Skills auto-discovered from `.agent/skills/`
- Caveman auto-activates via skill definition
- If `claude-mem` is installed, prefer its memory search flow before rereading old notes

### Codex
- Reads `AGENTS.md` (this file)
- Use `$caveman` syntax (not `/caveman`)
- Use `$graphify` syntax (not `/graphify`)
- Prefer `graphify-out/GRAPH_REPORT.md` before raw grep; keep `multi_agent = true` for graphify parallel extraction

### Gemini CLI / Antigravity
- Reads `GEMINI.md` at project root
- Skills referenced via `@` directives in `GEMINI.md`
- If `claude-mem` Gemini integration is installed, use it before rereading long session notes

## Organization Rules

- Agam is one coordinated organization, not a pile of disconnected repos
- Cross-repo work follows the implementation priorities in `.agent/phases/`
- Shared rules and skills belong in the root `.agent/` board
- Repo-specific constraints stay in each repo's own `.agent/` tree

## graphify

This project has a graphify knowledge graph at graphify-out/.

Rules:
- Before answering architecture or codebase questions, read graphify-out/GRAPH_REPORT.md for god nodes and community structure
- If graphify-out/wiki/index.md exists, navigate it instead of reading raw files
- After modifying code files in this session, run `graphify update .` to keep the graph current (AST-only, no API cost)
