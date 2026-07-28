# Project Overview

## What Agam Is

Agam is a next-generation, multi-paradigm, natively compiled, memory-safe language and toolchain implemented in Rust. It is designed to be the **one coherent omni-language** — systems, AI/ML, cybersecurity, GUI, automation, scientific computing, and beyond.

## The 22 Pillars of Agam

| # | Pillar | Core Guarantee |
|---|--------|----------------|
| 1 | **Performance** | Zero-overhead abstractions, LLVM, predictable memory |
| 2 | **Syntax** | Progressive complexity, multi-paradigm, metaprogramming |
| 3 | **Math & AI** | First-class tensors, native vectorization, auto-acceleration |
| 4 | **Security** | Provable memory safety, capability-based, formal verification |
| 5 | **Concurrency** | Fearless parallelism, data race prevention |
| 6 | **Interoperability** | Universal FFI: C/C++/Python/Rust/Java/JS zero-friction |
| 7 | **Tooling** | One-binary: compiler+pkg+fmt+lint+LSP+test+debugger |
| 8 | **Adaptive Compilation** | REPL+JIT prototyping, AOT production — same code |
| 9 | **Hardware Introspection** | Deep silicon access, cache-aware compilation |
| 10 | **AI Compiler** | Semantic errors, ML-guided optimization |
| 11 | **Error & State** | Deterministic resilience, Result/Option, immutable-first |
| 12 | **Multi-Target** | WASM native, cross-compilation via `--target` |
| 13 | **Omni-Platform Renderer** | Single codebase → native UI on Win/Mac/Linux/Web/Android |
| 14 | **Hardware-Accelerated Visuals** | GPU-native 120 FPS rendering, data visualization |
| 15 | **Modern Component Ecosystem** | 100+ widgets, theme engine (Material/Fluent/Bento/Neumorphic) |
| 16 | **Declarative & Reactive Syntax** | @ui DSL, automatic state-driven updates |
| 17 | **Zero-Friction Visual Toolchain** | Hot-reload, live preview, UI architect |
| 18 | **One Tool** | Compiler IS the pkg manager, test runner, everything |
| 19 | **Immutable Reproducibility** | Crypto lockfile, hermetic builds, time-travel builds |
| 20 | **Supply Chain Fortress** | Signed packages, capability sandbox, SBOM, typosquat guard |
| 21 | **Zero-Config Foreign Build** | Drop .c/.py/.rs in, auto-detect, compile, link |
| 22 | **Decentralized Registry** | Federated, mirrored, IPFS, no single point of failure |

## Ecosystem Libraries (parallel development)

| Library | Purpose | Python Equivalent |
|---------|---------|-------------------|
| `agam-http` | HTTP client/server | requests + aiohttp |
| `agam-json` | JSON parsing/serialization | json + orjson |
| `agam-crypto` | Cryptography primitives | cryptography |
| `agam-ml` | ML framework | PyTorch |
| `agam-db` | Database drivers + ORM | SQLAlchemy |
| `agam-web` | Web framework | Django/FastAPI |
| `agam-async` | Async runtime | asyncio + trio |
| `agam-cli` | CLI framework | click + rich |

## Architecture Documents

- **Compiler:** `agam/docs/architecture/compiler-architecture.md`
- **GUI Engine:** `agam/docs/architecture/gui-architecture.md`
- **Roadmap:** `.agent/phases/catalog.md` (22 pillars, 56 phases, 6 tiers)

## Strategic Roadmap (6 Tiers, 56 Phases)

| Tier | Theme | Key Phases |
|------|-------|------------|
| **0** | Foundation + **Declarative UI** | F1–F5, GUI4 |
| **1** | DX + **Visual Toolchain** + **One Tool** | DX1–DX5, GUI5, **PKG1** |
| **2** | Runtime, Security, **Reproducibility**, **Supply Chain** | R1–R3, SEC1, **PKG2**, **PKG3** |
| **3** | Platform, GUI, FFI, **Foreign Build**, **Federated Registry** | P1–P4, GUI1, GUI3, FFI1, **PKG4**, **PKG5** |
| **4** | Performance, **GPU Visuals**, Hardware | O1–O5, GUI2, META1, HW1 |
| **5** | AI-Native | AI1–AI3 |
| **6** | Frontier | X1–X3, AIC1 |
