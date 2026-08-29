#!/usr/bin/env python3
"""
Comprehensive Live Multi-Backend & Multi-Language Empirical Benchmark Runner
Platform: Windows 11 Native x86_64

Execution Targets:
  1. Agam JIT (Cranelift JIT Engine)
  2. Agam LLVM AOT (-O3 Native Binary via Clang 22)
  3. Agam C AOT (-O3 Native Binary via Zig Clang)
  4. C++ (Clang -O3)
  5. Rust (rustc -O)
  6. Go (go build)
  7. Python 3.14
"""

import os
import subprocess
import time
import shutil
import statistics
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parent

def find_agamc(root_dir):
    for candidate in [
        root_dir / "target" / "release" / "agamc.exe",
        root_dir / "target" / "release" / "agamc",
        root_dir / "agam" / "target" / "release" / "agamc.exe",
        root_dir / "agam" / "target" / "release" / "agamc",
    ]:
        if candidate.exists():
            return candidate
    return root_dir / "target" / "release" / ("agamc.exe" if os.name == "nt" else "agamc")

AGAM_BIN = find_agamc(ROOT)
SUITES_ROOT = ROOT / "benchmarks" / "suites"
if not SUITES_ROOT.exists() and (ROOT / "agam" / "benchmarks" / "suites").exists():
    SUITES_ROOT = ROOT / "agam" / "benchmarks" / "suites"

which_zig = shutil.which("zig")
ZIG_BIN = Path(which_zig) if which_zig else Path("C:/Users/ksvik/.tools/zig-windows-x86_64-0.13.0/zig.exe")

which_clang = shutil.which("clang")
LLVM_CLANG = Path(which_clang) if which_clang else Path("C:/Program Files/LLVM/bin/clang.exe")

which_go = shutil.which("go")
GO_BIN = Path(which_go) if which_go else Path("C:/Program Files/Go/bin/go.exe")

WORKLOADS = [
    # 01 Algorithms
    ("fibonacci", "01_algorithms"),
    ("quicksort", "01_algorithms"),
    ("prime_sieve", "01_algorithms"),
    ("binary_search", "01_algorithms"),
    ("edit_distance", "01_algorithms"),
    # 02 Numerical Computation
    ("matrix_multiply", "02_numerical_computation"),
    ("monte_carlo_pi", "02_numerical_computation"),
    ("fft", "02_numerical_computation"),
    ("polynomial_eval", "02_numerical_computation"),
    ("liquid_dsp_filter", "02_numerical_computation"),
    # 03 Data Structures
    ("hashmap_operations", "03_data_structures"),
    ("ring_buffer", "03_data_structures"),
    ("valkey_kv_store", "03_data_structures"),
    # 04 Compression
    ("lz77_compress", "04_compression_kernels"),
    ("rle_codec", "04_compression_kernels"),
    # 05 ML Primitives
    ("autodiff", "05_ml_primitives"),
    ("softmax", "05_ml_primitives"),
    # 07 Cryptography
    ("aes_sbox", "07_cryptography_kernels"),
    ("chacha20_cipher", "07_cryptography_kernels"),
    ("crc32_checksum", "07_cryptography_kernels"),
    ("sha256_hash", "07_cryptography_kernels"),
    # 08 Media Encoding
    ("audio_lpc", "08_media_encoding_kernels"),
    ("dct_transform", "08_media_encoding_kernels"),
    ("webp_encode", "08_media_encoding_kernels"),
    # 11 Ray Tracing
    ("c_ray_4k", "11_ray_tracing"),
    ("ray_sphere_intersect", "11_ray_tracing"),
    # 12 Game AI
    ("astar_pathfinding", "12_game_ai"),
    ("minimax_search", "12_game_ai"),
    # 13 SIMD Vectorization
    ("dot_product", "13_simd_vectorization"),
    ("mandelbrot_set", "13_simd_vectorization"),
    ("image_blur", "13_simd_vectorization"),
    # 14 String Processing
    ("base64_encode", "14_string_processing"),
    ("json_parse", "14_string_processing"),
]

def run_cmd(cmd, timeout=30, env=None):
    start = time.perf_counter()
    full_env = os.environ.copy()
    if env:
        full_env.update(env)
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=timeout, env=full_env)
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    return elapsed_ms, res.stdout.strip(), res.stderr.strip(), res.returncode

def benchmark_agam_jit(agm_file):
    cmd = [str(AGAM_BIN), "bench", str(agm_file)]
    _, stdout, stderr, rc = run_cmd(cmd)
    if rc != 0:
        return None, None, f"Agam JIT error: {stderr}"
    
    bench_lines = [l for l in stdout.splitlines() if l.startswith("bench ")]
    if not bench_lines:
        return None, None, "No bench output"
    
    parts = bench_lines[-1].split(":")
    if len(parts) >= 2:
        timing_part = parts[1].strip()
        ns_str = timing_part.split()[0]
        try:
            ns_val = float(ns_str)
            ms_val = ns_val / 1_000_000.0
            return ms_val, None, None
        except Exception as e:
            return None, None, str(e)
    return None, None, "Parse failed"

