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
- planned or future workloads tracked below: `92`
- total workload slots tracked in this matrix: `130`

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
| `sha3_hash` | C/C++/Rust/Python | sponge-style hashing, bit mixing, round scheduling | `planned` |
| `collatz_iteration` | C/C++/Rust/Python | branch-heavy integer iteration | `planned` |
| `ackermann_function` | C/C++/Rust/Python | extreme recursion, stack pressure, call overhead | `planned` |
| `ecdsa_sign_verify` | C/C++/Rust/Python | elliptic-curve arithmetic, modular inversion, scalar multiply | `planned` |
| `mersenne_twister_rng` | C/C++/Go/Rust/Python | PRNG state updates, integer throughput, predictable branching | `planned` |
| `n_queens` | C/C++/Rust/Python | combinatorial search, branch prediction | `planned` |
| `sudoku_backtracking` | C/C++/Rust/Python | recursive constraint solving | `planned` |
| `a_star_dense_grid` | C++/Rust/Python | priority queues, graph search, heuristic branching | `planned` |
| `traveling_salesman_dp` | C++/Rust/Python | dynamic programming, combinatorial explosion | `planned` |
| `minimax_tree_search` | C++/Rust/Python | adversarial tree search, pruning behavior, branch locality | `planned` |
| `mandelbrot` | C/C++/Rust/Python | floating point, branch divergence, escape-time loops | `planned` |
| `n_body_simulation` | C/C++/Rust/Python | floating point, data layout, pairwise force loops | `planned` |
| `taylor_series_eval` | C/C++/Rust/Python | polynomial accumulation, floating-point convergence, scalar math | `planned` |
| `finite_element_mesh` | C++/Rust/Python | sparse assembly, floating-point kernels, mesh traversal | `planned` |
| `burrows_wheeler_transform` | C++/Rust/Python | suffix-style ordering, string transforms, memory traffic | `planned` |
| `huffman_codec` | C++/Rust/Python | tree building, bit packing, compression logic | `planned` |
| `lz77_compression` | C++/Rust/Python | window scanning, dictionary reuse, I/O-bound parsing | `planned` |
| `xml_parsing_massive` | C++/Rust/Python | parser throughput, allocation pressure, branching | `planned` |
| `page_rank` | C++/Rust/Python | graph iteration, sparse updates, memory locality | `planned` |
| `sparse_matrix_vector` | C/C++/Rust/Python | memory bandwidth, sparse access locality | `planned` |
| `bloom_filter` | C/C++/Rust/Python | hashing throughput, probabilistic set membership | `planned` |
| `fisher_yates_shuffle` | C/C++/Go/Rust/Python | random indexing, memory writes, swap traffic | `planned` |
| `graph_bfs_unorganized` | C++/Rust/Python | pointer-heavy frontier traversal, cache misses, queue churn | `planned` |
| `stream_triad` | C/C++/Rust | memory bandwidth ceiling, sustained sequential throughput | `planned` |
| `mergesort_out_of_place` | C/C++/Rust/Python | sequential memory bandwidth, large working sets, copy traffic | `planned` |
| `matrix_transpose_out_of_cache` | C/C++/Rust | cache misses, bandwidth, stride penalties | `planned` |
| `radix_sort_large_ints` | C/C++/Rust | counting passes, sequential memory traffic, branch-light sorting | `planned` |
| `memcpy_bandwidth` | C/C++/Rust | bulk copy throughput, memory controller pressure, cache bypass effects | `planned` |
| `blocked_tiled_matrix_multiply` | C/C++/Rust | cache-fitting tiles, arithmetic intensity, L2/L3 reuse | `planned` |
| `prefix_sum_scan` | C/C++/Rust/Python | linear reductions, dependency chains, bandwidth pressure | `planned` |
| `sliding_window_strings` | C/C++/Rust/Python | rolling state updates, branch locality, cache reuse | `planned` |
| `histogram_generation` | C/C++/Rust/Python | contention-free counting, cache residency, integer throughput | `planned` |
| `bitboard_movegen` | C/C++/Rust | bit-twiddling throughput, branch-light mask operations | `planned` |
| `log_appending` | Agam/Rust/Python plus filesystem tooling | append-heavy sequential I/O, fsync policy, small-write overhead | `planned` |
| `tarball_create_extract` | Agam/Rust/Python plus archive tooling | archive streaming, metadata churn, sequential file I/O | `planned` |
| `file_dedup_chunking` | Agam/Rust/Python | chunk hashing, rolling windows, large-file scan throughput | `planned` |
| `external_sort` | Agam/C++/Rust/Python | data-larger-than-RAM sorting, spill files, merge passes | `planned` |
| `directory_tree_indexing` | Agam/Rust/Python plus OS metadata APIs | inode or directory metadata walks, syscall overhead, small-file locality | `planned` |
| `checkpoint_large_state` | Agam/Rust/Python | large snapshot serialization, write amplification, restore symmetry | `planned` |

