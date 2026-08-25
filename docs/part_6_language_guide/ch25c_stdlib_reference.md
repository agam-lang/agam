# Chapter 25c: Standard Library Reference (`agam_std`)

> **Part VI: The Agam Language Programming Guide**  
> **Compiler Module Focus**: [`agam_std`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/runtime/agam_std)

---

## 25c.1 Standard Library Architecture

The Agam standard library (`agam_std`) provides essential data structures, algorithms, and domain-specific modules that ship with every Agam installation:

```text
agam_std
  ├── collections/     High-performance containers
  ├── sparse/          Sparse matrix formats
  ├── fft/             Fast Fourier Transform
  ├── gpu/             GPU tile abstractions
  ├── math/            Mathematical functions
  ├── io/              Input/output utilities
  ├── string/          String manipulation
  └── fmt/             Formatting and display
```

---

## 25c.2 High-Performance Collections

### FastRingBuffer

A lock-free ring buffer optimized for single-producer, single-consumer (SPSC) communication:

```agam
let ring = FastRingBuffer[Int].new(capacity: 1024);

// Producer
ring.push(42);
ring.push(43);

// Consumer
let val = ring.pop();  // Option.Some(42)
```

| Operation | Time Complexity | Notes |
| :--- | :---: | :--- |
| `push(val)` | $O(1)$ amortized | Returns `false` if full |
| `pop()` | $O(1)$ | Returns `Option.None` if empty |
| `len()` | $O(1)$ | Current element count |
| `capacity()` | $O(1)$ | Fixed at creation |

### CompactGraph

An adjacency-list graph with built-in shortest path algorithms:

```agam
let mut graph = CompactGraph[String].new();

// Add vertices
let a = graph.add_vertex("A");
let b = graph.add_vertex("B");
let c = graph.add_vertex("C");
let d = graph.add_vertex("D");

// Add weighted edges
graph.add_edge(a, b, weight: 4.0);
graph.add_edge(a, c, weight: 2.0);
graph.add_edge(c, b, weight: 1.0);
graph.add_edge(b, d, weight: 3.0);
graph.add_edge(c, d, weight: 5.0);

// Dijkstra's shortest path
let path = graph.dijkstra(source: a, target: d);
// path: Some(["A", "C", "B", "D"], cost: 6.0)
```

| Algorithm | Time Complexity | Description |
| :--- | :---: | :--- |
| `dijkstra(s, t)` | $O((V + E) \log V)$ | Shortest path (non-negative weights) |
| `bfs(s)` | $O(V + E)$ | Breadth-first traversal |
| `dfs(s)` | $O(V + E)$ | Depth-first traversal |
| `topological_sort()` | $O(V + E)$ | DAG topological ordering |

---

## 25c.3 Sparse Matrix Operations

The `agam_std::sparse` module provides compressed sparse matrix formats for scientific computing:

### CSR (Compressed Sparse Row)

```agam
// Create a sparse matrix in CSR format
// Matrix: [[1, 0, 2], [0, 0, 3], [4, 5, 6]]
let csr = SparseCSR[Float].from_triplets(
    rows: 3, cols: 3,
    entries: [
        (0, 0, 1.0), (0, 2, 2.0),
        (1, 2, 3.0),
        (2, 0, 4.0), (2, 1, 5.0), (2, 2, 6.0),
    ]
);

// Sparse matrix-vector multiply (SpMV)
let x = [1.0, 2.0, 3.0];
let y = csr.spmv(x);
// y = [7.0, 9.0, 32.0]
```

### COO (Coordinate Format)

```agam
// COO format — efficient for construction, then convert to CSR for computation
let coo = SparseCOO[Float].new(rows: 1000, cols: 1000);
coo.add(row: 42, col: 99, value: 3.14);
coo.add(row: 99, col: 42, value: 2.71);

let csr = coo.to_csr();  // Convert for efficient SpMV
```

### Format Comparison

