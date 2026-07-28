# Next Implementation Order

Use this as the default answer to "what should Agam agents build next?"

## Recently Completed

- **Phase 15H** — Native LLVM SDK distribution and toolchain bundles
- **Phase 17F** — Standard library, effects runtime, and native I/O expansion
- **Phase 18** — Agent-facing execution tool with OS-level sandbox
- **Phase 19** — Wrapper foundation for agent ecosystems (Python/LangChain/LlamaIndex)
- **Phase 20** — Language Surface Expansion (first-class effect/handler/perform syntax)
- **Phase 21** — Runtime Hardening (Win32 Job Object and Linux prctl/setrlimit sandboxing)
- **Phase 22** — Omni-Targeting Directives (`@target.iot`, `@target.enterprise`, `@target.hpc`)

## Recommended Next Steps

1. **Phase 23: GPU and NPU Integration (Advanced)**
   - Support Rich Memory Types beyond the current annotated pointer/slice shared-memory path
   - Replace the current host-side kernel-launch placeholder with real CUDA launch lowering once the richer kernel argument model is in place
   - Detail file: `details/23.md`

## What Not To Prioritize First

- macOS/iOS backend bring-up beyond planning and driver hooks
- broad new language-surface expansion that distracts from the native LLVM product path
- long-horizon model-training phases ahead of the runtime hardening now in 20/21/22
- WSL-only shortcuts that weaken the real host-toolchain story
