# Current Development

## Program Goal

Native LLVM as first-class production backend for Windows, Linux, and Android.

## Active Workstreams

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| **15F** | completed | Incremental daemon, background prewarm, parallel compilation | `details/15F.md` |
| **15G** | completed | Premium experience layer (tooling unification) | `details/15G.md` |
| **15H** | completed | Native LLVM SDK distribution and toolchain bundles | `details/15H.md` |
| **16** | completed | Interactive REPL and structured headless execution | `details/16.md` |
| **17A** | completed | Workspace contract and dependency manifests | `details/17A.md` |
| **17B** | completed | Deterministic resolver and lockfile | `details/17B.md` |
| **17C** | completed | Registry index protocol and immutable publish flow | `details/17C.md` |
| **17D** | completed | Named environments and SDK linkage | `details/17D.md` |
| **17E** | completed | Curated first-party distributions and official package governance | `details/17E.md` |
| **17F** | completed | Standard library and native I/O expansion | `details/17F.md` |
| **18** | completed | Agent-facing execution tool | `details/18.md` |
| **19** | completed | Wrapper foundation for agent ecosystems | `details/19.md` |
| **20** | completed | Language surface: first-class effect/handler/perform syntax | — |
| **21** | completed | Runtime Hardening: concrete Win32 Job Object and Linux prctl/setrlimit sandboxing | — |

### 15F Progress
- ✅ Workspace snapshot + invalidation diff contract
- ✅ Foreground daemon loop with per-file AST/HIR/MIR warm state
- ✅ Entry-file warm-state reuse in `agamc dev`
- ✅ Multi-input `build` parallel worker scheduling
- ✅ Daemon-side entry-file prewarm (package + build cache)
- ✅ Cross-process reuse of daemon-prewarmed entry packages
- ✅ Multi-file warm artifacts now persist callable source-feature metadata for safe runnable reuse
- ✅ Multi-file warm-state reuse beyond the entry file via the persisted daemon warm index
- ✅ IPC-backed daemon/client coordination for synchronous warm-state queries
- ✅ Background prewarm for workspace files plus background daemon lifecycle management

### 15G Progress
- ✅ `agamc doctor`, `new`, `dev`, `cache status`
- ✅ Shared workspace session contract across CLI/LSP/fmt/test
- ✅ Keep the daemon on the shared workspace/session contract; reduce per-tool drift

### 15H Progress
- ✅ `agamc package sdk`, bundled LLVM layout, release-ready archive/checksum flow, release-publish workflow, and Linux Android target-pack staging/validation
- ✅ Downloaded-artifact checksum/extract revalidation plus packaged Android sysroot metadata in the SDK manifest contract
- ✅ Hardened `sdk-dist.yml` for real hosted runners: working-directory, artifact paths, publish checkout, Python 3.12 tarfile security, release-mode builds
- ✅ Local E2E validation script (`scripts/validate_sdk_e2e.py`) mirrors CI pipeline for contract proof without hosted runners

### 16 Progress
- ✅ `agamc repl` now provides a buffered interactive shell with `:run`, `:show`, `:reset`, `:load`, backend selection, and run-tuning controls
- ✅ Interactive `:run` now executes in-process on the shared CLI backend/JIT path and reuses the shared daemon/incremental warm-state contract across buffer edits
- ✅ `agam_notebook` now defines the strict JSON headless execution request/response contract
- ✅ `agamc repl --json` now executes one Agam source request from stdin and returns structured `stdout`, `stderr`, exit-code, and error metadata
- ✅ JIT-backed `agamc repl --json` requests now execute in-process with captured stdout and buffered diagnostics instead of shelling back through `agamc run`
- ✅ LLVM/C-backed `agamc repl --json` requests now execute in-process from the same warm MIR and capture native `stdout`/`stderr` without shelling back through the CLI

### 17A Progress
- ✅ Manifest data models, validation, compatibility policy
- ✅ `resolve_workspace_members`, LSP/formatter integration
- ✅ Direct local path dependency metadata now travels through `WorkspaceSession` + manifest snapshots
- ✅ Nested local path-dependency manifests now stay attached to the shared session/snapshot contract for deeper transitive graphs
- ✅ Daemon invalidation and later tooling surfaces now reuse the parsed manifest/session contract instead of rediscovering manifests ad hoc

