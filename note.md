# Agam Benchmark & Toolchain Future Work Notes

## 📌 Items for Future Benchmark Expansion

### 1. Java (OpenJDK 21) Comparison Suite
- **Toolchain Present:** `Microsoft.OpenJDK.21` (`C:\Program Files\Microsoft\jdk-21.0.12.101-hotspot\bin\java.exe` / `javac.exe`).
- **Objective:** Add Java implementations under `benchmarks/suites/<suite>/comparisons/<workload>.java` for all standard workloads.
- **Harness Integration:** Wire `javac` + `java` into `scripts/run_live_comparisons.py` and `scripts/verify_all_comparison_codes.py` with warm JVM JIT iterations.

### 2. CPython vs. PyPy / C-Extension Benchmarking
- **Objective:** Benchmark standard CPython 3.14 against PyPy JIT and NumPy/C extensions across tensor, ML, and numerical workloads to provide a tiered interpreted vs JIT vs native speedup profile.

### 3. Systematic Parity Audit Triggers
- When ready, run `/parity-audit` or `/verify-benchmarks` to trigger automated checksum verification, compiler hardening sweeps, and live benchmark updates across all active backends.

---

## ⚡ Batteries-Included Standard Library & Dynamic Scripting Initiative

### Objective
Provide Python-grade developer ergonomics with C++/Rust bare-metal execution speed for rapid scripting, data wrangling, and scratch code without requiring external dependencies or compilation ceremonies.

### 1. Built-in Core Standard Library Modules (`std.*` / `agam_std.*`)

| # | Module | Core Functionality & APIs | Agam Performance Advantage |
| :---: | :--- | :--- | :--- |
| **1** | **`std.math`** | `sin`, `cos`, `tan`, `asin`, `acos`, `atan2`, `exp`, `ln`, `log10`, `sqrt`, `pow`, `hypot`, `floor`, `ceil`, `round`, `abs`, `erf`, `gamma`, `PI`, `E`, `TAU` | Lowered directly to LLVM hardware intrinsics (`llvm.sin`, `llvm.exp`, `llvm.fma`) and FMA3/AVX SIMD units. |
| **2** | **`std.complex`** | `Complex(re, im)`, `conj()`, `abs()`, `arg()`, `exp(z)`, `sin(z)`, `cos(z)`, `pow(z, n)` | Native IEEE-754 double-precision complex arithmetic with operator overloading (`+`, `-`, `*`, `/`). |
| **3** | **`std.re`** | `re.search`, `re.match`, `re.find_all`, `re.find_iter`, `re.replace`, `re.split`, `re.compile` | Guaranteed linear-time $O(n)$ DFA/NFA engine (zero catastrophic backtracking crashes). |
| **4** | **`std.os`** | `os.env_or`, `os.set_env`, `os.current_dir`, `os.name`, `os.cpu_count` | Safe, typed environment variable and OS-level metadata access. |
| **5** | **`std.sys`** | `sys.args()`, `sys.exit(code)`, `sys.platform`, `sys.memory_info()` | Zero-overhead runtime control, CLI argument array, and target architecture queries. |
| **6** | **`std.path`** | `Path.new(p) / "sub"`, `.exists()`, `.extension()`, `.parent()`, `.to_absolute()` | Object-oriented path handling with clean operator overloads (`/`). |
| **7** | **`std.fs`** | `fs.read_text`, `fs.write_text`, `fs.copy`, `fs.remove_file`, `fs.glob`, `fs.walk` | High-throughput file and directory tree manipulation without boilerplate. |
| **8** | **`std.json`** | `json.parse(str)`, `json.stringify(obj)`, `json.get_string`, `json.get_float` | Zero-allocation streaming JSON parser/serializer (3x–5x faster than CPython `json`). |
| **9** | **`std.time`** | `time.now()`, `time.Instant.now()`, `time.sleep_ms()`, `DateTime.to_iso()` | Nanosecond monotonic hardware timers + ISO-8601 calendar date/time parsing. |
| **10** | **`std.collections`** | `Counter.from()`, `HashMap<K,V>`, `HashSet<K>`, `Deque<T>`, `Heap<T>`, `bisect` | Flat SwissTable SIMD-aligned hash maps and ring buffers without Python dict memory bloat. |
| **11** | **`std.iter`** | `iter.permutations`, `iter.combinations`, `iter.zip`, `iter.chunks`, `iter.cycle` | Zero-cost iterator pipelines and combinatorics compiled directly to native loops. |
| **12** | **`std.fn`** | `@fn.memoize(max_size)`, `fn.partial(f, arg)`, `fn.compose(f, g)`, `fn.reduce` | Compile-time inlined functional tools and automatic LRU memoization caches. |
| **13** | **`std.log`** | `log.debug()`, `log.info()`, `log.warn()`, `log.error()`, `log.set_level()` | Leveled structured logging with compile-time level stripping (0 runtime cost in release). |
| **14** | **`std.cli`** | `cli.App.new()`, `.arg("-i", "--input")`, `.flag("-v")`, `.parse()` | Declarative CLI argument parsing with automatic `--help` generation. |
| **15** | **`std.random`** | `random.int_range()`, `random.float()`, `random.choice()`, `random.shuffle()` | High-throughput PCG-64 and Xoshiro PRNGs with hardware entropy seeding. |
| **16** | **`std.stats`** | `stats.mean()`, `stats.median()`, `stats.stdev()`, `stats.variance()`, `stats.quantiles()` | SIMD-vectorized (AVX2/AVX-512) statistical summary metrics for data science. |
| **17** | **`std.http`** | `http.get(url)`, `http.post(url, body)`, `http.serve(port, handler)` | Built-in HTTP client and lightweight micro-server with URL parser. |

### 2. High-Performance Extensions for Data Science & Engineering
- **`std.csv`**: High-throughput SIMD delimiter scanner for tabular dataset ingestion into native `DataFrame`.
- **`std.linalg`**: SIMD-accelerated 1D/2D vector and matrix math (`linalg.dot`, `linalg.matmul`, `linalg.transpose`, `linalg.solve`).
- **`std.sync`**: True multi-core thread execution (`sync.spawn`, `sync.parallel_for`, `sync.Mutex`, `sync.Channel`) with **ZERO GIL**.
- **`std.db`**: Built-in zero-dependency embedded **SQLite3** database interface.

### 3. Zero-Compile Dynamic Scripting Loop
- **Instant Execution**: `agamc run script.agam` (uses Cranelift JIT with <15ms startup, zero `.exe` disk artifacts created).
- **Direct Shebang Execution**: Support `#!/usr/bin/env agamc` on Linux/macOS and direct execution on Windows.
- **Top-level Scripting**: Allow top-level scripting statements directly in `@lang.base` without mandatory `fn main() -> i32` boilerplate for quick one-off scripts.


