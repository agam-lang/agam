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
│   ├── specs/active/details/                # 77 phase spec files
│   ├── specs/active/catalog.md              # Full tier breakdown
│   ├── specs/archive/                       # Completed specs + INDEX.md
│   ├── skills/                              # Agent skills (caveman, cargo-lens, etc.)
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
- **All tests pass** (verified 2026-08-16, exit code 0)
- **Zero `todo!()` or `unimplemented!()`** remaining in core/middle/backends

---

## 2. Completed Phases (Recent)

| Phase | What Was Built |
|---|---|
| T0-type-system A–F | Option/Result, enums, match, struct fields, destructuring, generics, try operator |
| T0-object-model | `impl` blocks, `self` receiver, method dispatch |
| T0-module-system | Selective/wildcard imports, scope resolution |
| T0-stdlib-io | Network, Environment, Process native modules + 28 effect handlers |
| T0-effects-depth | F-string expression interpolation (`f"hello {name}"`) |
| Code hygiene | `cargo fmt --all`, CRLF normalization |

---

## 3. Recommended Next Phases (Priority Order)

### Phase A: T0-type-system — Remaining Gaps
**Status:** `current.md` says "open" for generic inference & const generics.
**What's done:** Phases A–F complete (enums, match, structs, generics param resolution, try operator).
**What's NOT done:**
- [ ] **Full type inference** — currently types must be annotated; infer from context
- [ ] **Const generics** — `[T; N]` where N is a compile-time constant
- [ ] **Generic constraint checking** — `where T: Add + Clone` enforcement in sema
- [ ] **Monomorphization completeness** — `agam_mir/src/monomorphize.rs` exists but needs edge cases

**Crates:** `agam_sema` (type checker), `agam_hir` (lower), `agam_mir` (monomorphize)

---

### Phase B: T0-effects-depth — Remaining Syntax Features
**Status:** F-string interpolation done. Many items remain from the original spec.
**What's NOT done:**
- [ ] **Closures/Lambdas** — `|x, y| x + y` syntax (AST node `ExprKind::Lambda` exists but parser/HIR/MIR incomplete)
- [ ] **Range expressions in for loops** — `for i in 0..n` (AST `ExprKind::Range` exists, needs HIR/MIR)
- [ ] **Named arguments** — `connect(host: "localhost", port: 8080)`
- [ ] **Default parameter values** — `fn connect(host: String, port: i32 = 80)`
- [ ] **Operator overloading** — via trait `impl Add for Vector { ... }`
- [ ] **Expression-oriented blocks** — last expression = return value

**Crates:** `agam_parser`, `agam_hir/lower.rs`, `agam_mir/lower.rs`

---

### Phase C: T1-error-messages — Diagnostic Quality
**Spec:** `.agent/specs/active/details/T1-error-messages.md`
- [ ] Multi-span error rendering with color
- [ ] Nyāya 4-part proof schema (Fact, Reason, Fix, Law)
- [ ] Single-pass multi-error recovery in parser

**Crates:** `agam_errors`, `agam_parser`

---

### Phase D: T1-compiler-agent-tool — MCP Server
**Spec:** `.agent/specs/active/details/T1-compiler-agent-tool.md`
- [ ] `agamc mcp serve` — Model Context Protocol server
- [ ] Structured `--json` diagnostics on all commands
- [ ] SARIF output format

**Crates:** `agam_driver`, `agam_errors`

---

## 4. Essential Conventions

### Build & Test Commands
```powershell
# Check compilation
cargo check --manifest-path agam\Cargo.toml

# Run all tests
cargo test --manifest-path agam\Cargo.toml --message-format=short

# Test specific crates
cargo test --manifest-path agam\Cargo.toml -p agam_parser -p agam_hir --message-format=short

# Format
cargo fmt --all --manifest-path agam\Cargo.toml
```

### Git Workflow
```powershell
# Always commit from agam/ subdirectory
cd c:\Users\ksvik\Projects\Agam-Lang\agam
git add . && git commit -m "feat(scope): description"

# Push via multi-repo script
python ..\push_repos.py
```

### Commit Message Prefixes
- `feat(parser):` — new language features
- `feat(std):` — stdlib additions
- `feat(syntax):` — parser/lexer changes
- `fix(sema):` — bug fixes in semantic analysis
- `style:` — formatting only
- `docs(spec):` — spec updates, archival

### Post-Completion Checklist
After completing any phase:
1. ✅ `cargo check` passes
2. ✅ `cargo test` passes (all 27 crates)
3. ✅ `cargo fmt --all` applied
4. ✅ Update `agam/.agent/memory/execution.log` with `[FEATURE]` entry
5. ✅ Update `.agent/memory/execution.log` (root copy)
6. ✅ Archive spec: move from `details/` → `archive/`, update `archive/INDEX.md`
7. ✅ Update `current.md` status from `open` → `complete`
8. ✅ Git commit + push

---

## 5. Skills System

Skills auto-load from `.agent/skills/`. **Do NOT add `@` directives to GEMINI.md** (causes duplicate token expansion).

| Skill | When | What |
|---|---|---|
| `caveman` | Always active | Terse output, ~75% token savings |
| `cargo-lens` | On build errors | Filter cargo output to errors/warnings only |
| `language-guard` | On syntax code | Prevent Python/Rust syntax assumptions in Agam code |
| `benchmark-guard` | On optimization | Require before/after benchmarks |
| `spec-archiver` | On phase completion | Archive specs to `.agent/specs/archive/` |

