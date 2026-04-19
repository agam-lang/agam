# Token Efficiency Rule

All AI agents (Claude, Codex, Gemini, Antigravity) follow these token-saving conventions:

## Output Compression
- Use caveman skill (`.agent/skills/caveman/SKILL.md`) at **full** intensity by default
- Drop filler, hedging, pleasantries — keep all technical substance
- Code blocks, error messages, commands: write normally (never compress code)
- Switch off with "stop caveman" / "normal mode"

## Input Compression
- Context files (CLAUDE.md, AGENTS.md, phase docs) may be caveman-compressed
- `.original.md` backups exist for human-readable versions
- Use `/caveman:compress <file>` to compress verbose files
- Keep root agent entry files short; prefer shared rules/docs over repeating the same prose in multiple entrypoints

## Architecture Navigation
- If `graphify-out/GRAPH_REPORT.md` exists, read it before grepping raw files
- Use graph structure for architecture questions — god nodes, community boundaries
- Run `/graphify . --update` after major structural changes
- Keep `.graphifyignore` current so graph runs skip generated/build/output folders

## Persistent Memory
- If `claude-mem` is available, use it before rereading long notes or prior-session transcripts
- Use progressive disclosure: `search`/index first, `timeline` second, full observation fetch last
- Prefer project-filtered memory lookups over broad history dumps
- Quote or cite observation IDs only when they materially help the next step

## Multi-Agent Handoff
- All agents share these tools — no agent-specific workflows
- Skills live in `.agent/skills/` and are tool-agnostic
- Keep output terse during handoffs — next agent reads less context
