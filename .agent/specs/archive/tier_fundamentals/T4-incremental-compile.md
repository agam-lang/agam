# Phase T4-incremental-compile -- Native Scientific Primitives, Sparse Matrices, FFT & Fast Collections

**Status:** complete
**Tier:** 4 (Performance and Optimization Depth -- High-Performance Scientific Surface)

## Goal

Provide hardware-optimized scientific datatypes, sparse matrix formats (`CsrMatrix`, `CooMatrix`), fast SpMV kernels, Cooley-Tukey Radix-2 Fast Fourier Transform (`fft`, `ifft`), signal windowing (`hanning_window`, `hamming_window`, `blackman_window`), and high-throughput collection structures (`FastRingBuffer`, `CompactGraph`) in `agam_std`.

## Deliverables

- [x] **Hardware-Aware Sparse Matrix Primitives (`agam_std::sparse`)**:
  - `CsrMatrix`: Compressed Sparse Row format with row offsets, column indices, non-zero values, and sparsity metrics.
  - `CooMatrix`: Dynamic coordinate triplet list format with conversion to CSR (`to_csr`).
  - `spmv`: High-performance sparse matrix-vector multiplication kernel ($y = A \cdot x$).
- [x] **Fast Fourier Transform & Signal Processing (`agam_std::fft`)**:
  - `fft`: 1D Radix-2 Cooley-Tukey Decimation-in-Time FFT with bit-reversal permutation and twiddle factor butterfly.
  - `ifft`: Inverse FFT with automatic $1/N$ normalization.
  - Signal windowing functions: Hanning, Hamming, and Blackman windows.
- [x] **High-Performance Collections & Graph Structures (`agam_std::collections`)**:
  - `FastRingBuffer<T>`: Fixed-capacity circular ring buffer with overwrite semantics.
  - `CompactGraph`: Adjacency list representation with Dijkstra shortest path solver.
- [x] **Verification**:
  - `sparse::tests::test_coo_to_csr_and_spmv`
  - `fft::tests::test_fft_and_ifft_roundtrip`
  - `fft::tests::test_windowing_functions`
  - `collections::tests::test_fast_ring_buffer`
  - `collections::tests::test_compact_graph_shortest_path`
  - 100% test pass rate across all 27 workspace crates.

## Test Results
- 143/143 tests pass in `agam_std`
- 100% test pass rate across all 27 workspace crates
- 0 Clippy warnings (`-D warnings`)
- 100% formatting compliance (`cargo fmt --check`)
