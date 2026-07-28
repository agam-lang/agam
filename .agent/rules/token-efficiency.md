# Token Efficiency Rule

All AI agents (Claude, Codex, Gemini, Antigravity) follow these token-saving conventions:

## Output Compression

- Use caveman skill (`.agent/skills/caveman/SKILL.md`) at **full** intensity by default
- Drop filler, hedging, pleasantries — keep all technical substance
- Code blocks, error messages, commands: write normally (never compress code)
- Switch off with "stop caveman" / "normal mode"

## Input Compression & Memory

- Context files (CLAUDE.md, AGENTS.md, phase docs) may be caveman-compressed
- Use the **Memory MCP Server** (`@modelcontextprotocol/server-memory`) for architectural questions instead of dumping entire source files.
- Persistent memory is an evolving knowledge graph. Query it before searching.

## Cargo Logs

- Do not dump 500+ lines of raw `cargo build` output into context.
- Use the `cargo-lens` skill (`.agent/skills/cargo-lens/SKILL.md`) to extract only errors.

## Multi-Agent Handoff

- All agents share these tools — no agent-specific workflows
- Skills live in `.agent/skills/` and are tool-agnostic
- Keep output terse during handoffs — next agent reads less context
