# Trigger Keywords for Operational Streams & Workflows

When the user enters any of the following trigger keywords in conversation, all AI agents MUST immediately execute the corresponding workflow sequence:

## Workflow Trigger Shortcuts

| Trigger Keyword | Operational Action | Stream |
| :--- | :--- | :--- |
| **`"Continue Development"`** | Read `.agent/specs/active/next.md`, pick up the highest priority uncompleted feature (currently `T0-type-system`), and resume coding across Technical Tiers (T0–T6). | **Stream 1 (Technical Tiers T0–T6)** |
| **`"Start Debug"`** | Execute full Stream 0 debugging & assurance in an autonomous continuous loop: scan codebase for edge-case bugs, run fuzzing loops, collect telemetry data, add new unit tests for uncovered branches, apply fixes, run `cargo check` & `cargo test`, update `execution.log`, git commit & `git push`.  | **Stream 0 (Assurance & Debug)** |
| **`"Start Build [Phase]"`** | Begin active code development for the specified Technical Tier (e.g. `T0-type-system`) following pipeline discipline (AST → HIR → MIR → Codegen). | **Stream 1 (Technical Tiers T0–T6)** |
| **`"Fix Error"`** | Inspect un-truncated error logs and tracebacks, identify underlying root causes, apply code fix, and run Stream 0 post-feature verification. | **Stream 0 / 1** |
| **`"Run Tests"`** | Execute `cargo check` and `cargo test` across all 27 workspace crates to verify cross-backend equivalence (LLVM / Universal GPU [NVPTX, AMDGPU, SPIR-V, Metal] / C / JIT). | **Stream 0** |
| **`"Horizon Review"`** | Execute Stream 2 frontier research synthesis: analyze mathematical algorithms (e.g., square-zero algebra tensor kernel fusion, MCP AST streaming, Hankel moment systems), update master specs, and archive digest. | **Stream 2** |
| **`"Run Benchmark"`** | Trigger benchmark execution (`benchmark-guard` skill), profile compilation throughput (MB/s IR), measure execution latency, and compare against reference baselines. | **Stream 0 / 1** |
| **`"Recommend Feature"`** | Analyze current compiler capability gaps, evaluate T0–T6 catalog, and provide AI-driven strategic algorithm recommendations (e.g., E-graph term rewriting, representation graph monomorphization, tensor tile lowering). | **Stream 2 (Frontier Horizon Sync)** |
| **`"Status Report"`** | Render current phase progress, active tier state, backend matrix status, and recent `execution.log` entries. | **System** |
| **`"Analyze"`** | Execute a full project-wide context initialization sequence: inspect `AGENTS.md`, `CLAUDE.md`, `MANIFESTO.md`, `design-principles.md` (architecture & Indic/Chāṇakya/Nyāya principles), read `.agent/specs/active/current.md`, `.agent/specs/active/next.md`, `.agent/memory/execution.log` (current & active state), review `.agent/specs/active/catalog.md` and horizon reviews (future roadmap & theoretical models), inspect `graphify-out/GRAPH_REPORT.md` (codebase architecture graph), and render a complete, ultra-sharp project synthesis. | **System / Context Sync** |
| **`"Inject Mathematical Models"`** | Analyze frontier papers, articles, and classical theorems (e.g., E-Graph term rewriting, Hankel moment systems, Kronecker Jacobians, representation graphs, resolvent purifications) and synthesize new algorithms/mathematical models into Agam to make the compiler, MIR passes, type system, and runtime unbeatably fast, secure, and highly optimized. | **Stream 2 (Frontier Theory & Algorithm Synthesis)** |
| **`"Push"`** | Verify codebase (`cargo check`), append entry to `.agent/memory/execution.log`, stage files (`git add .`), commit with structured message, and push to remote (`git push`). | **Stream 0 / System** |
| **`"Handoff"`** | Refresh `.agent/memory/handoff.md` with current branch, commit hash, active tier, and next instruction for incoming AI agents. | **System** |

---

## Log Update Requirement

Whenever **`"Start Debug"`**, **`"Continue Development"`**, **`"Push"`**, or any feature phase completes, the agent MUST append a timestamped entry to `.agent/memory/execution.log` in the following format:

```text
[YYYY-MM-DD HH:MM:SS TZ] [EVENT_CATEGORY] Description of completed task or action
```

### Examples:
- `[2026-07-28 21:43:00 IST] [CONTINUE_DEV] Resumed Stream 1 development on Phase T0-type-system (Generics & Enums)`
- `[2026-07-28 21:45:00 IST] [DEBUG] Completed Stream 0 verification on Option<T> and Result<T, E> TypeStore constructors`
