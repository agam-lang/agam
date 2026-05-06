#!/usr/bin/env python3
"""Build and optionally publish the agam-ffi package to PyPI.

Usage:
    python publish.py --dry-run       # Build sdist + wheel only
    python publish.py                 # Build and upload to PyPI
    python publish.py --test-pypi     # Build and upload to TestPyPI
"""

import argparse
import subprocess
import sys
from pathlib import Path

PACKAGE_DIR = Path(__file__).resolve().parent


def run(cmd: list[str], *, check: bool = True) -> None:
    print(f"  → {' '.join(cmd)}")
    subprocess.run(cmd, cwd=PACKAGE_DIR, check=check)


def main() -> None:
    parser = argparse.ArgumentParser(description="Build & publish agam-ffi")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Build sdist + wheel without uploading",
    )
    parser.add_argument(
        "--test-pypi",
        action="store_true",
        help="Upload to TestPyPI instead of production PyPI",
    )
    args = parser.parse_args()

    # 1. Clean old artifacts
    dist = PACKAGE_DIR / "dist"
    if dist.exists():
        for artifact in dist.iterdir():
            artifact.unlink()
        print("[clean] removed old dist/ artifacts")

    # 2. Build sdist + wheel
    print("[build] building sdist + wheel …")
    run([sys.executable, "-m", "build"])

    if args.dry_run:
        print("[done] dry-run complete — artifacts in dist/")
        for artifact in sorted(dist.iterdir()):
            print(f"  • {artifact.name}")
        return

    # 3. Upload
    repo_args: list[str] = []
    if args.test_pypi:
        repo_args = ["--repository", "testpypi"]
        print("[upload] uploading to TestPyPI …")
    else:
        print("[upload] uploading to PyPI …")

    run([sys.executable, "-m", "twine", "upload", *repo_args, "dist/*"])
    print("[done] published successfully ✓")


if __name__ == "__main__":
    main()
