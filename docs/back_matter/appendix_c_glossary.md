# Appendix C: Glossary of Compiler & Indic Design Terms

---

## 1. Compiler Engineering Terms

- **Abstract Syntax Tree (AST)**: A tree representation of source code syntax that omits concrete formatting noise (commas, semicolons, parentheses) while preserving structural semantics.
- **Application Binary Interface (ABI)**: A low-level machine contract defining parameter passing, register usage, stack frame layout, and return value mechanics between compiled binary modules.
- **Basic Block**: A straight-line sequence of instructions with a single entry point (first instruction) and a single exit point (terminator instruction).
- **Control Flow Graph (CFG)**: A directed graph where basic blocks form nodes and jump instructions (`Branch`, `Goto`, `Return`) form edges.
- **Dead Code Elimination (DCE)**: An optimization pass that removes instructions or basic blocks whose values are never consumed during execution.
- **Dominance Frontier ($DF$)**: The set of basic blocks where a node's dominance stops, determining exact placement locations for SSA $\phi$-nodes.
- **GlobalISel**: LLVM's modern global instruction selection framework developed by Quentin Colombet, operating directly on whole-function MachineIR.
- **Intermediate Representation (IR)**: Target-independent code representations (HIR, MIR, LLVM IR) used between frontend parsing and backend codegen.
- **Pratt Parsing**: Top-Down Operator Precedence parsing associating binding powers with infix and prefix tokens to resolve operator precedence cleanly.
- **SelectionDAG**: LLVM's legacy basic-block DAG-based instruction selection engine.
- **Static Single Assignment (SSA)**: An IR property guaranteeing that every variable is defined exactly once, using $\phi$-nodes to merge values at control flow join points.

---

## 2. Indic Grammatical Design Terms (Pāṇini & Tolkāppiyam)

- **Aṣṭādhyāyī**: Pāṇini's 4th-century BCE Sanskrit grammar treatise consisting of ~4,000 formal generative rules, serving as the world's oldest formal grammar system.
- **Dhātu (Root Verb)**: Canonical verbal roots (e.g., `kṛ`, `grah`, `dā`) used in Agam's standard library naming system for semantic consistency.
- **Pratyāhāra**: Concise shorthand notation for defining constrained sets or type sub-ranges.
- **Sandhi (Type Sandhi)**: Rules governing the composition, union, and transformation of types during type checking.
- **Tolkāppiyam**: The oldest extant Tamil grammatical work, detailing phonology, morphology, syntax, and structural semantics.
- **Vibhakti (Grammatical Case)**: Case roles (Kartṛ/Agent, Karman/Object, Karaṇa/Instrument) used to formalize function parameter roles.
