# Current Development

## Program Goal

Native LLVM as first-class production backend for Windows, Linux, and Android.

## 2026-05-14 Compiler Structure Note

- ✅ `crates/core/agam_interface/` now exists and owns the shared parse/semantic-check/HIR/MIR orchestration layer plus `WarmState`
- ✅ `agam_driver` is now materially split: `main.rs` is a thin entrypoint, command dispatch lives in `src/dispatch.rs`, and the large inline test module moved to `src/main_tests.rs`
- ✅ The workspace builds again with the newer MIR surface (`Switch`, `EnumConstruct`, `EnumTag`, `EnumPayload`) and all `agam_driver` tests pass
- ✅ The earlier “TypeStore O(n) interning” and “extract C runtime” items are already complete in-tree, so they are no longer active blockers
- ⬜ The next real compiler gap is not the build itself; it is semantic completeness: `agam_mir::lower` still has `todo!` paths for struct literals, enum variants, and `match` lowering, and backend enum-layout/codegen remains incomplete

## Active Workstreams

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| **15F** | completed | Incremental daemon, background prewarm, parallel compilation | `details/15F.md` |
| **15G** | completed | Premium experience layer (tooling unification) | `details/15G.md` |
| **15H** | partial | Native LLVM SDK distribution and toolchain bundles | `details/15H.md` |
| **16** | completed | Interactive REPL and structured headless execution | `details/16.md` |
| **17A** | completed | Workspace contract and dependency manifests | `details/17A.md` |
| **17B** | completed | Deterministic resolver and lockfile | `details/17B.md` |
| **17C** | completed | Registry index protocol and immutable publish flow | `details/17C.md` |
| **17D** | completed | Named environments and SDK linkage | `details/17D.md` |
| **17E** | completed | Curated first-party distributions and official package governance | `details/17E.md` |
| **17F** | partial | Standard library and native I/O expansion | `details/17F.md` |
| **18** | partial | Agent-facing execution tool | `details/18.md` |
| **19** | partial | Wrapper foundation for agent ecosystems | `details/19.md` |
| **20** | completed | Language surface: effect/handler/perform syntax | `details/20.md` |
| **21** | completed | Runtime hardening: OS-level sandbox enforcement | `details/21.md` |
| **22** | completed | Omni-Targeting Directives (`@target.iot`, `@target.enterprise`, `@target.hpc`) | `details/22.md` |
| **23** | partial | GPU and NPU Integration (`@gpu` kernel pipeline) | `details/23.md` |

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
- ⬜ Exercise hosted-runner SDK builds on real GitHub runners
- ⬜ Validate release-uploaded Windows/Linux SDK artifacts end to end on GitHub itself

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
- ✅ HIR lowering now translates AST `Perform`/`HandleWith` nodes to `HirExprKind::Perform`/`HirExprKind::HandleWith`
- ✅ C backend emits concrete effect dispatch functions (FileSystem + Console) instead of TODO stubs
- ✅ LLVM backend emits external function calls for effect dispatch instead of returning errors
- ✅ `Console` effect added as second stdlib effect (print, println, read_line, eprint, eprintln)
- ✅ `agam_std::effects` registers 14 handlers total (9 FileSystem + 5 Console)
- ✅ End-to-end integration tests verify `perform` compiles to real code in both backends
- ⬜ Align broader standard-library packaging/versioning with first-party distribution and governance contracts

### 18 Progress
- ✅ Dedicated `agamc exec` command now exposes the strict headless execution contract as an agent-facing surface instead of hiding it under `agamc repl --json`
- ✅ `agamc exec` can execute strict JSON requests or source provided through stdin, `--source`, or `--file`, while still returning structured JSON `stdout`/`stderr`/exit metadata
- ✅ The execution tool reuses the existing sanitized temp-workspace headless path instead of inventing a second execution engine
- ✅ Headless execution requests now carry explicit policy limits for source size, arg count, total arg bytes, and native-backend opt-in instead of relying only on the temp-workspace boundary
- ✅ `agamc exec` now routes production requests through an isolated worker subprocess with a sandbox cwd, scrubbed environment by default, wall-clock timeout enforcement, and platform-level memory/process controls where supported
- ⬜ Extend the current worker isolation beyond timeout/env/memory/process controls into explicit filesystem and network capability enforcement

### 19 Progress
- ✅ `agam_ffi` now provides an `AgamExecClient` that invokes `agamc exec --json` and parses the strict structured response contract
- ✅ `agam_ffi` now provides an `AgamReplTool` abstraction that can build configured execution requests for later Python/LangChain/LlamaIndex bindings
- ✅ `crates/agam_ffi/python` now provides Python-native `AgamExecClient`, `AgamREPLTool`, and request/response wrappers over the same `agamc exec --json` contract
- ✅ The Python package now exposes optional LangChain and LlamaIndex adapter hooks plus extras for installing those framework integrations
- ✅ The adapter hooks now smoke-test against live `langchain-core` and `llama-index-core` installs instead of only repo-local test doubles
- ✅ `crates/agam_ffi/python` now carries publish-ready package metadata plus a GitHub Actions build-and-publish workflow for external package releases
- ⬜ Exercise the external Python package release path end to end and keep the adapter surface current against upstream framework drift