## Future Lab Workloads

These matter, but they need dedicated harness work or subsystem integration before they should appear in the normal CPU-first result tables.

| Workload | Planned comparisons | Constraint focus | Status |
| --- | --- | --- | --- |
| `video_io_8k` | Agam/Rust/C++ plus dataset tooling | multi-gigabyte sequential read/write throughput, large-buffer staging | `future-lab` |
| `video_transcoding_io_phase` | Agam/C++/Python plus codec tooling | media pipeline read/write pressure, staging throughput, temporary-file churn | `future-lab` |
| `random_block_io_4k` | fio-style tooling plus Agam wrappers | random IOPS, latency tails, SSD firmware behavior | `future-lab` |
| `sqlite_concurrent_inserts` | Agam/Rust/Python plus SQLite tooling | fsync overhead, random I/O, transaction latency | `future-lab` |
| `btree_page_faults` | Agam/C++/Rust plus DB-page fixtures | on-disk page walks, fault latency, cache eviction behavior | `future-lab` |
| `grpc_serialization` | Agam/Rust/Python/C++ | protocol overhead, network-facing serialization cost | `future-lab` |
| `http_get_flood` | Agam/Rust/Go plus loopback or lab network harness | request fan-out, socket churn, protocol overhead | `future-lab` |
| `large_file_transfer_socket` | Agam/Rust/Go/C++ | socket-stream throughput, buffer sizing, copy avoidance | `future-lab` |
| `bittorrent_chunk_seeding` | Agam/Rust/Go plus peer harness | peer scheduling, chunk hashing, network and disk overlap | `future-lab` |
| `video_streaming_packets` | Agam/Rust/Go plus media harness | sustained packetization, jitter sensitivity, throughput shaping | `future-lab` |
| `tcp_handshake_storm` | Agam/Rust/Go plus loopback or lab network harness | connection setup rate, kernel handoff overhead, socket churn | `future-lab` |
| `icmp_echo_flood` | Agam plus network harness tooling | packet latency and rate handling outside normal app-layer paths | `future-lab` |
| `dns_resolution_lookups` | Agam/Rust/Go plus resolver harness | name-resolution latency, cache effects, protocol overhead | `future-lab` |
| `dht_routing` | Agam/Rust/Go plus distributed test harness | distributed lookup hops, routing-table churn, serialization | `future-lab` |
| `multiplayer_state_sync` | Agam/Rust/Go plus UDP harness | packet parsing, state-delta encoding, jitter tolerance | `future-lab` |
| `bgp_route_updates` | Agam/C++/Rust plus routing-table fixtures | large control-plane table churn, parsing, update propagation | `future-lab` |
| `transformer_self_attention` | Agam/Python/C++ accelerator path | tensor math, softmax, bandwidth vs compute balance | `future-lab` |
| `gpu_ray_tracing` | CUDA/HIP/Metal or Vulkan compute path | incoherent traversal, BVH access, massive parallel shading | `future-lab` |
| `gpu_rasterization_microtriangles` | Vulkan/DirectX/Metal harness | geometry throughput, fixed-function saturation, tiny-primitive pressure | `future-lab` |
| `gpu_global_illumination` | CUDA/HIP/Vulkan or engine harness | bounce accumulation, irregular memory access, shader occupancy | `future-lab` |
| `gpu_sgemm_dgemm` | BLAS or accelerator toolchain | dense matrix throughput, tensor-core or SIMD saturation, VRAM bandwidth | `future-lab` |
| `gpu_image_convolution` | CUDA/HIP/OpenCL/Vulkan compute | stencil throughput, shared-memory tiling, image bandwidth | `future-lab` |
| `gpu_fft` | cuFFT/rocFFT or equivalent | structured global-memory traffic, butterfly kernels, occupancy | `future-lab` |
| `fluid_dynamics_navier_stokes` | CUDA/HIP/C++ simulation harness | stencil sweeps, solver iterations, memory bandwidth and stability | `future-lab` |
| `molecular_dynamics` | CUDA/HIP/C++ simulation harness | neighbor lists, pairwise force kernels, memory locality | `future-lab` |
| `soft_body_cloth_physics` | CUDA/HIP/C++ simulation harness | constraint solves, irregular mesh access, iterative relaxation | `future-lab` |
| `particle_swarm_optimization` | Agam/Python/C++ accelerator path | swarm updates, reduction patterns, heuristic convergence cost | `future-lab` |
| `ethash_kawpow` | GPU mining kernels or equivalent | random memory access, hashing throughput, bandwidth pressure | `future-lab` |
| `bitonic_sort` | GPU compute or SIMD harness | compare-swap network regularity, parallel sorting throughput | `future-lab` |
| `parallel_reduction` | GPU compute or threaded host harness | tree reductions, synchronization, memory bandwidth | `future-lab` |
| `parallel_prefix_sum` | GPU compute or threaded host harness | scan primitives, synchronization, bandwidth | `future-lab` |
| `llm_prefill` | Agam/Python/C++ accelerator path | large batched GEMMs, attention setup, tensor-core saturation | `future-lab` |
| `llm_decode` | Agam/Python/C++ accelerator path | KV-cache bandwidth, small-step latency, token-by-token throughput | `future-lab` |
| `cnn_forward_pass` | Agam/Python/C++ accelerator path | convolution-heavy inference, layout transforms, activation bandwidth | `future-lab` |
| `lora_updates` | Agam/Python/C++ accelerator path | low-rank matrix updates, optimizer traffic, mixed precision | `future-lab` |
| `quantized_matrix_vector` | Agam/Python/C++ accelerator path | INT8 or INT4 dequantization, dot products, memory bandwidth | `future-lab` |
| `softmax_large_vocab` | Agam/Python/C++ accelerator path | wide reductions, exponentials, large-logit bandwidth | `future-lab` |
| `moe_token_routing` | Agam/Python/C++ accelerator path | gating, dispatch imbalance, scatter/gather traffic | `future-lab` |
| `embedding_table_lookup` | Agam/Python/C++ accelerator path | sparse lookup bandwidth, cache behavior, gather latency | `future-lab` |
| `activation_mapping` | Agam/Python/C++ accelerator path | GELU/Swish/ReLU kernels, elementwise throughput, vectorization | `future-lab` |
| `host_to_device_copy` | CUDA/HIP or equivalent device tooling | PCIe or accelerator interconnect saturation | `future-lab` |
| `device_to_host_copy` | CUDA/HIP or equivalent device tooling | return-path PCIe bandwidth, staging latency, synchronization cost | `future-lab` |
| `gpu_peer_to_peer_copy` | NVLink/PCIe peer harness | inter-GPU transfer throughput, topology sensitivity, copy overlap | `future-lab` |
| `rdma_operations` | RDMA-capable lab hardware and verbs tooling | zero-copy transport latency, NIC offload, queue-pair overhead | `future-lab` |
| `nvme_direct_storage` | DirectStorage or GPUDirect-style harness | bypassed copy paths, storage-to-device throughput, queue depth behavior | `future-lab` |
| `texture_streaming_to_gpu` | graphics engine or GPU asset harness | asset upload pacing, decompression overlap, bus saturation | `future-lab` |
| `shors_algorithm_sim` | quantum simulator tooling | large-state simulation cost, arithmetic structure, exponential scaling | `future-lab` |
| `grovers_algorithm_sim` | quantum simulator tooling | amplitude amplification, oracle cost, simulator memory growth | `future-lab` |
| `quantum_fourier_transform_sim` | quantum simulator tooling | structured gate patterns, simulator bandwidth, phase arithmetic | `future-lab` |
| `vqe_optimization` | quantum simulator plus optimizer tooling | variational loops, expectation aggregation, hybrid control overhead | `future-lab` |