### 17B Progress
- ✅ `WorkspaceLockfile`, `LockedPackage`, and deterministic workspace/path/git/registry resolution in `agam_pkg`
- ✅ `agamc lock` plus automatic `agam.lock` refresh from `agamc build`, `check`, and `dev`
- ✅ Path-dependency content drift diagnostics for stale local sources
- ✅ Lockfile freshness now validates dependency aliases, source selectors, and version requirements instead of only comparing package-name sets
- ✅ Lockfile freshness and diagnostics now validate named environment records so stale backend/SDK/target selections force `agam.lock` refresh
- ✅ `generate_or_refresh_lockfile()` now treats live path-dependency content drift as stale and rewrites `agam.lock`
- ✅ Workspace-member and shared-session metadata now stay on the same freshness/diagnostic contract used by the resolver and CLI flows

### 17C Progress
- ✅ Registry index metadata, sharded package paths, and package-name validation in `agam_pkg`
- ✅ Local index-backed resolver lookup plus immutable local publish helpers
- ✅ `agamc publish` with `--dry-run`, metadata overrides, and local `config.json` bootstrap
- ✅ `agamc registry inspect` and `agamc registry audit` on top of the thin registry contract
- ✅ `agamc registry install` and `agamc registry update` with manifest + lockfile refresh against a selected local index
- ✅ Release-level download metadata, provenance records, and `agamc registry yank`

### 17D Progress
- ✅ `ResolvedEnvironment` plus explicit default-selection rules (`dev` first, then sole environment) in `agam_pkg`
- ✅ `agamc env list` and `agamc env inspect` on top of manifest + in-memory lockfile resolution
- ✅ Environment selection integrated into build/run/dev/doctor/package SDK flows through `--env` and implicit project-local defaults
- ✅ Project-local selection and diagnose flows extended beyond direct inspection through `agamc doctor --env` and environment-aware SDK staging

### 17E Progress
- ✅ Curated first-party distribution profiles (`base`, `systems`, `data-ai`) plus official package governance in `agam_pkg`
- ✅ `agamc registry governance`, `agamc registry profile list`, and `agamc registry profile inspect`
- ✅ `agamc registry profile install` with manifest + lockfile refresh against a selected local index
- ✅ `agamc publish --official` for reserved `agam-` packages under the canonical registry/owner/repository contract

### 17F Progress
- ✅ `agam_std::io` now provides a first-party deterministic file/path I/O slice with path inspection, directory creation/listing, and UTF-8 text read/write helpers
- ✅ `IoError` plus crate-level tests now cover round-trip text I/O, append ordering, lexicographic directory listing, and missing-file diagnostics
- ✅ `agam_sema::effects` now exposes a matching builtin `FileSystem` effect definition plus `register_std_effects()` for the current stdlib I/O surface
- ✅ `agam_runtime::effects` provides a generic `EffectHandlerTable` dispatch infrastructure for `Op::EffectPerform` MIR instructions
- ✅ `agam_std::effects` registers 9 concrete FileSystem handlers backed by `agam_std::io` into the runtime dispatch table
- ✅ `Perform`/`HandleWith` HIR nodes lower to `EffectPerform`/`HandleWith` MIR ops through the full lowering pipeline
- ✅ All MIR optimization passes (DCE, inline, loop unroll, constant fold) and codegen backends (LLVM, C, JIT) handle the new effect ops
- ✅ `agam_pkg` now carries `StdlibRegistry` and `StdlibModuleMetadata` for standard-library packaging alignment
- ✅ `agamc exec --sandbox-level` / `--deny-network` / `--deny-process-spawn` wired through to `SandboxGuard::acquire` in the execution flow

