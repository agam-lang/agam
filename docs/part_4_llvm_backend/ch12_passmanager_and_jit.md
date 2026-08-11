# Chapter 12: Modern PassManager & In-Process JIT Engines

> **Core Literature Grounding**: *LLVM Techniques, Tips, and Best Practices* (Chapter 7) by Kai Nacke & Amy Kwan  
> **Compiler Module Focus**: [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen), [`agam_jit`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_jit)

---

## 12.1 Modern LLVM PassManager

LLVM uses the **New PassManager** pipeline to run modular transformations over LLVM IR modules:

```text
LLVM IR Module
       │
       ▼  PassBuilder (-O3 Pipeline)
  ┌────────────────────────────────────────────────────────┐
  │ ModulePassManager                                      │
  │  ├── FunctionPassManager                               │
  │  │    ├── Mem2RegPass (Promote stack allocas to regs) │
  │  │    ├── EarlyCSEPass (Common Subexpr Elimination)    │
  │  │    ├── InstCombinePass                              │
  │  │    └── LoopVectorizerPass                           │
  │  └── InlinerPass                                       │
  └────────────────────────────────────────────────────────┘
       │
       ▼  Optimized Bitcode
```

### Key LLVM Pass Categories:
- **Mem2Reg**: Transforms `alloca` memory locations into LLVM SSA registers (`%1`, `%2`).
- **InstCombine**: Combines redundant instruction sequences into simpler canonical primitives.
- **SLP / Loop Vectorizer**: Emits SIMD instructions (`AVX2`, `AVX-512`, `NEON`) for data-parallel operations.

---

## 12.2 In-Process JIT Compilation (`agam_jit`)

For interactive evaluation (`agamc repl`, `agamc exec`), generating `.o` files and invoking host linkers introduces unacceptable latency.

`agam_jit` compiles LLVM IR or MIR directly into executable memory pages (`PROT_READ | PROT_EXEC`) in process memory:

$$\text{LLVM Bitcode / MIR} \xrightarrow{\text{Cranelift / ORC JIT}} \text{Memory Buffer} \xrightarrow{\text{Cast to fn()}} \text{Direct Invocation}$$

```rust
pub struct AgamJitEngine {
    // Cranelift / LLVM ORC JIT instance
}

impl AgamJitEngine {
    pub unsafe fn execute_function(&mut self, fn_name: &str) -> Result<i64, JitError> {
        let symbol_ptr = self.lookup_symbol(fn_name)?;
        let func: extern "C" fn() -> i64 = std::mem::transmute(symbol_ptr);
        Ok(func())
    }
}
```
