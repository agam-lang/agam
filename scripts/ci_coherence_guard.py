#!/usr/bin/env python3
"""
Agam CI Coherence Guard & Ratchet Verifier.

Asserts compiler stability invariants:
1. Total unwrap line count <= 1084
2. Total expect line count <= 1318
3. Total panic! line count <= 87
4. Combined Panic/Unwrap Sites <= 2489
5. Verifies required specification documents exist
"""

import os
import re
import sys
from pathlib import Path

# Baseline Measured Line Caps (Ratchet must strictly decrease from here)
CAP_UNWRAPS = 1084
CAP_EXPECTS = 1318
CAP_PANICS = 87
CAP_TOTAL = 2489

# Locate crates directory flexibly
SCRIPT_DIR = Path(__file__).resolve().parent
if (SCRIPT_DIR.parent / "crates").exists():
    AGAM_ROOT = SCRIPT_DIR.parent
    WORKSPACE_ROOT = SCRIPT_DIR.parent.parent
elif (SCRIPT_DIR.parent / "agam" / "crates").exists():
    AGAM_ROOT = SCRIPT_DIR.parent / "agam"
    WORKSPACE_ROOT = SCRIPT_DIR.parent
else:
    AGAM_ROOT = Path(".").resolve()
    WORKSPACE_ROOT = AGAM_ROOT

AGAM_CRATES_DIR = AGAM_ROOT / "crates"

UNWRAP_RE = re.compile(r'\.unwrap\(')
EXPECT_RE = re.compile(r'\.expect\(')
PANIC_RE = re.compile(r'panic!\(')

def count_panics():
    total_unwraps = 0
    total_expects = 0
    total_panics = 0

    if not AGAM_CRATES_DIR.exists():
        print(f"Error: Agam crates directory not found at {AGAM_CRATES_DIR}")
        sys.exit(1)

    for root, _, files in os.walk(AGAM_CRATES_DIR):
        for file in files:
            if file.endswith('.rs'):
                filepath = Path(root) / file
                try:
                    with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                        for line in f:
                            if UNWRAP_RE.search(line):
                                total_unwraps += 1
                            if EXPECT_RE.search(line):
                                total_expects += 1
                            if PANIC_RE.search(line):
                                total_panics += 1
                except Exception as e:
                    print(f"Warning: Could not read {filepath}: {e}")

    total_sites = total_unwraps + total_expects + total_panics

    print("==================================================")
    print("        AGAM CI COHERENCE & RATCHET REPORT        ")
    print("==================================================")
    print(f"Total .unwrap() sites : {total_unwraps:<5} (Cap: {CAP_UNWRAPS})")
    print(f"Total .expect() sites : {total_expects:<5} (Cap: {CAP_EXPECTS})")
    print(f"Total panic!() sites  : {total_panics:<5} (Cap: {CAP_PANICS})")
    print(f"Combined Total Sites  : {total_sites:<5} (Cap: {CAP_TOTAL})")
    print("--------------------------------------------------")

    failed = False
    if total_unwraps > CAP_UNWRAPS:
        print(f"[FAIL]: Unwrap count regressed! {total_unwraps} > {CAP_UNWRAPS}")
        failed = True
    else:
        print(f"[PASS]: Unwrap count within limit ({total_unwraps} <= {CAP_UNWRAPS})")

    if total_expects > CAP_EXPECTS:
        print(f"[FAIL]: Expect count regressed! {total_expects} > {CAP_EXPECTS}")
        failed = True
    else:
        print(f"[PASS]: Expect count within limit ({total_expects} <= {CAP_EXPECTS})")

    if total_panics > CAP_PANICS:
        print(f"[FAIL]: Panic count regressed! {total_panics} > {CAP_PANICS}")
        failed = True
    else:
        print(f"[PASS]: Panic count within limit ({total_panics} <= {CAP_PANICS})")

    if total_sites > CAP_TOTAL:
        print(f"[FAIL]: Total sites regressed! {total_sites} > {CAP_TOTAL}")
        failed = True
    else:
        print(f"[PASS]: Total sites within limit ({total_sites} <= {CAP_TOTAL})")

    if failed:
        print("Error: You must reduce unwrap/panic calls before merging.")
        sys.exit(1)

    return total_unwraps, total_expects, total_panics

def verify_required_docs():
    required = [
        AGAM_ROOT / "docs" / "MEMORY_MODEL.md",
        AGAM_ROOT / "docs" / "grammar.ebnf",
        AGAM_ROOT / "docs" / "ADOPTED_DEPENDENCIES.md",
        AGAM_ROOT / "docs" / "FUTURE_ARCHITECTURE.md",
    ]
    print("\n--- Verifying Required Specification Artifacts ---")
    all_ok = True
    for doc in required:
        if doc.exists() and doc.stat().st_size > 0:
            print(f"[FOUND]: {doc.relative_to(AGAM_ROOT)} ({doc.stat().st_size} bytes)")
        else:
            print(f"[MISSING/EMPTY]: {doc.relative_to(AGAM_ROOT)}")
            all_ok = False
    
    if not all_ok:
        print("[FAIL]: Required specification artifacts are missing!")
        sys.exit(1)
    print("[PASS]: All required specification artifacts present.")

if __name__ == "__main__":
    count_panics()
    verify_required_docs()
    print("==================================================")
    print("[SUCCESS]: CI COHERENCE GUARD PASSED (0 REGRESSIONS)")
    print("==================================================")
