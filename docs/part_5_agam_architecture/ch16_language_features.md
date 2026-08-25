# Chapter 16: Advanced Language Features: Native Tensors & Algebraic Effects

> **System Scope**: Agam First-Class Language Primitives  
> **Compiler Module Focus**: [`agam_ast`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_ast), [`agam_sema`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_sema), [`agam_mir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir)

---

## 16.1 Native Tensor Operations

In Agam, multi-dimensional numerical tensors are first-class compiler primitives rather than external C++ library bindings. The compiler understands tensor shapes, verifies dimension compatibility at compile time, and lowers tensor operations directly to SIMD, BLAS, or GPU kernel instructions.

### Tensor Type Syntax

```agam
// Static-shape tensors — dimensions known at compile time
let A: Tensor[Float, 2x3] = Tensor.from_array([
    [1.0, 2.0, 3.0],
    [4.0, 5.0, 6.0]
]);

let B: Tensor[Float, 3x2] = Tensor.ones([3, 2]);

// Native matrix multiplication compiled to SIMD / BLAS / GPU
let C = A * B;  // C: Tensor[Float, 2x2]

// Dynamic-shape tensors — dimensions known at runtime
let D: Tensor[Float] = Tensor.random([batch_size, 784]);
```

### Compile-Time Shape Verification

The type checker (`agam_sema`) enforces tensor dimension compatibility using compile-time shape arithmetic:

| Operation | Shape Rule | Example |
| :--- | :--- | :--- |
| Matrix Multiply `A * B` | $\text{Cols}(A) = \text{Rows}(B)$, result: $[\text{Rows}(A), \text{Cols}(B)]$ | `[2,3] * [3,4]` → `[2,4]` |
| Element-wise `A + B` | Shapes must be identical or broadcastable | `[2,3] + [2,3]` → `[2,3]` |
| Broadcasting | Dimensions of size 1 expand to match | `[2,3] + [1,3]` → `[2,3]` |
| Transpose `.T` | Reverses dimensions | `[2,3].T` → `[3,2]` |
| Reshape `.reshape(s)` | Total elements must be preserved | `[2,3].reshape([6])` → `[6]` |

**Shape mismatch errors** are reported at compile time with a diagnostic showing the incompatible dimensions:

```text
error[E0412]: tensor dimension mismatch in matrix multiply
  ┌─ src/main.agam:5:15
  │
5 │ let C = A * B;
  │             ^ inner dimensions do not match
  │
  = thesis: Tensor[Float, 2x3] cannot multiply with Tensor[Float, 4x2]
  = reason: inner dimension 3 ≠ 4
  = help: reshape B to [3, 2] or transpose with B.T
```

### Compiler Lowering for Tensors

The compiler lowers tensor operations through multiple strategies depending on the target and tensor size:

```text
Tensor Operation (A * B)
      │
      ├── Small static shape (≤ 16x16)
      │     └── Inline SIMD loop nest (SSE/AVX/NEON)
      │
      ├── Medium shape (≤ 1024x1024)
      │     └── Tiled loop nest with cache blocking
      │
      ├── Large shape or @gpu annotation
      │     └── GPU kernel dispatch (SPIR-V / NVPTX)
      │           └── Cooperative Tile<T, M, N> matmul
      │
      └── External BLAS available
            └── Direct call to agam_runtime_matmul → cblas_sgemm
```

### Tensor Activation Functions

Built-in activation primitives are lowered to vectorized intrinsics:

```agam
let h1 = Tensor.relu(x);      // max(0, x) — vectorized
let h2 = Tensor.sigmoid(x);   // 1 / (1 + exp(-x))
let h3 = Tensor.tanh(x);      // hyperbolic tangent
let h4 = Tensor.softmax(x);   // exp(x_i) / Σexp(x_j)
let h5 = Tensor.gelu(x);      // Gaussian Error Linear Unit
```

---

## 16.2 Algebraic Effect Handlers

Agam implements algebraic effect handlers as a first-class control flow mechanism, allowing side effects to be declared, performed, and intercepted without callbacks, monads, or dependency injection frameworks.

### Core Concepts

| Concept | Agam Keyword | Purpose |
| :--- | :--- | :--- |
| **Effect Declaration** | `effect` | Defines an interface of operations that may have side effects |
| **Effect Performance** | `perform` | Invokes an effect operation, suspending to the nearest handler |
| **Effect Handling** | `handle` | Intercepts performed effects and provides concrete implementations |
| **Resumption** | `resume(value)` | Continues the suspended computation with a provided value |

### Syntax & Semantics

```agam
// 1. Declare effect interfaces — pure type signatures
effect Database {
    fn query(sql: String) -> String;
    fn execute(sql: String) -> Int;
}

effect Logger {
    fn log(level: String, msg: String) -> Nil;
}

// 2. Pure business logic — performs effects without knowing implementations
fn fetch_user_profile(id: Int) -> String {
    perform Logger.log("INFO", "Fetching user " + id.to_string());

    let result = perform Database.query(
        "SELECT * FROM users WHERE id = " + id.to_string()
    );

    perform Logger.log("INFO", "Query returned: " + result);
    return result;
}

// 3. Application entry — provides concrete handlers
fn main() {
    let profile = handle fetch_user_profile(42) {
        Database.query(sql) => {
            // Could be a real DB, mock, or test double
            resume("{ \"name\": \"Alice\", \"role\": \"Admin\" }");
        },
        Database.execute(sql) => {
            resume(1);  // Rows affected
        },
        Logger.log(level, msg) => {
            println("[" + level + "]: " + msg);
            resume();   // Nil-returning effects resume with unit
        }
    };

    println("Profile: " + profile);
}
```

### Effect Type Checking

The semantic analyzer (`agam_sema`) tracks which effects a function may perform and verifies that all performed effects are handled at every call site:

```text
Effect Checking Rules:
  1. If function f performs effect E, then f's type signature
     implicitly carries E in its effect set.
  2. A `handle` block must provide handlers for ALL effects
     performed by its body expression.
  3. Unhandled effects propagate outward to the caller.
  4. The `main()` function must have an empty effect set
     (all effects fully handled).
```

**Unhandled effect error:**
```text
error[E0501]: unhandled algebraic effect
  ┌─ src/main.agam:12:5
   │
12 │     fetch_user_profile(42);
   │     ^^^^^^^^^^^^^^^^^^^^^^ performs effect `Database`
   │
   = thesis: effect `Database` is performed but not handled
   = reason: no `handle` block intercepts Database operations
   = help: wrap this call in `handle ... { Database.query(sql) => { ... } }`
```

### Compiler Lowering: Effects to State Machines

Algebraic effects are compiled by transforming the effect-performing function into a **stackless state machine** with explicit continuation frames:

```text
Source:  fn f() { ... perform E.op(x) ... more code ... }

Lowered State Machine:

  State 0: Execute code before perform
           → Save local variables to continuation frame
           → Yield EffectRequest { effect: E, op: "op", arg: x }

  State 1: (Entered when handler calls resume(val))
           → Restore local variables from continuation frame
           → Bind `val` as the return value of `perform`
           → Execute "more code"
           → Return final result
```

This transformation is analogous to how Rust compiles `async fn` into `Future` state machines, but generalized to arbitrary effect types rather than being limited to async I/O.

### Why Algebraic Effects Over Alternatives

| Approach | Limitation Agam Avoids |
| :--- | :--- |
| **Callback functions** | Callback hell, inversion of control, no resumption |
| **Monads (Haskell-style)** | Complex type gymnastics, monad transformer stacks |
| **Dependency injection** | Runtime overhead, no compiler verification |
| **async/await only** | Limited to I/O effects, cannot express logging/state/exceptions |
| **Algebraic effects** | ✓ Composable, ✓ Type-checked, ✓ Zero-overhead state machines |
