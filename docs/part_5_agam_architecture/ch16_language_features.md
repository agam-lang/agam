# Chapter 16: Advanced Language Features: Native Tensors & Algebraic Effects

> **System Scope**: Agam First-Class Language Primitives  
> **Compiler Module Focus**: [`agam_ast`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_ast), [`agam_sema`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_sema), [`agam_mir`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_mir)

---

## 16.1 Native Tensor Operations

In Agam, multi-dimensional numerical tensors are first-class compiler primitives rather than external C++ library bindings.

```agam
let A: Tensor[Float, 2x3] = Tensor.from_array([
    [1.0, 2.0, 3.0],
    [4.0, 5.0, 6.0]
]);

let B: Tensor[Float, 3x2] = Tensor.ones([3, 2]);

// Native matrix multiplication compiled directly to SIMD / BLAS instructions
let C = A * B; 
```

### Compiler Lowering for Tensors
1. **Type Checker (`agam_sema`)**: Verifies tensor dimension compatibility at compile time ($\text{InnerDim}(A) == \text{OuterDim}(B)$).
2. **MIR Generation (`agam_mir`)**: Emits SIMD loop constructs or BLAS external call nodes (`agam_runtime_matmul`).

---

## 16.2 Algebraic Effect Handlers (`perform`, `handle`, `effect`)

Agam implements algebraic effect handlers (`Phase 20`), allowing control flow and side-effects to be handled modularly without callback structures:

```agam
effect Database {
    fn query(sql: String) -> String;
}

fn fetch_user_profile(id: Int) -> String {
    perform Database.query("SELECT * FROM users WHERE id = " + id.to_string());
    return "User Profile";
}

fn main() {
    handle fetch_user_profile(42) {
        Database.query(sql) => {
            println("Intercepted SQL: " + sql);
            resume("Mocked Result"); // Resumes execution back to caller
        }
    }
}
```
