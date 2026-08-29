# Chapter 34: Tile Abstractions, Asynchronous Memory & TMA Pipelines

> **Part VIII: GPU, Hardware Acceleration & AI-Native Infrastructure**  
> **Compiler Module Focus**: [`agam_std::gpu`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_std/src/gpu.rs), [`agam_codegen::tma_pipeline`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen/src/tma_pipeline.rs)

---

## 34.1 Tile-Centric Programming Model

Modern GPU programming has shifted from **thread-centric** models (where each thread independently computes one element) to **tile-centric** models (where a group of threads cooperatively loads, computes, and stores a tile of data). Agam provides first-class tile abstractions that express this pattern naturally:

```agam
// Collaborative 2D tile — a fixed-size matrix fragment held in shared memory
let tile: Tile[Float, 16, 16] = Tile.zeros();

// Tile operations are executed cooperatively by all threads in a block
tile.load_strided(global_ptr, stride: N);   // Cooperative load from global memory
tile.store_strided(global_ptr, stride: N);  // Cooperative store to global memory

// Matrix multiplication between tiles
let C_tile = tile_matmul(A_tile, B_tile);   // Hardware-accelerated when available
```

### The `Tile<T, ROWS, COLS>` Type

`Tile<T, ROWS, COLS>` is a compile-time-sized 2D matrix fragment that maps to either:
- **Shared memory** (software tiles) for general GPU architectures
- **Register files** (hardware tiles) when targeting tensor cores via cooperative matrix operations

| Method | Description |
| :--- | :--- |
| `Tile.zeros()` | Create a zero-initialized tile |
| `Tile.load_strided(ptr, stride)` | Cooperative strided load from global memory |
| `Tile.store_strided(ptr, stride)` | Cooperative strided store to global memory |
| `tile_matmul(A, B)` | Cooperative matrix multiply (tensor core when available) |
| `tile.apply_relu()` | Element-wise ReLU activation |
| `tile.apply_gelu()` | Element-wise GELU activation |
| `tile.element_at(row, col)` | Access a single element |

### Tile-Based Matrix Multiplication

```agam
@gpu
fn gemm_tiled(A: Tensor[Float], B: Tensor[Float], C: Tensor[Float],
              M: Int, N: Int, K: Int) {
    const TILE_M: Int = 16;
    const TILE_N: Int = 16;
    const TILE_K: Int = 16;

    let block_row = gpu.block_id_y();
    let block_col = gpu.block_id_x();

    let mut accum: Tile[Float, TILE_M, TILE_N] = Tile.zeros();

    // Iterate over K dimension in tile-sized steps
    for k_tile in 0..(K / TILE_K) {
        // Cooperative tile loads
        let a_tile: Tile[Float, TILE_M, TILE_K] = Tile.zeros();
        let b_tile: Tile[Float, TILE_K, TILE_N] = Tile.zeros();

        a_tile.load_strided(A.ptr_at(block_row * TILE_M, k_tile * TILE_K), stride: K);
        b_tile.load_strided(B.ptr_at(k_tile * TILE_K, block_col * TILE_N), stride: N);

        gpu.sync_threads();

        // Tile matrix multiply-accumulate
        accum = tile_matmul(a_tile, b_tile) + accum;

        gpu.sync_threads();
    }

    // Write result tile back to global memory
    accum.store_strided(C.ptr_at(block_row * TILE_M, block_col * TILE_N), stride: N);
}
```

---

## 34.2 Multi-Dimensional Partition Views

For complex data access patterns beyond simple 2D tiles, Agam provides `PartitionView` — a strided sub-tensor view that enables zero-copy slicing of multi-dimensional tensors:

### Extent and PartitionView Types

```text
Extent<DIMS>:
  Describes the shape of a multi-dimensional region.
  Example: Extent<3> with dimensions [128, 64, 32] = a 3D volume

PartitionView<'a, T>:
  A strided view into a tensor's memory without copying data.
  Contains: data pointer, extents, strides per dimension
```

