# Algorithm & Mathematical Synthesis Rule

All AI agents (Gemini, Claude, Codex, etc.) participating in the Agam-Lang project MUST strictly follow this synthesis rule:

## 1. Extract Underlying Algorithms, Not Brand Names
- When reviewing or processing research papers, frontier articles, or classical theorems (e.g., OpenAI 10 Advances, Triton passes, compiler literature):
  - **DO NOT** copy superficial third-party brand names into Agam specs or try to convert Agam into external tools.
  - **DO** extract the exact **underlying mathematical algorithms, formal proofs, data structures, and working principles** (e.g., *commuting square-zero algebra term-rewriting*, *Hankel moment systems*, *Kronecker-product Jacobians*, *subspace representation graphs*, *resolvent operator purifications*).

## 2. Native Compiler Engine Synthesis
- Synthesize the extracted algorithms natively into Agam's core crates:
  - **`agam_mir`**: E-graph superoptimization, equality saturation, and block-selective tensor rewrite rules.
  - **`agam_sema`**: Type-sandhi junction resolution, monomorphization lattice graphs, and static rank/shape inference.
  - **`agam_errors`**: Nyāya 4-part proof diagnostic engine (*Pratyakṣa, Anumāna, Upamāna, Śabda*).
  - **`agam_codegen`**: Hardware-agnostic vectorization, memory-treasury bounds, and backend lowering.

## 3. Unbeatable & Independent Identity
- Agam is its own language and self-contained compiler toolchain. The goal of incorporating literature is to make Agam’s internal passes mathematically unshakeable, secure, and optimal—never dependent on foreign wrappers or external frameworks.