def benchmark_agam_llvm_aot(agm_file, runs=5):
    exe_file = ROOT / "temp_bench_agam_llvm.exe"
    ll_file = ROOT / "temp_bench_agam_llvm.ll"
    for f in [exe_file, ll_file]:
        if f.exists():
            try: f.unlink()
            except: pass

    compile_cmd = [str(AGAM_BIN), "build", "--backend", "llvm", "-O", "3", "-o", str(exe_file), str(agm_file)]
    env = {"AGAM_LLVM_CLANG": str(LLVM_CLANG)} if LLVM_CLANG.exists() else {}
    c_time, _, c_err, c_rc = run_cmd(compile_cmd, env=env)
    
    if not exe_file.exists() and ll_file.exists() and ZIG_BIN.exists():
        zig_cmd = [str(ZIG_BIN), "cc", "-O3", str(ll_file), "-o", str(exe_file)]
        _, _, z_err, z_rc = run_cmd(zig_cmd)
        if z_rc != 0:
            return None, None, f"Zig LLVM compile error: {z_err}"

    if not exe_file.exists():
        return None, None, f"Agam LLVM compile error: {c_err}"

    timings = []
    output = ""
    for _ in range(runs):
        t, out, _, rc = run_cmd([str(exe_file)])
        if rc == 0:
            timings.append(t)
            output = out

    for f in [exe_file, ll_file]:
        if f.exists():
            try: f.unlink()
            except: pass

    return statistics.median(timings) if timings else None, output, None

def benchmark_agam_c_aot(agm_file, runs=5):
    c_file = ROOT / "temp_bench_agam_c.c"
    exe_file = ROOT / "temp_bench_agam_c.exe"
    for f in [exe_file, c_file]:
        if f.exists():
            try: f.unlink()
            except: pass

    # 1. Generate C code
    gen_cmd = [str(AGAM_BIN), "build", "--backend", "c", "-o", str(exe_file), str(agm_file)]
    run_cmd(gen_cmd)

    if not c_file.exists() and not exe_file.exists():
        return None, None, "Agam C generation failed"

    # 2. Compile with C compiler -O3 if needed
    if not exe_file.exists() and c_file.exists():
        cc_bin = shutil.which("clang") or shutil.which("gcc") or shutil.which("cc")
        if cc_bin:
            compile_cmd = [cc_bin, "-O3", str(c_file), "-o", str(exe_file), "-lm"]
        elif ZIG_BIN.exists():
            compile_cmd = [str(ZIG_BIN), "cc", "-O3", str(c_file), "-o", str(exe_file), "-lm"]
        else:
            compile_cmd = None

        if compile_cmd:
            c_time, _, c_err, c_rc = run_cmd(compile_cmd)
            if c_rc != 0 or not exe_file.exists():
                return None, None, f"C backend compile error: {c_err}"

    timings = []
    output = ""
    for _ in range(runs):
        t, out, _, rc = run_cmd([str(exe_file)])
        if rc == 0:
            timings.append(t)
            output = out

    for f in [exe_file, c_file]:
        if f.exists():
            try: f.unlink()
            except: pass

    return statistics.median(timings) if timings else None, output, None

def benchmark_cpp(cpp_file, runs=5):
    exe_file = ROOT / ("temp_cpp_bench.exe" if os.name == "nt" else "temp_cpp_bench")
    if exe_file.exists():
        try: exe_file.unlink()
        except: pass
    
    is_cpp = cpp_file.suffix == ".cpp"
    cxx_bin = shutil.which("clang++") or shutil.which("g++") or shutil.which("c++")
    cc_bin = shutil.which("clang") or shutil.which("gcc") or shutil.which("cc")
    
    if is_cpp and cxx_bin:
        compile_cmd = [cxx_bin, "-O3", str(cpp_file), "-o", str(exe_file)]
    elif not is_cpp and cc_bin:
        compile_cmd = [cc_bin, "-O3", str(cpp_file), "-o", str(exe_file), "-lm"]
    elif ZIG_BIN.exists():
        comp = "c++" if is_cpp else "cc"
        compile_cmd = [str(ZIG_BIN), comp, "-O3", str(cpp_file), "-o", str(exe_file)]
        if not is_cpp:
            compile_cmd.append("-lm")
    else:
        return None, None, "No C/C++ compiler found"

    c_time, _, c_err, c_rc = run_cmd(compile_cmd)
    if c_rc != 0 or not exe_file.exists():
        return None, None, f"C++ compile error: {c_err}"

    timings = []
    output = ""
    for _ in range(runs):
        t, out, _, rc = run_cmd([str(exe_file)])
        if rc == 0:
            timings.append(t)
            output = out
    
    if exe_file.exists():
        try: exe_file.unlink()
        except: pass
        
    return statistics.median(timings) if timings else None, output, None

