# Agam Compiler — Agent Briefing

> **Compiler Core Briefing.** Read this first when working inside `agam/`.  
> **Workspace Authority:** All global rules, skills, and memory live at workspace root [`../.agent/`](../.agent/).  
> **Syntax Reality:** Always check [`../note.md`](../note.md) and [`../examples/`](../examples/) before writing or documenting Agam code. Test via `python ../scripts/prove.py`.  
> **Live Problem Ledger:** Log all bugs, lowering gaps, and crashes in [`../issues.md`](../issues.md).

---

## 1. What Agam Is

Agam is a next-generation compiled systems language implemented as a 27-crate Rust workspace. It combines Python-level ergonomics, mathematical rigor, and native LLVM/C execution performance with zero-panic reliability.

**Agam is its own distinct language.** It is not Python and not Rust. Verified syntax:
- Keywords: `let`, `mut`, `fn`, `if`, `else`, `while`, `for`, `in`, `return`, `struct`, `enum`, `type`, `import`, `export`, `break`, `continue`, `defer`.
- Primitive types: `i32`, `i64`, `f32`, `f64`, `bool`, `str`, `void`.
- Collections: dynamic arrays `[T]` with `.len()`, `.push(val)`, `.pop()`.

---

## 2. Compiler Architecture & Pipeline

```text
Source (.agam)
   │
   ▼
agam_lexer ──► Tokens
   │
   ▼
agam_parser ──► AST (agam_ast)
   │
   ▼
agam_sema ──► HIR (agam_hir) ── Type Checking & Symbol Resolution
   │
   ▼
agam_mir ──► MIR & Optimization Passes (inline, const fold, dce, loop unroll)
   │
   ├──► agam_codegen (LLVM AOT / C emit) ──► Native Object / Binary
   └──► agam_jit (Cranelift JIT) ──► In-Process Execution
```

### Crate Map (27 Crates)

| Layer | Crates |
|---|---|
| **Core** | `agam_errors`, `agam_lexer`, `agam_parser`, `agam_ast` |
| **Middle** | `agam_sema` (resolver + type checker), `agam_hir`, `agam_mir` (with `opt`) |
| **Backends** | `agam_codegen` (LLVM IR / C emit), `agam_jit` (Cranelift JIT) |
| **Runtime** | `agam_runtime` (ABI contract, memory allocator, host detection), `agam_std` |
| **Tooling** | `agam_driver` (`agamc` CLI), `agam_pkg`, `agam_fmt`, `agam_lsp`, `agam_test`, `agam_profile`, `agam_doc`, `agam_debug`, `agam_lint` |
| **Experimental** | `agam_ffi`, `agam_notebook`, `agam_macro`, `agam_smt`, `agam_ui`, `agam_game` |

Physical layout: `crates/{core,middle,backends,runtime,tooling,experiments}/...`

### Key CLI (`agamc`)

`agamc {build, run, check, lock, new, dev, daemon, fmt, test, lsp, repl, exec, doctor, env, publish, registry, cache, package}`

---

## 3. Active Roadmap & Directives

- **Active Stage:** **Stage 0: Crate Decoupling, Inverted Driver Modularization & Zero-Panic** ([`.agent/specs/active/details/STAGE-00-driver-modularization-and-hardening.md`](../.agent/specs/active/details/STAGE-00-driver-modularization-and-hardening.md))
- **Next Stage:** **Stage 3: Direct Syscall & OS Subsystem Engine**
- **Roadmap Overview:** [`.agent/specs/active/current.md`](../.agent/specs/active/current.md)
- **Influencing Audits:** [`../docs/AUDIT-optimizer-pipeline-honesty-2026-09-05.md`](../docs/AUDIT-optimizer-pipeline-honesty-2026-09-05.md), [`../docs/RFC-memory-model-default.md`](../docs/RFC-memory-model-default.md).

---

## 4. Engineering Invariants

1. **Smallest Responsible Crate:** Never dump shared logic into `agam_driver`. Extract to appropriate crates (`crates/tooling/`, `crates/core/`).
2. **Zero-Panic Rule:** Never use `.unwrap()`, `.expect()`, or `panic!()` in compiler passes or library code. Return `Result<T, Diagnostic>`.
3. **Dual-Backend Parity:** Every language feature must produce bitwise/semantic parity between Cranelift JIT and LLVM AOT.
4. **Token Hygiene:** Always use `python ../scripts/cargo_lens.py <check|test|build>` to avoid flooding agent context with raw compiler output.
5. **Continuous Verification:**
   ```powershell
   python ../scripts/prove.py                # Verify 8 canonical examples across JIT & LLVM
   python ../scripts/cargo_lens.py check      # Check 27 crates with zero warnings
   python ../scripts/unwrap_ratchet.py        # Ensure unwrap ratchet strictly decreases
   python ../scripts/diff_fuzz.py             # Differential JIT vs LLVM parity test
   ```
