# Agam Compiler — Agent Briefing

> **This file is the single entrypoint.** Read this first. Everything you need for most tasks is here.
> Only open `.agent/phases/details/` when you need the exact checklist for a specific phase.

> **🤖 Unified Multi-AI Workflow:** Whether you are Gemini, Claude, Codex, or another AI, you are operating in a continuous, hand-off rotation. Read the existing context, respect the ongoing phase checklists, and do not invent your own workflows.

---

## 1. What Agam Is

Agam is a **next-generation compiled language** implemented as a Rust workspace.
It combines Python-level readability, Rust-like memory safety, and native-speed execution.
AI, tensor, and numerical workflows are first-class language concerns — not library wrappers.

**This is its own language.** It is not Python and not Rust. Use `examples/*.agam`,
`.agent/test/*.agam`, and `benchmarks/benchmarks/**/*.agam` as syntax reality checks.

---

## 2. Current Program Goal

**Make native LLVM the first-class production backend for Windows, Linux, and Android.**

- Prefer native host LLVM over WSL fallback
- WSL is a development/verification environment, not the shipped path
- macOS/iOS are planned but not validation-complete targets yet
- Performance target: optimized `clang++`-class output on proven workloads
- VS Community 2026 is the canonical Windows-side toolchain inventory

---

## 3. Architecture

```
Source → Lexer → Parser → AST → Sema → HIR → MIR → Codegen → Native Binary
                                                         ↘ JIT Runtime
```

### Crate Map

| Layer | Crates |
|-------|--------|
| **Frontend** | `agam_lexer`, `agam_parser`, `agam_ast` |
| **Semantics** | `agam_sema` (resolver + type checker) |
| **Lowering** | `agam_hir`, `agam_mir` (with `agam_mir::opt` optimizer) |
| **Backends** | `agam_codegen` (C/LLVM IR emit), `agam_jit` (Cranelift JIT) |
| **Runtime** | `agam_runtime` (ABI contract, cache store, host detection) |
| **Tooling** | `agam_driver` (`agamc` CLI), `agam_fmt`, `agam_lsp`, `agam_test`, `agam_profile` |
| **Packaging** | `agam_pkg` (manifest, workspace, snapshot, portable packages, SDK distribution) |
| **Diagnostics** | `agam_errors` (spans, labels, diagnostic emitter) |
| **Future** | `agam_std`, `agam_ffi`, `agam_lint`, `agam_doc`, `agam_debug`, `agam_macro`, `agam_smt`, `agam_notebook`, `agam_ui`, `agam_game` |

### Key CLI (`agamc`)

`build`, `run`, `check`, `new`, `dev`, `daemon`, `fmt`, `test`, `lsp`, `repl`, `doctor`, `cache status`, `package {pack,inspect,run,sdk}`

---

## 4. Active Phases — What's Being Built

### Phase 17B: Deterministic Resolver and Lockfile (completed)

**Goal:** Build deterministic dependency resolution and a stable `agam.lock` format for reproducible builds.

**Done:**
- `DependencySourceKind` enum, `content_hash_directory()`, path/git/registry/workspace resolution
- `resolve_dependencies()` main entry: `WorkspaceSession` → `WorkspaceLockfile` (deterministic, alphabetical)
- Transitive dependency resolution via `resolve_dependency_tables_recursive()`
- `is_lockfile_fresh()`, `generate_or_refresh_lockfile()`, `lockfile_diagnostics()`, `lockfile_content_drift()`
- CLI integration: `agamc build`/`check`/`dev` auto-refresh `agam.lock`
- Explicit `agamc lock` subcommand
- 11 resolver tests covering all source kinds, freshness, determinism, content hashing, transitive resolution, and drift detection
**Detail:** `.agent/phases/details/17B.md`

### Phase 15F: Incremental Daemon & Parallel Compilation (completed)

**Goal:** Keep parsed/typed/lowered state warm across edits; parallelize independent work.

**Done:**
- `WorkspaceSnapshot` + `WorkspaceSnapshotDiff` invalidation contract in `agam_pkg`
- Foreground warm-state daemon loop with per-file AST/HIR/MIR caching
- `DaemonSession` + `IncrementalPipeline` + manifest-aware cache invalidation
- Daemon heartbeat/status at `.agam_cache/daemon/status.json`
- Entry-file warm-state reuse in `agamc dev` (skips re-parse/re-lower)
- Deterministic multi-input `build` request planning + parallel worker scheduling
- Daemon-side entry-file prewarm (fills package/build caches from warm MIR)
- Cross-process reuse of daemon-prewarmed entry packages in `build`/`run`/`pack`
- Multi-file `DaemonWarmIndex` with per-file MIR artifact serialization
- `agamc check`/`build`/`run`/`dev` consume warm index for all workspace files
- Stale MIR artifact self-cleaning + `daemon clear` cleanup
- Parallel `warm_workspace_session` with scoped thread work-stealing
- Background daemon lifecycle: `agamc daemon start`/`stop` with PID lock + sentinel shutdown
- IPC request/response (`127.0.0.1:0` TCP loopback) protocol for strict synchronous daemon queries.

**Detail:** `.agent/phases/details/15F.md`

### Phase 15G: Premium Experience Layer (completed)