## Constraint Clusters To Watch

- compute and branch behavior
  - current: `fibonacci`, `prime_sieve`, `quicksort`, `binary_search`, `edit_distance`, `regex_matching`
  - next: `rsa_encrypt_decrypt`, `aes256_software`, `sha256_hash`, `sha3_hash`, `ecdsa_sign_verify`, `mersenne_twister_rng`, `collatz_iteration`, `ackermann_function`, `n_queens`, `sudoku_backtracking`, `a_star_dense_grid`, `traveling_salesman_dp`, `minimax_tree_search`
- floating-point and numerical throughput
  - current: `fft`, `matrix_multiply`, `monte_carlo_pi`, `polynomial_eval`, `tensor_matmul`, `tensor_operations`, `convolution`, `softmax`
  - next: `mandelbrot`, `n_body_simulation`, `taylor_series_eval`, `finite_element_mesh`, `blocked_tiled_matrix_multiply`
- memory latency and bandwidth
  - current: `linked_list`, `hashmap_operations`, `btree_operations`, `ring_buffer`, `memory_allocation`, `garbage_collection`, `arc_contention`
  - next: `page_rank`, `sparse_matrix_vector`, `bloom_filter`, `fisher_yates_shuffle`, `graph_bfs_unorganized`, `stream_triad`, `mergesort_out_of_place`, `matrix_transpose_out_of_cache`, `radix_sort_large_ints`, `memcpy_bandwidth`, `prefix_sum_scan`, `sliding_window_strings`, `histogram_generation`, `bitboard_movegen`