```agam
// Create a 3D tensor
let volume: Tensor[Float, 128x64x32] = Tensor.zeros([128, 64, 32]);

// Create a partition view into a sub-region
let extent = Extent.new([16, 16, 16]);  // 16×16×16 sub-volume
let view = PartitionView.from_tensor(volume, offset: [32, 0, 8], extent: extent);

// The view provides zero-copy access to the sub-region
let value = view.get(4, 7, 2);  // Reads volume[36, 7, 10]
```

### Use Case: Tiled 3D Convolution

Partition views enable efficient tiled iteration over multi-dimensional data:

```agam
@gpu
fn conv3d_tiled(input: Tensor[Float], kernel: Tensor[Float],
                output: Tensor[Float]) {
    let tile_extent = Extent.new([8, 8, 8]);

    // Each thread block processes one tile of the output
    let bx = gpu.block_id_x();
    let by = gpu.block_id_y();
    let bz = gpu.block_id_z();

    // Create a view into the input region needed for this output tile
    // (includes halo for kernel overlap)
    let halo = kernel.shape() / 2;
    let input_view = PartitionView.from_tensor(
        input,
        offset: [bx * 8 - halo.x, by * 8 - halo.y, bz * 8 - halo.z],
        extent: Extent.new([8 + kernel.dim(0), 8 + kernel.dim(1), 8 + kernel.dim(2)])
    );

    // Compute convolution within the tile
    // ...
}
```

---

## 34.3 Asynchronous Memory Pipeline Architecture

On modern GPUs (NVIDIA Ampere/Hopper, AMD CDNA), data transfers between global memory (VRAM) and shared memory can execute **asynchronously** — the compute units continue executing while the memory controller handles the copy in the background.

### The Problem: Memory Latency Hiding

```text
Traditional Synchronous Pattern:
  Load tile → [400 cycles wait] → Compute → Load next tile → [400 cycles wait] → ...
  Utilization: ~40% (GPU stalls waiting for memory)

Asynchronous Pipeline Pattern:
  Stage 0: Load tile_0 (async)
  Stage 1: Load tile_1 (async), Compute tile_0
  Stage 2: Load tile_2 (async), Compute tile_1
  ...
  Utilization: ~95% (compute overlaps with memory transfers)
```

### `AsyncPipelineStage` — Multi-Buffer Token Tracking

```agam
// Create a 3-stage pipeline (triple buffering)
let mut stage_0 = AsyncPipelineStage.new(stage_index: 0);
let mut stage_1 = AsyncPipelineStage.new(stage_index: 1);
let mut stage_2 = AsyncPipelineStage.new(stage_index: 2);

// Stage 0: Issue async load for first tile
stage_0.begin();
async_copy(shared_buf[0], global_ptr_0, size: TILE_BYTES);
stage_0.commit();

// Stage 1: Issue async load for second tile + wait for stage 0
stage_1.begin();
async_copy(shared_buf[1], global_ptr_1, size: TILE_BYTES);
stage_1.commit();
stage_0.wait();  // Wait only for stage 0 to complete

// Now compute on tile 0 while tile 1 is still loading
compute(shared_buf[0]);

// Stage 2: Issue async load for third tile + wait for stage 1
stage_2.begin();
async_copy(shared_buf[2], global_ptr_2, size: TILE_BYTES);
stage_2.commit();
stage_1.wait();

compute(shared_buf[1]);
// ... continues rotating through buffers
```

---

## 34.4 Hardware TMA (Tensor Memory Accelerator) Pipelines

The NVIDIA Hopper architecture introduces the **Tensor Memory Accelerator (TMA)** — a dedicated hardware unit that can perform multi-dimensional asynchronous copies from global memory directly to shared memory without consuming SM compute cycles.

### TMA Copy Descriptors

The Agam compiler generates TMA copy descriptors that configure hardware-accelerated transfers:

```text
TmaCopyDescriptor:
  ┌──────────────────────────────────────┐
  │ Global Base Address (VRAM pointer)    │
  │ Dimensions:                           │
  │   Dim 0: size=128, stride=512 bytes   │
  │   Dim 1: size=64,  stride=65536 bytes │
  │ Element Size: 4 bytes (Float32)       │
  │ Swizzle Mode: None / 32B / 64B / 128B│
  │ Fill Mode: None (or zero-fill OOB)    │
  └──────────────────────────────────────┘
```

