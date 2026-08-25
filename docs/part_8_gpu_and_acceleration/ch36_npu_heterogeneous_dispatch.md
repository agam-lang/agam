# Chapter 36: NPU Heterogeneous Dispatch & Neural Accelerator Offloading

> **Part VIII: GPU, Hardware Acceleration & AI-Native Infrastructure**  
> **Compiler Module Focus**: [`agam_codegen::npu`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen/src/npu.rs), [`agam_std::gpu`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_std/src/gpu.rs)

---

## 36.1 The Heterogeneous Compute Landscape

Modern devices contain multiple compute units with radically different performance profiles:

```text
┌─────────────────────────────────────────────────────────────┐
│                    Modern SoC / System                       │
│                                                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │   CPU     │  │   GPU    │  │   NPU    │  │   DSP    │   │
│  │ General   │  │ Parallel │  │ Neural   │  │ Signal   │   │
│  │ Purpose   │  │ Compute  │  │ Accel    │  │ Process  │   │
│  │           │  │          │  │          │  │          │   │
│  │ 100 GFLOPS│  │ 10 TFLOPS│  │ 40 TOPS  │  │ 2 TFLOPS │   │
│  │ FP64/FP32 │  │ FP32/FP16│  │ INT8/INT4│  │ Fixed-pt │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
│                                                              │
│  Best for:     Best for:     Best for:     Best for:        │
│  Control flow  Parallel math  ML inference  Audio/Sensor    │
│  I/O, OS      Training       Edge AI       Filtering       │
└─────────────────────────────────────────────────────────────┘
```

Agam's heterogeneous dispatch system automatically routes computation to the optimal accelerator based on workload characteristics and hardware availability.

---

## 36.2 Supported NPU Targets

| NPU Architecture | Vendor | Key Features | Agam Support |
| :--- | :--- | :--- | :---: |
| **Hexagon HVX** | Qualcomm | 1024-bit SIMD, INT8/FP16 | ✅ |
| **Apple Neural Engine** | Apple | 16-core matrix engine, INT8/FP16 | ✅ |
| **ARM Ethos-U** | ARM | Micro-NPU for Cortex-M, INT8/INT4 | ✅ |
| **Intel NPU (Meteor Lake)** | Intel | 10 TOPS INT8, integrated | 🔄 Planned |
| **AMD XDNA** | AMD | AI engine tiles, INT8/FP16 | 🔄 Planned |

---

## 36.3 NPU Kernel Compilation

NPU kernels are annotated with `@npu` and follow the tile-based programming model:

```agam
@npu
fn conv2d_npu(input: Tensor[Float, 1x3x224x224],
              weights: Tensor[Float, 64x3x3x3],
              output: Tensor[Float, 1x64x222x222]) {
    // Tile-based convolution optimized for NPU vector units
    let tile_h: Int = 8;
    let tile_w: Int = 8;

    for oc in 0..64 {
        for oh in range_step(0, 222, tile_h) {
            for ow in range_step(0, 222, tile_w) {
                let mut accum = Tile[Float, tile_h, tile_w].zeros();

                for ic in 0..3 {
                    for kh in 0..3 {
                        for kw in 0..3 {
                            let input_tile = input.load_tile(
                                batch: 0, channel: ic,
                                row: oh + kh, col: ow + kw,
                                height: tile_h, width: tile_w
                            );
                            let weight = weights[oc, ic, kh, kw];
                            accum = accum + input_tile * weight;
                        }
                    }
                }

                accum.apply_relu();
                output.store_tile(batch: 0, channel: oc,
                                  row: oh, col: ow, tile: accum);
            }
        }
    }
}
```

### NPU-Specific Lowering

The compiler lowers `@npu` kernels to target-specific tile instructions:

| Agam Operation | Hexagon HVX | Apple ANE | ARM Ethos |
| :--- | :--- | :--- | :--- |
| `Tile.load()` | `vmem()` load | DMA descriptor | SRAM DMA |
| `tile * scalar` | `vmpy()` | MAC unit | INT8 multiply |
| `tile + tile` | `vadd()` | Accumulator | INT8 add |
| `apply_relu()` | `vmax(tile, 0)` | Fused activation | LUT activation |
| `Tile.store()` | `vmem()` store | DMA descriptor | SRAM DMA |

