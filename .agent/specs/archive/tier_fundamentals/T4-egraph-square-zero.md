# Phase T4-egraph-square-zero — E-Graph Superoptimization & Square-Zero Tensor Kernel Fusion

## Phase Focus

Incorporating square-zero algebraic rewrite rules ($\mathcal{S} = \mathbb{C}[z_1, \dots, z_r]/(z_i^2)$) and root-of-unity phase filters into the `agam_mir` equality saturation engine for zero-overhead tensor contraction loop fusion.

## Key Capabilities & Algorithms

1. **Square-Zero Algebraic Rewrite Rules (`agam_mir::optimize::egraph`)**:
   - Commutative nilpotent variable expansion to cancel intermediate multi-block tensor allocations.
   - Block-selective cancellation to fuse multi-head attention and convolution-add-relu chains into unified MIR blocks.

2. **Zero-Overhead Native Assembly Generation (`agam_codegen`)**:
   - Direct SIMD / AVX-512 / CUDA translation of fused E-graph terms without external C++ or BLAS runtime dependencies.

## Verification Plan

- MIR optimization tests verifying tensor contraction loop fusion on E-graphs.
- Benchmarks comparing fused kernel execution latency against unfused baselines.
