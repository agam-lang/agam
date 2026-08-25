# Phase T1: Ecosystem & Tooling Deficits

**Goal:** Address the major ecosystem, tooling, and memory management gaps preventing Agam from reaching parity with mature languages (like Rust, Python, TypeScript) in production environments.

## 1. The Ecosystem Void
- **Current State:** `agam_pkg` exists for local dependency resolution, but there is no centralized, community-driven package registry (no PyPI/crates.io equivalent).
- **Future Implementation:** 
  - Deploy a global package registry and authentication system.
  - Expand the standard library with high-demand `@lang.base` features (e.g., dataframes, SQL drivers, HTTP clients).
  - Create standard frameworks for web, data science, and ML to prevent users from starting from scratch.

## 2. Error Message Quality
- **Current State:** `agam_errors` is functional but emits raw, compiler-centric diagnostics. Failures in complex Sandhi (type junctions) or generics are often opaque to beginners.
- **Future Implementation:**
  - Emulate Rust-tier diagnostics: Point exactly to the problematic span with rich context.
  - Provide actionable, inline fix suggestions ("Did you mean...?", "Consider adding a constraint here...").
  - Make the compiler actively friendly when rejecting code.

## 3. Smart Garbage Collector for `@lang.base`
- **Current State:** The strict memory model works perfectly for `@lang.advance` systems programming, but data scientists expect Python-like memory management in `@lang.base`.
- **Future Implementation:**
  - Implement and perfectly optimize the planned Hybrid ARC (Automatic Reference Counting) model.
  - Introduce cycle-detection or a minimal tracing collector fallback to prevent memory leaks in circular references (e.g., `A -> B -> A`), ensuring `@lang.base` feels seamless.

## 4. Tooling Integration (IDE Support)
- **Current State:** A basic `agam_lsp` (Language Server Protocol) implementation exists.
- **Future Implementation:**
  - Achieve flawless VS Code / IntelliJ integration on par with TypeScript or C#.
  - Build robust support for "Go to Definition", "Find All References", and intelligent autocomplete.
  - Implement "Ghost Text" auto-insertion of Vibhakti role labels (e.g., auto-filling `from:` and `to:` in IDEs) to enhance readability without forcing the user to type them.
