# Chapter 35: Hardware Introspection, Layout Optimization & SIMD Multi-Versioning

> **Part VIII: GPU, Hardware Acceleration & AI-Native Infrastructure**  
> **Compiler Module Focus**: [`agam_runtime::hwinfo`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime), [`agam_codegen::layout_opt`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen/src/layout_opt.rs), [`agam_codegen::gpu_tuner`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen/src/gpu_tuner.rs)

---

## 35.1 Runtime Hardware Introspection

The Agam runtime detects hardware capabilities at startup to enable adaptive optimization decisions. The `agam_runtime::hwinfo` module queries:

### CPU Telemetry

| Property | Detection Method | Purpose |
| :--- | :--- | :--- |
| Architecture | `cpuid` (x86), `/proc/cpuinfo` (Linux) | Backend selection |
| SIMD Features | `cpuid` leaf 1/7 (SSE, AVX, AVX-512) | SIMD multi-versioning |
| Cache Hierarchy | `cpuid` leaf 4 (L1/L2/L3 sizes, line size) | Struct layout optimization |
| Core Count | OS API (`GetSystemInfo` / `sysconf`) | Parallel build scheduling |
| NUMA Topology | `GetLogicalProcessorInformation` / `lscpu` | Memory affinity |

### GPU Telemetry

| Property | Detection Method | Purpose |
| :--- | :--- | :--- |
| VRAM Size | Vulkan `vkGetPhysicalDeviceMemoryProperties` | Tile size selection |
| Compute Units / SMs | Vulkan `vkGetPhysicalDeviceProperties` | Thread block config |
| Shared Memory Size | Vulkan device limits | Tile buffer allocation |
| Tensor Core Support | Vulkan extensions query | Cooperative matrix dispatch |
| Max Threads/Block | Device properties | Kernel launch bounds |

### NPU Telemetry

| Property | Purpose |
| :--- | :--- |
| Vector Width | Tile kernel dimensions for Hexagon HVX / ARM Ethos |
| Peak TOPS | Workload scheduling priority |
| Supported Precisions | FP16/INT8/INT4 kernel selection |

---

## 35.2 Cache-Aware Struct Field Reordering

The `StructLayoutOptimizer` in `agam_codegen::layout_opt` reorders struct fields to minimize padding holes and optimize cache line utilization:

### The Problem: Padding Waste

```agam
// Programmer-defined order (naive)
struct Sensor {
    active: Bool,      //  1 byte
    // [7 bytes padding]  ← wasted
    timestamp: Int,    //  8 bytes
    value: Float,      //  8 bytes
    channel: u8,       //  1 byte
    // [7 bytes padding]  ← wasted
}
// Total: 32 bytes (14 bytes wasted = 44% padding!)
```

### Compiler-Optimized Layout

The `StructLayoutOptimizer` sorts fields by alignment (largest first) to eliminate padding:

```agam
// Compiler-reordered layout (transparent to programmer)
struct Sensor {
    timestamp: Int,    //  8 bytes  (align 8)
    value: Float,      //  8 bytes  (align 8)
    active: Bool,      //  1 byte   (align 1)
    channel: u8,       //  1 byte   (align 1)
    // [6 bytes padding]  ← only end padding
}
// Total: 24 bytes (6 bytes padding = 25% — saved 8 bytes per instance!)
```

**Impact at scale:** For an array of 1 million `Sensor` values, this saves **8 MB** of memory and significantly improves cache utilization.

### When Reordering is Disabled

Field reordering is **disabled** for:
- Structs annotated with `@repr(C)` — must match C ABI layout
- Structs used in FFI — field order is part of the binary contract
- Structs annotated with `@repr(packed)` — no padding allowed

---

## 35.3 Array-of-Structs to Struct-of-Arrays (AoS → SoA)

The `AosToSoaTransform` automatically restructures data layout when the compiler detects that only a subset of fields is accessed in hot loops:

### The Problem: Cache Pollution

```agam
struct Particle {
    position: Vec3,  // 24 bytes — accessed in physics loop
    color: Color,    // 16 bytes — NOT accessed in physics loop
    velocity: Vec3,  // 24 bytes — accessed in physics loop
    metadata: String // 24 bytes — NOT accessed in physics loop
}

// Array-of-Structs: each particle is 88 bytes
let particles: [Particle; 10000];

// Physics loop accesses only position and velocity (48 of 88 bytes)
// But every cache line loads all 88 bytes per particle
for p in particles {
    p.position += p.velocity * dt;  // 45% useful data per cache line
}
```

### Compiler-Transformed SoA Layout

