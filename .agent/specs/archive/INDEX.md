# Completed Specifications Archive Index

- **[T0-type-system.md](T0-type-system.md)** (2026-08-14): Completed full type system: Option/Result, Enums, Structs, Match Guards, Or/Range patterns, Generics & Monomorphization, ? Try operator, and Type inference.
- **[T0-object-model.md](T0-object-model.md)** (2026-08-14): Completed object model: Struct impl blocks, self receiver, and method call dispatch.
- **[T0-grammar-spec.md](T0-grammar-spec.md)** (2026-08-14): Completed formal EBNF/PEG grammar and CI validation.
- **[T0-effects-handlers.md](T0-effects-handlers.md)** (2026-08-14): Completed algebraic effects: effect, handler, perform syntax, and CPS lowering.
- **[T0-stdlib-completion.md](T0-stdlib-completion.md)** (2026-08-14): Completed Indic grammatical design principles, naming conventions, and composition rules.
- **[T0-type-system-plan.md](T0-type-system-plan.md)** (2026-08-14): Planning document for T0-type-system.
- **[T0-type-system-task.md](T0-type-system-task.md)** (2026-08-14): Task tracking document for T0-type-system.
- **[F2-Emitters-Plan.md](F2-Emitters-Plan.md)** (2026-08-14): Planning document for F2-Emitters.
- **[F2-Emitters-Task.md](F2-Emitters-Task.md)** (2026-08-14): Task tracking document for F2-Emitters.
- **[F2-Emitters-Walkthrough.md](F2-Emitters-Walkthrough.md)** (2026-08-14): Walkthrough document for F2-Emitters.
- **[implementation_plan.md](implementation_plan.md)** (2026-08-14): Legacy implementation plan in details folder.
- **[task.md](task.md)** (2026-08-14): Legacy task in details folder.
- **[walkthrough.md](walkthrough.md)** (2026-08-14): Legacy walkthrough in details folder.
- **[T0-module-system.md](T0-module-system.md)** (2026-08-14): Completed module imports: selective item imports (import path::{A, B as C}), wildcard imports (import path::*), and resolver scope symbol binding.
- **[T0-stdlib-io.md](T0-stdlib-io.md)** (2026-08-14): Completed standard library and native I/O: FileSystem, Network, Environment, and Process modules in agam_std with sema effect definitions and runtime handlers.
- **[T0-effects-depth.md](T0-effects-depth.md)** (2026-08-14): Completed ergonomics and syntax cohesion: f-string embedded expression interpolation, brace escapes, sub-expression parsing, and HIR chained concatenation lowering.
- **[T1-compiler-agent-tool.md](T1-compiler-agent-tool.md)** (2026-08-16): Completed Compiler-as-Agent-Tool: native Model Context Protocol (MCP) server `agamc mcp serve` exposing tools (`check`, `format`, `explain_error`, `ast_inspect`, `sarif_diagnostics`, `run`) and resources (`workspace://structure`, `diagnostics://workspace`), plus SARIF 2.1.0 diagnostic export in `agam_errors`.
- **[T0-type-sandhi-graph.md](T0-type-sandhi-graph.md)** (2026-08-16): Completed Type Sandhi & Representation Graph Monomorphization: TraitLattice meet/join/subsumption, SandhiGraph transitive supertrait compilation with $O(1)$ bound satisfaction queries in `agam_sema`, and MonomorphGraph cycle detection with topological instantiation scheduling in `agam_mir`.
- **[tier_fundamentals/](tier_fundamentals/)** (2026-08-25): Completed and archived all 78 foundational Tier 1 through Tier 6 phase specifications (Advanced Testing, Build Systems, Async Coroutines, Security Sandbox, eBPF Kernel, PQC, GPU/SPIR-V backends, E-Graph Superoptimization, Autodiff, and Stage-0 Self-Hosting modules).

