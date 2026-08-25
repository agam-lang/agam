# Agent Handoff — Agam Compiler Development

> **Read this before writing any code.** This document is the complete briefing for continuing Agam compiler development.

---

## 1. Project State (as of 2026-08-16)

### What Agam Is
Agam is a **compiled systems language** with algebraic effects, GPU kernels, and AI-native features. The compiler (`agamc`) targets LLVM, C, JIT, and NVPTX backends. Written in Rust.

### Repository Layout
```
c:\Users\ksvik\Projects\Agam-Lang\          # Organization root
├── agam/                                    # Main compiler repo (Rust workspace)
│   ├── crates/
│   │   ├── core/       agam_ast, agam_parser, agam_lexer, agam_errors
│   │   ├── middle/     agam_hir, agam_mir, agam_sema
│   │   ├── backends/   agam_codegen (C/LLVM/NVPTX), agam_jit
│   │   ├── runtime/    agam_runtime, agam_std
│   │   ├── tools/      agam_driver, agam_lsp, agam_fmt, agam_lint, agam_test
│   │   └── ...         agam_pkg, agam_ffi, agam_notebook, agam_game, etc.
│   ├── Cargo.toml                           # Workspace manifest
│   └── .agent/memory/execution.log          # Execution history
├── .agent/                                  # Organization-level agent config
│   ├── specs/active/current.md              # Active workstream tracker
│   ├── specs/active/next.md                 # Recommended next phases
│   ├── specs/active/details/                # Spec files
│   ├── specs/active/catalog.md              # Full tier breakdown
│   ├── specs/archive/                       # Completed specs + INDEX.md
│   ├── skills/                              # Agent skills (caveman, compiler-harden, etc.)
│   ├── rules/                               # Coding rules
│   └── memory/execution.log                 # Root execution history
├── AGENTS.md, GEMINI.md, CLAUDE.md          # Agent briefing files
└── push_repos.py                            # Multi-repo push script
```

### Compiler Pipeline
```
Source → Lexer → Parser → AST → HIR (typed) → MIR (SSA) → Backend (C/LLVM/JIT/NVPTX)
            ↓                        ↓              ↓
        agam_lexer            agam_sema        agam_codegen
        agam_parser           agam_hir         agam_jit
                              agam_mir
```

### Current Test Stats
- **27 crates** in workspace
- **100% tests pass** across all 27 crates (verified 2026-08-16, exit code 0)
- **Zero compiler warnings** on `cargo clippy --all-targets -- -D warnings`
- **Zero `todo!()` or `unimplemented!()`** remaining in core/middle/backends

---

## 2. Completed Phases & Capabilities (Current Status)

| Phase / Stream | What Was Built | Status |
|---|---|---|
| **T0-type-system (A–F)** | Option/Result, enums, match guards/or-patterns/destructuring, struct fields, generics, try `?` operator | **Complete** |
| **T0-object-model** | Struct `impl` blocks, `self` receiver, method call dispatch (`Type::method`) | **Complete** |
| **T0-module-system** | Selective imports (`import path::{A, B as C}`), wildcard imports (`import path::*`), scope resolution | **Complete** |
| **T0-stdlib-io** | Native Network, Environment, Process modules + 28 effect handlers in `agam_std` & `agam_sema` | **Complete** |
| **T0-effects-depth** | F-strings (`f"hello {name}"`), ranges (`..`, `..=`), counting `for` loops, closures & lambda lowering | **Complete** |
| **T0-type-sandhi-graph** | `TraitLattice`, `SandhiGraph` harmonic lattice, `MonomorphGraph` cycle detection & topological sorting | **Complete** |
| **T1-compiler-agent-tool** | Native MCP server (`agamc mcp serve`), tools (check, format, explain_error, ast_inspect, sarif_diagnostics, run) | **Complete** |
| **T1-error-messages** | Nyāya 4-part proofs (*Pratijñā, Hetu, Udāharaṇa, Nigamana*), Hankel moment matrix root solvers, SARIF export | **Complete** |
| **Compiler Hardening & Fixes** | 5 hardening sweeps: if-else expressions, trait impl syntax, JIT indirect calling (`func_addr`/`call_indirect`), qualified enum matching, 26 end-to-end integration test suites in `agam_test` | **Complete** |

**All 9 foundational phases of Tier 0 are 100% complete.**

---

## 3. Recommended Next Phases for Tomorrow (Priority Order)

### Phase 1: T3-gpu-target-adapter — Universal GPU Target Adapter Interface
**Spec:** `.agent/specs/active/details/T3-gpu-target-adapter.md`
- Abstract `GpuTargetAdapter` trait in `agam_codegen` to decouple target-agnostic GPU MIR lowering from target assembly generation.
- Enable AMDGPU (ROCm/HIP), SPIR-V (Vulkan/oneAPI), and Metal adapters alongside existing NVPTX backend.

### Phase 2: T1-lsp-production — Production LSP Quality
**Spec:** `.agent/specs/active/details/T1-lsp-production.md`
- Implement go-to-definition, hover documentation, completion, and workspace diagnostics in `agam_lsp`.

### Phase 3: T1-sdk-distribution — Hosted SDK Release Packaging
**Spec:** `.agent/specs/active/details/T1-sdk-distribution.md`
- Verify Windows, Linux, and Android SDK distribution bundles on GitHub Actions.

### Phase 4: T1-doc-generation — Searchable HTML Documentation
**Spec:** `.agent/specs/active/details/T1-doc-generation.md`
- Implement `agamc doc` producing searchable, cross-linked HTML documentation for Agam projects and stdlib.

---

## 4. Essential Conventions

### Build & Test Commands
```powershell
# Check compilation
cargo check --manifest-path agam\Cargo.toml

# Run all tests
cargo test --manifest-path agam\Cargo.toml

# Run linter
cargo clippy --all-targets --manifest-path agam\Cargo.toml -- -D warnings

# Format
cargo fmt --all --manifest-path agam\Cargo.toml -- --check
```

### Git Workflow
```powershell
# Always commit from agam/ subdirectory
cd c:\Users\ksvik\Projects\Agam-Lang\agam
git add . && git commit -m "feat(scope): description"

# Push via multi-repo script
python ..\push_repos.py
```

### Post-Completion Checklist
After completing any phase:
1. ✅ `cargo check` passes
2. ✅ `cargo test` passes (all 27 crates)
3. ✅ `cargo clippy --all-targets -- -D warnings` (0 warnings)
4. ✅ `cargo fmt --all -- --check`
5. ✅ Update `agam/.agent/memory/execution.log` & `.agent/memory/execution.log` with `[FEATURE]` / `[HARDEN]` entries
6. ✅ Update `catalog.md` and `current.md` status from `open` → `complete`
7. ✅ Git commit + push via `python push_repos.py`

---

## 5. Skills System

Skills auto-load from `.agent/skills/`.

| Skill | Purpose | Trigger |
|---|---|---|
| `compiler-harden` | 5-step workspace audit, bug fix & test expansion | `/harden` / `/audit-sweep` / `harden` |
| `caveman` | ~75% token cut via terse output | Auto |
| `cargo-lens` | Compress build logs | Auto on build |
| `language-guard` | Prevent Python/Rust syntax drift | Auto on syntax |
| `benchmark-guard` | Validate performance claims | Auto on optimization |
| `spec-archiver` | Archive completed specs | Auto on phase close |