### `AsyncPipelineTracker` — Codegen Intrinsic Emission

The `AsyncPipelineTracker` in `agam_codegen::tma_pipeline` manages the state machine for multi-stage TMA pipelines and emits the correct GPU intrinsics:

```text
AsyncPipelineTracker State Machine:

  ┌─────────┐  begin()   ┌──────────┐  commit()  ┌───────────┐
  │  Idle    │───────────►│ Loading  │────────────►│ Committed │
  └─────────┘            └──────────┘             └─────┬─────┘
       ▲                                                 │
       │              wait_prior(N)                      │
       └─────────────────────────────────────────────────┘
```

**Emitted GPU intrinsics:**

| Tracker Method | Emitted Intrinsic | Purpose |
| :--- | :--- | :--- |
| `begin()` | (state transition only) | Mark pipeline stage as active |
| `async_copy_2d(desc)` | `__tma_async_copy_2d(desc, shared_ptr)` | Issue 2D TMA copy |
| `commit()` | `__pipeline_commit_group()` | Close the current async group |
| `wait_prior(N)` | `__pipeline_wait_prior(N)` | Wait until ≤N groups remain in flight |

### Complete TMA Pipeline Example

```agam
@gpu
fn gemm_tma(A: Tensor[Float], B: Tensor[Float], C: Tensor[Float]) {
    const TILE_M: Int = 128;
    const TILE_N: Int = 128;
    const TILE_K: Int = 32;
    const NUM_STAGES: Int = 3;

    // Shared memory buffers for triple-buffered pipeline
    let smem_a: [Tile[Float, TILE_M, TILE_K]; NUM_STAGES];
    let smem_b: [Tile[Float, TILE_K, TILE_N]; NUM_STAGES];

    let mut accum: Tile[Float, TILE_M, TILE_N] = Tile.zeros();

    // Prologue: fill pipeline stages
    for stage in 0..NUM_STAGES {
        let k_offset = stage * TILE_K;
        tma_async_copy_2d(smem_a[stage], A, row: block_row * TILE_M, col: k_offset);
        tma_async_copy_2d(smem_b[stage], B, row: k_offset, col: block_col * TILE_N);
        pipeline_commit();
    }

    // Main loop: rotate through pipeline stages
    let num_k_tiles = K / TILE_K;
    for k in 0..num_k_tiles {
        let stage = k % NUM_STAGES;

        // Wait for current stage's data to arrive
        pipeline_wait_prior(NUM_STAGES - 1);

        // Compute on the arrived tile
        accum = tile_matmul(smem_a[stage], smem_b[stage]) + accum;

        // Issue next async copy (pipeline ahead)
        let next_k = k + NUM_STAGES;
        if next_k < num_k_tiles {
            let next_stage = next_k % NUM_STAGES;
            tma_async_copy_2d(smem_a[next_stage], A, row: block_row * TILE_M, col: next_k * TILE_K);
            tma_async_copy_2d(smem_b[next_stage], B, row: next_k * TILE_K, col: block_col * TILE_N);
            pipeline_commit();
        }
    }

    // Write result
    accum.store_strided(C.ptr_at(block_row * TILE_M, block_col * TILE_N), stride: N);
}
```

---

## 34.5 Performance Impact

The combination of tile abstractions, partition views, and asynchronous TMA pipelines yields substantial performance improvements:

| Technique | Improvement | Mechanism |
| :--- | :--- | :--- |
| Tiled shared memory | **3–5×** over naive global | Reduces global memory bandwidth pressure |
| Cooperative matrix (tensor cores) | **8–16×** over CUDA cores | Dedicated matrix multiply-accumulate hardware |
| Async pipeline (double buffer) | **1.5–2×** over synchronous | Overlaps compute with memory transfer |
| TMA hardware copy | **1.2–1.5×** over software async | Frees SM warps from copy work |
| Combined (all above) | **30–50×** over naive | Approaches peak hardware FLOPS |

These optimizations are critical for achieving competitive performance on matrix-heavy AI workloads (GEMM, convolution, attention), where memory bandwidth — not compute — is typically the bottleneck.
