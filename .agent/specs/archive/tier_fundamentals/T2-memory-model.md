# Phase T2-memory-model � Memory Model Decision and Implementation

**Status:** decided (design decision required before implementation)
**Tier:** 2 (Runtime and Concurrency)
**Priority:** THE most important design decision for the language's identity

## Scope

Decide and implement Agam's memory management model. This is the single most consequential design decision in the language — it determines the safety story, the performance ceiling, the ergonomics, and the competitive positioning.

## Design Options

### Option A: Rust-Style Ownership + Borrow Checker
- **Pros:** Zero-cost safety, no GC pauses, proven model, systems-level control
- **Cons:** Steep learning curve, fights the "Python readability" promise, complex compiler implementation (borrow checker is notoriously hard)
- **Who does this:** Rust

### Option B: Swift-Style ARC (Automatic Reference Counting)
- **Pros:** Familiar to OOP developers, predictable performance, simpler than borrow checking
- **Cons:** Reference cycles require weak references, ARC overhead on every copy/move, not truly zero-cost
- **Who does this:** Swift, Objective-C

### Option C: Effect-Based Memory Model
- **Pros:** Novel — leverages Agam's existing effects system, potentially unique value proposition
- **Cons:** Unproven, may confuse developers, research-grade risk
- **Who does this:** No production language yet

### Option D: GC with Escape Analysis
- **Pros:** Simplest for developers, Go/Java proven model, compile-time optimization can reduce GC pressure
- **Cons:** GC pauses (even if rare), not suitable for real-time or embedded, may conflict with Agam's performance promise
- **Who does this:** Go, Java, C#, OCaml

### Recommended: Hybrid ARC + Value Semantics (Option B+)
- Default to value semantics for structs (like Swift)
- ARC for heap-allocated reference types
- Escape analysis to elide ARC operations where possible
- Optional `@move` annotation for zero-copy semantics on hot paths
- No mandatory borrow checker, but optional strict mode for systems code

## Deliverables (once decision is made)

### Core Memory Model
- [ ] Memory ownership rules formalized and documented
- [ ] Stack vs heap allocation decisions
- [ ] Copy/move semantics for value types
- [ ] Reference semantics for reference types
- [ ] Destruction/drop semantics (deterministic cleanup)

### Compiler Implementation
- [ ] Sema pass for ownership/lifetime/reference tracking
- [ ] HIR representation of memory operations
- [ ] MIR optimization for memory operations (elide redundant copies, ARC operations)
- [ ] Codegen for allocation, deallocation, reference counting, or GC barriers

### Safety Guarantees
- [ ] No use-after-free (compile-time or runtime enforcement)
- [ ] No double-free
- [ ] No dangling references
- [ ] Data race freedom in concurrent code (integrates with R2)
- [ ] Clear unsafe escape hatch for FFI and low-level code

### Standard Library Alignment
- [ ] Collection types use the chosen memory model consistently
- [ ] String type with clear ownership semantics
- [ ] Smart pointer types if applicable (Box, Rc, Arc equivalents)

## Responsible Crates

- `agam_sema` — ownership/lifetime/reference analysis
- `agam_hir` — memory operation representation
- `agam_mir` — memory optimization passes
- `agam_codegen` — allocation/deallocation/ARC emission
- `agam_runtime` — runtime memory management (ARC, GC, or allocator)

## Dependencies

- Phase T0-type-system (type system) — generics needed for smart pointer types
- Phase T0-object-model (object model) — struct semantics depend on memory model

## Test Strategy

- Safety tests: programs that would cause use-after-free, double-free must be rejected
- Performance tests: measure overhead of chosen model vs raw allocation
- Interop tests: memory model works correctly across FFI boundaries
