# Gemini CLI / Antigravity Context — Agam-Lang

> Read `agam/CLAUDE.md` for full compiler briefing. This file provides Gemini-specific setup.

## 🤖 Unified Multi-AI Workflow

You are participating in a continuous multi-agent development session.
Other agents (Claude, Codex) share the same workspace, phases, and skills.
Read existing context, respect ongoing checklists, do not invent separate workflows.

## Active Skills

@./.agent/skills/caveman/SKILL.md
@./.agent/skills/caveman-compress/SKILL.md
@./.agent/skills/graphify/SKILL.md

## Rules

- Follow `.agent/rules/token-efficiency.md` — terse output by default
- Follow `.agent/rules/language-guardrails.md` — Agam is its own language
- Follow `.agent/rules/project-structure.md` — crate boundaries matter
- If `claude-mem` is installed, use memory search before rereading long notes

## Quick Start

| Task | Read |
|------|------|
| Claude-compatible root brief | `CLAUDE.md` |
| Compiler work | `agam/CLAUDE.md` |
| Current phases | `.agent/phases/current.md` |
| What to build next | `.agent/phases/next.md` |
| Phase checklists | `.agent/phases/details/` |
| Package architecture | `.agent/policy/` |

## Build & Verify

```powershell
cargo check --manifest-path agam/Cargo.toml        # must pass
cargo test --manifest-path agam/Cargo.toml          # must pass
cargo fmt --manifest-path agam/Cargo.toml -- --check  # should pass
```

## Architecture Navigation

If `graphify-out/GRAPH_REPORT.md` exists, read it before grepping raw files.
Use `/graphify . --update` after major structural changes.
Keep `.graphifyignore` accurate so graph runs stay fast at workspace-root scale.

## graphify

This project has a graphify knowledge graph at graphify-out/.

Rules:
- Before answering architecture or codebase questions, read graphify-out/GRAPH_REPORT.md for god nodes and community structure
- If graphify-out/wiki/index.md exists, navigate it instead of reading raw files
- After modifying code files in this session, run `graphify update .` to keep the graph current (AST-only, no API cost)
