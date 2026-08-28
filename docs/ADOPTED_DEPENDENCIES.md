# Adopted Dependencies & Architectural Boundary Policy

**Status**: Active Architectural Governance Policy  
**Last Updated**: 2026-08-27  

---

## 1. Executive Policy: Do Not Hand-Roll Mature Research Domains

Agam strictly distinguishes between **core compiler innovations** (which we build natively) and **mature, peer-reviewed computer science infrastructure** (which we adopt as battle-tested dependencies).

Attempting to hand-roll term-rewriting engines, SMT decision procedures, polyhedral affine schedulers, cryptographic primitives, regular expression DFA/NFA engines, RFC 4180 CSV parsers, CLI argument parsers, raw JSON parsers, statistical PRNGs, calendar date/time math, or foundational hash table probing algorithms from scratch introduces severe reliability and security risks. We therefore formalize the following **Adopt vs. Build Boundary**:

---

## 2. The Adopt vs. Build Matrix

| Compiler Subsystem | Architectural Policy | Adopted Open-Source Dependency | Rationale & Trade-off |
|:---|:---:|:---|:---|
| **E-Graph Equality Saturation** | **ADOPT** | **`egg`** (e-graphs good, UW PLSE) | Hand-rolling equality saturation requires multi-year research for term-rewriting confluence, extraction cost functions, and e-class analyses. `egg` is the gold standard. |
| **SMT Refinement Solving** | **ADOPT** | **`z3` / `z3-sys`** (Microsoft Research) | Presburger arithmetic and first-order theory solvers have complex decision procedures. Z3 is verified, robust, and industry-proven. |
| **Polyhedral Loop Scheduling** | **ADOPT** | **`isl-rs` / LLVM Polly** | Polyhedral integer set libraries represent decades of PhD-level optimization. We bind to `isl` rather than authoring a bespoke affine solver. |
| **Post-Quantum Cryptography** | **ADOPT** | **`ml-kem`**, **`pqcrypto`**, **`sha2`**, **`blake3`**, **`aes-gcm`** | Hand-rolled crypto is insecure by default. Production builds will only utilize audited, constant-time, FIPS-compliant crates. |
| **Hash Table & SwissTable Probing** | **ADOPT** | **`std::collections::HashMap` / `hashbrown`** | Hand-rolling hash tables risks HashDoS vulnerabilities, quadratic collision degradation, and subtle probing bugs. We wrap battle-tested SwissTable storage. |
| **Deterministic B-Tree Map** | **ADOPT** | **`std::collections::BTreeMap`** | B-tree node layout, cache-line alignment, and rebalancing are mature CS infrastructure. We wrap std BTreeMap for deterministic ordered iteration. |
| **UTF-8 Validation & Boundary Safety** | **ADOPT** | **`core::str` / `std::char`** | UTF-8 multibyte boundary validation is guaranteed by Rust `str`. Hand-rolling UTF-8 state machines causes off-by-one indexing panics. |
| **Regular Expression Engine** | **ADOPT** | **`regex`** | DFA/NFA compilation and PikeVM execution guarantee $O(n)$ search time with zero catastrophic backtracking (ReDoS) crashes. |
| **CSV Tabular Ingestion** | **ADOPT** | **`csv`** | RFC 4180 CSV parsing involves complex quote escaping and newline boundaries. We wrap `csv` for robust dataset ingestion into DataFrames. |
| **JSON Parsing & Serialization** | **ADOPT** | **`serde_json`** | Hand-rolling RFC 8259 JSON parsers introduces numeric precision loss, escape injection, and malformed UTF-8 bugs. We wrap `serde_json` behind Agam dynamic APIs. |
| **Declarative CLI Parsing Engine** | **ADOPT** | **`clap`** (Builder API) | Reuses existing workspace `clap` dependency for robust argument, flag, and auto `--help` generation rather than duplicating parser logic. |
| **Statistical PRNG Engines** | **ADOPT** | **`rand` / `rand_pcg` / `rand_xoshiro`** | PRNGs require statistically verified uniform distribution and entropy seeding. Hand-rolling leads to statistical bias and correlation artifacts. |
| **Calendar & ISO-8601 Formatting** | **ADOPT** | **`chrono`** | Gregorian calendar math, leap-year calculations (400-year rules), and RFC 3339 / ISO-8601 formatting are mature CS infrastructure. We wrap `chrono` for date/time parsing. |
| **Monotonic Hardware Timers** | **BUILD** | First-Party Native (`std::time::Instant`) | Zero-overhead wrapping of OS high-resolution monotonic clocks (`clock_gettime` / `QueryPerformanceCounter`) for `Instant::now()`, `elapsed_ms()`, and `sleep_ms()`. |
| **Zero-GIL Thread Concurrency** | **BUILD** | First-Party Native (`std::sync` / `std::thread`) | Native multi-core thread execution (`sync.spawn`, `sync.parallel_for`, `sync.Mutex`, `sync.Channel`) with zero GIL overhead per `note.md`. Lightweight chunk partitioning is chosen over `rayon` to maintain zero-overhead standalone binary targets without runtime scheduler bloat. |
| **Structured Leveled Logging** | **BUILD** | First-Party Native (`agam_std::log`) | Compile-time stripped leveled logger (`debug`, `info`, `warn`, `error`) with thread-safe timestamped formatting. |
| **Scripting Combinatorics & Iterators** | **BUILD** | First-Party Native (`agam_std::iter`) | Zero-allocation permutations, combinations, chunking, and zipping compiled directly to native loops. |
| **JIT Machine Codegen** | **ADOPT** | **`cranelift-codegen`** (Bytecode Alliance) | Provides safe, sub-millisecond in-process native machine code generation for dev loops. |
| **AOT Machine Codegen** | **ADOPT** | **LLVM 18+ (C-API / in-process bindings)** | Leverages LLVM's global scalar optimizations, ThinLTO, and cross-target hardware backends. |
| **Pāṇinian Frontend & Pratt Parser** | **BUILD** | First-Party Native (`agam_parser`, `agam_lexer`) | Core language identity: dual `@lang.base` / `@lang.advance` profiles, span tracking, and panic-mode synchronization. |
| **Type Sandhi Matrix & SEMA** | **BUILD** | First-Party Native (`agam_sema`) | Unique Pāṇinian type-theoretic lattice and bidirectional constraint inference. |
| **Mid-Level IR & Escape Analysis** | **BUILD** | First-Party Native (`agam_mir`) | Tailored SSA control flow, Cooper–Harvey–Kennedy dominators, and interprocedural static ARC elision. |
| **Standard Library Ergonomics (`note.md`)** | **BUILD** | First-Party Native (`agam_std`) | Thin, zero-panic wrappers integrating adopted storage with Agam's ARC memory model, `Counter`, `bisect`, `Path`, `StringBuilder`, and 64-byte aligned tensors. |

