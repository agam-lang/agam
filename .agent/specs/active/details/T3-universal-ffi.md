# Phase T3-universal-ffi -- Universal Foreign Function Interface (FFI)

**Status:** complete
**Tier:** 3 (Platform and Ecosystem Breadth -- FFI Engine)

## Goal

Provide cross-language zero-overhead foreign function interface (FFI) primitives, C ABI layout and struct alignment engine (`repr(C)`), C header bindgen parser generating Agam `extern "C"` declarations, and NumPy / Python 3 Buffer Protocol zero-copy tensor descriptor in `agam_ffi`.

## Deliverables

- [x] **C ABI & Layout Engine (`agam_ffi::c_abi`)**:
  - `CPrimitive`: Type sizing and natural alignment for `I8`, `U8`, `I16`, `U16`, `I32`, `U32`, `I64`, `U64`, `F32`, `F64`, `Pointer`, `Void`.
  - `CStructLayout`: Strict ISO C `repr(C)` field alignment, offset calculations, and tail padding.
  - `CFuncSig`: C function signatures and calling conventions (`Cdecl`, `Stdcall`, `Fastcall`, `SysV64`, `Win64`).
- [x] **C Header Bindgen (`agam_ffi::bindgen`)**:
  - `parse_c_function_prototype`: Extracts typed prototypes from C headers.
  - `generate_agam_extern_block`: Generates idiomatic Agam language `extern "C" { fn ...; }` interface blocks.
- [x] **Python / NumPy Buffer Protocol Interop (`agam_ffi::python`)**:
  - `PyBufferDescriptor`: Zero-copy multi-dimensional tensor sharing (`data_ptr`, `shape`, `strides`, `format`, `is_c_contiguous`).
- [x] **Verification**:
  - `c_abi::tests::test_c_struct_layout_padding_and_alignment`
  - `bindgen::tests::test_parse_c_prototype_and_generate_agam_extern`
  - `python::tests::test_numpy_buffer_descriptor_strides_and_contiguity`
  - 100% test pass rate across all 27 workspace crates.

## Test Results
- 5/5 tests pass in `agam_ffi`
- 100% test pass rate across all 27 workspace crates
- 0 Clippy warnings (`-D warnings`)
- 100% formatting compliance (`cargo fmt --check`)
