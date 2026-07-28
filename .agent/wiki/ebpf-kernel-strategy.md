# eBPF and Kernel Observability Strategy

**Context:** Agam-Lang aims to deliver high-performance systems and observability tools with modern language ergonomics. The standard approach to eBPF in languages like Go and Rust involves separating the eBPF kernel C code (compiled with clang) from the userspace loading and control logic.

**Observation:** The release of **KernelScript** highlights a growing demand for a unified development experience where eBPF kernel probes, tracepoints, userspace code, and modules coexist in a single source file, drastically reducing boilerplate and friction.

**Relevance to Agam-Lang:**
- **Unified Codebase:** Agam will introduce an `@ebpf` annotation. Functions marked with this will be compiled using LLVM's `bpf` target backend, while the rest of the file compiles to the host target.
- **Automated Lifecycle:** The Agam runtime will automatically synthesize the equivalent of `libbpf` loading, map linking, and event polling code, hiding the low-level sys-calls from the user.
- **Type Safety:** By maintaining both kernel space and userspace code in the same typed AST before lowering, Agam guarantees that data structures shared via eBPF Maps (like RingBuffers) perfectly align, eliminating silent struct-packing bugs.
- **Strategic Impact:** This makes Agam-Lang an exceptionally powerful tool for writing high-performance network filters (XDP), security monitoring, and distributed tracing, natively outcompeting raw C or complex Rust/eBPF setups in developer velocity.

*Added in response to the KernelScript evaluation (Phase 30).*