---

## 3. Dependency Audit Invariant

Every third-party crate brought into `agam/Cargo.toml` must:
1. Be written in pure Rust (or supply hermetic C bindings with deterministic builds).
2. Pass `cargo audit` with **zero known security vulnerabilities**.
3. Be explicitly listed in this document with its associated architectural layer.

---

## 4. The Facade-Completeness & Zero-Identity-Leak Invariant

Whenever an external crate is adopted (e.g. `clap`, `regex`, `csv`, `serde_json`, `chrono`, `rand`), the Agam wrapper layer in `agam_std` must **fully own the user-facing voice and error semantics**:

1. **Zero Third-Party Error Passthrough**: Under no circumstances may raw third-party error text, formatting styles (e.g. clap's terminal color escape sequences or default usage phrases), or internal error types leak directly to script authors. All errors must map into structured Agam diagnostic types (`CliError`, `RegexError`, `CsvError`, `JsonError`, `TimeError`) adhering to the compiler's established Nyāya diagnostic voice (Cause, Source Context, Remedy).
2. **Panic-Immune Configuration**: Third-party APIs that can panic on invalid developer input (e.g., clap builder duplicate flags, regex invalid byte flags) must be guarded via validation checks so misconfigurations return structured `Result<T, E>` and never surface raw Rust panic backtraces to users.
3. **Hermetic Identity & Banners**: Banners, `--help` text, and `--version` strings must never default to Cargo package metadata or toolchain versions; all versioning and identity strings must be explicitly supplied by the script author or Agam runtime.
