# Agam Object System & Method Dispatch Architecture

> **Document Status:** Active Standard  
> **Crates:** `agam_ast`, `agam_sema`, `agam_mir`, `agam_codegen`  
> **Test Suite:** `agam_test::tests` (structs, traits, enums, monomorphization)

---

## 1. Executive Summary

Agam implements a **Trait-Based Nominal Object Model** enhanced by the **Type Sandhi Harmonic Lattice**. It emphasizes zero-cost static dispatch through monomorphization by default while offering explicit, bounded dynamic dispatch (`dyn Trait`) when polymorphic encapsulation is required.

```
                           Agam Declaration Layer
                ┌───────────────────┬───────────────────┐
                ▼                   ▼                   ▼
        struct Point {          enum Shape {         trait Renderable {
          x: f64,                 Circle(f64),         fn render(&self);
          y: f64,                 Rect(f64, f64),    }
        }                       }
                │                   │                   │
                └───────────────────┼───────────────────┘
                                    │
                                    ▼
                     ┌─────────────────────────────┐
                     │   Type Sandhi Lattice       │
                     │  - TraitLattice subtyping   │
                     │  - Transitive closure       │
                     │  - O(1) bound verification  │
                     └──────────────┬──────────────┘
                                    │
                ┌───────────────────┴───────────────────┐
                ▼                                       ▼
┌───────────────────────────────┐       ┌───────────────────────────────┐
│     Static Dispatch (MIR)     │       │    Dynamic Dispatch (dyn)     │
│  - Monomorphized functions    │       │  - Fat pointer (data, vtable) │
│  - Zero runtime overhead      │       │  - Polymorphic interface call │
│  - Direct inlining & devirt   │       │  - Bounded runtime cost       │
└───────────────────────────────┘       └───────────────────────────────┘
```

---

## 2. Core Object Primitives

### 2.1 Structs & Memory Layout
- **Field Packing:** Struct fields are laid out contiguously according to standard C ABI alignment rules (`#[repr(C)]`) or compiler-reordered layouts to eliminate internal padding.
- **Constructors & Initializers:** Clean struct literal instantiation syntax:
  ```rust
  let pt = Point { x: 10.0, y: 20.0 };
  ```

### 2.2 Tagged Unions (Enums)
- **Typed Payloads:** Variants support primitive scalars, nested structs, or unit values.
- **Discriminant Header:** A 32-bit tag discriminant is paired with aligned payload memory.

### 2.3 Trait Composition & Inherent Methods
- **Inherent `impl`:** `impl Point:` defines methods directly associated with the type.
- **Trait `impl`:** `impl Renderable for Point:` satisfies trait bounds.
- **Receiver Forms:**
  - `fn consume(self)`: By-value move semantics.
  - `fn inspect(&self)`: Shared immutable borrow.
  - `fn modify(&mut self)`: Exclusive mutable borrow.

---

## 3. Type Sandhi Harmonic Lattice (`agam_sema`)

Agam's trait system uses the **Sandhi Harmonic Lattice** to resolve trait satisfaction and subtyping:
- **`TraitLattice`:** Computes transitive supertrait closures (e.g. `trait Graphic: Renderable + Serializable`).
- **`SandhiGraph`:** Performs $O(1)$ constraint checking during bidirectional type inference without recursive search overhead.

---

## 4. Method Dispatch Pipeline

### 4.1 Static Monomorphization (Default)
- The `MonomorphGraph` in `agam_mir` tracks all concrete type instantiations.
- Cycles in generic parameter expansion are detected and rejected at compile-time.
- Enables aggressive function inlining, constant folding, and direct vectorization.

### 4.2 Dynamic VTable Dispatch (`dyn Trait`)
- Represented as a two-word fat pointer `(data_ptr: *const (), vtable_ptr: *const VTable)`.
- VTable structure contains:
  1. Size and alignment of the concrete type.
  2. Drop glue function pointer.
  3. Function pointers for all trait methods.
