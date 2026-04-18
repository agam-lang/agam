#!/usr/bin/env python3
"""End-to-end local validation of the Agam SDK packaging pipeline.

Mirrors the CI workflow (sdk-dist.yml) so the same contract can be proven
locally without hosted runners:

  1. Build agamc via cargo
  2. Discover local LLVM tools
  3. Run package_sdk.py with archive + checksum
  4. Extract the archive to a temp directory
  5. Validate checksum matches
  6. Validate extracted manifest, compiler binary, and LLVM bundle
  7. Print a pass/fail summary
"""

from __future__ import annotations

import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPTS_DIR = REPO_ROOT / "scripts"


def main() -> int:
    verbose = "--verbose" in sys.argv or "-v" in sys.argv
    release = "--release" in sys.argv

    print("=" * 60)
    print("Agam SDK End-to-End Local Validation")
    print("=" * 60)
    print(f"  repo root:  {REPO_ROOT}")
    print(f"  platform:   {platform.system()} {platform.machine()}")
    print(f"  release:    {release}")
    print()

    # Step 1: Run package_sdk.py to build, package, archive, and validate
    print("--- Step 1: Build + Package + Archive SDK ---")
    staging_dir = REPO_ROOT / "dist" / "e2e-validation"
    if staging_dir.exists():
        shutil.rmtree(staging_dir)

    archive_format = "zip" if os.name == "nt" else "tar.gz"
    archive_ext = archive_format

    cmd = [
        sys.executable,
        str(SCRIPTS_DIR / "package_sdk.py"),
        "--output", str(staging_dir),
        "--require-llvm-bundle",
        "--archive-format", archive_format,
        "--checksum",
    ]
    if release:
        cmd.append("--release")
    if verbose:
        cmd.append("--verbose")

    try:
        subprocess.run(cmd, cwd=REPO_ROOT, check=True)
    except subprocess.CalledProcessError as exc:
        print(f"\nFAILED: package_sdk.py exited with code {exc.returncode}")
        return 1

    print("\n--- Step 2: Locate produced artifacts ---")
    archives = sorted(
        path
        for path in staging_dir.parent.iterdir()
        if path.is_file()
        and (path.suffix == ".zip" or path.name.endswith(".tar.gz"))
        and "agam-sdk-" in path.name
    )
    if not archives:
        print(f"FAILED: no SDK archive found under {staging_dir.parent}")
        return 1

    archive = archives[0]
    checksum_path = Path(f"{archive}.sha256")
    print(f"  archive:  {archive}")
    print(f"  checksum: {checksum_path}")

    if not checksum_path.is_file():
        print(f"FAILED: checksum file {checksum_path} not found")
        return 1

    # Step 3: Validate checksum
    print("\n--- Step 3: Validate archive checksum ---")
    expected = checksum_path.read_text(encoding="utf-8").strip().split()[0]
    actual = sha256(archive)
    if actual != expected:
        print(f"FAILED: checksum mismatch")
        print(f"  expected: {expected}")
        print(f"  actual:   {actual}")
        return 1
    print(f"  sha256: {actual[:16]}... OK")

    # Step 4: Extract and validate
    print("\n--- Step 4: Extract and validate archive contents ---")
    with tempfile.TemporaryDirectory(prefix="agam_sdk_e2e_") as tempdir:
        extract_root = Path(tempdir)

        if archive.suffix == ".zip":
            with zipfile.ZipFile(archive) as bundle:
                bundle.extractall(extract_root)
        else:
            with tarfile.open(archive, "r:gz") as bundle:
                if sys.version_info >= (3, 12):
                    bundle.extractall(extract_root, filter="data")
                else:
                    bundle.extractall(extract_root)

        # Find sdk-manifest.json
        manifests = sorted(extract_root.glob("**/sdk-manifest.json"))
        if len(manifests) != 1:
            found = [str(m.relative_to(extract_root)) for m in manifests]
            print(f"FAILED: expected exactly one sdk-manifest.json, found {found}")
            return 1

        manifest_path = manifests[0]
        sdk_root = manifest_path.parent
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))

        print(f"  manifest:   {manifest_path.relative_to(extract_root)}")
        print(f"  sdk_name:   {manifest.get('sdk_name', '?')}")
        print(f"  platform:   {manifest.get('host_platform', '?')}")

        # Validate compiler binary
        compiler_rel = manifest.get("compiler_binary", "")
        compiler_path = sdk_root / compiler_rel
        if not compiler_path.is_file():
            print(f"FAILED: compiler binary not found at {compiler_rel}")
            return 1
        print(f"  compiler:   {compiler_rel} OK")

        # Validate LLVM bundle
        llvm_bundle = manifest.get("llvm_bundle_root")
        llvm_driver = manifest.get("preferred_llvm_driver")
        if llvm_bundle:
            if not (sdk_root / llvm_bundle).is_dir():
                print(f"FAILED: LLVM bundle root not found at {llvm_bundle}")
                return 1
            print(f"  llvm_root:  {llvm_bundle} OK")
        if llvm_driver:
            if not (sdk_root / llvm_driver).is_file():
                print(f"FAILED: LLVM driver not found at {llvm_driver}")
                return 1
            print(f"  llvm_drv:   {llvm_driver} OK")

        # Validate target packs
        for target in manifest.get("supported_targets", []):
            packaged_sysroot = target.get("packaged_sysroot")
            if not packaged_sysroot:
                continue
            sysroot_path = sdk_root / packaged_sysroot
            if not sysroot_path.is_dir():
                print(f"FAILED: target pack sysroot not found at {packaged_sysroot}")
                return 1
            if not (sysroot_path / "usr").is_dir():
                print(f"FAILED: target pack sysroot missing usr/ at {packaged_sysroot}")
                return 1
            print(f"  target:     {target.get('name', '?')} ({packaged_sysroot}) OK")

    # Step 5: Clean up staging
    print("\n--- Step 5: Cleanup ---")
    if staging_dir.exists():
        shutil.rmtree(staging_dir)
    if archive.is_file():
        archive.unlink()
    if checksum_path.is_file():
        checksum_path.unlink()
    print("  cleaned up staging artifacts")

    print("\n" + "=" * 60)
    print("ALL CHECKS PASSED — SDK pipeline validated end to end")
    print("=" * 60)
    return 0


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


if __name__ == "__main__":
    sys.exit(main())
