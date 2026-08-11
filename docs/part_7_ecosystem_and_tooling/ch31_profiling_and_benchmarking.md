# Chapter 31: Compiler Profiling & Performance Measurement

> **Part VII: Advanced Tooling, Testing & Ecosystem Engineering**  
> **Compiler Module Focus**: [`agam_profile`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_profile)

---

## 31.1 Compiler Performance Methodology

Claims about compiler optimization performance gains must be verified through empirical measurement, not intuition.

`agam_profile` provides automated benchmarking harnesses and profiling instrumentation to track two distinct metrics:
1. **Compilation Speed (Throughput)**: Time taken to parse, type check, optimize, and generate code ($\text{Lines of Code / Second}$).
2. **Generated Binary Speed (Execution Latency)**: Runtime performance of optimized target binary code compared against `clang++ -O3` C++ implementations.

---

## 31.2 Flamegraph & Phase Timings

When running `agamc build --profile`, `agam_profile` records duration metrics across compiler phases:

```text
=====================================================
            AGAM COMPILER PROFILE SUMMARY
=====================================================
  Phase 1: Lexer & Parsing          :   1.2 ms  ( 4%)
  Phase 2: Semantic Analysis        :   3.1 ms  (10%)
  Phase 3: HIR & MIR Lowering       :   2.8 ms  ( 9%)
  Phase 4: MIR Optimizations        :   4.5 ms  (15%)
  Phase 5: LLVM IR & Codegen        :  18.4 ms  (62%)
-----------------------------------------------------
  TOTAL COMPILATION TIME            :  30.0 ms
=====================================================
```
