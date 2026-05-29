# Agam Compiler — Agent Briefing

> **This file is the single entrypoint.** Read this first. Everything you need for most tasks is here.
> Only open `.agent/phases/details/` when you need the exact checklist for a specific phase.
> **🤖 Unified Multi-AI Workflow:** Whether you are Gemini, Claude, Codex, or another AI, you are operating in a continuous, hand-off rotation. Read the existing context, respect the ongoing phase checklists, and do not invent your own workflows.
> **🪨 Token Efficiency:** Follow `.agent/rules/token-efficiency.md`. Use caveman skill (full intensity) for terse output. Use the MCP Memory server for architectural tracking. Do not read massive files blindly; use progressive discovery.

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

```text
Source → Lexer → Parser → AST → Sema → HIR → MIR → Codegen → Native Binary
                                                         ↘ JIT Runtime
```

### Crate Map

| Layer | Crates |
| --- | --- |
| **Core** | `agam_errors`, `agam_lexer`, `agam_parser`, `agam_ast` |
| **Middle** | `agam_sema` (resolver + type checker), `agam_hir`, `agam_mir` (with `agam_mir::opt`) |
| **Backends** | `agam_codegen` (C/LLVM IR emit), `agam_jit` (Cranelift JIT) |
| **Runtime** | `agam_runtime` (ABI contract, cache store, host detection), `agam_std` |
| **Tooling** | `agam_driver` (`agamc` CLI), `agam_pkg`, `agam_fmt`, `agam_lsp`, `agam_test`, `agam_profile`, `agam_doc`, `agam_debug`, `agam_lint` |
| **Experimental** | `agam_ffi`, `agam_notebook`, `agam_macro`, `agam_smt`, `agam_ui`, `agam_game` |

Physical layout: `crates/{core,middle,backends,runtime,tooling,experiments}/...`

### Key CLI (`agamc`)

`build`, `run`, `check`, `lock`, `new`, `dev`, `daemon`, `fmt`, `test`, `lsp`, `repl`, `exec`, `doctor`, `env`, `publish`, `registry`, `cache status`, `package {pack,inspect,run,sdk}`

---

## 4. Active Phases — What's Being Built

### Phase 15H: Native LLVM SDK Distribution (completed)

**Done:** `agamc package sdk`, bundled LLVM layout, release-ready archives/checksums, release-publish workflow, packaged Android target-pack staging/validation, downloaded-artifact revalidation job, hardened `sdk-dist.yml` for real hosted runners, local E2E validation script

**Detail:** `.agent/phases/details/15H.md`

### Phase 16: Interactive REPL and Headless Execution (completed)

**Done:** buffered `agamc repl`, strict `--json` request/response contract, REPL-owned incremental `DaemonSession` reuse across buffer edits, and in-process JIT/LLVM/C `agamc repl --json` execution with captured `stdout` and buffered diagnostics

**Detail:** `.agent/phases/details/16.md`

### Phase 17A: Workspace Contract & Dependency Manifests (completed)

**Done:** `agam.toml` manifest contract frozen at V1Stable, shared `WorkspaceSession` + `WorkspaceSnapshot`, `resolve_workspace_members`, direct/transitive path-dependency metadata reuse, and manifest validation across CLI/LSP/daemon/resolver flows

**Detail:** `.agent/phases/details/17A.md`

### Phase 17B: Deterministic Resolver and Lockfile (completed)

**Done:** deterministic workspace/path/git/registry resolution, `agam.lock`, automatic lock refresh, content drift diagnostics, and freshness checks that now validate aliases, workspace-member/session metadata, environments, and source/version-selector drift

**Detail:** `.agent/phases/details/17B.md`

### Phase 18: Agent-Facing Execution Tool (partial)

**Done:** dedicated `agamc exec`, strict request/response contract reuse from `agam_notebook`, direct stdin/source/file execution flows, and request-level policy limits for source size, argument size, and native-backend opt-in

**Remaining:** Add stronger OS-level isolation beyond the current request-policy contract

**Detail:** `.agent/phases/details/18.md`

### Phase 19: LangChain and LlamaIndex Wrappers (partial)

**Done:** Rust and Python `agam_ffi` clients/tool wrappers plus optional Python extras and adapter hooks for LangChain and LlamaIndex

**Remaining:** Validate and publish the adapter story against live upstream framework releases

**Detail:** `.agent/phases/details/19.md`

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

### Phase 20: Language Surface Expansion (completed)

**Goal:** Expand parser and syntax to support effects natively.

**Done:** `perform`, `handle`, and `effect` keywords, native parsing, and integration with semantic checker and resolver.

**Detail:** `.agent/phases/details/20.md`

### Phase 21: Runtime Hardening (completed)

**Goal:** Implement OS-level sandboxing for headless execution.

