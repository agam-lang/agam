# Next Implementation Order

Use this as the default answer to "what should Agam agents build next?"

## Recommended Order

1. **Phase T4-game-engine: 2D/3D ECS Architecture & Math Pipeline**
   - Entity-Component-System engine, spatial hierarchies, and linear algebra
   - Detail file: `details/T4-game-engine.md`

2. **Phase T3-npu-dispatch: Heterogeneous NPU & SIMD Tile Offloading**
   - Native SIMD tile tensor instruction emission and heterogeneous acceleration backend
   - Detail file: `details/T3-npu-dispatch.md`

3. **Phase T3-universal-ffi: Zero-Overhead Multi-Language C/Rust/Python Interop**
   - Direct FFI bindings, ABI layout conversion, and hot-path C-call bridge
   - Detail file: `details/T3-universal-ffi.md`

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
