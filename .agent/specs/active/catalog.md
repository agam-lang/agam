# Master Execution Catalog

Canonical whole-program roadmap for Agam Compiler Engineering & Execution.

---

## 🚀 Active Production Execution Stages

| Stage | Name | Status | Focus Area | Detail File |
|:---|:---|:---:|:---|:---|
| **Stage 0** | **Crate Decoupling & Driver Modularization** | 🔄 `active` | Inverted driver modularization, extract `agam_target` & `agam_session`, zero-panic hardening, Pratt parser synchronization | [`details/STAGE-00-driver-modularization-and-hardening.md`](details/STAGE-00-driver-modularization-and-hardening.md) |
| **Stage 1** | **Dynamic Buffers & Runtime FFI ABI** | ✅ `verified` | LLVM GEP/Load/Store, SSA Phi nodes, `Op::GetIndex`/`Op::StoreIndex`, C-ABI `agam_alloc`/`agam_free`, modulo strength reduction | [`details/STAGE-01-dynamic-buffers-ffi.md`](details/STAGE-01-dynamic-buffers-ffi.md) |
| **Stage 2** | **Dynamic Structs & Enums** | ✅ `verified` | Arbitrary-field dynamic struct & enum sizing (`%AgamStruct`, `%AgamEnum`), cross-block type propagation for `Op::GetField` | [`details/STAGE-02-dynamic-structs-enums.md`](details/STAGE-02-dynamic-structs-enums.md) |
| **Stage 3** | **Direct Syscalls & OS Subsystems** | 🚀 `next` | Direct `Op::Syscall` & LLVM inline asm, zero-cost memory (`mmap`/`VirtualAlloc`), OS event multiplexing (`epoll`/`kqueue`/`IOCP`), raw non-blocking sockets | [`details/STAGE-03-direct-syscalls-pal.md`](details/STAGE-03-direct-syscalls-pal.md) |
| **Stage 4** | **C-ABI Foreign Binding Generator** | 📋 `planned` | `agam-bindgen` automated C header parser, `extern fn` bindings, direct linkage to `libc`, `libm`, `libz`, `libpng`, `libflac` | [`details/STAGE-04-foreign-bindgen.md`](details/STAGE-04-foreign-bindgen.md) |
| **Stage 5** | **SIMD Vector Engine** | 📋 `planned` | Native vector types (`vec4f32`, `vec8i32`, `vec16u8`), hardware intrinsics for AVX2, AVX-512, ARM NEON, and RISC-V RVV | [`details/STAGE-05-simd-vector-engine.md`](details/STAGE-05-simd-vector-engine.md) |
| **Stage 6** | **Stdlib Media Codecs & Async HTTP** | 📋 `planned` | 4K image convolution kernels, pure Agam 24-bit FLAC audio encoding (LPC & Rice coding), high-throughput async HTTP/1.1 & HTTP/2 | [`details/STAGE-06-stdlib-media-codecs.md`](details/STAGE-06-stdlib-media-codecs.md) |
| **Stage 7** | **Self-Hosting Bootstrap & 1:1 Benchmarks** | 📋 `planned` | Stage 0 $\rightarrow$ Stage 1 $\rightarrow$ Stage 2 self-hosting compiler compilation, transparent 1:1 benchmarks vs C++ (`clang++ -O3`) and Rust (`release`) | [`details/STAGE-07-self-hosting-bootstrap.md`](details/STAGE-07-self-hosting-bootstrap.md) |

---

## 🏛️ Archived Foundational Tier Specifications (T0–T6)

All 78 foundational specifications that formed the initial compiler architecture across T0 (Foundation), T1 (DX), T2 (Runtime/Security), T3 (Platform/Hardware), T4 (Optimization), T5 (AI/Math), and T6 (Frontier) have been compiled, verified, and archived:

- **Archive Location**: [`../archive/tier_fundamentals/`](../archive/tier_fundamentals/)
- **Archive Index & Summaries**: [`../archive/INDEX.md`](../archive/INDEX.md)
