# Token Efficiency Rule

All AI agents follow these token-saving conventions for compiler work:

## Output Compression
- Caveman skill at **full** intensity — ALWAYS ON by default
- Drop filler/hedging/pleasantries. Keep all technical substance exact.
- Code blocks, error messages, commands: write normally
- Off only with "stop caveman" / "normal mode"

## Input Compression
- Context files may be caveman-compressed (`.original.md` backups exist)
- Use `/caveman:compress <file>` on verbose docs

## Architecture Navigation
- If `graphify-out/GRAPH_REPORT.md` exists, read it before grepping raw files
- Run `/graphify . --update` after major structural changes
- Keep `.graphifyignore` current

## Persistent Memory
- If `claude-mem` available, use progressive disclosure: search → timeline → full fetch
- Prefer project-filtered lookups over broad history dumps

## Multi-Agent Handoff
- All agents share skills in `.agent/skills/` — no agent-specific workflows
- Keep output terse during handoffs — next agent reads less context
