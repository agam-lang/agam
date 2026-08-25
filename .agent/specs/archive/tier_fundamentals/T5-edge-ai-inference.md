# Phase T5-edge-ai-inference � Edge AI Inference Runtime

**Status:** open
**Tier:** 5 (AI-Native Differentiation)
**Pillar:** 3 (Math & AI)

## Vision

Enable Agam as a **first-class language for deploying ML models at the edge** — compile trained models (ONNX, TensorFlow Lite) directly into optimized Agam binaries with zero external runtime dependencies.

## Motivation

In 2026, edge AI inference is a massive and growing market:
- **ONNX Runtime**: Cross-platform inference with hardware-specific Execution Providers
- **TensorRT Edge-LLM**: NVIDIA's specialized runtime for LLMs on Jetson
- **Apache TVM**: Open-source ML compiler for diverse hardware targets
- **The "memory wall"**: Edge performance is bottlenecked by data movement, not compute

Agam's combination of native compilation, GPU support, and RISC-V targeting makes it uniquely positioned to be the language for edge inference — if it can import and execute ML models natively.

## Deliverables

### Model Import
- [ ] `import_model("model.onnx")` — compile-time ONNX model import
- [ ] Model weights embedded directly in the compiled binary (no external files)
- [ ] Compile-time shape validation against model input/output signatures
- [ ] Support for common model formats: ONNX, TFLite, SafeTensors

### Inference API
- [ ] `model.predict(input: Tensor)` — type-safe inference API
- [ ] Batched inference with automatic padding
- [ ] Streaming inference for sequential models (RNNs, Transformers)
- [ ] Async inference for non-blocking execution (after Phase T2-async-concurrency)

### Optimization
- [ ] Post-training quantization (INT8, FP16) at compile time
- [ ] Operator fusion in the Agam IR (reuses Phase T4-multi-level-ir tensor dialect)
- [ ] Memory planning: minimize peak memory usage for edge constraints
- [ ] Hardware-specific kernel selection (CPU SIMD, GPU, NPU via Phase T4-hardware-introspection)

### Deployment Targets
- [ ] x86/ARM desktop/server inference
- [ ] RISC-V embedded inference (Phase T3-riscv-backend)
- [ ] Android on-device inference (Phase T3-cross-compilation)
- [ ] WASM browser-based inference (Phase T3-wasm-backend)
- [ ] `agamc build --target aarch64-android --embed-model model.onnx`

## Design References

- **ONNX Runtime**: Cross-platform inference with Execution Providers. EP context caching.
- **Apache TVM**: ML compiler for diverse hardware. Auto-tuning kernel generation.
- **TensorRT**: Maximum performance via layer fusion and precision calibration.
- **Agam advantage**: Compile model + application into a single binary. No Python, no runtime dependency, no Docker.

## Responsible Crates

- `agam_sema` — model import validation, shape checking
- `agam_mir` — model graph → MIR ops, operator fusion
- `agam_codegen` — quantized kernel emission, hardware dispatch
- `agam_std` — inference API surface, tensor operations

## Dependencies

- Phase T5-autodiff-tensors (autodiff/tensors) — tensor types and operations
- Phase T4-multi-level-ir (Multi-Level IR) — tensor dialect for operator fusion
- Phase T4-gpu-optimization-depth (GPU) — GPU-accelerated inference kernels
- Phase T0-type-system (type system) — const generics for model shapes
