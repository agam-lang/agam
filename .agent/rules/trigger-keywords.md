# Trigger Keywords for Operational Streams & Workflows

When the user enters any of the following trigger keywords in conversation, all AI agents MUST immediately execute the corresponding workflow sequence:

## Workflow Trigger Shortcuts

| Trigger Keyword | Operational Action | Stream |
| :--- | :--- | :--- |
| **`"Start Debug"`** or **`"Run Stream 0"`** | Execute full Stream 0 verification: `cargo check`, unit/integration tests, multi-backend checks, diagnostic checks, log update, git commit & `git push`. | **Stream 0** |
| **`"Start Build [Phase]"`** or **`"Construct [Phase]"`** | Begin active code development for the specified Technical Tier (e.g. `T0-type-system`) following pipeline discipline (AST → HIR → MIR → Codegen). | **Stream 1** |
| **`"Weekend Sync"`** or **`"Horizon Review"`** | Execute Stream 2 frontier research synthesis: analyze latest AI/compiler developments, update master specs, and archive digest in `.agent/specs/active/horizon/`. | **Stream 2** |
| **`"Status Report"`** or **`"Check Status"`** | Render current phase progress, active tier state, backend matrix status, and recent execution log entries. | **System** |
| **`"Handoff"`** or **`"Update Handoff"`** | Refresh `.agent/memory/handoff.md` with current branch, commit hash, active tier, and next instruction for incoming AI agents. | **System** |

---

## Log Update Requirement

Whenever **`"Start Debug"`**, **`"Git Push"`**, or any feature phase completes, the agent MUST append a timestamped entry to `.agent/memory/execution.log` in the following format:

```text
[YYYY-MM-DD HH:MM:SS TZ] [EVENT_CATEGORY] Description of completed task or action
```

### Examples:
- `[2026-07-28 21:14:48 IST] [GIT_PUSH] Pushed commit 45b4240 to origin/main — Completed Phase 23 GPU Rich Memory Types`
- `[2026-07-28 21:40:00 IST] [DEBUG] Completed Stream 0 verification on Option<T> and Result<T, E> TypeStore constructors`