def benchmark_rust(rs_file, runs=5):
    exe_file = ROOT / ("temp_rs_bench.exe" if os.name == "nt" else "temp_rs_bench")
    if exe_file.exists():
        try: exe_file.unlink()
        except: pass
    
    compile_cmd = ["rustc", "-O", str(rs_file), "-o", str(exe_file)]
    c_time, _, c_err, c_rc = run_cmd(compile_cmd)
    if c_rc != 0 or not exe_file.exists():
        return None, None, f"Rust compile error: {c_err}"

    timings = []
    output = ""
    for _ in range(runs):
        t, out, _, rc = run_cmd([str(exe_file)])
        if rc == 0:
            timings.append(t)
            output = out
    
    if exe_file.exists():
        try: exe_file.unlink()
        except: pass
        
    return statistics.median(timings) if timings else None, output, None

def benchmark_go(go_file, runs=5):
    exe_file = ROOT / ("temp_go_bench.exe" if os.name == "nt" else "temp_go_bench")
    if exe_file.exists():
        try: exe_file.unlink()
        except: pass
    
    go_cmd = shutil.which("go") or str(GO_BIN)
    compile_cmd = [go_cmd, "build", "-o", str(exe_file), str(go_file)]
    c_time, _, c_err, c_rc = run_cmd(compile_cmd)
    if c_rc != 0 or not exe_file.exists():
        return None, None, f"Go compile error: {c_err}"

    timings = []
    output = ""
    for _ in range(runs):
        t, out, _, rc = run_cmd([str(exe_file)])
        if rc == 0:
            timings.append(t)
            output = out
    
    if exe_file.exists():
        try: exe_file.unlink()
        except: pass
        
    return statistics.median(timings) if timings else None, output, None

def benchmark_python(py_file, runs=5):
    timings = []
    output = ""
    py_cmd = sys.executable if sys.executable else "python"
    for _ in range(runs):
        t, out, _, rc = run_cmd([py_cmd, str(py_file)])
        if rc == 0:
            timings.append(t)
            output = out
    return statistics.median(timings) if timings else None, output, None

def main():
    print("=" * 140)
    print(f"{'Workload':<20} | {'Agam JIT':<10} | {'Agam LLVM':<10} | {'Agam C':<10} | {'C++ -O3':<10} | {'Rust -O':<10} | {'Go':<10} | {'Python':<10} | {'Agam vs Py':<11}")
    print("=" * 140)

    for name, suite in WORKLOADS:
        agm_file = SUITES_ROOT / suite / f"{name}.agam"
        cpp_file = SUITES_ROOT / suite / "comparisons" / f"{name}.cpp"
        c_file = SUITES_ROOT / suite / "comparisons" / f"{name}.c"
        rs_file = SUITES_ROOT / suite / "comparisons" / f"{name}.rs"
        go_file = SUITES_ROOT / suite / "comparisons" / f"{name}.go"
        py_file = SUITES_ROOT / suite / "comparisons" / f"{name}.py"

        target_c = cpp_file if cpp_file.exists() else (c_file if c_file.exists() else None)

        t_jit, _, _ = benchmark_agam_jit(agm_file) if agm_file.exists() else (None, None, None)
        t_llvm, _, _ = benchmark_agam_llvm_aot(agm_file) if agm_file.exists() else (None, None, None)
        t_c_aot, _, _ = benchmark_agam_c_aot(agm_file) if agm_file.exists() else (None, None, None)
        t_cpp, _, _ = benchmark_cpp(target_c) if target_c else (None, None, None)
        t_rust, _, _ = benchmark_rust(rs_file) if rs_file.exists() else (None, None, None)
        t_go, _, _ = benchmark_go(go_file) if go_file.exists() else (None, None, None)
        t_py, _, _ = benchmark_python(py_file) if py_file.exists() else (None, None, None)

        s_jit = f"{t_jit:.2f}" if t_jit is not None else "—"
        s_llvm = f"{t_llvm:.2f}" if t_llvm is not None else "—"
        s_c = f"{t_c_aot:.2f}" if t_c_aot is not None else "—"
        s_cpp = f"{t_cpp:.2f}" if t_cpp is not None else "—"
        s_rust = f"{t_rust:.2f}" if t_rust is not None else "—"
        s_go = f"{t_go:.2f}" if t_go is not None else "—"
        s_py = f"{t_py:.2f}" if t_py is not None else "—"

        ref_agam = t_llvm if t_llvm is not None else (t_c_aot if t_c_aot is not None else t_jit)
        vs_py = f"{t_py/ref_agam:.1f}x fast" if (ref_agam and t_py and ref_agam > 0) else "—"

        row = f"{name:<20} | {s_jit:<10} | {s_llvm:<10} | {s_c:<10} | {s_cpp:<10} | {s_rust:<10} | {s_go:<10} | {s_py:<10} | {vs_py:<11}"
        print(row)
        
        results.append({
            "name": name,
            "suite": suite,
            "jit": t_jit,
            "llvm": t_llvm,
            "c": t_c_aot,
            "cpp": t_cpp,
            "rust": t_rust,
            "go": t_go,
            "python": t_py,
        })

    print("=" * 140)

if __name__ == "__main__":
    main()
