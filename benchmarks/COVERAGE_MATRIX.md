# Benchmark Coverage Matrix

This file is the broad workload map for the benchmark workspace.

The public same-host table in [`../README.md`](../README.md) is a measured snapshot for one constant workload. It exists because `fibonacci` is currently the only workload that has both:

- a checked-in, measured, same-host result set under [`results/raw/2026-04-02_17-00-55/`](results/raw/2026-04-02_17-00-55/)
- the broadest current comparison-language coverage in-repo: C, C++, Go, Rust, and Python

That table should not be read as "Agam benchmarks only Fibonacci." The broader benchmark plan lives here.

## Summary

- runnable Agam workloads in-repo today: `38`
- comparison-ready workloads in-repo today: `8`
  - `fibonacci`
  - `edit_distance`
  - `matrix_multiply`
  - `polynomial_eval`
  - `ring_buffer`
  - `tensor_matmul`
  - `token_frequency`
  - `csv_scanning`
- checked-in comparison-language sources today: `38`
- planned or future workloads tracked below: `22`
- total workload slots tracked in this matrix: `60`

## Status Legend

- `comparison-ready`
  - `.agam` source exists now and at least one checked-in non-Agam comparison source exists now
- `runnable`
  - `.agam` source exists now, but comparison-language sources are not checked in yet
- `planned`
  - good candidate for the current benchmark harness on normal CPU/RAM/SSD hosts
- `future-lab`
  - useful workload, but needs a dedicated harness or subsystem beyond the current CPU-first benchmark path

## Current CPU, Numerical, String, And ML Workloads

| Workload | Repo source | Comparisons today | Constraint focus | Status |
| --- | --- | --- | --- | --- |
| `binary_search` | `01_algorithms/binary_search.agam` | none | branch prediction, cache-friendly lookup | `runnable` |
| `edit_distance` | `01_algorithms/edit_distance.agam` | C/C++/Go/Rust/Python | dynamic programming, string DP cost | `comparison-ready` |
| `fibonacci` | `01_algorithms/fibonacci.agam` | C/C++/Go/Rust/Python | recursion, branch behavior, call-cache reuse | `comparison-ready` |
| `prime_sieve` | `01_algorithms/prime_sieve.agam` | none | integer math, loop throughput, memory locality | `runnable` |
| `quicksort` | `01_algorithms/quicksort.agam` | none | branching, partitioning, recursion | `runnable` |
| `fft` | `02_numerical_computation/fft.agam` | none | floating-point throughput, structured memory access | `runnable` |
| `matrix_multiply` | `02_numerical_computation/matrix_multiply.agam` | C/C++/Rust/Python | arithmetic throughput, cache reuse, loop lowering | `comparison-ready` |
| `monte_carlo_pi` | `02_numerical_computation/monte_carlo_pi.agam` | none | floating point, random sampling, branch noise | `runnable` |
| `polynomial_eval` | `02_numerical_computation/polynomial_eval.agam` | C/C++/Go/Rust/Python | scalar arithmetic, tight loop optimization | `comparison-ready` |
| `tensor_operations` | `02_numerical_computation/tensor_operations.agam` | none | vectorizable arithmetic, memory layout | `runnable` |
| `autodiff` | `05_ml_primitives/autodiff.agam` | none | graph building, derivative propagation | `runnable` |
| `convolution` | `05_ml_primitives/convolution.agam` | none | stencil-style memory reuse, arithmetic intensity | `runnable` |
| `softmax` | `05_ml_primitives/softmax.agam` | none | exponentials, reductions, numerical stability | `runnable` |
| `tensor_matmul` | `05_ml_primitives/tensor_matmul.agam` | C/C++/Rust/Python | dense linear algebra, tensor lowering | `comparison-ready` |
| `regex_matching` | `06_string_processing/regex_matching.agam` | none | pattern matching, branching, parser/runtime cost | `runnable` |
| `string_search` | `06_string_processing/string_search.agam` | none | substring scan, branch prediction, memory locality | `runnable` |
| `token_frequency` | `06_string_processing/token_frequency.agam` | C/C++/Go/Rust/Python | tokenization, hashmap pressure, string handling | `comparison-ready` |

