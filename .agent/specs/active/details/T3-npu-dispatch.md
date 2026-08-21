# Phase T3-npu-dispatch -- Heterogeneous NPU & SIMD Tile Offloading

**Status:** complete
**Tier:** 3 (Platform and Ecosystem Breadth -- Neural Processing Unit Backend)

## Goal

Provide a heterogeneous Neural Processing Unit (NPU) and SIMD Tile instruction emission pipeline supporting Qualcomm Hexagon HVX, Apple Neural Engine (ANE), Intel NPU / AVX-512 VNNI, ARM Ethos / NEON DotProd, and Generic SIMD tile engines in `agam_codegen::npu`.

## Deliverables

- [x] **NPU Device & Target Abstractions (`agam_codegen::npu`)**:
  - `NpuTargetKind`: `QualcommHexagon`, `AppleNeuralEngine`, `IntelNpu`, `ArmEthos`, `GenericSimdTile`.
  - `NpuPrecision`: `Fp32`, `Fp16`, `Bf16`, `Int8`, `Int4`.
  - `NpuActivation`: `Relu`, `Gelu`, `Silu`, `Tanh`, `Sigmoid`.
  - `NpuTileShape`: 3D matrix tile dimensions (`m`, `n`, `k`).
  - `NpuKernelDescriptor`: Target architecture, unroll factor, precision, and tile geometry.
- [x] **Heterogeneous Tile Emitter (`emit_npu_tile_kernel`)**:
  - Vectorized tile tensor kernel generator.
  - Multi-level loop unrolling (`#pragma unroll`).
  - Fused epilogue with hardware-native activations (fast polynomial GELU, SiLU, ReLU, Tanh).
- [x] **Verification**:
  - `npu::tests::test_npu_kernel_descriptor_and_emission`
  - `npu::tests::test_npu_arm_ethos_gelu_emission`
  - 100% test pass rate across all 27 workspace crates.

## Test Results
- 74/74 tests pass in `agam_codegen`
- 100% test pass rate across all 27 workspace crates
- 0 Clippy warnings (`-D warnings`)
- 100% formatting compliance (`cargo fmt --check`)
