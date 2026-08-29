# Chapter 11: Emitting Textual & Bitcode LLVM IR

> **Core Literature Grounding**: *LLVM Techniques, Tips, and Best Practices* (Chapters 3–5) by Kai Nacke & Amy Kwan  
> **Compiler Module Focus**: [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen)

---

## 11.1 The LLVM IR Code Generation Architecture

`agam_codegen` bridges Agam's Medium-Level IR (MIR) to target-independent **LLVM IR**.

LLVM IR is a strongly typed, RISC-like instruction set in SSA form with infinite virtual registers (`%0`, `%1`, `%2`):

```text
Agam MIR (`agam_mir`)
       │
       ▼  LLVM Code Generator (`agam_codegen`)
LLVM Module Context & IR Builder
       │
       ├────────────────────────────────┐
       ▼                                ▼
Textual LLVM IR (.ll)         Binary Bitcode (.bc)
```

---

## 11.2 LLVM Module & Builder Infrastructure

Nacke & Kwan describe the core C++ / Rust LLVM API objects:

- **`Context`**: Owns core LLVM types, global constants, and thread-local state.
- **`Module`**: A single translation unit containing functions, global variables, target triple specifications, and data layouts.
- **`Builder`**: An instruction construction helper that appends newly created LLVM IR instructions onto basic block endpoints.

```rust
pub struct LLVMEmitter<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
}

impl<'ctx> LLVMEmitter<'ctx> {
    pub fn emit_function(&mut self, mir_fn: &MirFunction) {
        let ret_ty = self.convert_type(&mir_fn.return_ty);
        let param_tys: Vec<_> = mir_fn.params.iter().map(|p| self.convert_type(&p.ty)).collect();
        let fn_type = ret_ty.fn_type(&param_tys, false);
        
        let function = self.module.add_function(&mir_fn.name, fn_type, None);
        let entry_bb = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_bb);
        
        // Lower MIR basic blocks -> LLVM Basic Blocks
    }
}
```

---

## 11.3 Textual IR vs. Bitcode Output

- **Textual LLVM IR (`.ll`)**: Human-readable assembly format used for debugging and inspecting compiler codegen output.
- **Bitcode (`.bc`)**: Compact binary representation passed directly into LLVM optimization passes and linkers.

```llvm
; Textual LLVM IR generated for a simple function
define i64 @calculate_sum(i64 %a, i64 %b) #0 {
entry:
  %0 = add nsw i64 %a, %b
  ret i64 %0
}
```
