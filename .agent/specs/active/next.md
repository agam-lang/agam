# Next Implementation Order

Use this document as the canonical answer to **"what should Agam compiler engineers build next?"**

---

## 🎯 Immediate Priority Queue

1. **Stage 3: Direct System Call & OS Subsystem Engine** 🚀
   - **Why**: Essential for bare-metal OS interaction without intermediate C stdlib wrappers.
   - **Key Deliverables**:
     - MIR `Op::Syscall` & LLVM inline assembly lowering (`syscall` on x86_64, `svc #0` on aarch64, Windows NT fastcalls).
     - Direct memory management in `agam_runtime::pal::memory` (`mmap`/`munmap` on POSIX, `VirtualAlloc`/`VirtualFree` on Windows).
     - High-throughput async I/O multiplexing in `agam_runtime::pal::event` (`epoll_create1` / `kqueue` / Windows `IOCP`).
     - Raw non-blocking TCP/UDP sockets with zero-copy ring buffers.
   - **Detail Spec**: [`details/STAGE-03-direct-syscalls-pal.md`](details/STAGE-03-direct-syscalls-pal.md)

2. **Stage 0: Crate Decoupling & Inverted Driver Modularization (Parallel Track)** 🔄
   - **Why**: Fixes Windows MSVC debug stack frame overflow by decomposing the 16.7K-line god-file `agam_driver/src/main.rs`.
   - **Key Deliverables**:
     - Extract `crates/tooling/agam_target` (MSVC / LLVM / Android NDK discovery).
     - Extract `crates/tooling/agam_session` (headless compiler worker pool).
     - Resilient Pratt parser panic-mode synchronization.
   - **Detail Spec**: [`details/STAGE-00-driver-modularization-and-hardening.md`](details/STAGE-00-driver-modularization-and-hardening.md)

3. **Stage 4: C-ABI Foreign Binding Generator (`agam-bindgen`)** 📋
   - **Why**: Enables zero-overhead linkage to native system libraries (`libc`, `libm`, `libz`, `libpng`, `libflac`).
   - **Detail Spec**: [`details/STAGE-04-foreign-bindgen.md`](details/STAGE-04-foreign-bindgen.md)

4. **Stage 5: High-Performance SIMD Vector Engine** 📋
   - **Why**: First-class vector types (`vec8f32`, `vec16u8`) with AVX2/AVX-512/NEON/RVV hardware acceleration.
   - **Detail Spec**: [`details/STAGE-05-simd-vector-engine.md`](details/STAGE-05-simd-vector-engine.md)

5. **Stage 6: Production Standard Library & Media Codecs** 📋
   - **Why**: Production 4K image convolution kernels, 24-bit FLAC audio encoding, and async HTTP/1.1 & HTTP/2.
   - **Detail Spec**: [`details/STAGE-06-stdlib-media-codecs.md`](details/STAGE-06-stdlib-media-codecs.md)

6. **Stage 7: Self-Hosting Bootstrap & 1:1 Benchmark Verification** 📋
   - **Why**: Stage 0 $\rightarrow$ Stage 1 $\rightarrow$ Stage 2 self-hosting proof and transparent benchmarks vs C++ (`clang++ -O3`) and Rust (`release`).
   - **Detail Spec**: [`details/STAGE-07-self-hosting-bootstrap.md`](details/STAGE-07-self-hosting-bootstrap.md)

---

## ⛔ Anti-Priorities (Do Not Build Ahead of Fundamentals)
- Synthetic micro-benchmarks that test simple arithmetic loops without memory allocations.
- Complex speculative type theories that add complexity before the C-ABI FFI and direct syscall layers are solid.
- Windows-only or Linux-only shortcuts that break cross-platform compilation.