## Current Memory, Data Structure, And I/O Workloads

| Workload | Repo source | Comparisons today | Constraint focus | Status |
| --- | --- | --- | --- | --- |
| `btree_operations` | `03_data_structures/btree_operations.agam` | none | cache-aware tree traversal, allocator behavior | `runnable` |
| `hashmap_operations` | `03_data_structures/hashmap_operations.agam` | none | hashing, probe locality, resize cost | `runnable` |
| `linked_list` | `03_data_structures/linked_list.agam` | none | pointer chasing, cache misses, allocator cost | `runnable` |
| `ring_buffer` | `03_data_structures/ring_buffer.agam` | C/C++/Go/Rust/Python | circular-buffer indexing, branch-light queue traffic | `comparison-ready` |
| `memory_allocation` | `04_memory_intensive/memory_allocation.agam` | none | allocator throughput, working-set growth | `runnable` |
| `garbage_collection` | `04_memory_intensive/garbage_collection.agam` | none | retention pressure, traversal cost, reclaim behavior | `runnable` |
| `arc_contention` | `04_memory_intensive/arc_contention.agam` | none | shared ownership traffic, atomic overhead | `runnable` |
| `file_reading` | `07_io_operations/file_reading.agam` | none | sequential file I/O, parser boundary cost | `runnable` |
| `json_parsing` | `07_io_operations/json_parsing.agam` | none | parsing throughput, allocation, string handling | `runnable` |
| `csv_scanning` | `07_io_operations/csv_scanning.agam` | C/C++/Go/Rust/Python | text scanning, token splitting, I/O throughput | `comparison-ready` |

## Current JIT And Compilation Workloads

| Workload | Repo source | Comparisons today | Constraint focus | Status |
| --- | --- | --- | --- | --- |
| `adaptive_optimization` | `08_jit_optimization/adaptive_optimization.agam` | none | runtime specialization policy, JIT behavior | `runnable` |
| `call_cache_hotset` | `08_jit_optimization/call_cache_hotset.agam` | none | high-hit-rate memoization locality | `runnable` |
| `call_cache_mixed_locality` | `08_jit_optimization/call_cache_mixed_locality.agam` | none | mixed hit/miss behavior, cache churn | `runnable` |
| `call_cache_phase_shift` | `08_jit_optimization/call_cache_phase_shift.agam` | none | changing working-set phases, eviction behavior | `runnable` |
| `call_cache_profile` | `08_jit_optimization/call_cache_profile.agam` | none | recursive overlap, call-cache sensitivity | `runnable` |
| `call_cache_unique_inputs` | `08_jit_optimization/call_cache_unique_inputs.agam` | none | near-zero reuse, cache overhead visibility | `runnable` |
| `specialization_demo` | `08_jit_optimization/specialization_demo.agam` | none | specialization and warm-state effects | `runnable` |
| `tiny_program` | `09_compilation_metrics/tiny_program.agam` | none | minimal frontend/backend fixed cost | `runnable` |
| `medium_program` | `09_compilation_metrics/medium_program.agam` | none | moderate compile scaling | `runnable` |
| `large_program` | `09_compilation_metrics/large_program.agam` | none | compile throughput under larger input sets | `runnable` |
| `complex_generics` | `09_compilation_metrics/complex_generics.agam` | none | type-system and monomorphization stress | `runnable` |

## Planned Next Runnable Workloads

These should fit the current benchmark workspace model once the corresponding `.agam` sources and optional comparison sources are added.

