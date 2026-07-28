# Future Ecosystem and Tooling Priorities

While the Agam compiler backend and grammatical foundations are world-class, the surrounding ecosystem requires significant investment to reach parity with established languages like Rust or Python.

Agents working on Agam must prioritize the following long-term areas whenever possible:

## 1. The Ecosystem Void
- **Current State:** `agam_pkg` exists, but there is no centralized, community-driven registry (like PyPI or crates.io).
- **Future Need:** Expand the standard library and build a robust, secure package ecosystem. Data science users need pre-built packages (like pandas or SQL drivers) rather than writing them from scratch.

## 2. Error Message Quality
- **Current State:** `agam_errors` is functional but can emit raw or confusing diagnostics, especially when dealing with complex Sandhi (type junctions) or generics.
- **Future Need:** Emulate Rust's best-in-class error messages. Diagnostics must point exactly to the issue and offer actionable fix suggestions. Make the compiler "friendly" when rejecting code.

## 3. Smart Garbage Collector for `@lang.base`
- **Current State:** The strict memory model shines in `@lang.advance`. However, data scientists using `@lang.base` expect Python-like memory management.
- **Future Need:** We are planning a Hybrid ARC (Automatic Reference Counting) model. This must be perfectly optimized to prevent memory leaks in circular references (e.g., Object A -> Object B -> Object A).

## 4. Tooling Integration (IDE Support)
- **Current State:** Basic `agam_lsp` (Language Server Protocol) exists.
- **Future Need:** Achieve TypeScript/C# levels of IDE integration. Features like "Go to Definition", "Find All References", and "Ghost Text" auto-insertion of role labels (e.g., `copy(from: a, to: b)`) must work flawlessly in VS Code and other editors.
