# Next Implementation Order

Use this as the default answer to "what should Agam agents build next?"

## Recommended Order

1. **Phase T4-egraph-square-zero: E-Graph Superoptimization & Algebraic Tensor Fusion**
   - E-Graph algebraic equality saturation and square-zero tensor kernel transforms
   - Detail file: `details/T4-egraph-square-zero.md`

2. **Phase T5-jit-compilation: Tiered JIT Execution Engine & On-Stack Replacement**
   - Tier-2 optimizing JIT, profile-guided deoptimization, and OSR transitions
   - Detail file: `details/T5-jit-compilation.md`

3. **Phase T5-polyhedral-loop: Polyhedral Loop Transformations & Affine Scheduling**
   - Integer polyhedral models, iteration space skewing, and affine loop tiling
   - Detail file: `details/T5-polyhedral-loop.md`

5. **Continue Phase T1-headless-exec**
   - Extend the execution-policy contract beyond source/arg limits and native-backend gating
   - Add stronger OS-level isolation (Chāṇakya *Durdharṣa* sandboxing) for filesystem, network, process, and runtime resource usage
   - Detail file: `details/T1-headless-exec.md`

## What Not To Prioritize First

- macOS/iOS backend bring-up beyond planning and driver hooks
- broad new language-surface expansion that distracts from the native LLVM product path
- long-horizon model-training phases ahead of the hosted SDK proof, execution sandbox hardening, and wrapper validation
- WSL-only shortcuts that weaken the real host-toolchain story
- Tier 5–6 AI-native phases before Tier 0-2 foundation is solid

## Tier Dependency Flow

```
Tier 0 (Foundation) — 100% COMPLETE
  └→ Tier 1 (DX) — in progress
  └→ Tier 2 (Runtime + Security) — blocks Tier 3+
       └→ Tier 3 (Platform & Hardware)
            └→ Tier 4 (Optimization Depth)
                 └→ Tier 5 (AI-Native)
                      └→ Tier 6 (Frontier)
```

See `catalog.md` for the full roadmap and `.agent/wiki/dependency-map.md` for the detailed phase dependency graph.
