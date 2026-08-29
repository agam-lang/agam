# Chapter 30: Cross-Compilation, Target Triplets & Target Packs

> **Part VII: Advanced Tooling, Testing & Ecosystem Engineering**  
> **Compiler Module Focus**: [`agam_pkg`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_pkg), [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen), [`agam_runtime`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_runtime)

---

## 30.1 Target Triplet Architecture

Agam uses **LLVM Target Triplets** to identify compilation targets. Each triplet encodes the architecture, vendor, operating system, and environment ABI:

```text
Format: <arch>-<vendor>-<os>-<env>

Examples:
  x86_64-pc-windows-msvc         Windows x64 (MSVC ABI)
  x86_64-unknown-linux-gnu       Linux x64 (glibc)
  x86_64-unknown-linux-musl      Linux x64 (static musl libc)
  aarch64-apple-darwin            macOS Apple Silicon
  aarch64-linux-android           Android ARM64
  riscv64gc-unknown-linux-gnu    RISC-V 64-bit (GC extensions)
  wasm32-wasi                     WebAssembly (WASI 0.2)
  thumbv7em-none-eabihf           ARM Cortex-M (bare-metal, hard-float)
```

---

## 30.2 Supported Target Matrix

| Target Triplet | Architecture | OS/Platform | Backend | Status |
| :--- | :--- | :--- | :--- | :---: |
| `x86_64-pc-windows-msvc` | x86-64 | Windows | LLVM | ✅ Primary |
| `x86_64-unknown-linux-gnu` | x86-64 | Linux (glibc) | LLVM | ✅ Primary |
| `aarch64-apple-darwin` | ARM64 | macOS | LLVM | ✅ Primary |
| `aarch64-linux-android` | ARM64 | Android | LLVM | ✅ Supported |
| `x86_64-unknown-linux-musl` | x86-64 | Linux (musl) | LLVM | ✅ Supported |
| `riscv64gc-unknown-linux-gnu` | RISC-V 64 | Linux | LLVM | 🔄 Experimental |
| `wasm32-wasi` | WebAssembly | WASI 0.2 | Direct WASM | ✅ Supported |
| `thumbv7em-none-eabihf` | ARM Cortex-M | Bare-metal | LLVM/C11 | 🔄 Experimental |
| `nvptx64-nvidia-cuda` | NVIDIA GPU | CUDA | NVPTX | ✅ Supported |
| `spirv64-unknown-unknown` | GPU (Vendor-Neutral) | Vulkan/OpenCL | SPIR-V | ✅ Supported |

---

## 30.3 Cross-Compilation Workflow

Cross-compilation in Agam uses the `--target` flag to select a different target than the host:

```bash
# Cross-compile from Windows host to Android ARM64
agamc build --target aarch64-linux-android src/main.agam

# Cross-compile to WebAssembly
agamc build --target wasm32-wasi src/main.agam

# Cross-compile to Linux (from macOS host)
agamc build --target x86_64-unknown-linux-gnu src/main.agam
```

### Cross-Compilation Pipeline

```text
Source (.agam)
    │
    ▼
  Lexer → Parser → Sema → HIR → MIR → Opt
    │                                    │
    │  (Target-independent up to here)   │
    │                                    ▼
    │                         ┌─────────────────────┐
    │                         │ Target Configuration │
    │                         │  • LLVM Triple        │
    │                         │  • Data Layout        │
    │                         │  • CPU Features       │
    │                         │  • ABI Convention     │
    │                         └────────┬────────────┘
    │                                  │
    │                                  ▼
    │                         LLVM IR (target-specific)
    │                                  │
    │                                  ▼
    │                         LLVM Backend (target MC)
    │                                  │
    │                                  ▼
    │                         Object File (.o)
    │                                  │
    │                                  ▼
    │                         Cross-Linker (lld / target ld)
    │                                  │
    │                                  ▼
    │                         Target Binary
    └──────────────────────────────────┘
```

---

## 30.4 Target Packs & SDK Staging

Target Packs are modular distribution bundles containing everything needed to cross-compile for a specific platform. Each Target Pack includes:

