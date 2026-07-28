# Phase T5-distributed-training � Distributed Training

**Status:** open
**Tier:** 5 (AI-Native Differentiation)

## Scope

Expose hardware matrix multiplication and accumulation acceleration (e.g., NVIDIA Tensor Cores, Intel Xe Matrix Extensions, AMD Matrix Core) natively within Agam. This bridges the gap between raw compute capabilities and language-level AI primitives without requiring vendor-specific assembly or lock-in.

## Deliverables

### Language-Native Tile Operations
- [ ] Define first-class matrix tile types in the Agam type system
- [ ] Implement matrix multiply-accumulate operations at the language level
- [ ] Expose cooperative work-group capabilities for tile loading/storing

### Backend Implementation
- [ ] Map tile operations to `cl_khr_cooperative_matrix` / `SPV_KHR_cooperative_matrix` in the SPIR-V backend
- [ ] Map tile operations to NVVM matrix intrinsics for the NVPTX backend
- [ ] Introduce compiler builtins to query device capabilities (matrix sizes, data types)

## Design References
- **OpenCL `cl_khr_cooperative_matrix`**: The standardized Khronos extension bringing cooperative matrix capabilities to OpenCL and SPIR-V. Serves as the primary spec for cross-vendor tensor acceleration design.
- **LLVM Clang Cooperative Matrix RFC**: Reference for compiler frontend implementation and type system integration.

## Dependencies
- Phase T4-link-time-opts (SPIR-V Multi-Vendor GPU Path) — provides the foundation for portable SPIR-V emission.
- Phase T0-type-system (Type System Completion) — requires advanced generics and type traits for matrix shapes and scalar types.
