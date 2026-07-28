# Phase Catalog

Compact whole-program roadmap for Agam. Every phase uses the unified `T{tier}-{slug}` naming scheme.

## Status Legend

- `complete` — all checklist items shipped
- `partial` — some items shipped, open items remain
- `open` — exists on roadmap, no items shipped yet
- `decided` — design decision made, implementation pending

## Tier Definitions

| Tier | Name | Description |
|------|------|-------------|
| **T0** | Foundation | Language surface, type system, stdlib, effects. Hard dependency for everything. |
| **T1** | Developer Experience | DX, build system, package tooling, LSP, docs, testing, daemon, REPL. |
| **T2** | Runtime & Security | Memory model, async, OS sandbox, networking, security, observability. |
| **T3** | Platform & Ecosystem | GPU pipeline, WASM, cross-compilation, UI, FFI, SPIR-V, targeting. |
| **T4** | Optimization Depth | PGO, LTO, multi-level IR, hardware introspection, GPU tuning, comptime. |
| **T5** | AI-Native | Autodiff, tensor types, ML training, edge inference, tensor core lowering. |
| **T6** | Frontier | Self-hosting, formal verification, AI-driven compiler. Long-horizon. |

---

## Tier 0 — Foundation

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| T0-grammar-spec | complete | Formal EBNF/PEG grammar and CI validation | `details/T0-grammar-spec.md` |
| T0-stdlib-completion | complete | Indic grammatical design principles (naming, composition rules) | `details/T0-stdlib-completion.md` |
| T0-effects-handlers | complete | Language surface: effect/handler/perform syntax | `details/T0-effects-handlers.md` |
| T0-stdlib-io | partial | Standard library and native I/O expansion | `details/T0-stdlib-io.md` |
| T0-type-system | open | Generics, sum types, pattern matching, type inference, Option/Result | `details/T0-type-system.md` |
| T0-object-model | open | Struct/trait/impl pipeline, method dispatch, visibility | `details/T0-object-model.md` |
| T0-module-system | open | File-to-module mapping, imports, re-exports, visibility rules | `details/T0-module-system.md` |
| T0-effects-depth | open | Named args, closures, destructuring, ranges, operator overloading | `details/T0-effects-depth.md` |

---

## Tier 1 — Developer Experience

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| T1-daemon-prewarm | complete | Incremental daemon, background prewarm, parallel compilation | `details/T1-daemon-prewarm.md` |
| T1-premium-cli | complete | Premium experience layer: `agamc doctor`, `new`, `dev`, `cache` | `details/T1-premium-cli.md` |
| T1-sdk-distribution | partial | Native LLVM SDK distribution and toolchain bundles | `details/T1-sdk-distribution.md` |
| T1-repl-execution | complete | Interactive REPL and structured headless execution | `details/T1-repl-execution.md` |
| T1-workspace-manifests | complete | Workspace contract and dependency manifests | `details/T1-workspace-manifests.md` |
| T1-resolver-lockfile | complete | Deterministic resolver and lockfile | `details/T1-resolver-lockfile.md` |
| T1-registry-protocol | complete | Registry index protocol and immutable publish flow | `details/T1-registry-protocol.md` |
| T1-sdk-environments | complete | Named environments and SDK linkage | `details/T1-sdk-environments.md` |
| T1-official-distributions | complete | Curated first-party distributions and package governance | `details/T1-official-distributions.md` |
| T1-headless-exec | partial | Agent-facing headless execution tool (`agamc exec`) | `details/T1-headless-exec.md` |
| T1-python-wrappers | partial | Agent ecosystem wrappers (LangChain/LlamaIndex/Python) | `details/T1-python-wrappers.md` |
| T1-error-messages | open | Elite diagnostics: multi-span, suggestions, error recovery | `details/T1-error-messages.md` |
| T1-lsp-production | open | LSP production quality: go-to-def, completion, hover, refactoring | `details/T1-lsp-production.md` |
| T1-doc-generation | open | `agamc doc` — rendered HTML docs, cross-linked, searchable | `details/T1-doc-generation.md` |
| T1-formatter-linter | open | `agam_fmt` stability, `agam_lint` rules, IDE integration | `details/T1-formatter-linter.md` |
| T1-testing-framework | open | `agamc test` maturity, parallel test runner, coverage | `details/T1-testing-framework.md` |
| T1-advanced-testing | open | Property-based testing, fuzzing, doctests, snapshot testing | `details/T1-advanced-testing.md` |
| T1-build-system | partial | `agamc` unified CLI completeness (`add`, `remove`, `update`, `bench`) | `details/T1-build-system.md` |
| T1-cross-platform-pkg | open | Binary packaging for Windows/Linux/macOS/Android | `details/T1-cross-platform-pkg.md` |
| T1-supply-chain-sec | open | Package signing, SBOM, provenance, capability manifests | `details/T1-supply-chain-sec.md` |
| T1-plugin-system | open | First-party plugin/extension architecture for `agamc` | `details/T1-plugin-system.md` |
| T1-decentralized-registry | open | Federated package registry beyond the central index | `details/T1-decentralized-registry.md` |
| T1-compiler-agent-tool | open | MCP server for `agamc`, structured diagnostics API, agent SDK | `details/T1-compiler-agent-tool.md` |

