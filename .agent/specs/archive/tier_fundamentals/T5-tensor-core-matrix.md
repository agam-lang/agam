# Phase T5-tensor-core-matrix — Tensor Core and Cooperative Matrix Integration

**Status:** complete
**Tier:** 5 (AI-Native & Hardware Tensor Accelerators)

## Goal

Provide native, vendor-neutral hardware acceleration for tensor multiplication workflows in Agam-Lang by mapping language-level primitives directly to GPU Tensor/Matrix Cores via SPIR-V's `SPV_KHR_cooperative_matrix` extension.

## Deliverables

- [x] **Type & Opcode Mapping (`agam_codegen::spirv`)**:
  - `Capability::CooperativeMatrixKHR` (6022) and `OpExtension "SPV_KHR_cooperative_matrix"`
  - `OpTypeCooperativeMatrixKHR` (4456)
  - `OpCooperativeMatrixLoadKHR` (4457)
  - `OpCooperativeMatrixStoreKHR` (4458)
  - `OpCooperativeMatrixMulAddKHR` (4459)
  - `OpCooperativeMatrixLengthKHR` (4460)
- [x] **MIR Intrinsic Propagation**:
  - Added `CooperativeMatrixLoad`, `CooperativeMatrixStore`, `CooperativeMatrixMulAdd`, `CooperativeMatrixLength` to `agam_mir::ir::GpuIntrinsicKind`.
  - Lowered to `OpCooperativeMatrixMulAddKHR` and related instructions in SPIR-V backend.
  - Multi-target adapter symbol mapping for NVPTX (`@llvm.nvvm.wmma.*`), AMDGPU (`@llvm.amdgcn.mfma.*`), SPIR-V (`@spirv.CooperativeMatrix*KHR`), and Metal (`@air.simdgroup_matrix.*`).
- [x] **Verification**:
  - `spirv::tests::test_cooperative_matrix_spirv_emission` asserting presence of `OpCooperativeMatrixMulAddKHR` (opcode 4459) in generated binary word streams.
  - 100% test pass rate across all 27 crates.

## Test Results
- 70/70 tests pass in `agam_codegen`
- 92/92 tests pass in `agam_test`
- 100% test pass rate across all 27 workspace crates
- 0 Clippy warnings (`-D warnings`)
- 100% formatting compliance (`cargo fmt --check`)
