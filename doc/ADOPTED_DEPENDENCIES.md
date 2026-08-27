# Adopted Dependencies & Architectural Boundary Policy

**Status**: Active Architectural Governance Policy  
**Last Updated**: 2026-08-27  

---

## 1. Executive Policy: Do Not Hand-Roll Mature Research Domains

Agam strictly distinguishes between **core compiler innovations** (which we build natively) and **mature, peer-reviewed computer science infrastructure** (which we adopt as battle-tested dependencies).

Attempting to hand-roll term-rewriting engines, SMT decision procedures, polyhedral affine schedulers, or cryptographic primitives from scratch introduces severe reliability and security risks. We therefore formalize the following **Adopt vs. Build Boundary**:

---

## 2. The Adopt vs. Build Matrix

| Compiler Subsystem | Architectural Policy | Adopted Open-Source Dependency | Rationale & Trade-off |
|:---|:---:|:---|:---|
| **E-Graph Equality Saturation** | **ADOPT** | **`egg`** (e-graphs good, UW PLSE) | Hand-rolling equality saturation requires multi-year research for term-rewriting confluence, extraction cost functions, and e-class analyses. `egg` is the gold standard. |
| **SMT Refinement Solving** | **ADOPT** | **`z3` / `z3-sys`** (Microsoft Research) | Presburger arithmetic and first-order theory solvers have complex decision procedures. Z3 is verified, robust, and industry-proven. |
| **Polyhedral Loop Scheduling** | **ADOPT** | **`isl-rs` / LLVM Polly** | Polyhedral integer set libraries represent decades of PhD-level optimization. We bind to `isl` rather than authoring a bespoke affine solver. |
| **Post-Quantum Cryptography** | **ADOPT** | **`ml-kem`**, **`pqcrypto`**, **`sha2`**, **`blake3`**, **`aes-gcm`** | Hand-rolled crypto is insecure by default. Production builds will only utilize audited, constant-time, FIPS-compliant crates. |
| **JIT Machine Codegen** | **ADOPT** | **`cranelift-codegen`** (Bytecode Alliance) | Provides safe, sub-millisecond in-process native machine code generation for dev loops. |
| **AOT Machine Codegen** | **ADOPT** | **LLVM 18+ (C-API / in-process bindings)** | Leverages LLVM's global scalar optimizations, ThinLTO, and cross-target hardware backends. |
| **Pāṇinian Frontend & Pratt Parser** | **BUILD** | First-Party Native (`agam_parser`, `agam_lexer`) | Core language identity: dual `@lang.base` / `@lang.advance` profiles, span tracking, and panic-mode synchronization. |
| **Type Sandhi Matrix & SEMA** | **BUILD** | First-Party Native (`agam_sema`) | Unique Pāṇinian type-theoretic lattice and bidirectional constraint inference. |
| **Mid-Level IR & Escape Analysis** | **BUILD** | First-Party Native (`agam_mir`) | Tailored SSA control flow, Lengauer–Tarjan dominators, and interprocedural static ARC elision. |
| **Standard Library Core** | **BUILD** | First-Party Native (`agam_std`) | Zero-allocation HTTP/2, 64-byte aligned tensors, 4K media encoding, and lock-free concurrency. |

---

## 3. Dependency Audit Invariant

Every third-party crate brought into `agam/Cargo.toml` must:
1. Be written in pure Rust (or supply hermetic C bindings with deterministic builds).
2. Pass `cargo audit` with **zero known security vulnerabilities**.
3. Be explicitly listed in this document with its associated architectural layer.
