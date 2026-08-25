# Phase T5-llm-inference � ML Training Loop Primitives

**Status:** open
**Tier:** 5 (AI-Native Differentiation)
**Prerequisite:** Phase T5-autodiff-tensors (Autodiff and Tensor Types)

## Scope

Build language-native ML training infrastructure: optimizers, data loading, distributed training, mixed precision, and model serialization.

## Deliverables

- [ ] Built-in optimizer abstractions (SGD, Adam, AdamW, RMSProp)
- [ ] Data loading and batching primitives with async prefetching
- [ ] Distributed training coordination (data parallel, model parallel)
- [ ] Mixed-precision training support (fp16/bf16 with fp32 accumulation)
- [ ] Model serialization and checkpointing
- [ ] Training loop utilities (epoch management, learning rate scheduling)
- [ ] Loss function library (cross-entropy, MSE, etc.)

## Responsible Crates

- `agam_std` — ML primitives and training utilities
- `agam_runtime` — distributed coordination

## Dependencies

- Phase T5-autodiff-tensors — tensor types and autodiff
- Phase T2-async-concurrency — async runtime for data loading
- Phase T4-gpu-optimization-depth — GPU backend for training acceleration