| Component | Description | Example |
| :--- | :--- | :--- |
| **Sysroot** | Platform headers and system libraries | `libc.so`, `kernel32.lib` |
| **Runtime Library** | Pre-compiled `libagam_runtime.a` for the target | Static archive for ARM64 Android |
| **LLVM Target Description** | CPU features, register info, ABI rules | `aarch64` target machine config |
| **Linker Configuration** | Cross-linker binary and flags | `aarch64-linux-android-ld` |
| **SDK Metadata** | Version, checksum, compatibility matrix | `target-pack.toml` |

### Installing & Managing Target Packs

```bash
# List available target packs
agamc target list

# Install a target pack
agamc target add aarch64-linux-android

# Remove a target pack
agamc target remove riscv64gc-unknown-linux-gnu

# Show installed packs and their sysroot locations
agamc target info aarch64-linux-android
```

### Target Pack Directory Structure

```text
$AGAM_HOME/target-packs/
  └── aarch64-linux-android/
      ├── target-pack.toml          # Metadata and version
      ├── sysroot/
      │   ├── include/              # Platform headers
      │   └── lib/                  # System libraries (.so / .a)
      ├── lib/
      │   └── libagam_runtime.a     # Pre-compiled Agam runtime
      └── bin/
          └── aarch64-linux-android-ld  # Cross-linker
```

---

## 30.5 Target Profile Annotations

Agam provides high-level **target profile annotations** that configure compilation strategy without requiring manual target triplet selection:

```agam
// IoT/Embedded profile — strict affine ownership, no heap, no ARC
@target.iot
fn sensor_read() -> u16 {
    // Heap allocation would be a compile error here
    let reading: u16 = read_adc(0);
    return reading;
}

// HPC profile — aggressive SIMD vectorization, large stack, no bounds checks
@target.hpc
fn matrix_compute(A: Tensor[Float, 1024x1024]) -> Tensor[Float, 1024x1024] {
    return A * A.T;  // Compiled with AVX-512 + loop tiling
}

// Enterprise profile — full safety, ARC, bounds checks, observability
@target.enterprise
fn handle_request(req: HttpRequest) -> HttpResponse {
    // Full runtime safety enabled
    return HttpResponse.ok(process(req));
}
```

### Profile Configuration Matrix

| Feature | `@target.iot` | `@target.hpc` | `@target.enterprise` |
| :--- | :---: | :---: | :---: |
| Memory Model | Affine ownership | ARC + Arena | Full ARC |
| Heap Allocation | ❌ Prohibited | ✅ Pool allocator | ✅ General allocator |
| Bounds Checking | ✅ Static only | ❌ Disabled | ✅ Full runtime |
| SIMD Vectorization | Minimal (NEON) | Aggressive (AVX-512) | Standard (SSE4.2) |
| Stack Size | 4 KB | 64 MB | 8 MB |
| Observability | ❌ None | ❌ None | ✅ OpenTelemetry |
| Code Size Priority | ✅ Size-optimized | ❌ Speed-optimized | Balanced |

---

## 30.6 Fat-Binary Bundling

For applications that need to run on multiple architectures, `agam_codegen::link_opt::FatBinaryBundle` packages multiple target binaries into a single distributable:

```bash
# Build a fat binary for x86-64 and ARM64
agamc build --target x86_64-unknown-linux-gnu,aarch64-unknown-linux-gnu \
    --fat-binary src/main.agam
```

```text
Fat Binary Layout (.agpkg):
  ┌──────────────────────────┐
  │ Header                    │
  │  • Magic: "AGAM"          │
  │  • Version: 1             │
  │  • Entry Count: 2         │
  ├──────────────────────────┤
  │ Entry 0: x86_64-linux-gnu │
  │  • Offset: 0x100          │
  │  • Size: 2.3 MB           │
  │  • CPU Features: avx2     │
  ├──────────────────────────┤
  │ Entry 1: aarch64-linux-gnu│
  │  • Offset: 0x241000       │
  │  • Size: 1.8 MB           │
  │  • CPU Features: neon     │
  ├──────────────────────────┤
  │ Binary Data               │
  │  [x86_64 ELF bytes]       │
  │  [aarch64 ELF bytes]      │
  └──────────────────────────┘
```

At runtime, the fat-binary launcher detects the host architecture via `cpuid` or equivalent intrinsics and executes the matching binary slice.
