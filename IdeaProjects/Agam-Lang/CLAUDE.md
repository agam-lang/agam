# Agam-Lang Organization — Claude Instructions

Treat root `.agent/` as canonical shared source of truth for org-level work.
Use `AGENTS.md` as concise cross-agent entrypoint, then fall through to docs below.

## Read First

- `AGENTS.md`
- `.agent/README.md`
- `.agent/include/project-context.md`
- `.agent/include/workflow.md`
- `.agent/phases/current.md`
- `.agent/phases/next.md`
- `.agent/rules/`

## Key Rules

- Compiler-specific work: switch to `agam/CLAUDE.md`
- Use `caveman` at full intensity by default per `.agent/rules/token-efficiency.md`
- If `graphify-out/GRAPH_REPORT.md` exists, read it before grepping raw files
- If `claude-mem` is available, use memory search with progressive disclosure before rereading long notes
- Keep shared changes in root `.agent/`; keep repo-specific changes in repo-local `.agent/`

## Shared Assets

- Skills: `.agent/skills/`
- Rules: `.agent/rules/`
- Phases: `.agent/phases/`
- Policy docs: `.agent/policy/`

## graphify

This project has a graphify knowledge graph at graphify-out/.

Rules:
- Before answering architecture or codebase questions, read graphify-out/GRAPH_REPORT.md for god nodes and community structure
- If graphify-out/wiki/index.md exists, navigate it instead of reading raw files
- After modifying code files in this session, run `graphify update .` to keep the graph current (AST-only, no API cost)
