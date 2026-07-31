# Next Implementation Order

Use this as the default answer to "what should Agam agents build next?"

## Recommended Order

1. **Phase T0-type-system: Foundation Type System (Phase C: Struct Aggregates & Tuple Destructuring)**
   - Single highest-leverage gap: completes `Option<T>`, `Result<T, E>`, struct construction (`Point { x, y }`), field access (`Op::FieldAccess`), and tuple destructuring (`let (a, b) = pair`) in MIR across all backends.
   - Implement type inference, sum types/enums, pattern matching, and monomorphization end-to-end through AST → HIR → MIR → LLVM/Universal GPU/C/JIT
   - Detail file: `details/T0-type-system.md`

2. **Phase T1-compiler-agent-tool: Native MCP Server (`agamc mcp serve`)**
   - Implement native Model Context Protocol (MCP) server directly inside `agamc` (`agamc mcp serve`)
   - Expose zero-latency diagnostic streaming, SARIF output, AST symbol inspection, and automated code refactoring tools for LLM agent integration
   - Detail file: `details/T1-compiler-agent-tool.md`

3. **Phase T1-error-messages: Parser Recovery & Visual Spans**
   - Upgrade parser error recovery and visual source-span highlights using `miette` and `codespan-reporting`
   - Enable single-pass multi-error reporting without panicking on the first error
   - Detail file: `details/T1-error-messages.md`

4. **Phase T3-gpu-target-adapter: Universal GPU Target Adapter Interface**
   - Introduce abstract `GpuTargetAdapter` trait interface in `agam_codegen`
   - Decouple target-agnostic GPU MIR lowering from target assembly generation, enabling AMDGPU (ROCm/HIP), SPIR-V (Vulkan/oneAPI), and Metal adapters alongside NVPTX
   - Detail file: `details/T3-gpu-target-adapter.md`

5. **Finish Phase T1-sdk-distribution**
   - Exercise the hosted-runner Windows/Linux SDK flow on GitHub with bundled LLVM and post-download archive validation
   - Confirm one end-to-end release publication and Android target-pack packaging path on hosted infrastructure
   - Detail file: `details/T1-sdk-distribution.md`

6. **Continue Phase T1-headless-exec**
   - Extend the execution-policy contract beyond source/arg limits and native-backend gating
   - Add stronger OS-level isolation for filesystem, network, process, and runtime resource usage
   - Detail file: `details/T1-headless-exec.md`

## What Not To Prioritize First

- macOS/iOS backend bring-up beyond planning and driver hooks
- broad new language-surface expansion that distracts from the native LLVM product path
- long-horizon model-training phases ahead of the hosted SDK proof, execution sandbox hardening, and wrapper validation now in 15H/18/19
- WSL-only shortcuts that weaken the real host-toolchain story
- Tier 5–6 AI-native phases before Tier 0 foundation is solid

## Tier Dependency Flow

```
Tier 0 (Foundation)
  └→ Tier 1 (DX) — can start in parallel
  └→ Tier 2 (Runtime + Security) — blocks Tier 3+
       └→ Tier 3 (Platform)
            └→ Tier 4 (Optimization)
                 └→ Tier 5 (AI-Native)
                      └→ Tier 6 (Frontier)
```

See `catalog.md` for the full roadmap and `.agent/wiki/dependency-map.md` for the detailed phase dependency graph.
