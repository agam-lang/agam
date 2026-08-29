# Chapter 27: Testing Methodologies, Fuzzing & Differential Verification

> **Part VII: Advanced Tooling, Testing & Ecosystem Engineering**  
> **Compiler Module Focus**: [`agam_test`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_test)

---

## 27.1 Multi-Tier Compiler Testing Framework

Compiler bugs can manifest as incorrect diagnostic reporting, silent code miscompilation, or unexpected crashes during code generation. `agam_test` enforces a multi-tier verification strategy:

```text
 ┌─────────────────────────────────────────────────────────────┐
 │                1. Unit Tests (Rust `#[test]`)               │
 │  Validates individual passes (Lexer, Parser, Sema, MIR Opt) │
 └──────────────────────────────┬──────────────────────────────┘
                                │
                                ▼
 ┌─────────────────────────────────────────────────────────────┐
 │           2. End-to-End Integration Test Suite            │
 │  Executes `.agam` test fixtures against expected stdout     │
 └──────────────────────────────┬──────────────────────────────┘
                                │
                                ▼
 ┌─────────────────────────────────────────────────────────────┐
 │          3. Differential Testing & AST Fuzzing              │
 │  Compares JIT results against LLVM native compiled binary   │
 └─────────────────────────────────────────────────────────────┘
```

---

## 27.2 Integration Test Harness (`agam_test`)

Integration tests use inline test annotations inside `.agam` files:

```agam
// RUN: agamc run %s | FileCheck %s
// CHECK: Calculated Result: 150

fn main() {
    let a = 100;
    let b = 50;
    println("Calculated Result: " + (a + b).to_string());
}
```

The test runner compiles each fixture, executes the generated binary, and compares `stdout` against `CHECK` directives.

---

## 27.3 Differential Verification

`agam_test` verifies correctness across different execution backends:

$$\text{Evaluate(Source, Backend::JIT)} \stackrel{?}{=} \text{Evaluate(Source, Backend::LLVM\_Native)}$$

If the Cranelift JIT engine produces a result that differs from the native LLVM machine executable, a differential test failure is flagged.