### 18 Progress
- ✅ Dedicated `agamc exec` command now exposes the strict headless execution contract as an agent-facing surface instead of hiding it under `agamc repl --json`
- ✅ `agamc exec` can execute strict JSON requests or source provided through stdin, `--source`, or `--file`, while still returning structured JSON `stdout`/`stderr`/exit metadata
- ✅ The execution tool reuses the existing sanitized temp-workspace headless path instead of inventing a second execution engine
- ✅ Headless execution requests now carry explicit policy limits for source size, arg count, total arg bytes, and native-backend opt-in instead of relying only on the temp-workspace boundary
- ✅ `agam_runtime::sandbox` provides `SandboxPolicy` + RAII `SandboxGuard` with watchdog timeout and platform-specific stubs
- ✅ `HeadlessExecutionPolicy` now carries `deny_network`, `deny_process_spawn`, and `sandbox_level` fields
- ✅ `agamc exec --sandbox-level` / `--deny-network` / `--deny-process-spawn` activate `SandboxGuard` RAII enforcement around headless execution

### 19 Progress
- ✅ `agam_ffi` now provides an `AgamExecClient` that invokes `agamc exec --json` and parses the strict structured response contract
- ✅ `agam_ffi` now provides an `AgamReplTool` abstraction that can build configured execution requests for later Python/LangChain/LlamaIndex bindings
- ✅ `crates/agam_ffi/python` now provides Python-native `AgamExecClient`, `AgamREPLTool`, and request/response wrappers over the same `agamc exec --json` contract
- ✅ The Python package now exposes optional LangChain and LlamaIndex adapter hooks plus extras for installing those framework integrations
- ✅ The adapter hooks now smoke-test against live `langchain-core` and `llama-index-core` installs instead of only repo-local test doubles
- ✅ Integration test harness validates adapter output types, metadata, and ImportError diagnostics against real framework APIs
- ✅ `publish.py` + `MANIFEST.in` provide PyPI-ready build/upload flow with `--dry-run` and `--test-pypi` support

### 20 Progress
- ✅ `perform` keyword added to the lexer (`TokenKind::Perform`) alongside existing `effect`, `handle`, `resume`
- ✅ All four keywords registered in `is_keyword` and `lookup_keyword`
- ✅ `ExprKind::Perform { effect, operation, args }` and `ExprKind::HandleWith { body, handler }` added to the AST
- ✅ Parser dispatches `effect` and `handle` as top-level declaration keywords
- ✅ `parse_effect_decl` parses `effect Name { fn op(params) -> Ret; }` into `EffectDecl`
- ✅ `parse_handler_decl` parses `handle name for Effect { fn op(params): body; }` into `HandlerDecl`
- ✅ `perform Effect.op(args)` parsed as prefix expression → `ExprKind::Perform`
- ✅ `handle expr with handler` parsed as prefix expression → `ExprKind::HandleWith`
- ✅ `resume(value)` parsed as prefix expression → `ExprKind::Resume`
- ✅ `agam_sema` resolver and type checker handle all three new expression kinds
- ✅ 4 new parser tests validate all syntactic forms

### 21 Progress
- ✅ Implemented concrete Win32 Job Object sandbox enforcement in `agam_runtime`
- ✅ Applied `ProcessMemoryLimit` and `ActiveProcessLimit` via `SetInformationJobObject` on Windows
- ✅ Isolated UI restrictions (clipboard, desktop switch) via `JOB_OBJECT_BASIC_UI_RESTRICTIONS`
- ✅ Implemented RAII cleanup for Windows Job Object via `Drop` on `PlatformSandboxState`
- ✅ Implemented concrete Linux sandbox enforcement using `prctl(PR_SET_NO_NEW_PRIVS)`
- ✅ Applied `setrlimit` constraints for `RLIMIT_AS` (memory), `RLIMIT_NPROC` (processes), and `RLIMIT_CPU` (time limit) on Linux

## Decision Rules

- Prefer native host LLVM over WSL fallback
- Keep `agamc doctor` and SDK packaging aligned with the readiness contract
- VS Community 2026 is the canonical Windows toolchain inventory
- No macOS/iOS backend claims until native validation hardware exists
- If a phase decision needs more context → open `details/`