| Workload | Planned comparisons | Constraint focus | Status |
| --- | --- | --- | --- |
| `rsa_encrypt_decrypt` | C/C++/Rust/Python | big-integer arithmetic, modular exponentiation | `planned` |
| `aes256_software` | C/C++/Rust/Python | byte-level ALU work without AES-NI shortcuts | `planned` |
| `sha256_hash` | C/C++/Rust/Python | bit mixing, tight loop hashing throughput | `planned` |
| `collatz_iteration` | C/C++/Rust/Python | branch-heavy integer iteration | `planned` |
| `ackermann_function` | C/C++/Rust/Python | extreme recursion, stack pressure, call overhead | `planned` |
| `n_queens` | C/C++/Rust/Python | combinatorial search, branch prediction | `planned` |
| `sudoku_backtracking` | C/C++/Rust/Python | recursive constraint solving | `planned` |
| `a_star_dense_grid` | C++/Rust/Python | priority queues, graph search, heuristic branching | `planned` |
| `traveling_salesman_dp` | C++/Rust/Python | dynamic programming, combinatorial explosion | `planned` |
| `mandelbrot` | C/C++/Rust/Python | floating point, branch divergence, escape-time loops | `planned` |
| `n_body_simulation` | C/C++/Rust/Python | floating point, data layout, pairwise force loops | `planned` |
| `huffman_codec` | C++/Rust/Python | tree building, bit packing, compression logic | `planned` |
| `lz77_compression` | C++/Rust/Python | window scanning, dictionary reuse, I/O-bound parsing | `planned` |
| `xml_parsing_massive` | C++/Rust/Python | parser throughput, allocation pressure, branching | `planned` |
| `sparse_matrix_vector` | C/C++/Rust/Python | memory bandwidth, sparse access locality | `planned` |
| `bloom_filter` | C/C++/Rust/Python | hashing throughput, probabilistic set membership | `planned` |
| `stream_triad` | C/C++/Rust | memory bandwidth ceiling, sustained sequential throughput | `planned` |
| `matrix_transpose_out_of_cache` | C/C++/Rust | cache misses, bandwidth, stride penalties | `planned` |

## Future Lab Workloads

These matter, but they need dedicated harness work or subsystem integration before they should appear in the normal CPU-first result tables.

| Workload | Planned comparisons | Constraint focus | Status |
| --- | --- | --- | --- |
| `sqlite_concurrent_inserts` | Agam/Rust/Python plus SQLite tooling | fsync overhead, random I/O, transaction latency | `future-lab` |
| `grpc_serialization` | Agam/Rust/Python/C++ | protocol overhead, network-facing serialization cost | `future-lab` |
| `transformer_self_attention` | Agam/Python/C++ accelerator path | tensor math, softmax, bandwidth vs compute balance | `future-lab` |
| `host_to_device_copy` | CUDA/HIP or equivalent device tooling | PCIe or accelerator interconnect saturation | `future-lab` |

## Constraint Clusters To Watch

- compute and branch behavior
  - current: `fibonacci`, `prime_sieve`, `quicksort`, `binary_search`, `edit_distance`, `regex_matching`
  - next: `rsa_encrypt_decrypt`, `aes256_software`, `sha256_hash`, `n_queens`, `sudoku_backtracking`
- floating-point and numerical throughput
  - current: `fft`, `matrix_multiply`, `monte_carlo_pi`, `polynomial_eval`, `tensor_matmul`, `convolution`, `softmax`
  - next: `mandelbrot`, `n_body_simulation`
- memory latency and bandwidth
  - current: `linked_list`, `hashmap_operations`, `btree_operations`, `ring_buffer`, `memory_allocation`, `garbage_collection`
  - next: `sparse_matrix_vector`, `bloom_filter`, `stream_triad`, `matrix_transpose_out_of_cache`
- I/O and parser pressure
  - current: `file_reading`, `json_parsing`, `csv_scanning`, `token_frequency`
  - next: `xml_parsing_massive`, `huffman_codec`, `lz77_compression`, `sqlite_concurrent_inserts`
- compiler, JIT, and cache-policy behavior
  - current: all `08_jit_optimization/*` cases plus `09_compilation_metrics/*`
- accelerator and interconnect work
  - future: `transformer_self_attention`, `host_to_device_copy`

## Immediate Next Comparisons To Add

If the goal is to move beyond Fibonacci without pretending the whole matrix already exists, the next practical comparison-language additions should be:

1. `binary_search`
2. `prime_sieve`
3. `quicksort`
4. `fft`
5. `monte_carlo_pi`
6. `hashmap_operations`
7. the four `call_cache_*` locality shapes

Those remaining workloads are already present as Agam benchmarks, exercise different constraints, and are the next good comparison surface before jumping to GPU/NPU/network-specific labs.