---

## Tier 2 — Runtime, Concurrency, and Security

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| T2-os-sandbox | complete | OS-level sandbox enforcement (Win32 JobObject, Linux prctl/setrlimit) | `details/T2-os-sandbox.md` |
| T2-memory-model | decided | Memory ownership: Hybrid ARC + Value Semantics (decision made) | `details/T2-memory-model.md` |
| T2-async-concurrency | open | async/await, structured concurrency, channels, event loop | `details/T2-async-concurrency.md` |
| T2-networking-stack | open | `agam_std::net` — TCP/UDP, HTTP/2, TLS 1.3, DNS-over-HTTPS | `details/T2-networking-stack.md` |
| T2-cybersecurity | open | Memory safety, capability model, crypto primitives, taint tracking | `details/T2-cybersecurity.md` |
| T2-sandboxing | open | Language-level permissions (Deno-style), MicroVM isolation | `details/T2-sandboxing.md` |
| T2-observability | open | OpenTelemetry integration, `@trace`/`@metric` annotations, OTLP export | `details/T2-observability.md` |
| T2-zero-copy-serial | open | Zero-copy serialization, shared memory IPC, memory-mapped files | `details/T2-zero-copy-serial.md` |
| T2-ebpf-kernel | open | eBPF kernel observability: `@ebpf` annotation, unified kernel+userspace | `details/T2-ebpf-kernel.md` |
| T2-post-quantum-crypto | open | ML-KEM/ML-DSA primitives, formally verified crypto, quantum-resistant supply chain | `details/T2-post-quantum-crypto.md` |

---

## Tier 3 — Platform and Ecosystem

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| T3-target-profiles | complete | Omni-targeting: `@target.iot`, `@target.enterprise`, `@target.hpc` | `details/T3-target-profiles.md` |
| T3-gpu-npu-pipeline | partial | GPU/NPU integration: `@gpu` kernel pipeline, NVPTX emitter | `details/T3-gpu-npu-pipeline.md` |
| T3-spirv-backend | open | Vendor-neutral GPU via SPIR-V (chipStar/OpenCL/Vulkan/Level Zero) | `details/T3-spirv-backend.md` |
| T3-wasm-backend | open | WASM Component Model (WASI 0.2), browser embedding, WIT generation | `details/T3-wasm-backend.md` |
| T3-cross-compilation | open | Cross-compilation matrix: Windows, Linux, macOS, Android, RISC-V | `details/T3-cross-compilation.md` |
| T3-pkg-manager-maturity | open | Registry protocol maturity, resolver improvements | `details/T3-pkg-manager-maturity.md` |
| T3-mobile-targets | open | iOS and Android first-class targets | `details/T3-mobile-targets.md` |
| T3-native-renderer | open | Omni-platform native UI renderer (Win32/Cocoa/GTK/WASM/Android) | `details/T3-native-renderer.md` |
| T3-gpu-rendering | open | GPU-accelerated UI via DirectX 12 / Metal / Vulkan / WebGPU | `details/T3-gpu-rendering.md` |
| T3-reactive-state | open | Reactive state management for declarative UI | `details/T3-reactive-state.md` |
| T3-declarative-ui | open | `@ui` declarative UI syntax, DSL for layouts and components | `details/T3-declarative-ui.md` |
| T3-design-system | open | First-party design system, theming, accessibility | `details/T3-design-system.md` |
| T3-universal-ffi | open | C/C++/Python/Rust/JS/JVM interop with zero-overhead hot paths | `details/T3-universal-ffi.md` |
| T3-riscv-backend | open | RISC-V backend validation (RVA23, RVV auto-vectorization) | `details/T3-riscv-backend.md` |
| T3-generative-ui | open | Agent-driven UI composition via MCP + A2UI protocol | `details/T3-generative-ui.md` |

