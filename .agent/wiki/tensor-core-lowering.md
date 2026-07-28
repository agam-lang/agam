# Tensor Core Lowering via SPIR-V Cooperative Matrix

**Context:** Agam-Lang treats AI and numerical workflows as first-class language concerns. To achieve "native speed execution" in machine learning, the compiler must map matrix operations to specialized hardware units like NVIDIA Tensor Cores or Intel AMX.

**Observation:** The Khronos Group's `cl_khr_cooperative_matrix` extension (and its SPIR-V counterpart `SPV_KHR_cooperative_matrix`) introduces a vendor-neutral way to access these hardware acceleration units directly from the compiler IR.

**Relevance to Agam-Lang:**
- **No Heavy Vendor SDKs:** Agam does not need to link against `cuDNN` or `rocBLAS` to achieve peak AI inference performance. By lowering Agam's internal tensor operations directly to `OpCooperativeMatrixMulAddKHR` in SPIR-V, the compiler produces standalone, high-performance GPU binaries.
- **Direct Mapping:** Language-level constructs like matrix multiplication between native `tensor<T, N>` types can be semantically lowered in the MIR layer directly to cooperative matrix intrinsics for `@gpu` targets.
- **Portability:** This guarantees that Agam-Lang's AI features are fully portable across any OpenCL/Level Zero compliant hardware supporting the extension (NVIDIA, AMD, Intel, ARM), future-proofing the language against fragmentation in the AI hardware space.

*Added in response to the OpenCL Cooperative Matrix extension announcement (Phase 31).*
