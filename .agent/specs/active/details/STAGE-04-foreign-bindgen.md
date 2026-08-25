# Stage 4: C-ABI Foreign Function Binding Generator (`agam-bindgen`)

**Stage**: `Stage 4 (Planned Execution)`  
**Domain**: Foreign Function Interop & Automated C Bindings  
**Status**: **PLANNED**  

---

## 1. Executive Summary & Problem Definition

To allow Agam applications and standard library codecs to seamlessly integrate with native system libraries (`libc`, `libm`, `libz`, `libpng`, `libflac`, `libuv`), Agam requires an automated binding generator (`agam-bindgen`) that parses C headers (`.h`) and emits type-safe Agam `extern fn` bindings.

---

## 2. Technical Deliverables & Architecture

```mermaid
flowchart LR
    Header["C Header (*.h)"] --> ClangAST["Clang / Tree-sitter C AST Parser"]
    ClangAST --> TypeMapper["C <-> Agam Type Mapper\n• int -> i32, size_t -> usize\n• struct -> Agam Struct\n• fn pointer -> fn(...) -> ..."]
    TypeMapper --> Emitter["Agam Binding File (*.agam)\n• extern \"C\" fn ...\n• @link(\"png\") annotations"]
```

### 2.1 C Header AST Parser (`agam_bindgen::parser`)
- Extract `typedef`, `struct`, `union`, `enum`, and function declarations from C headers.
- Handle preprocessor macro constants (`#define CONST 100`).

### 2.2 Agam Type Representation & Memory Layout
- Map C primitive types to Agam scalar types.
- Ensure strict C-compatible alignment and struct padding annotations (`#[repr(C)]`).

### 2.3 Direct Linkage Annotations
- Annotate modules with `@link("z")`, `@link("png")`, `@link("flac")` for seamless LLVM linker pass-through.

---

## 3. Verification & Acceptance Criteria
- [ ] Parse standard `<math.h>` and generate valid Agam bindings for `sin`, `cos`, `sqrt`.
- [ ] Parse `<zlib.h>` and verify compression/decompression calls through generated bindings.
- [ ] 0 warnings in `agam_bindgen` crate.
