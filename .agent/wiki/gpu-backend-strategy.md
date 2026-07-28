# GPU Backend Strategy & SPIR-V

**Context:** Agam-Lang treats AI, tensor, and numerical workflows as first-class language concerns. Our design principles outline support for a `@gpu` annotation (`Avyayībhāva`). 

**Observation:** The release of **chipStar 1.3** (formerly CHIP-SPV) demonstrates that SPIR-V has matured into a highly capable and robust intermediate representation for GPU compute. chipStar allows full HIP and CUDA applications to be compiled down to SPIR-V and executed in a vendor-neutral manner on platforms supporting OpenCL or Intel Level Zero (including macOS, ARM, and Intel architectures).

**Relevance to Agam-Lang:**
- **Vendor-Neutrality:** Instead of building and maintaining separate backend targets for CUDA (NVIDIA), HIP (AMD), and Metal (Apple), Agam could exclusively target **SPIR-V** for its GPU-accelerated blocks.
- **Portability:** Leveraging SPIR-V as a universal GPU IR ensures that Agam's AI and tensor features will run on a wide variety of hardware out-of-the-box, without requiring users to have NVIDIA-specific hardware.
- **Architecture:** When designing the lowerings from MIR to native code for `@gpu` blocks, we should consider using the LLVM SPIR-V backend as the primary path. 

*Added in response to the chipStar 1.3 release.*