| Format | Construction | SpMV | Memory | Best For |
| :--- | :---: | :---: | :---: | :--- |
| CSR | $O(nnz \cdot \log nnz)$ | $O(nnz)$ | $O(nnz + n)$ | Row-oriented access |
| COO | $O(1)$ per entry | $O(nnz)$ | $O(3 \cdot nnz)$ | Incremental construction |

---

## 25c.4 Fast Fourier Transform (FFT)

The `agam_std::fft` module provides Radix-2 Cooley-Tukey FFT with windowing support:

```agam
// Forward FFT
let signal: [Float; 1024] = generate_signal(freq: 440.0, sample_rate: 44100.0);
let spectrum = fft.forward(signal);

// Inverse FFT
let reconstructed = fft.inverse(spectrum);

// Windowed FFT (reduces spectral leakage)
let windowed = fft.forward_windowed(signal, window: fft.Window.Hanning);
```

### Supported Window Functions

| Window | Sidelobe Level | Main Lobe Width | Use Case |
| :--- | :---: | :---: | :--- |
| `Rectangular` | -13 dB | Narrowest | Maximum frequency resolution |
| `Hanning` | -31 dB | Medium | General purpose |
| `Hamming` | -43 dB | Medium | Speech processing |
| `Blackman` | -58 dB | Widest | Low sidelobe requirements |

### Performance

| Input Size | FFT Time | Algorithm |
| :---: | :---: | :--- |
| 1,024 | 12 μs | Radix-2 Cooley-Tukey |
| 4,096 | 58 μs | Radix-2 Cooley-Tukey |
| 65,536 | 1.1 ms | Radix-2 Cooley-Tukey |
| 1,048,576 | 22 ms | Radix-2 Cooley-Tukey |

---

## 25c.5 GPU Tile Abstractions

The `agam_std::gpu` module provides high-level tile primitives for GPU programming (detailed in Chapter 34):

```agam
// 2D collaborative tile
let tile: Tile[Float, 16, 16] = Tile.zeros();
tile.load_strided(ptr, stride: N);

// Multi-dimensional partition view
let extent = Extent.new([128, 64]);
let view = PartitionView.from_tensor(tensor, offset: [0, 0], extent: extent);

// Asynchronous pipeline stage
let stage = AsyncPipelineStage.new(stage_index: 0);
stage.begin();
stage.commit();
stage.wait();

// Tile matrix multiply with fused activation
let result = tile_matmul(A_tile, B_tile);
result.apply_relu();
```

---

## 25c.6 Mathematical Functions

```agam
// Trigonometric
let s = math.sin(1.57);    // 1.0
let c = math.cos(0.0);     // 1.0
let t = math.tan(0.785);   // ~1.0

// Exponential / Logarithmic
let e = math.exp(1.0);     // 2.718...
let l = math.ln(2.718);    // ~1.0
let l2 = math.log2(256.0); // 8.0

// Power / Root
let p = math.pow(2.0, 10.0);  // 1024.0
let r = math.sqrt(144.0);     // 12.0
let c = math.cbrt(27.0);      // 3.0

// Special functions
let g = math.gamma(5.0);      // 24.0 (4!)
let b = math.beta(2.0, 3.0);  // 0.0833...
let erf = math.erf(1.0);      // 0.8427...

// Constants
let pi = math.PI;             // 3.14159265...
let e = math.E;               // 2.71828182...
let phi = math.PHI;           // 1.61803398... (golden ratio)
```

---

## 25c.7 I/O Utilities

```agam
// File I/O
let content = File.read("data.txt");
File.write("output.txt", "Hello, Agam!");
File.append("log.txt", timestamp() + ": event occurred\n");

// Buffered I/O for large files
let reader = BufferedReader.open("large_dataset.csv");
while let Option.Some(line) = reader.read_line() {
    process(line);
}

// Standard I/O
let input = io.read_line();      // Read from stdin
io.write("Enter name: ");       // Write to stdout without newline
io.write_err("Warning!\n");     // Write to stderr
```
