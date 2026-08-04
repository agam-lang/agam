# Phase T2-compiler-fuzzer — AST Mutation & LLVM Backend Vulnerability Fuzzer

## Phase Focus

AST procedural mutation fuzzing and LLVM codegen invariant verification (`agam_fuzzer`) targeting compiler stability, semantic check bypass prevention, and LLVM/MIR lowering memory safety.

## Key Capabilities

1. **AST & Grammar Fuzzing**:
   - `arbitrary`-based procedural AST generation (`FuzzExpr`, `FuzzStmt`, `FuzzDecl`) targeting `agam_sema` and `agam_mir`.
   - Raw bytes lexer/parser fuzzing via `cargo-fuzz` / `libFuzzer` targeting recursive parser stack safety.

2. **Backend & LLVM IR Invariants**:
   - `LLVMVerifyModule` automated verification harness in `agam_codegen`.
   - AddressSanitizer (ASan), LeakSanitizer (LSan), and UBSan integration for FFI & LLVM bindings memory leak/overflow detection.
   - Deterministic diagnostic assertions (compiler must return `NyayaDiagnostic`, never crash or panic).

## Verification Plan

- `cargo fuzz run fuzz_sema` and `cargo fuzz run fuzz_llvm` harnesses passing with zero panics or unhandled IR errors over corpus runs.
