# Chapter 17: Incremental Compilation Daemon & Sandboxed Execution

> **System Scope**: Tooling Infrastructure & Security Hardening  
> **Compiler Module Focus**: [`agam_driver`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_driver), [`agam_pkg`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_pkg), [`agam_runtime`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime)

---

## 17.1 Incremental Background Daemon (`Phase 15F`)

To deliver sub-millisecond compile loops during development, `agamc` runs a background daemon process (`DaemonSession`):

```text
 ┌─────────────────────────────────────────────────────────────────┐
 │                      agamc daemon process                       │
 │                                                                 │
 │  ┌───────────────────────┐            ┌──────────────────────┐  │
 │  │ WorkspaceSnapshot Index│            │ DaemonSession Cache  │  │
 │  │ (Fingerprint Maps)    │            │ (Warm AST/HIR/MIR)   │  │
 │  └───────────┬───────────┘            └──────────▲───────────┘  │
 └──────────────┼───────────────────────────────────┼──────────────┘
                │                                   │
                ▼                                   │
 ┌───────────────────────────┐                      │
 │ WorkspaceSnapshotDiff     │ ─────────────────────┘
 │ Detects Changed Files     │  Updates Warm MIR Cache
 └───────────────────────────┘
```

- **`WorkspaceSnapshot`**: Fingerprints source file contents to detect modifications instantly.
- **`DaemonSession`**: Holds pre-parsed ASTs, HIR, and serialized MIR artifacts in warm memory, eliminating redundant parsing of unchanged workspace modules.
- **IPC TCP Loopback (`127.0.0.1:0`)**: Standard binary CLI commands query the background daemon over localhost TCP sockets.

---

## 17.2 Sandboxed Execution Hardening (`Phase 21`)

When executing untrusted user code or running headless agent tool calls (`agamc exec`), the Agam runtime enforces strict operating system-level process sandboxing:

- **Windows Platform**: Enforces Windows `JobObject` limits restricting maximum memory usage, CPU rate limits, and child process creation.
- **Linux Platform**: Invokes `prctl` (`PR_SET_NO_NEW_PRIVS`) and `setrlimit` syscalls to restrict RAM allocation, file descriptor counts, and execution timeouts.