### 20 Progress
- ✅ Registered `perform`, `handle`, and `effect` keywords in the lexer
- ✅ Extended AST `ExprKind` with `Perform`, `HandleWith`, and `Resume` nodes
- ✅ Implemented `parse_effect_decl` and `parse_handler_decl` with Pratt parser support
- ✅ Updated `agam_sema` to identify and type-check algebraic effect declarations and handlers
- ✅ Added and verified parser unit tests covering the new syntax

### 21 Progress
- ✅ Implemented OS-native sandboxing in `agam_runtime` for headless execution
- ✅ Added Win32 `JobObject` enforcement for memory, active processes, and UI isolation
- ✅ Added Linux `prctl(PR_SET_NO_NEW_PRIVS)` and `setrlimit` enforcement for resources
- ✅ Built robust RAII handle lifecycle management for sandbox state
- ✅ Added platform-specific crate dependencies (`windows-sys`, `libc`)

### 22 Progress
- ✅ Parser already supports `@target.iot`, `@target.enterprise`, `@target.hpc` via existing dotted annotation parsing
- ✅ `agam_sema::target` module: `TargetProfile` enum, `TargetConstraints`, `resolve_target_profile()`, and `validate_effect_for_target()`
- ✅ HIR carries `target: TargetProfile` on `HirFunction`, resolved from annotations during lowering
- ✅ MIR carries `target: TargetProfile` on `MirFunction`, propagated from HIR with serde default for backwards compatibility
- ✅ C emitter: IoT modules skip effect/dataframe/tensor preludes, emit `AGAM_NO_HEAP` define; HPC modules emit `AGAM_HPC` define
- ✅ 12 new sema tests covering profile resolution, constraint derivation, validation, and error cases
- ✅ LLVM emitter: function-level target comments and module-level `!agam.target.*` metadata nodes for IoT/HPC/Enterprise
- ✅ HIR lowering rejects `perform` in `@target.iot` functions at compile time with diagnostic errors
- ✅ 5 new HIR tests for target propagation (Iot, Hpc, Default) and IoT effect rejection

### 23 Progress
- ✅ `@gpu(...)` annotation arguments now parse from source, including tuple-style `grid=(x, y, z)`, and sema resolves `threads`, `shared`, and `grid`
- ✅ `GpuKernelLaunch` MIR op and propagation across all backends
- ✅ High-throughput NVPTX64 IR emitter with CUDA linkage and pre-allocated formatting
- ✅ GPU kernel parameter ABI hints now preserve scalar and buffer signatures (`float`, `float*`, `i32*`) through HIR → MIR → NVPTX entry binding
- ✅ Reference-wrapped GPU buffers (`&[T]`, `&[T; N]`, `&mut [T; N]`) now preserve typed pointer ABI through HIR → MIR → NVPTX instead of degrading to `i8*`
- ✅ GPU integer width support now extends through the LLVM/NVPTX path for `i128/u128`, `i256/u256`, and `i512/u512` kernel params and shared-memory element typing
- ✅ Fixed-size array type expressions (`[T; N]`) now parse/resolve across parser, HIR, and sema, and lower through GPU buffer/shared-memory paths as element-typed device pointers
- ✅ Source-level GPU builtins now resolve and lower end-to-end (`agam.gpu.thread_id_*`, `agam.gpu.block_id_*`, `agam.gpu.block_dim_*`, `agam.gpu.barrier`)
- ✅ GPU kernel validation now runs during HIR lowering, rejects effects/strings/heap-style allocation/direct recursion, and still permits shared-memory plus indexed pointer/array access
- ✅ Indexed GPU buffer reads and writes now lower from source (`input[idx]`, `output[idx] = value`) through MIR into NVPTX `getelementptr` + load/store
- ✅ Host calls to known `@gpu` functions now lower into `GpuKernelLaunch` with the current scalar launch contract (`grid.x`, block size, shared bytes) instead of falling back to plain calls
- ⬜ Rich Memory Types (pointer/array lowering in kernels)
- ✅ Shared Memory (`agam.gpu.shared_alloc(...)` now lowers to `addrspace(3)` NVPTX shared allocations for annotated pointer/slice/reference targets)
- ✅ Reference-wrapped shared-memory targets (`&mut [T]`) now preserve typed shared alloc lowering instead of degrading to opaque pointers
- ✅ GPU math builtins now lower to NVVM fast-math intrinsics (`agam.gpu.sin`, `agam.gpu.cos`, `agam.gpu.sqrt`, `agam.gpu.exp`)
- ✅ Host-Device memory transfer APIs (`gpu_malloc`, `gpu_free`, `gpu_memcpy_to_device`, `gpu_memcpy_to_host`) now lower through the stdlib plus both the C and LLVM host backends
- ✅ LLVM host emission now lowers `GpuKernelLaunch` into concrete CUDA runtime IR with argument packing and `cudaLaunchKernel(...)` calls

## Decision Rules

- Prefer native host LLVM over WSL fallback
- Keep `agamc doctor` and SDK packaging aligned with the readiness contract
- VS Community 2026 is the canonical Windows toolchain inventory
- No macOS/iOS backend claims until native validation hardware exists
- If a phase decision needs more context → open `details/`
