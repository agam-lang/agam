# Chapter 32: GPU Compute Pipeline & Kernel Architecture

> **Part VIII: GPU, Hardware Acceleration & AI-Native Infrastructure**  
> **Compiler Module Focus**: [`agam_codegen::gpu_emitter`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen/src/gpu_emitter.rs), [`agam_codegen::gpu_adapter`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen/src/gpu_adapter.rs), [`agam_std::gpu`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_std/src/gpu.rs)

---

## 32.1 The GPU Programming Model in Agam

Agam provides first-class GPU compute through the `@gpu` kernel annotation. Unlike CUDA or OpenCL, which require separate source files and host-device bridging boilerplate, Agam compiles GPU kernels from the same source language using the same type system:

```agam
// GPU kernel — compiled to SPIR-V, NVPTX, or Metal shader
@gpu
fn vector_add(a: Tensor[Float], b: Tensor[Float], out: Tensor[Float]) {
    let idx = gpu.thread_id();
    out[idx] = a[idx] + b[idx];
}

// Host code — launches the kernel
fn main() {
    let a = Tensor.ones([1024]);
    let b = Tensor.ones([1024]);
    let out = Tensor.zeros([1024]);

    // Kernel launch with 1024 threads, 256 threads per block
    gpu.launch(vector_add, threads: 1024, block_size: 256, args: (a, b, out));

    println("Result: " + out[0].to_string()); // "2.0"
}
```

---

## 32.2 Compilation Pipeline: Source to GPU Binary

The GPU compilation pipeline is fully integrated with the standard compiler pipeline:

```text
Source (.agam) with @gpu annotation
       │
       ▼
  Lexer → Parser → Sema (type check kernel constraints)
       │
       ▼
  HIR (detect @gpu functions, validate GPU-compatible types)
       │
       ▼
  MIR (generate GPU-specific MIR dialect ops)
       │
       ├── GPU Dialect Lowering
       │     │
       │     ▼
       │   ┌─────────────────────────────────────────────┐
       │   │           Target Selection                   │
       │   │                                              │
       │   │  NVIDIA GPU?  ──► NVPTX Adapter ──► .ptx    │
       │   │  Vendor-Neutral? ► SPIR-V Emitter ──► .spv  │
       │   │  Apple GPU?   ──► Metal Adapter ──► .metal   │
       │   │  AMD GPU?     ──► AMDGPU via SPIR-V          │
       │   └─────────────────────────────────────────────┘
       │
       ▼
  Host Code (standard LLVM/C11 pipeline)
       │
       ▼
  Linked Binary (embeds GPU kernel binaries)
```

---

## 32.3 GPU Execution Model

### Thread Hierarchy

Agam exposes the standard GPU thread hierarchy through built-in intrinsics:

```text
Grid (entire kernel launch)
  └── Block (cooperative thread group, shared memory)
        └── Thread (individual SIMT lane)
              └── Warp/Wave (hardware scheduling unit, 32/64 threads)
```

```agam
@gpu
fn matmul_kernel(A: Tensor[Float], B: Tensor[Float], C: Tensor[Float],
                 M: Int, N: Int, K: Int) {
    let row = gpu.block_id_y() * gpu.block_dim_y() + gpu.thread_id_y();
    let col = gpu.block_id_x() * gpu.block_dim_x() + gpu.thread_id_x();

    if row < M && col < N {
        let mut sum: Float = 0.0;
        for k in 0..K {
            sum += A[row * K + k] * B[k * N + col];
        }
        C[row * N + col] = sum;
    }
}
```

### GPU Intrinsics

| Intrinsic | Returns | Description |
| :--- | :--- | :--- |
| `gpu.thread_id()` | `Int` | Global linear thread index |
| `gpu.thread_id_x/y/z()` | `Int` | Thread index within block (per dimension) |
| `gpu.block_id_x/y/z()` | `Int` | Block index within grid |
| `gpu.block_dim_x/y/z()` | `Int` | Block dimensions |
| `gpu.grid_dim_x/y/z()` | `Int` | Grid dimensions |
| `gpu.warp_id()` | `Int` | Warp index within block |
| `gpu.lane_id()` | `Int` | Lane index within warp (0–31) |
| `gpu.sync_threads()` | `Nil` | Block-level barrier synchronization |
| `gpu.sync_warp(mask)` | `Nil` | Warp-level synchronization |
| `gpu.shared_memory(size)` | `Ptr` | Allocate shared memory |
| `gpu.atomic_add(ptr, val)` | `Float` | Atomic addition |

---

## 32.4 Memory Spaces

