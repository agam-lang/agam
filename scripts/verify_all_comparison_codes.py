#!/usr/bin/env python3
"""Comprehensive Audit & Verification of All Cross-Language Benchmark Implementations.

Verifies:
1. Compilation without error (C++, C, Rust, Agam)
2. Execution without panic or crash
3. Deterministic output / numerical checksum parity across ALL languages
"""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parent
if (ROOT / "target" / "release" / "agamc.exe").exists():
    AGAM_BIN = ROOT / "target" / "release" / "agamc.exe"
    SUITES_ROOT = ROOT / "benchmarks" / "suites"
elif (ROOT / "agam" / "target" / "release" / "agamc.exe").exists():
    AGAM_BIN = ROOT / "agam" / "target" / "release" / "agamc.exe"
    SUITES_ROOT = ROOT / "benchmarks" / "suites"
else:
    AGAM_BIN = ROOT / "target" / "release" / "agamc.exe"
    SUITES_ROOT = ROOT / "benchmarks" / "suites"

if not SUITES_ROOT.exists() and (ROOT / "agam" / "benchmarks" / "suites").exists():
    SUITES_ROOT = ROOT / "agam" / "benchmarks" / "suites"

ZIG_BIN = Path("C:/Users/ksvik/.tools/zig-windows-x86_64-0.13.0/zig.exe")

def run_cmd(cmd, timeout=30):
    try:
        res = subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=timeout
        )
        return res.stdout.strip(), res.stderr.strip(), res.returncode
    except Exception as e:
        return "", str(e), -1

def clean_output(out_str):
    """Normalize output by taking the last non-empty line (the return/checksum value)."""
    lines = [l.strip() for l in out_str.strip().splitlines() if l.strip()]
    if not lines:
        return ""
    # Strip any warning lines or info prefixes
    for l in reversed(lines):
        if not l.startswith("warning:") and not l.startswith("note:"):
            return l
    return lines[-1]

def verify_all_suites():
    suites_root = SUITES_ROOT
    
    total_workloads = 0
    passed_workloads = 0
    issues = []

    print("=" * 110)
    print("STARTING FULL CROSS-LANGUAGE BENCHMARK AUDIT & OUTPUT VERIFICATION")
    print("=" * 110)
    print(f"{'Suite / Workload':<35} | {'Agam':<15} | {'C++':<15} | {'Rust':<15} | {'Python':<15} | {'Status'}")
    print("-" * 110)

    for suite_dir in sorted(suites_root.iterdir()):
        if not suite_dir.is_dir():
            continue
        
        comp_dir = suite_dir / "comparisons"
        agam_files = sorted(suite_dir.glob("*.agam"))
        
        for agm_file in agam_files:
            name = agm_file.stem
            workload_id = f"{suite_dir.name}/{name}"
            total_workloads += 1
            
            cpp_file = comp_dir / f"{name}.cpp" if comp_dir.exists() else None
            c_file = comp_dir / f"{name}.c" if comp_dir.exists() else None
            rs_file = comp_dir / f"{name}.rs" if comp_dir.exists() else None
            py_file = comp_dir / f"{name}.py" if comp_dir.exists() else None

            # 1. Agam Output
            out_agam = None
            out, err, rc = run_cmd([str(AGAM_BIN), "run", "--backend", "jit", str(agm_file)])
            if rc == 0:
                out_agam = clean_output(out)
            else:
                issues.append((workload_id, "Agam", f"Runtime error: {err}"))

            # 2. C++ / C Output
            out_cpp = None
            target_c_file = cpp_file if (cpp_file and cpp_file.exists()) else (c_file if (c_file and c_file.exists()) else None)
            if target_c_file and target_c_file.exists() and ZIG_BIN.exists():
                temp_exe = ROOT / f"temp_audit_{name}.exe"
                comp = "c++" if target_c_file.suffix == ".cpp" else "cc"
                c_out, c_err, c_rc = run_cmd([str(ZIG_BIN), comp, "-O3", str(target_c_file), "-o", str(temp_exe)])
                if c_rc == 0 and temp_exe.exists():
                    r_out, r_err, r_rc = run_cmd([str(temp_exe)])
                    if r_rc == 0:
                        out_cpp = clean_output(r_out)
                    else:
                        issues.append((workload_id, "C++", f"Execution failed: {r_err}"))
                    if temp_exe.exists():
                        try:
                            temp_exe.unlink()
                        except:
                            pass
                else:
                    issues.append((workload_id, "C++", f"Compilation failed: {c_err}"))

            # 3. Rust Output
            out_rs = None
            if rs_file and rs_file.exists():
                temp_rs_exe = ROOT / f"temp_rs_audit_{name}.exe"
                c_out, c_err, c_rc = run_cmd(["rustc", "-O", str(rs_file), "-o", str(temp_rs_exe)])
                if c_rc == 0 and temp_rs_exe.exists():
                    r_out, r_err, r_rc = run_cmd([str(temp_rs_exe)])
                    if r_rc == 0:
                        out_rs = clean_output(r_out)
                    else:
                        issues.append((workload_id, "Rust", f"Execution failed: {r_err}"))
                    if temp_rs_exe.exists():
                        try:
                            temp_rs_exe.unlink()
                        except:
                            pass
                else:
                    issues.append((workload_id, "Rust", f"Compilation failed: {c_err}"))

            # 4. Python Output
            out_py = None
            if py_file and py_file.exists():
                r_out, r_err, r_rc = run_cmd(["python", str(py_file)])
                if r_rc == 0:
                    out_py = clean_output(r_out)
                else:
                    issues.append((workload_id, "Python", f"Execution failed: {r_err}"))

            # Parity check
            outputs = [o for o in [out_agam, out_cpp, out_rs, out_py] if o is not None]
            all_match = len(outputs) >= 2 and all(o == outputs[0] for o in outputs)
            
            s_agm = out_agam if out_agam is not None else "ERR/MISS"
            s_cpp = out_cpp if out_cpp is not None else "ERR/MISS"
            s_rs = out_rs if out_rs is not None else "ERR/MISS"
            s_py = out_py if out_py is not None else "ERR/MISS"

            status = "VERIFIED" if all_match else "MISMATCH / PARTIAL"
            if all_match:
                passed_workloads += 1

            print(f"{workload_id:<35} | {s_agm:<15} | {s_cpp:<15} | {s_rs:<15} | {s_py:<15} | {status}")

    print("=" * 110)
    print(f"AUDIT SUMMARY: {passed_workloads} / {total_workloads} Workloads Fully Verified for 100% Output Parity.")
    print("=" * 110)

    if issues:
        print("\nIdentified Diagnostics / Differences to Align:")
        for w_id, lang, desc in issues[:10]:
            print(f"  - [{w_id}] ({lang}): {desc[:100]}")

if __name__ == "__main__":
    verify_all_suites()
