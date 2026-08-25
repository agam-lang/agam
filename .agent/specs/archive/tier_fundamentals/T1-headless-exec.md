# Phase T1-headless-exec — Agent-Facing Headless Execution Tool

**Status:** complete
**Tier:** 1 (Developer Experience & External Tooling)

## Goal

Build a secure, machine-consumable headless execution interface so LLM agent frameworks can execute Agam code directly under enforced OS-level resource and capability sandboxing.

## Responsible Crates

- `agam_driver`
- `agam_runtime`
- `agam_notebook`

## Deliverables

- [x] **Agent-Facing Headless Execution Tool (`agamc exec`)**:
  - Direct source input via `--source`, `--file`, stdin source text, or structured JSON request.
  - Strict machine-consumable JSON response with `success`, `exit_code`, `stdout`, `stderr`, and `error`.
  - In-process JIT/LLVM/C execution with sanitized temporary workspace materialization.
- [x] **Request-Level Resource & Capability Controls (`HeadlessExecutionPolicy`)**:
  - Enforced limits on max source bytes, max arg count, total arg bytes, and native-backend gating.
- [x] **OS-Level Isolation & Sandboxing (`agam_runtime::sandbox`)**:
  - `SandboxPolicy` & `SandboxGuard` with configurable timeout, memory limits, active process limits, and network/process spawn denial.
  - **Windows**: Win32 Job Object limits with background timeout watchdog thread.
  - **Linux**: `prctl(PR_SET_NO_NEW_PRIVS)` and `setrlimit` resource bounds.
  - CLI integration via `--sandbox none|process|strict`.

## Test Results
- 57/57 tests pass in `agam_runtime` (including 7/7 sandbox tests)
- 100% test pass rate across all 27 crates in workspace
- 0 Clippy warnings (`-D warnings`)
- 100% formatting compliance (`cargo fmt --check`)