- I/O and parser pressure
  - current: `file_reading`, `json_parsing`, `csv_scanning`, `token_frequency`
  - next: `burrows_wheeler_transform`, `huffman_codec`, `lz77_compression`, `xml_parsing_massive`, `log_appending`, `tarball_create_extract`, `file_dedup_chunking`, `external_sort`, `directory_tree_indexing`, `checkpoint_large_state`
- storage-heavy lab workloads
  - future: `video_io_8k`, `video_transcoding_io_phase`, `random_block_io_4k`, `sqlite_concurrent_inserts`, `btree_page_faults`
- compiler, JIT, and cache-policy behavior
  - current: all `08_jit_optimization/*` cases plus `09_compilation_metrics/*`
- accelerator and interconnect work
  - future: `transformer_self_attention`, `gpu_ray_tracing`, `gpu_rasterization_microtriangles`, `gpu_global_illumination`, `gpu_sgemm_dgemm`, `gpu_image_convolution`, `gpu_fft`, `fluid_dynamics_navier_stokes`, `molecular_dynamics`, `soft_body_cloth_physics`, `particle_swarm_optimization`, `ethash_kawpow`, `bitonic_sort`, `parallel_reduction`, `parallel_prefix_sum`, `host_to_device_copy`, `device_to_host_copy`, `gpu_peer_to_peer_copy`, `rdma_operations`, `nvme_direct_storage`, `texture_streaming_to_gpu`
- AI accelerator workloads
  - future: `llm_prefill`, `llm_decode`, `transformer_self_attention`, `cnn_forward_pass`, `lora_updates`, `quantized_matrix_vector`, `softmax_large_vocab`, `moe_token_routing`, `embedding_table_lookup`, `activation_mapping`
- network and distributed-system workloads
  - future: `grpc_serialization`, `http_get_flood`, `large_file_transfer_socket`, `bittorrent_chunk_seeding`, `video_streaming_packets`, `tcp_handshake_storm`, `icmp_echo_flood`, `dns_resolution_lookups`, `dht_routing`, `multiplayer_state_sync`, `bgp_route_updates`
- quantum-simulator workloads
  - future: `shors_algorithm_sim`, `grovers_algorithm_sim`, `quantum_fourier_transform_sim`, `vqe_optimization`

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
