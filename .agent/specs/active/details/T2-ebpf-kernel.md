# Phase T2-ebpf-kernel � eBPF and Kernel Observability Integration

## Goal

Provide a unified, type-safe development experience for writing eBPF kernel probes, tracepoints, and their userspace loaders/controllers directly in a single `.agam` file.

## Background

Traditional eBPF development involves writing raw C code, using external utilities like `bpftool` or `clang` directly, and writing separate userspace components to load and interface with the kernel probes (e.g., via `libbpf-rs`). Inspired by projects like **KernelScript**, Agam-Lang can abstract away this boilerplate.

Because Agam already targets LLVM and LLVM natively supports the `bpf` target backend, Agam is perfectly positioned to handle both userspace application logic and kernel-space eBPF logic seamlessly within the same source file.

## Implementation Checklist

- [ ] **eBPF Annotation:** Introduce the `@ebpf` (Avyayībhāva) annotation for functions or blocks that should be compiled as eBPF kernel objects.
- [ ] **IR Lowering & Backend Target:** Enhance `agam_codegen` to invoke the LLVM `bpf` backend when generating code for `@ebpf` annotated blocks. This ensures the generated object files comply with the eBPF verifier's strict requirements.
- [ ] **Map Management:** Provide a standard library interface (`agam_std::ebpf::Map`) that safely creates and manages eBPF maps (Hash, Array, RingBuffer) and unifies the memory access across both userspace and kernel spaces safely.
- [ ] **Automated Loading:** Extend the Agam runtime to automatically parse, load, and attach the embedded eBPF objects using Linux system calls (wrapping functionality similar to `libbpf`), hiding the lifecycle complexity from the user.
- [ ] **Validation:** Create an end-to-end benchmark/test suite that drops packets (XDP) and traces syscalls (kprobes), asserting that the compiler effectively catches verifier constraints at compile time.

## Dependencies

- Phase T2-os-sandbox (Runtime Hardening)
- Phase 15A/15B (LLVM backend stabilization)