---

## Tier 4 — Performance and Optimization Depth

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| T4-llvm-optimization | open | PGO, LTO, auto-vectorization, loop opts, LLVM 22.1 upgrade | `details/T4-llvm-optimization.md` |
| T4-comptime-execution | open | Compile-time expression evaluation, `@comptime` blocks | `details/T4-comptime-execution.md` |
| T4-incremental-compile | open | Fine-grained incremental recompilation beyond daemon warm state | `details/T4-incremental-compile.md` |
| T4-parallel-build | open | Parallel crate-level and inter-crate build scaling | `details/T4-parallel-build.md` |
| T4-gpu-optimization-depth | open | GPU occupancy tuning, shared memory optimization, NPU dispatch | `details/T4-gpu-optimization-depth.md` |
| T4-link-time-opts | open | Distributed ThinLTO, whole-program dead code elimination | `details/T4-link-time-opts.md` |
| T4-hardware-introspection | open | Cache-aware data layout, SIMD multi-versioning, `@accelerate` | `details/T4-hardware-introspection.md` |
| T4-multi-level-ir | open | Dialect-extensible MIR (core + gpu + tensor dialects), MLIR-inspired | `details/T4-multi-level-ir.md` |
| T4-metaprogramming | open | Declarative and procedural macros, DSL construction, `@comptime` reflection | `details/T4-metaprogramming.md` |
| T4-gpu-auto-tuning | open | GPU auto-tuning (genetic pass selection) and `Tile<T,N>` abstractions | `details/T4-gpu-auto-tuning.md` |
| T4-tile-async-memory | open | Tile-centric programming model and asynchronous memory (TMA-style) | `details/T4-tile-async-memory.md` |

---

## Tier 5 — AI-Native Differentiation

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| T5-autodiff-tensors | open | IR-level autodiff (Enzyme-style), first-class `Tensor<T, Shape>` | `details/T5-autodiff-tensors.md` |
| T5-llm-inference | open | ML training loop primitives: optimizers, data loading, distributed training | `details/T5-llm-inference.md` |
| T5-neural-net-dsl | open | `@nn { }` DSL for neural network definition | `details/T5-neural-net-dsl.md` |
| T5-distributed-training | open | Multi-node distributed training coordination | `details/T5-distributed-training.md` |
| T5-edge-ai-inference | open | ONNX/TFLite model import, compile-time embedding, edge targets | `details/T5-edge-ai-inference.md` |
| T5-probabilistic-prog | open | `sample`/`observe` algebraic effects, HMC/VI inference engines | `details/T5-probabilistic-prog.md` |
| T5-tensor-core-matrix | open | Tensor core access via SPIR-V `OpCooperativeMatrixMulAddKHR` | `details/T5-tensor-core-matrix.md` |

---

## Tier 6 — Frontier and Long-Horizon

| Phase | Status | Focus | Detail |
|-------|--------|-------|--------|
| T6-self-hosting | open | Rewrite compiler in Agam — the maturity milestone | `details/T6-self-hosting.md` |
| T6-formal-verification | open | SMT solver integration, `@verify`, `requires`/`ensures` contracts | `details/T6-formal-verification.md` |
| T6-language-research | open | Experimental language features and research proposals | `details/T6-language-research.md` |
| T6-ai-compiler-intel | open | ML-guided optimization, semantic error messages, profile-guided AI | `details/T6-ai-compiler-intel.md` |

---

## Where To Dive Deeper

- **Foundation work:** `details/T0-*.md`
- **Developer experience + build + package:** `details/T1-*.md`
- **Runtime, security, and OS:** `details/T2-*.md`
- **Platform targets and ecosystem:** `details/T3-*.md`
- **Optimization depth:** `details/T4-*.md`
- **AI-native features:** `details/T5-*.md`
- **Frontier research:** `details/T6-*.md`
- **Architectural knowledge:** `.agent/wiki/`
- **Naming convention explained:** `.agent/wiki/phase-naming-convention.md`
- **Current compiler gaps:** `.agent/wiki/mir-todo-gaps.md`
- **Dependency map:** `.agent/wiki/dependency-map.md`