---

## 6. Architecture Patterns to Follow

### Adding a New Language Feature (end-to-end)
1. **Lexer** (`agam_lexer/src/lexer.rs`): Add `TokenKind` if new keyword
2. **Parser** (`agam_parser/src/parser.rs`): Parse into `ExprKind`/`StmtKind`/`DeclKind`
3. **AST** (`agam_ast/src/`): Add AST node types if needed
4. **HIR Lower** (`agam_hir/src/lower.rs`): `lower_expr` / `lower_stmt` match arm
5. **HIR Types** (`agam_hir/src/lib.rs`): Add `HirExprKind` variant
6. **Sema** (`agam_sema/src/`): Type checking, resolver entries
7. **MIR Lower** (`agam_mir/src/lower.rs`): Generate MIR `Op` instructions
8. **MIR Types** (`agam_mir/src/lib.rs`): Add `Op` variant if needed
9. **Backends**: Update C emitter, LLVM emitter, JIT (if applicable)
10. **Tests**: Parser test + HIR test + (optionally) MIR test

### Adding a New Effect
1. **`agam_sema/src/effects.rs`**: Define `std_xxx_effect() -> EffectDef`
2. **`agam_sema/src/effects.rs`**: Register in `EffectRegistry::register_std_effects()`
3. **`agam_std/src/xxx.rs`**: Implement native module
4. **`agam_std/src/effects.rs`**: Add handler functions + register in `register_all_builtin_handlers()`
5. **`agam_std/src/lib.rs`**: Export `pub mod xxx;`

### Key Type Structures
- `agam_ast::expr::ExprKind` — all expression variants
- `agam_hir::HirExprKind` — typed expression variants
- `agam_mir::Op` — SSA instruction opcodes
- `agam_runtime::effects::EffectValue` — runtime effect argument types (Unit, Bool, Int, Float, String, List)
- `agam_sema::effects::EffectDef` — semantic effect definitions
- `agam_sema::effects::EffectRegistry` — effect registration and lookup

---

## 7. Known Gotchas

1. **Path delimiters**: Agam supports both `.` and `::` as path separators. Parser must handle both.
2. **`parse_path` peek-ahead**: When parsing selective imports `import a.b::{c, d}`, `parse_path` must NOT consume `::` or `.` when followed by `{` or `*`.
3. **`process::exit() -> !`**: The effect handler wrapper has signature `-> Result<EffectValue, EffectError>` but `!` coerces. This is intentional.
4. **`env::set_var` / `env::remove_var`**: Require `unsafe {}` blocks since Rust 1.66.
5. **Duplicate skill loading**: Never put `@./.agent/skills/...` in `GEMINI.md` — Antigravity auto-discovers skills and `@` causes double loading.
6. **Windows line endings**: Git on Windows may introduce CRLF. Run `cargo fmt --all` after edits.
7. **`push_repos.py`**: Pushes all org repos. Non-`agam` repos may reject (fetch first). Only `agam` repo matters.

---

## 8. File Quick Reference

| Need | File |
|---|---|
| Parser entry | `agam/crates/core/agam_parser/src/parser.rs` |
| Lexer/tokens | `agam/crates/core/agam_lexer/src/lexer.rs` + `token.rs` |
| AST expressions | `agam/crates/core/agam_ast/src/expr.rs` |
| AST patterns | `agam/crates/core/agam_ast/src/pattern.rs` |
| HIR lowering | `agam/crates/middle/agam_hir/src/lower.rs` |
| MIR lowering | `agam/crates/middle/agam_mir/src/lower.rs` |
| Type checker | `agam/crates/middle/agam_sema/src/checker.rs` |
| Resolver | `agam/crates/middle/agam_sema/src/resolver.rs` |
| Effects (sema) | `agam/crates/middle/agam_sema/src/effects.rs` |
| Effects (runtime) | `agam/crates/runtime/agam_std/src/effects.rs` |
| Monomorphization | `agam/crates/middle/agam_mir/src/monomorphize.rs` |
| C backend | `agam/crates/backends/agam_codegen/src/c_emitter.rs` |
| LLVM backend | `agam/crates/backends/agam_codegen/src/llvm_emitter.rs` |
| JIT backend | `agam/crates/backends/agam_jit/src/lib.rs` |
| Execution log | `agam/.agent/memory/execution.log` |
| Phase specs | `.agent/specs/active/details/*.md` |
| Next priorities | `.agent/specs/active/next.md` |
| Current status | `.agent/specs/active/current.md` |

---

## 9. Execution Pattern Template

When you start a phase, follow this exact pattern:

```
1. Read the spec: .agent/specs/active/details/T<tier>-<name>.md
2. Read relevant source files for the feature area
3. Implement changes (parser → AST → HIR → MIR → backend)
4. Add unit tests in the modified files
5. cargo check --manifest-path agam\Cargo.toml
6. cargo test --manifest-path agam\Cargo.toml -p <changed_crates>
7. cargo test --manifest-path agam\Cargo.toml (full suite)
8. cargo fmt --all --manifest-path agam\Cargo.toml
9. git add . && git commit -m "feat(scope): description"
10. Update execution.log (both copies)
11. Archive spec if phase complete
12. python ..\push_repos.py
```

---

> **Start with `.agent/specs/active/next.md` to pick your phase. Read the spec. Build it. Test it. Ship it.**
