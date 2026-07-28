# Trigger Keywords for Operational Streams & Workflows

When the user enters any of the following trigger keywords in conversation, all AI agents MUST immediately execute the corresponding workflow sequence:

## Workflow Trigger Shortcuts

| Trigger Keyword | Operational Action | Stream |
| :--- | :--- | :--- |
| **`"Continue Development"`** or **`"Next Task"`** | Read `.agent/specs/active/next.md`, pick up the highest priority uncompleted feature (currently `T0-type-system`), and resume coding across Technical Tiers (T0–T6). | **Stream 1 (Technical Tiers T0–T6)** |
| **`"Start Debug"`** or **`"Run Stream 0"`** | Execute full Stream 0 verification: `cargo check`, unit/integration tests, multi-backend checks, diagnostic checks,telemetry data or running autonomous fuzzing loops, update `execution.log`, git commit & `git push`. | **Stream 0 (Assurance & Debug)** |
| **`"Start Build [Phase]"`** or **`"Construct [Phase]"`** | Begin active code development for the specified Technical Tier (e.g. `T0-type-system`) following pipeline discipline (AST → HIR → MIR → Codegen). | **Stream 1 (Technical Tiers T0–T6)** |
| **`"Fix Error"`** or **`"Debug Issue"`** | Inspect un-truncated error logs and tracebacks, identify underlying root causes, apply code fix, and run Stream 0 post-feature verification. | **Stream 0 / 1** |
| **`"Run Tests"`** or **`"Verify All"`** | Execute `cargo check` and `cargo test` across all 27 workspace crates to verify cross-backend equivalence (LLVM / NVPTX / C / JIT). | **Stream 0** |
| **`"Weekend Sync"`** or **`"Horizon Review"`** | Execute Stream 2 frontier research synthesis: analyze latest AI/compiler developments (e.g., MiniTriton, Kimi K3), update master specs, and archive digest. | **Stream 2** |
| **`"Run Benchmark"`** or **`"Bench Target"`** | Trigger benchmark execution (`benchmark-guard` skill), profile compilation throughput (MB/s IR), measure execution latency, and compare against reference baselines. | **Stream 0 / 1** |
| **`"Recommend Feature"`** or **`"Suggest Feature"`** | Analyze current compiler capability gaps, evaluate T0–T6 catalog, and provide AI-driven strategic feature recommendations based on industry trends (e.g. MiniTriton, Hopper TMA, async I/O). | **Stream 2 (Frontier Horizon Sync)** |
| **`"Status Report"`** or **`"Check Status"`** | Render current phase progress, active tier state, backend matrix status, and recent `execution.log` entries. | **System** |
| **`"Handoff"`** or **`"Update Handoff"`** | Refresh `.agent/memory/handoff.md` with current branch, commit hash, active tier, and next instruction for incoming AI agents. | **System** |

---

## Log Update Requirement

Whenever **`"Start Debug"`**, **`"Continue Development"`**, **`"Git Push"`**, or any feature phase completes, the agent MUST append a timestamped entry to `.agent/memory/execution.log` in the following format:

```text
[YYYY-MM-DD HH:MM:SS TZ] [EVENT_CATEGORY] Description of completed task or action
```

### Examples:
- `[2026-07-28 21:43:00 IST] [CONTINUE_DEV] Resumed Stream 1 development on Phase T0-type-system (Generics & Enums)`
- `[2026-07-28 21:45:00 IST] [DEBUG] Completed Stream 0 verification on Option<T> and Result<T, E> TypeStore constructors`