```text
// Struct-of-Arrays (compiler-generated):
struct ParticleSoA {
    positions:  [Vec3; 10000],   // Contiguous position data
    velocities: [Vec3; 10000],   // Contiguous velocity data
    colors:     [Color; 10000],  // Separate, not loaded by physics
    metadata:   [String; 10000], // Separate, not loaded by physics
}

// Physics loop now accesses contiguous memory:
// 100% useful data per cache line → 2.2× throughput improvement
```

The transformation is applied automatically when the compiler's **field access analysis** determines that a hot loop accesses fewer than 50% of a struct's fields.

---

## 35.4 SIMD Multi-Versioning Dispatch

The `SimdMultiVersionDispatcher` generates multiple versions of performance-critical functions, each optimized for a different SIMD instruction set, and selects the best version at runtime:

### Architecture

```text
Compile Time:
  fn hot_function(data: [Float]) → { body }
       │
       ├── Compile with SSE4.2 target features  → hot_function_sse42
       ├── Compile with AVX2 target features     → hot_function_avx2
       ├── Compile with AVX-512 target features  → hot_function_avx512
       └── Compile with scalar fallback          → hot_function_scalar

Runtime (first call):
  cpuid → detect available features
       │
       ├── AVX-512 supported? → dispatch = hot_function_avx512
       ├── AVX2 supported?    → dispatch = hot_function_avx2
       ├── SSE4.2 supported?  → dispatch = hot_function_sse42
       └── Otherwise          → dispatch = hot_function_scalar
```

### SIMD Feature Tiers

| Tier | Features | Vector Width | Typical Hardware |
| :--- | :--- | :---: | :--- |
| **Tier 0** | Scalar | 1 | Any x86-64 |
| **Tier 1** | SSE4.2 | 128-bit (4 floats) | Intel Core 2+ / AMD Phenom II+ |
| **Tier 2** | AVX2 + FMA | 256-bit (8 floats) | Intel Haswell+ / AMD Zen+ |
| **Tier 3** | AVX-512 | 512-bit (16 floats) | Intel Skylake-X+ / AMD Zen 4+ |
| **ARM Tier 1** | NEON | 128-bit (4 floats) | All ARM64 |
| **ARM Tier 2** | SVE/SVE2 | 128–2048-bit | ARM Neoverse V1+ |

### Usage with `@accelerate`

```agam
@accelerate
fn dot_product(a: [Float], b: [Float]) -> Float {
    let mut sum: Float = 0.0;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    return sum;
}

// The @accelerate annotation triggers multi-version generation.
// At runtime, the fastest available version is automatically selected.
```

---

## 35.5 GPU Genetic Auto-Tuner

The `GpuGeneticAutoTuner` in `agam_codegen::gpu_tuner` uses an **evolutionary algorithm** to search for optimal GPU kernel configurations:

### Search Space

| Parameter | Range | Description |
| :--- | :--- | :--- |
| Thread Block X | 32–1024 | Threads per block (X dimension) |
| Thread Block Y | 1–32 | Threads per block (Y dimension) |
| Tile Size M | 16–256 | Tile height for tiled algorithms |
| Tile Size N | 16–256 | Tile width for tiled algorithms |
| Unroll Factor | 1–8 | Loop unrolling depth |
| Vector Width | 1–4 | Elements per vector load |
| Shared Memory Padding | 0–4 | Bank conflict avoidance padding |
| Pipeline Stages | 1–4 | Async pipeline depth |

### Evolutionary Algorithm

```text
1. INITIALIZATION
   Generate 64 random kernel configurations (population)

2. EVALUATION
   For each configuration:
     • Compile kernel with configuration parameters
     • Execute on GPU with representative input data
     • Measure execution time (fitness = 1/time)

3. SELECTION
   Tournament selection: pick 2 random candidates, keep the faster one

4. CROSSOVER
   Combine parameters from two parent configurations:
     Parent A: block_x=256, tile_m=64, unroll=4
     Parent B: block_x=128, tile_m=128, unroll=2
     Child:    block_x=256, tile_m=128, unroll=4  (mixed)

5. MUTATION
   Randomly perturb one parameter with 10% probability:
     block_x=256 → block_x=192 (random neighbor)

6. REPEAT steps 2-5 for 20 generations

7. OUTPUT
   Best configuration found across all generations
```

### Integration with Kernel Launch

When `gpu.launch_auto()` is used, the auto-tuner runs during the first invocation and caches the optimal configuration for subsequent calls:

```agam
// First call: auto-tuner runs (~2 seconds of search)
gpu.launch_auto(gemm_tiled, args: (A, B, C));

// Subsequent calls: uses cached optimal configuration (~0 overhead)
gpu.launch_auto(gemm_tiled, args: (A2, B2, C2));
```

Cached configurations are stored in `$AGAM_HOME/cache/gpu_tuning/<kernel_hash>.json` and are keyed by the kernel function signature and GPU device identifier.
