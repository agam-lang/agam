#!/usr/bin/env python3
"""
Agam CI Coherence Guard & Ratchet Verifier.

Asserts compiler stability invariants:
1. Total unwrap line count <= 1080
2. Total expect line count <= 993
3. Total panic! line count <= 81
4. Combined Panic/Unwrap Sites <= 2154
5. Verifies structural invariant: single canonical docs/ directory (no doc/ split)
6. Verifies required specification documents exist in canonical docs/
7. Verifies literature and algorithm citations
"""

import os
import re
import sys
from pathlib import Path

# Baseline Measured Line Caps (Ratchet must strictly decrease from here)
CAP_UNWRAPS = 1080
CAP_EXPECTS = 993
CAP_PANICS = 81
CAP_TOTAL = 2154

# Locate crates and docs directories flexibly
SCRIPT_DIR = Path(__file__).resolve().parent
if (SCRIPT_DIR.parent / "crates").exists():
    AGAM_ROOT = SCRIPT_DIR.parent
    WORKSPACE_ROOT = SCRIPT_DIR.parent.parent if (SCRIPT_DIR.parent.parent / "docs").exists() else SCRIPT_DIR.parent
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

def verify_no_doc_docs_split():
    """
    Structural Invariant: Enforce single canonical documentation directory (`docs/`).
    Asserts that `doc/` and `docs/` are never simultaneously present as independent
    physical directories, preventing doc/ vs docs/ synchronization drift.
    """
    print("\n--- Verifying Documentation Directory Structure Invariant ---")
    roots_to_check = set([WORKSPACE_ROOT, AGAM_ROOT])
    for root in roots_to_check:
        doc_path = root / "doc"
        docs_path = root / "docs"

        if doc_path.exists():
            if doc_path.is_symlink():
                resolved_target = doc_path.resolve()
                if resolved_target != docs_path.resolve():
                    print(f"[FAIL]: Symlink '{doc_path}' points to '{resolved_target}', expected '{docs_path.resolve()}'")
                    sys.exit(1)
                print(f"[PASS]: 'doc/' verified as symlink to canonical 'docs/' at {root}")
            else:
                print(f"[FAIL]: Non-canonical directory '{doc_path}' detected!")
                print(f"        'docs/' is the single canonical documentation directory.")
                print(f"        'doc/' must not exist as an independent directory.")
                sys.exit(1)

    canonical_docs = WORKSPACE_ROOT / "docs" if (WORKSPACE_ROOT / "docs").exists() else AGAM_ROOT / "docs"
    if not canonical_docs.exists():
        print(f"[FAIL]: Canonical documentation directory '{canonical_docs}' missing!")
        sys.exit(1)

    print(f"[PASS]: Single canonical documentation directory verified at: {canonical_docs.resolve()}")

def verify_required_docs():
    required_names = [
        "MEMORY_MODEL.md",
        "grammar.ebnf",
        "ADOPTED_DEPENDENCIES.md",
        "FUTURE_ARCHITECTURE.md",
        "RFC-std-db.md",
    ]
    print("\n--- Verifying Required Specification Artifacts in Canonical docs/ ---")
    docs_dir = WORKSPACE_ROOT / "docs" if (WORKSPACE_ROOT / "docs").exists() else AGAM_ROOT / "docs"
    all_ok = True
    for name in required_names:
        doc = docs_dir / name
        if doc.exists() and doc.stat().st_size > 0:
            print(f"[FOUND]: docs/{name} ({doc.stat().st_size} bytes)")
        else:
            print(f"[MISSING/EMPTY]: docs/{name} in {docs_dir}")
            all_ok = False
    
    if not all_ok:
        print("[FAIL]: Required specification artifacts are missing in canonical docs/!")
        sys.exit(1)
    print("[PASS]: All required specification artifacts present in canonical docs/.")

def verify_literature_citations():
    """
    Automated Literature & Algorithm Citation Verifier.
    Cross-references claims in architectural docs against codebase reality.
    """
    print("\n--- Verifying Literature & Algorithm Citations ---")
    docs_dir = WORKSPACE_ROOT / "docs" if (WORKSPACE_ROOT / "docs").exists() else AGAM_ROOT / "docs"
    doc_path = docs_dir / "FUTURE_ARCHITECTURE.md"
    if not doc_path.exists():
        print("[FAIL]: FUTURE_ARCHITECTURE.md not found for citation check")
        sys.exit(1)

    with open(doc_path, 'r', encoding='utf-8', errors='ignore') as f:
        doc_content = f.read()

    # Banned unverified / fabricated phrases that must never re-appear
    banned_claims = [
        ("Tarjan SCC Monomorphization", "Monomorphization is worklist-based in monomorphize.rs, not Tarjan SCC"),
        ("Lengauer–Tarjan Dominators", "Dominance computation uses Cooper-Harvey-Kennedy in analysis.rs"),
    ]

    for banned, reason in banned_claims:
        if banned in doc_content:
            print(f"[FAIL]: Disallowed unverified claim found in docs: '{banned}' ({reason})")
            sys.exit(1)

    # Required verified claims that must match real code
    required_citations = [
        ("Cooper–Harvey–Kennedy Dominators", AGAM_CRATES_DIR / "middle" / "agam_mir" / "src" / "analysis.rs", "Cooper-Harvey-Kennedy"),
        ("`egg`", AGAM_CRATES_DIR / "middle" / "agam_mir" / "src" / "opt" / "egg_engine.rs", "egg"),
    ]

    for claim, code_path, code_keyword in required_citations:
        if not code_path.exists():
            print(f"[FAIL]: Implementation file for '{claim}' missing at {code_path}")
            sys.exit(1)
        with open(code_path, 'r', encoding='utf-8', errors='ignore') as f:
            code_content = f.read()
        if code_keyword not in code_content:
            print(f"[FAIL]: Keyword '{code_keyword}' not found in {code_path} for claim '{claim}'")
            sys.exit(1)
        print(f"[PASS]: Verified citation '{claim}' -> {code_path.name}")

    print("[PASS]: All literature and algorithm citations verified against code.")

if __name__ == "__main__":
    count_panics()
    verify_no_doc_docs_split()
    verify_required_docs()
    verify_literature_citations()
    print("==================================================")
    print("[SUCCESS]: CI COHERENCE GUARD PASSED (0 REGRESSIONS)")
    print("==================================================")
