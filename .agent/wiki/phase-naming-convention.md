# Phase Naming Convention

This file explains the two naming schemes in `.agent/specs/active/details/` and how to read the roadmap.

## One Unified Scheme — `T{tier}-{slug}`

**Every phase in `.agent/specs/active/details/` uses the same format:**

```
T{tier}-{descriptive-slug}.md
```

Examples: `T0-type-system.md`, `T1-daemon-prewarm.md`, `T3-gpu-npu-pipeline.md`

This makes the **priority level** (tier) and **domain** (slug) immediately readable from the filename without opening the file.

> **Note on history:** Earlier phases used sequential numbers (15F, 16, 17A…). These have all been renamed to the T-prefix scheme. `current.md` still documents their progress under the new names.


## Tier Definitions

| Tier | Name | Description |
|------|------|-------------|
| **T0** | Foundation | Language surface, type system, module system. Hard dependency for everything. |
| **T1** | Developer Experience | DX, tooling, build system, LSP, docs, testing. Can start in parallel with T0. |
| **T2** | Runtime & Security | Memory model, async, networking, sandboxing, observability. Blocks T3+. |
| **T3** | Platform & Ecosystem | WASM, cross-compilation, UI, FFI, RISC-V. Needs T0–T2 stability. |
| **T4** | Optimization Depth | PGO, LTO, multi-level IR, hardware introspection, comptime. Builds on T0–T2. |
| **T5** | AI-Native | Autodiff, tensor types, ML training, edge inference. Needs T0–T4. |
| **T6** | Frontier | Self-hosting, formal verification, AI compiler intelligence. Long-horizon. |

## Naming Rules for New Phases

When creating a new open phase:

1. Pick the correct tier based on the dependency table above
2. Choose a descriptive slug: `{domain}-{capability}` (e.g., `memory-model`, `lsp-production`, `edge-ai-inference`)
3. File goes in `specs/active/details/T{tier}-{slug}.md`
4. H1 header: `# Phase T{tier}-{slug} — {Full Human Title}`
5. Add to `catalog.md` under the correct tier section
6. Add to `next.md` if it should be prioritized soon

## Cross-Reference Convention

Inside spec files, cross-references to other phases use the full phase identifier:

```
- Phase T0-type-system (generics needed for trait bounds)
- Phase T2-async-concurrency (needed for streaming inference)
- Phase 23 (GPU and NPU Integration) — note: numbered phases keep their number
```

Do NOT use the old opaque codes (`Phase F2`, `Phase O5`, `Phase AI1`) — these have all been replaced.
