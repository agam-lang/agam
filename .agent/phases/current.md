# Current Development

## Program Goal

Native LLVM as first-class production backend for Windows, Linux, and Android.

## Active Workstreams

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| **15F** | partial | Incremental daemon, background prewarm, parallel compilation | `details/15F.md` |
| **15G** | partial | Premium experience layer (tooling unification) | `details/15G.md` |
| **15H** | partial | Native LLVM SDK distribution and toolchain bundles | `details/15H.md` |
| **17A** | partial | Workspace contract and dependency manifests | `details/17A.md` |

### 15F Progress
- ✅ Workspace snapshot + invalidation diff contract
- ✅ Foreground daemon loop with per-file AST/HIR/MIR warm state
- ✅ Entry-file warm-state reuse in `agamc dev`
- ✅ Multi-input `build` parallel worker scheduling
- ✅ Daemon-side entry-file prewarm (package + build cache)
- ✅ Cross-process reuse of daemon-prewarmed entry packages
- ⬜ Multi-file warm-state reuse beyond entry file
- ⬜ IPC-backed daemon/client coordination
- ⬜ Background prewarm for all workspace files

### 15G Progress
- ✅ `agamc doctor`, `new`, `dev`, `cache status`
- ✅ Shared workspace session contract across CLI/LSP/fmt/test
- ⬜ Keep daemon on the same contract; reduce per-tool drift

### 15H Progress
- ✅ `agamc package sdk`, bundled LLVM layout, CI skeleton
- ⬜ Validate hosted-runner SDK builds
- ⬜ Publish real Windows/Linux SDK artifacts
- ⬜ Android target-pack validation

### 17A Progress
- ✅ Manifest data models, validation, compatibility policy
- ✅ `resolve_workspace_members`, LSP/formatter integration
- ⬜ Reuse parsed manifest in daemon surfaces

## Decision Rules

- Prefer native host LLVM over WSL fallback
- Keep `agamc doctor` and SDK packaging aligned with the readiness contract
- VS Community 2026 is the canonical Windows toolchain inventory
- No macOS/iOS backend claims until native validation hardware exists
- If a phase decision needs more context → open `details/`
