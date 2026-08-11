# Chapter 24: Modules, Package Management (`agam.toml`) & FFI

> **Part VI: The Agam Language Programming Guide**  
> **Target Audience**: Systems Engineers and Application Developers

---

## 24.1 Package Manifests (`agam.toml`)

Agam projects use `agam.toml` for package declaration and dependency management:

```toml
[project]
name = "my_ai_app"
version = "0.1.0"
authors = ["Developer <dev@example.com>"]
edition = "2026"

[dependencies]
std = "1.0"
math_utils = { path = "../math_utils" }
network_pkg = { git = "https://github.com/example/network_pkg.git", tag = "v1.2.0" }

[toolchain]
llvm_version = "18.1"
```

---

## 24.2 Modules & Code Importing

Split code across multiple files:

```agam
// File: src/math.agam
pub fn add_vectors(a: Array[Float], b: Array[Float]) -> Array[Float] {
    // ...
}

// File: src/main.agam
import src.math as math;

fn main() {
    let result = math.add_vectors([1.0], [2.0]);
}
```

---

## 24.3 Foreign Function Interface (FFI) Interop

Agam can interface directly with external C libraries or Python frameworks (`agam_ffi`):

### Calling C Native Libraries
```agam
extern "C" {
    fn puts(str: *const Char) -> Int;
    fn malloc(size: Int) -> *mut Nil;
    fn free(ptr: *mut Nil);
}

fn main() {
    unsafe {
        puts("Direct C string call via FFI");
    }
}
```