**Done:** Windows `JobObject` enforcement (memory/process limits), Linux `prctl` + `setrlimit` enforcement.

**Detail:** `.agent/phases/details/21.md`

### Phase F6: Indic Grammatical Design Principles (in-progress)

**Goal:** Formalize 7 design principles drawn from Pāṇini's Aṣṭādhyāyī (Sanskrit) and the Tolkāppiyam (Tamil) — the world's oldest formal grammar systems — as a design philosophy that shapes F2–F5.

**Done:** Design specification documents — `design-principles.md` (7 principles), `naming-conventions.md` (30 root verbs), `type-sandhi.md` (7 type composition rules). Phase integrated into Tier 0 roadmap as Pillar 29.

**Remaining:** Cross-reference with F2 (pratyāhāra constraints), F3 (anuvṛtti defaults), F5 (vibhakti roles, dhātu naming)

**Detail:** `.agent/phases/details/F6.md`

### Build Priority Order

F6 (design principles, parallel) → F2 (type system) → F3 (object model) → F4 (modules) → F5 (ergonomics)

---

## 5. Key Data Models (quick reference)

### `agam_pkg` (`crates/tooling/agam_pkg/src/lib.rs`)

- **`WorkspaceManifest`** — parsed `agam.toml` (project, workspace, dependencies, toolchain, environments)
- **`WorkspaceSession`** — manifest + resolved layout + workspace members
- **`WorkspaceLayout`** — root, manifest path, project name, entry file, source files, test files
- **`WorkspaceSnapshot`** — point-in-time fingerprints of all workspace files for invalidation
- **`WorkspaceSnapshotDiff`** — added/changed/removed/unchanged file lists
- **`PortablePackage`** — verified MIR + runtime metadata (`.agpkg.json`)
- **`SdkDistributionManifest`** — host-native SDK layout (`sdk-manifest.json`)

### `agam_driver` (`crates/tooling/agam_driver/src/main.rs`)

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
- **Design principles** from `docs/specification/design-principles.md` (dhātu naming, vibhakti roles, sandhi composition, pratyāhāra constraints) inform all API and type system decisions.

### Process

- After major changes, commit locally. After a final milestone or substantial batch of changes, commit and push to GitHub.
- If CLI, packaging, or platform support changes, update `README.md`, `docs/architecture/project-brief.md`, `info.md`, and `.agent/`.
- Keep agent guidance in `.agent/`; root entrypoints (`CLAUDE.md`, `AGENTS.md`) are pointers, not competing sources.

### Build & Verify

```powershell
cargo check --manifest-path Cargo.toml        # must pass
cargo test --manifest-path Cargo.toml          # must pass
cargo fmt --manifest-path Cargo.toml -- --check  # should pass
```

---

## 7. Repo Layout

```text
agam/
├── crates/
│   ├── core/            # diagnostics, lexer, parser, AST
│   ├── middle/          # sema, HIR, MIR
│   ├── backends/        # C/LLVM codegen and JIT
│   ├── runtime/         # runtime and stdlib
│   ├── tooling/         # CLI, packaging, fmt, LSP, test, profiling
│   └── experiments/     # FFI, notebook, macro, SMT, UI, game
├── examples/            # Runnable .agam source examples
├── benchmarks/          # Organized benchmark suites, harnesses, CI helpers
├── docs/                # Public docs and architecture notes
├── devops/              # Canonical operational automation and runbooks
├── integrations/        # External integration packages (for example Python)
├── fixtures/            # Smoke fixtures and root-level clutter moved out of the top directory
├── scripts/             # Compatibility shims to canonical devops entrypoints
├── justfile             # Human-friendly local task runner
├── .agent/              # Agent-facing project guidance (see below)
│   ├── phases/          # current.md, next.md, catalog.md, details/
│   │   └── details/     # Per-phase implementation checklists

---

## 8. When To Read More

| Question | Read |
| --- | --- |
| What exact work remains for a phase? | `.agent/phases/details/{phase}.md` |
| What phase to build next? | `.agent/phases/next.md` |
| Full phase history and catalog? | `.agent/phases/catalog.md` |
| Package/registry/environment architecture? | `.agent/policy/package-ecosystem.md` |
| Syntax questions about `.agam` files? | `examples/*.agam`, `.agent/test/*.agam` |
| Design principles (naming, type composition)? | `docs/specification/design-principles.md` |
| Stdlib naming conventions? | `docs/specification/naming-conventions.md` |
| Type composition rules (sandhi table)? | `docs/specification/type-sandhi.md` |
| Platform/SDK/LLVM toolchain details? | Run `agamc doctor` or read `README.md` |
| Benchmark methodology? | `benchmarks/README.md` |
| Architecture notes and deep-dives? | `.agent/wiki/` |
| Token efficiency tools? | `.agent/rules/token-efficiency.md` |
