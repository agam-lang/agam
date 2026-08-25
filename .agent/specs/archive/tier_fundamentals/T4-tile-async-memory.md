# Phase T4-tile-async-memory -- Tile-Centric Programming Model & Asynchronous Memory Pipelines

**Status:** complete
**Tier:** 4 (Performance and Optimization Depth -- Async Memory & TMA)

## Goal

Provide multi-dimensional tensor partition views (`Extent`, `PartitionView`), multi-buffered asynchronous pipeline tokens (`AsyncPipelineStage`) in `agam_std::gpu`, and hardware TMA asynchronous copy pipeline tracking (`AsyncPipelineTracker`, `TmaCopyDescriptor`, `TmaCopyDimension`) in `agam_codegen::tma_pipeline`.

## Deliverables

- [x] **Multi-Dimensional Data Views & Sub-Tensor Partitioning (`agam_std::gpu`)**:
  - `Extent<DIMS>`: High-level coordinate dimension descriptor tracking linear element capacity.
  - `PartitionView<'a, T>`: Strided sub-tensor slice representation enabling zero-copy sub-tensor loads into collaborative `Tile<T, ROWS, COLS>` memory.
- [x] **Asynchronous Memory Transfer Pipeline (`agam_std::gpu`, `agam_codegen::tma_pipeline`)**:
  - `AsyncPipelineStage`: Multi-stage asynchronous pipeline token tracking stage index and commitment state.
  - `TmaCopyDescriptor`, `TmaCopyDimension`: Configures 2D/3D hardware TMA box copies from global VRAM directly to shared memory.
  - `AsyncPipelineTracker`: Emits asynchronous copy intrinsics (`__tma_async_copy_2d`), commit groups (`__pipeline_commit_group()`), and multi-stage wait instructions (`__pipeline_wait_prior(N)`).
- [x] **Verification**:
  - `gpu::tests::test_partition_view_and_async_pipeline`
  - `tma_pipeline::tests::test_tma_copy_dimension_and_descriptor`
  - `tma_pipeline::tests::test_async_pipeline_stages_and_emissions`
  - 100% test pass rate across all 27 workspace crates.

## Test Results
- 87/87 tests pass in `agam_codegen`
- 145/145 tests pass in `agam_std`
- 100% test pass rate across all 27 workspace crates
- 0 Clippy warnings (`-D warnings`)
- 100% formatting compliance (`cargo fmt --check`)
