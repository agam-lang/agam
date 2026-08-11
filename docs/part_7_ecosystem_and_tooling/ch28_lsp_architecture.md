# Chapter 28: Language Server Protocol (LSP) Architecture

> **Part VII: Advanced Tooling, Testing & Ecosystem Engineering**  
> **Compiler Module Focus**: [`agam_lsp`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_lsp)

---

## 28.1 Overview of the Language Server Protocol

The **Language Server Protocol (LSP)** standardizes communication between code editors (VS Code, Neovim, Visual Studio, IntelliJ) and programming language compilers.

`agam_lsp` implements the LSP JSON-RPC server specification over stdin/stdout or TCP loopback, allowing IDEs to query compiler state in real time as developers edit files.

```text
  IDE / Text Editor (VS Code / Neovim)
                   │
                   ▼  JSON-RPC 2.0 (Requests / Notifications)
      ┌───────────────────────────┐
      │  agam_lsp Server Engine   │
      └─────────────┬─────────────┘
                    │
                    ▼  Queries Warm State
      ┌───────────────────────────┐
      │  DaemonSession / Incremental│
      └───────────────────────────┘
```

---

## 28.2 Key LSP Features Implemented in `agam_lsp`

1. **`textDocument/publishDiagnostics`**: Pushes real-time type errors, syntax warnings, and unhandled effect diagnostics to the editor canvas upon every keypress.
2. **`textDocument/hover`**: Provides hover tooltips displaying function signatures, variable inferenced types, and docstrings.
3. **`textDocument/definition`**: Navigates from symbol references directly to their source definition locations (`Span`).
4. **`textDocument/completion`**: Offers contextual autocomplete suggestions for struct fields, module functions, and keywords.
