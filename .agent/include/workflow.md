# Organization Workflow

> Full compiler-specific rules are in `agam/CLAUDE.md` § 6 (Rules).

## Core Rules

- Start at the workspace root only for cross-repo or organizational tasks
- Once work becomes repo-specific → switch to that repo's `AGENTS.md` and `.agent/`
- Keep shared guidance in the root `.agent/` when it applies across repositories
- Keep repo-specific guidance in repo-local `.agent/` trees
- Prefer optimal time/space complexity; justify tradeoffs

## Planning

- Use `.agent/phases/current.md` and `next.md` as the default priority order
- Use `details/` when execution depends on exact implementation slices
- The compiler roadmap is the main dependency chain

## Documentation

- Update root `.agent/` for shared changes
- Keep root and repo-local boards in sync when both exist