**Done:** `agamc doctor`, `agamc new`, `agamc dev`, `agamc cache status`, shared workspace session contract across CLI/LSP/fmt/test/daemon

**Detail:** `.agent/phases/details/15G.md`

### Phase 15H: Native LLVM SDK Distribution (supporting)

**Done:** `agamc package sdk`, bundled LLVM layout, CI workflow skeleton

**Next:** Validate real hosted-runner SDK builds; publish Windows/Linux artifacts; Android target-pack validation

**Detail:** `.agent/phases/details/15H.md`

### Phase 17A: Workspace Contract & Dependency Manifests (completed)

**Done:** `agam.toml` manifest contract frozen at V1Stable, `WorkspaceManifest`/`DependencySpec`/`ToolchainRequirement`/`EnvironmentSpec` data models, manifest validation, `ManifestCompatibility` enum, `resolve_workspace_members`, LSP + formatter + daemon + resolver integration

**Detail:** `.agent/phases/details/17A.md`

### Build Priority Order

15F → 15G → 15H → 17A → 17B (lockfile) → 17C (registry) → 17D (environments) → 17E (distributions) → 17F (std lib)

---

## 5. Key Data Models (quick reference)

### `agam_pkg` (`crates/agam_pkg/src/lib.rs`)

- **`WorkspaceManifest`** — parsed `agam.toml` (project, workspace, dependencies, toolchain, environments)
- **`WorkspaceSession`** — manifest + resolved layout + workspace members
- **`WorkspaceLayout`** — root, manifest path, project name, entry file, source files, test files
- **`WorkspaceSnapshot`** — point-in-time fingerprints of all workspace files for invalidation
- **`WorkspaceSnapshotDiff`** — added/changed/removed/unchanged file lists
- **`PortablePackage`** — verified MIR + runtime metadata (`.agpkg.json`)
- **`SdkDistributionManifest`** — host-native SDK layout (`sdk-manifest.json`)

### `agam_driver` (`crates/agam_driver/src/main.rs`, ~8900 lines)

- **`DaemonSession`** — snapshot + per-file warm-state cache (`BTreeMap<PathBuf, BTreeMap<String, WarmState>>`)
- **`WarmState`** — per-file-version: optional AST Module, HIR, MIR, source features
- **`IncrementalPipeline`** — applies snapshot diffs to the daemon session cache
- **`DaemonStatusRecord`** — persisted daemon health at `.agam_cache/daemon/status.json`
- **`DaemonPrewarmSummary`** — entry-file package/build prewarm readiness

---

## 6. Rules

### Code
- Work in the **smallest responsible crate**. Avoid cross-crate churn.
- Route failures through `agam_errors`. Preserve `SourceId`, `Span`, and debug metadata.
- Avoid `.unwrap()` / `.expect()` in compiler passes.
- Prefer asymptotically optimal time/space complexity; justify tradeoffs explicitly.
- Optimization work requires **measured benchmarks**, not intuition.

### Language
- Agam is **not** Python and **not** Rust. Use real `.agam` files as syntax references.
- ML/tensor features are native compiler/runtime concerns, not wrappers.
- New language features must strengthen simplicity, safety, performance, portability, or AI/ML usability.

### Process
- After completing a slice, update the relevant `.agent/phases/` record.
- If CLI, packaging, or platform support changes, update `README.md`, `info.md`, and `.agent/`.
- Keep agent guidance in `.agent/`; root entrypoints (`CLAUDE.md`, `AGENTS.md`) are pointers, not competing sources.

### Build & Verify
```powershell
cargo check --manifest-path agam/Cargo.toml        # must pass
cargo test --manifest-path agam/Cargo.toml          # must pass
cargo fmt --manifest-path agam/Cargo.toml -- --check  # should pass
```

---

## 7. Repo Layout

```
agam/
├── crates/              # All compiler, runtime, and tooling crates (26 crates)
├── examples/            # Runnable .agam source examples
├── benchmarks/          # Organized benchmark suites, harnesses, CI helpers
├── docs/                # Documentation
├── scripts/             # Build/CI scripts
├── .agent/              # Agent-facing project guidance (see below)
│   ├── phases/          # current.md, next.md, catalog.md, details/
│   │   └── details/     # Per-phase implementation checklists
│   ├── policy/          # Package ecosystem architecture, project overview
│   ├── rules/           # Language guardrails, project structure rules
│   ├── skills/          # benchmark-guard, language-guard
│   ├── include/         # Legacy shared context (now mostly in this file)
│   └── test/            # Localized phase-work benchmark sources
├── CLAUDE.md            # ← You are here
├── AGENTS.md            # Universal agent entrypoint (mirrors this)
├── Cargo.toml           # Workspace manifest
└── README.md            # Public-facing project docs
```

---

## 8. When To Read More

| Question | Read |
|----------|------|
| What exact work remains for a phase? | `.agent/phases/details/{phase}.md` |
| What phase to build next? | `.agent/phases/next.md` |
| Full phase history and catalog? | `.agent/phases/catalog.md` |
| Package/registry/environment architecture? | `.agent/policy/package-ecosystem.md` |
| Syntax questions about `.agam` files? | `examples/*.agam`, `.agent/test/*.agam` |
| Platform/SDK/LLVM toolchain details? | Run `agamc doctor` or read `README.md` |
| Benchmark methodology? | `benchmarks/README.md` |
