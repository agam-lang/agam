# Chapter 15: End-to-End Agam Compiler Pipeline Walkthrough

> **System Scope**: Full Agam Compiler Lifecycle & Driver Architecture  
> **Compiler Module Focus**: [`agam_driver`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_driver), [`agam_pkg`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_pkg)

---

## 15.1 Complete Source-to-Binary Execution Flow

The `agamc` CLI orchestrates the full compilation lifecycle:

```text
 1. Source Code (.agam)
        │
        ▼
 2. Lexer (`agam_lexer`)       -> Spans & Token Stream
        │
        ▼
 3. Parser (`agam_parser`)     -> Abstract Syntax Tree (`agam_ast`)
        │
        ▼
 4. Sema (`agam_sema`)         -> Type Checked & Scope Resolved AST
        │
        ▼
 5. HIR (`agam_hir`)           -> Pattern Match & Desugared AST
        │
        ▼
 6. MIR (`agam_mir`)           -> Basic Blocks, SSA Form, CFG
        │
        ▼
 7. Opt (`agam_mir::opt`)      -> DCE, Inlining, Constant Folding
        │
        ├──────────────────────────────┐
        ▼                              ▼
 8. Codegen (`agam_codegen`)    9. JIT Engine (`agam_jit`)
        │                              │
        ▼                              ▼
 Native Binary (.exe / elf)     In-Process JIT Execution
```

---

## 15.2 Driver Coordination & Command CLI (`agam_driver`)

The CLI entrypoint (`agamc`) handles key developer workflows:

| CLI Command | Action Executed | Primary Crate Targets |
| :--- | :--- | :--- |
| `agamc build` | Complete compilation to native binary executable | `agam_driver` $\rightarrow$ `agam_codegen` |
| `agamc run` | Build and execute target binary | `agam_driver` $\rightarrow$ `agam_runtime` |
| `agamc check` | Fast type checking and diagnostic verification | `agam_lexer` $\rightarrow$ `agam_sema` |
| `agamc repl` | Interactive REPL buffer execution | `agam_driver` $\rightarrow$ `agam_jit` |
| `agamc dev` | Incremental warm-daemon execution loop | `agam_driver` $\rightarrow$ `DaemonSession` |
| `agamc exec` | Sandboxed headless agent execution | `agam_driver` $\rightarrow$ `agam_notebook` |
| `agamc doctor` | Host system LLVM and C toolchain verification | `agam_driver` $\rightarrow$ `agam_runtime` |