---

## 36.4 Heterogeneous Device Selection & Fallback

The runtime automatically selects the best available accelerator for each computation:

```text
Workload Analysis
      │
      ├── Is it a matrix/tensor operation?
      │     │
      │     ├── GPU available with sufficient VRAM?
      │     │     └── Yes → Dispatch to GPU (SPIR-V/NVPTX)
      │     │
      │     ├── NPU available with supported precision?
      │     │     └── Yes → Dispatch to NPU (INT8/FP16)
      │     │
      │     └── CPU SIMD available?
      │           └── Yes → Dispatch to CPU (AVX-512/NEON)
      │
      ├── Is it a neural network inference workload?
      │     │
      │     ├── NPU available? → Prefer NPU (best perf/watt)
      │     ├── GPU available? → Fallback to GPU
      │     └── CPU only?      → Fallback to CPU SIMD
      │
      └── Is it general compute?
            └── CPU (default)
```

### Priority Configuration

Users can override the default dispatch priority:

```toml
# In agam.toml
[accelerator]
priority = ["npu", "gpu", "cpu"]     # Prefer NPU over GPU
gpu_min_vram_mb = 2048               # Skip GPU if < 2GB VRAM
npu_precision = "int8"               # Quantize to INT8 for NPU
fallback = "cpu"                     # Always fall back to CPU
```

---

## 36.5 Fused Activation Primitives

NPU architectures typically fuse computation and activation into a single instruction. Agam's tile operations support fused execution:

```agam
// Unfused (2 separate operations):
let h = tile_matmul(A, B);  // Matrix multiply
let y = h.apply_relu();     // Activation

// Fused (single NPU instruction):
let y = tile_matmul_relu(A, B);  // Fused MMA + ReLU
```

### Supported Fused Operations

| Fused Operation | Description | NPU Instruction |
| :--- | :--- | :--- |
| `tile_matmul_relu(A, B)` | MMA + ReLU | Single MAC cycle |
| `tile_matmul_gelu(A, B)` | MMA + GELU | MAC + LUT activation |
| `tile_matmul_sigmoid(A, B)` | MMA + Sigmoid | MAC + LUT activation |
| `tile_conv_relu(input, kernel)` | Conv2D + ReLU | Fused convolution pipeline |
| `tile_add_relu(A, B)` | Element-wise add + ReLU | Fused vector op |

The compiler's **fusion pass** automatically detects unfused patterns and rewrites them to fused versions when the target NPU supports the fused instruction.

---

## 36.6 Quantization for NPU Deployment

NPU hardware typically operates at reduced precision (INT8, INT4) for efficiency. The compiler supports automatic quantization:

```agam
// Quantize a floating-point model for NPU deployment
@quantize(precision: "int8", calibration: "minmax")
fn inference(input: Tensor[Float]) -> Tensor[Float] {
    let h1 = Tensor.relu(input * weights_1 + bias_1);
    let h2 = Tensor.relu(h1 * weights_2 + bias_2);
    return h2 * weights_3 + bias_3;
}
```

### Quantization Pipeline

```text
FP32 Model
    │
    ▼
  Calibration Pass (run representative inputs, collect min/max ranges)
    │
    ▼
  Scale/Zero-Point Computation (per-tensor or per-channel)
    │
    ▼
  INT8 Kernel Generation (quantized matmul, quantized activations)
    │
    ▼
  NPU Binary (optimized INT8 tile kernels)
```

| Precision | Compute Efficiency | Accuracy Loss | Use Case |
| :--- | :---: | :---: | :--- |
| FP32 | 1× | None | Training, research |
| FP16 | 2× | ~0.1% | GPU inference |
| INT8 | 4× | ~1% | NPU edge inference |
| INT4 | 8× | ~3-5% | Ultra-low-power edge |
