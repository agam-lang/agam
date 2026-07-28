---
name: spec-archiver
description: Safely archives completed specifications to prevent context bloat.
---

# spec-archiver

**Purpose**: Moves completed phase documents out of active context to save tokens, while leaving a 1-line summary in an index file.

## Workflow

When a specification in `.agent/specs/active/` is fully verified and marked complete:

1. Write a 1-sentence summary of what the spec accomplished.
2. Append this summary, the filename, and the completion date to `.agent/specs/archive/INDEX.md`.
3. Move the `.md` file from `active/` to `archive/`.

**Strict Rule**: Do not leave completed specs in `active/`. If an AI has to read a 15KB file for a completed task on every prompt, it is a catastrophic waste of tokens.