GPU kernels operate across multiple memory spaces with different performance characteristics:

```text
┌───────────────────────────────────────────────────────┐
│                    GPU Device                          │
│                                                        │
│  ┌──────────────────────────────────────────────────┐ │
│  │ Global Memory (VRAM)    ~1-80 GB, ~900 GB/s      │ │
│  │  • Accessible by all threads                      │ │
│  │  • Highest latency (~400 cycles)                  │ │
│  └──────────────────────────────────────────────────┘ │
│                                                        │
│  ┌───────────────┐  ┌───────────────┐                 │
│  │ Shared Memory │  │ Shared Memory │  Per-Block      │
│  │ Block 0       │  │ Block 1       │  ~48-228 KB     │
│  │ ~20 cycles    │  │               │  ~12 TB/s       │
│  └───────────────┘  └───────────────┘                 │
│                                                        │
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐    Per-Thread       │
│  │Regs │ │Regs │ │Regs │ │Regs │    ~255 regs/thread │
│  │ T0  │ │ T1  │ │ T2  │ │ T3  │    ~0 cycles        │
│  └─────┘ └─────┘ └─────┘ └─────┘                     │
│                                                        │
│  ┌──────────────────────────────────────────────────┐ │
│  │ Constant Memory        ~64 KB, cached             │ │
│  │ Texture Memory         Spatial locality caching   │ │
│  └──────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────┘
```

### Shared Memory Usage in Agam

```agam
@gpu
fn tiled_matmul(A: Tensor[Float], B: Tensor[Float], C: Tensor[Float]) {
    const TILE_SIZE: Int = 16;

    // Allocate shared memory tiles
    let tile_A = gpu.shared_memory(TILE_SIZE * TILE_SIZE * 4); // Float = 4 bytes
    let tile_B = gpu.shared_memory(TILE_SIZE * TILE_SIZE * 4);

    let tx = gpu.thread_id_x();
    let ty = gpu.thread_id_y();
    let row = gpu.block_id_y() * TILE_SIZE + ty;
    let col = gpu.block_id_x() * TILE_SIZE + tx;

    let mut sum: Float = 0.0;

    // Tile loop over K dimension
    for t in 0..(K / TILE_SIZE) {
        // Cooperative load: each thread loads one element
        tile_A[ty * TILE_SIZE + tx] = A[row * K + t * TILE_SIZE + tx];
        tile_B[ty * TILE_SIZE + tx] = B[(t * TILE_SIZE + ty) * N + col];

        gpu.sync_threads();  // Wait for all threads to finish loading

        // Compute partial sum from tiles
        for k in 0..TILE_SIZE {
            sum += tile_A[ty * TILE_SIZE + k] * tile_B[k * TILE_SIZE + tx];
        }

        gpu.sync_threads();  // Wait before loading next tile
    }

    C[row * N + col] = sum;
}
```

---

## 32.5 Kernel Launch Configuration

The compiler and runtime collaborate to configure optimal kernel launches:

```agam
// Explicit launch configuration
gpu.launch(
    kernel: vector_add,
    grid: [num_blocks_x, num_blocks_y, 1],
    block: [threads_per_block_x, threads_per_block_y, 1],
    shared_memory: 48 * 1024,  // 48 KB shared memory
    stream: gpu.default_stream(),
    args: (A, B, C)
);

// Auto-configured launch (compiler selects optimal config)
gpu.launch_auto(vector_add, args: (A, B, C));
```

### Auto-Tuning Integration

When `gpu.launch_auto()` is used, the GPU genetic auto-tuner (`agam_codegen::gpu_tuner`) selects optimal thread block sizes, unrolling factors, and shared memory configurations through evolutionary search (see Chapter 35).

---

## 32.6 GPU Type Safety

The compiler enforces several GPU-specific type constraints at compile time:

| Constraint | Compile-Time Check |
| :--- | :--- |
| No heap allocation in GPU kernels | `Vec.new()`, `String.concat` → error |
| No algebraic effects in GPU code | `perform` → error |
| No recursion in GPU code | Recursive calls → error |
| No function pointers | Closures → error |
| Tensor element types must be GPU-compatible | `Float`, `Int`, `Bool` only |
| Shared memory size must be compile-time constant | Dynamic size → error |

```text
error[E0801]: heap allocation not permitted in GPU kernel
  ┌─ src/kernel.agam:5:5
  │
5 │     let v = Vec.new();
  │             ^^^^^^^^^^ heap allocation inside @gpu function
  │
  = reason: GPU kernels cannot allocate heap memory
  = help: use shared_memory() for block-local storage, or pre-allocate on host
```
