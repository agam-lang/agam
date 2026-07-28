# Next Implementation Order

Use this as the default answer to "what should Agam agents build next?"

## Recommended Order

1. **Phase T3-gpu-npu-pipeline: GPU and NPU Integration (Advanced)**
   - Support Rich Memory Types beyond the new fixed-size array parameter/shared-memory path
   - Keep extending the richer kernel argument/memory model beyond the current typed pointer-depth and shared-memory launch path
   - Detail file: `details/T3-gpu-npu-pipeline.md`

2. **Finish Phase T1-sdk-distribution**
   - Exercise the hosted-runner Windows/Linux SDK flow on GitHub with bundled LLVM and the new post-download archive validation
   - Confirm one end-to-end release publication and Android target-pack packaging path on hosted infrastructure
   - Detail file: `details/T1-sdk-distribution.md`

3. **Continue Phase T1-headless-exec**
   - Extend the execution-policy contract beyond source/arg limits and native-backend gating
   - Add stronger OS-level isolation for filesystem, network, process, and runtime resource usage
   - Detail file: `details/T1-headless-exec.md`

4. **Continue Phase T1-python-wrappers**
   - Validate the packaged LangChain/LlamaIndex adapter hooks against live framework releases
   - Publish the optional-extras integration story beyond the repo-local test doubles
   - Detail file: `details/T1-python-wrappers.md`

5. **Continue Phase T0-stdlib-io**
   - Build on the new `agam_std::io` file/path helpers and move native I/O toward an effects-aware language contract
   - Keep standard-library growth aligned with the effects model and official package governance
   - Detail file: `details/T0-stdlib-io.md`

6. **Begin T0-type-system** *(Foundation Tier 0)*
   - This is the single highest-leverage foundation gap: no real stdlib, no safe error handling, no collections without generics
   - Implement generics, sum types, pattern matching, Option/Result end-to-end through the compiler pipeline
   - Detail file: `details/T0-type-system.md`

7. **Begin T1-error-messages** *(can run in parallel with T0)*
   - Error message quality is a top adoption driver — can start during T0 implementation
   - Parser error recovery and rich source-span diagnostics are independent of the type system
   - Detail file: `details/T1-error-messages.md`

8. **Begin T1-compiler-agent-tool** *(extends 18/19)*
   - Natural follow-on once Phase T1-headless-exec/19 stabilize
   - `agamc mcp serve`, SARIF output, diagnostic stability contract
   - Detail file: `details/T1-compiler-agent-tool.md`

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
