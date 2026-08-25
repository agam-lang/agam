# Chapter 24c: Foreign Function Interface (FFI) & Cross-Language Interop

> **Part VI: The Agam Language Programming Guide**  
> **Compiler Module Focus**: [`agam_ffi`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/experiments/agam_ffi), [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen)

---

## 24c.1 FFI Architecture Overview

Agam provides zero-overhead Foreign Function Interface (FFI) for interoperating with code written in C, C++, Python, Rust, JavaScript, and JVM languages:

```text
┌──────────────────────────────────────────────────────────┐
│                    Agam Application                       │
│                                                           │
│  ┌─────────┐  ┌─────────┐  ┌──────────┐  ┌───────────┐ │
│  │ C ABI    │  │ Python  │  │ Rust     │  │ JVM / JS  │ │
│  │ repr(C)  │  │ Buffer  │  │ ABI      │  │ Bridge    │ │
│  │ layout   │  │ Protocol│  │ compat   │  │           │ │
│  └────┬─────┘  └────┬────┘  └────┬─────┘  └─────┬─────┘ │
│       │              │            │               │       │
│       ▼              ▼            ▼               ▼       │
│  libfoo.so     numpy arrays   librust.a      JNI / WASM  │
└──────────────────────────────────────────────────────────┘
```

---

## 24c.2 C ABI Interop

### Calling C Functions from Agam

```agam
// Declare external C function signatures
extern "C" {
    fn printf(format: *const u8, ...) -> Int;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn strlen(s: *const u8) -> usize;
}

fn main() {
    // Call C standard library functions directly
    let msg = "Hello from Agam!\n\0";
    printf(msg.as_ptr());
}
```

### Exposing Agam Functions to C

```agam
// Export a function with C calling convention
@export("C")
fn agam_compute(input: *const Float, output: *mut Float, len: Int) {
    for i in 0..len {
        output[i] = input[i] * 2.0 + 1.0;
    }
}

// Generates C header:
// void agam_compute(const float* input, float* output, int len);
```

### `@repr(C)` Struct Layout

```agam
// Struct with C-compatible memory layout (no field reordering)
@repr(C)
struct Point3D {
    x: Float,   // offset 0, size 8
    y: Float,   // offset 8, size 8
    z: Float,   // offset 16, size 8
}
// Total: 24 bytes, matching C struct layout exactly

@repr(C, packed)
struct PackedHeader {
    magic: u32,    // offset 0, size 4
    version: u16,  // offset 4, size 2
    flags: u8,     // offset 6, size 1
}
// Total: 7 bytes, no padding
```

---

## 24c.3 C Header Bindgen Parser

Agam includes a built-in C header parser that automatically generates Agam FFI bindings from C headers:

```bash
# Generate bindings from a C header file
agamc bindgen include/mylib.h --output src/bindings.agam
```

**Input (`mylib.h`):**
```c
typedef struct {
    double x, y, z;
} Vec3;

int compute_distance(const Vec3* a, const Vec3* b, double* result);
void free_buffer(void* ptr);
```

**Generated (`bindings.agam`):**
```agam
@repr(C)
struct Vec3 {
    x: Float,
    y: Float,
    z: Float,
}

extern "C" {
    fn compute_distance(a: *const Vec3, b: *const Vec3, result: *mut Float) -> Int;
    fn free_buffer(ptr: *mut u8);
}
```

---

## 24c.4 Python Interop & NumPy Buffer Protocol

Agam provides zero-copy tensor interop with Python/NumPy through the Buffer Protocol:

```agam
// Export a tensor computation as a Python-callable function
@export("python")
fn matrix_multiply(a: PyBuffer[Float], b: PyBuffer[Float]) -> PyBuffer[Float] {
    let tensor_a = Tensor.from_pybuffer(a);  // Zero-copy view
    let tensor_b = Tensor.from_pybuffer(b);  // Zero-copy view
    let result = tensor_a * tensor_b;
    return result.to_pybuffer();  // Zero-copy export
}
```

### Python Side

```python
import agam_bindings

import numpy as np

a = np.random.randn(256, 256).astype(np.float32)
b = np.random.randn(256, 256).astype(np.float32)

# Calls Agam code with zero-copy — no data serialization
c = agam_bindings.matrix_multiply(a, b)

print(f"Result shape: {c.shape}")  # (256, 256)
```

### Buffer Protocol Descriptor

The `PyBuffer` type wraps a NumPy `Py_buffer` struct, providing:
- `ptr`: Raw pointer to the data buffer (no copy)
- `shape`: Array dimensions
- `strides`: Byte strides per dimension
- `format`: Element type (`'f'` for float32, `'d'` for float64)

```text
Zero-Copy Data Flow:
  Python NumPy ndarray (owns memory)
        │
        ▼ Py_buffer* (pointer + metadata)
  Agam Tensor[Float] (borrows memory, same pointer)
        │
        ▼ Computation in Agam (SIMD/GPU)
        │
        ▼ Result Tensor (new memory)
  Python NumPy ndarray (takes ownership)
```

---

## 24c.5 Rust ABI Compatibility

Since Agam compiles through LLVM, it can link directly with Rust static libraries:

```agam
// Link against a Rust crate compiled as a static library
@link("rust_crypto_lib")
extern "C" {
    fn rust_aes_encrypt(key: *const u8, data: *const u8, len: usize, out: *mut u8);
    fn rust_aes_decrypt(key: *const u8, data: *const u8, len: usize, out: *mut u8);
}
```

```bash
# Build Rust library
cd rust_crypto_lib && cargo build --release
# Link with Agam
agamc build --link-lib rust_crypto_lib/target/release/librust_crypto_lib.a src/main.agam
```

---

## 24c.6 WASM & JavaScript Interop

Through the WASM backend, Agam functions can be called from JavaScript:

```agam
// Export to WASM
@export("wasm")
fn fibonacci(n: Int) -> Int {
    if n <= 1 { return n; }
    return fibonacci(n - 1) + fibonacci(n - 2);
}
```

```javascript
// JavaScript side
const wasm = await WebAssembly.instantiateStreaming(fetch('agam_module.wasm'));
const result = wasm.instance.exports.fibonacci(30);
console.log(`Fibonacci(30) = ${result}`);  // 832040
```

---

## 24c.7 Safety Boundaries

FFI is inherently unsafe because the compiler cannot verify the correctness of foreign code. Agam enforces explicit safety boundaries:

```agam
// FFI calls must be wrapped in an `unsafe` block
fn safe_wrapper(data: [Float]) -> Float {
    unsafe {
        let ptr = data.as_ptr();
        return c_library_compute(ptr, data.len());
    }
}

// The compiler tracks unsafe blocks and warns about unchecked FFI usage
```

| Safety Check | Enforced By |
| :--- | :--- |
| Null pointer dereference | `Option[*T]` wrapping for nullable pointers |
| Buffer overflow | Length parameters must accompany raw pointers |
| Use-after-free | Lifetime annotations on borrowed FFI data |
| Type mismatch | `@repr(C)` layout verification against C headers |
