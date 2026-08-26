# Agam Memory Model Specification

**Specification Version**: `0.1.0-alpha.1`  
**Status**: Formal Architectural Specification  

---

## 1. Overview & Design Philosophy

Agam employs a **two-tier hybrid memory management architecture** designed to balance high-level developer ergonomics (comparable to Python/Swift) with zero-cost systems control (comparable to Rust/C++):

1. **Tier 1 (Default `@lang.base`)**: **Automatic Reference Counting (ARC)** with Copy-on-Write (CoW) semantics. Values are ref-counted at runtime, eliminating manual lifetime annotations and borrow-checker friction for application code.
2. **Tier 2 (Systems `@lang.advance` / `strict`)**: **Lexically Scoped Affine Types**. Inside explicit `strict { ... }` blocks, moves are affine (single-ownership, used at most once), mutation requires exclusive access, and reference counting is bypassed.

Agam **does not use a tracing, stop-the-world garbage collector (GC)**. All deallocations are deterministic.

---

## 2. The Two Memory Tiers

```
┌────────────────────────────────────────────────────────────────────────┐
│                        AGAM MEMORY HIERARCHY                           │
├───────────────────────────────────┬────────────────────────────────────┤
│   Tier 1: Default (ARC & CoW)     │  Tier 2: Strict Affine Regions     │
│   • Move becomes copy (refcount++)│  • Move transfers ownership (O(0)) │
│   • Shared pointers by default    │  • Single-owner invariant          │
│   • Deterministic drop on zero    │  • Compile-time lifetime check     │
│   • Python/Swift-class ergonomics │  • Rust-class zero-cost throughput │
└───────────────────────────────────┴────────────────────────────────────┘
```

### 2.1 Tier 1: ARC Default Semantics

In standard Agam code (`@lang.base` and unmarked `@lang.advance` blocks):
- **Scalar Primitives** (`i8`..`i64`, `u8`..`u64`, `f32`, `f64`, `bool`) are copied on the stack with zero overhead.
- **Heap Structures & Aggregates** (dynamic arrays, strings, structs, tensors) are wrapped in atomic reference-counted headers.
- **Assignment & Parameter Passing**: Passing a heap structure increments the reference count.
- **Deterministic Teardown**: When the reference count reaches zero, the memory is immediately deallocated via the system allocator without GC pause.
- **Copy-on-Write (CoW)**: Mutating a shared structure whose reference count $> 1$ triggers a transparent shallow copy before modification.

### 2.2 Tier 2: `strict` Affine Single-Ownership

When zero-overhead, non-reference-counted allocation is required:

```agam
@lang.advance
fn process_buffer(buf: Buffer) -> i32 {
    strict {
        // Inside strict: affine move semantics are enforced.
        let mut local_buf = buf; // Ownership transferred (move)
        local_buf.transform();
        return local_buf.checksum();
    } // local_buf dropped deterministically here with 0 refcount overhead
}
```

**Rules inside `strict` blocks**:
1. **Affine Move Invariant**: Binding a non-`Copy` variable transfers ownership. Using the variable after a move triggers a compile-time diagnostic (`E0382: Use of moved value`).
2. **Exclusive Mutation**: Mutable references (`&mut T`) must be exclusive; no concurrent shared references (`&T`) may coexist within the same lexical scope.
3. **Zero Refcount Overhead**: Codegen emits direct stack/heap allocations without ARC headers or atomic increments.

---

## 3. Scoped Region Arenas

For high-throughput workloads (e.g. AST construction, compiler passes, tensor graph compilation), Agam provides **Lexical Arenas**:

- Memory allocated within an arena is pooled into contiguous chunks.
- Individual allocations inside the arena have $O(1)$ allocation cost (bump allocation).
- All objects within the arena are freed collectively in $O(1)$ time when the enclosing lexical scope exits.

---

## 4. Concurrency & Thread Safety

- **Send & Sync Boundaries**: Types shared across thread boundaries must satisfy thread-safety invariants.
- **Atomic Reference Counts**: Default heap objects use atomic reference counting (`Arc<T>`) to guarantee safety under multi-threaded read access.
- **No Shared Mutable State**: Concurrent mutation across threads requires explicit synchronization primitives (mutexes, channels, or lock-free atomics).

---

## 5. Summary Table

| Property | Tier 1 (Default Mode) | Tier 2 (`strict` Mode) |
| :--- | :--- | :--- |
| **Ownership** | Shared (Reference Counted) | Single Owner (Affine) |
| **Assignment Semantics** | Shallow Copy / Refcount Increment | Move Semantics |
| **Deallocation** | Deterministic (Refcount == 0) | Lexical Scope Exit (RAII) |
| **Runtime Overhead** | Atomic refcount inc/dec | Zero (Direct machine code) |
| **Borrow Checker** | Disabled | Lexically Enforced |
| **Primary Use Case** | Everyday application & scripting code | Low-level systems, kernel, & inner loops |
