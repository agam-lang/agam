#!/usr/bin/env python3
"""Build, package, and validate a host-native Agam SDK distribution."""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
COMPILER_NAME = "agamc.exe" if os.name == "nt" else "agamc"
LLVM_DRIVER_NAMES = ["clang.exe", "clang++.exe"] if os.name == "nt" else ["clang", "clang++"]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Build and validate a host-native Agam SDK distribution.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=default_output_dir(),
        help="output directory for the packaged SDK",
    )
    parser.add_argument(
        "--compiler",
        type=Path,
        help="path to an existing agamc binary; defaults to building target/debug/agamc",
    )
    parser.add_argument(
        "--llvm-bundle",
        type=Path,
        help="existing LLVM bundle root or platform directory to package",
    )
    parser.add_argument(
        "--llvm-driver",
        type=Path,
        help="explicit clang or clang++ binary used to seed a minimal LLVM bundle",
    )
    parser.add_argument(
        "--require-llvm-bundle",
        action="store_true",
        help="fail if no LLVM bundle can be staged",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="print command execution details",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    compiler = resolve_compiler(args.compiler, args.verbose)

    args.output = args.output.resolve()
    if args.output.exists():
        shutil.rmtree(args.output)

    with temporary_llvm_bundle(args.llvm_bundle, args.llvm_driver, args.verbose) as bundle_root:
        if args.require_llvm_bundle and bundle_root is None:
            raise SystemExit("no LLVM bundle source was found; pass --llvm-bundle or --llvm-driver")

        package_sdk(compiler, args.output, bundle_root, args.verbose)
        validate_sdk(args.output, expect_llvm=bundle_root is not None)

    print(f"SDK packaged and validated at {args.output}")
    return 0


def default_output_dir() -> Path:
    return REPO_ROOT / "dist" / host_platform_dir()


def host_platform_dir() -> str:
    system = platform.system().lower()
    machine = normalize_machine(platform.machine())
    mapping = {
        ("windows", "x86_64"): "windows-x86_64",
        ("windows", "aarch64"): "windows-aarch64",
        ("linux", "x86_64"): "linux-x86_64",
        ("linux", "aarch64"): "linux-aarch64",
        ("darwin", "x86_64"): "darwin-x86_64",
        ("darwin", "aarch64"): "darwin-aarch64",
    }
    return mapping.get((system, machine), f"{system}-{machine}")


def normalize_machine(machine: str) -> str:
    machine = machine.lower()
    aliases = {
        "amd64": "x86_64",
        "x64": "x86_64",
        "arm64": "aarch64",
    }
    return aliases.get(machine, machine)


def resolve_compiler(explicit: Path | None, verbose: bool) -> Path:
    if explicit is not None:
        compiler = explicit.resolve()
        if not compiler.is_file():
            raise SystemExit(f"compiler `{compiler}` does not exist")
        return compiler

    run(["cargo", "build", "-p", "agam_driver", "--bin", "agamc"], verbose=verbose)
    compiler = REPO_ROOT / "target" / "debug" / COMPILER_NAME
    if not compiler.is_file():
        raise SystemExit(f"expected compiler binary at `{compiler}` after cargo build")
    return compiler


def package_sdk(compiler: Path, output: Path, llvm_bundle: Path | None, verbose: bool) -> None:
    command = [str(compiler), "package", "sdk", "--output", str(output)]
    if llvm_bundle is not None:
        command.extend(["--llvm-bundle", str(llvm_bundle)])
    run(command, verbose=verbose)


class temporary_llvm_bundle:
    def __init__(self, explicit_bundle: Path | None, explicit_driver: Path | None, verbose: bool):
        self.explicit_bundle = explicit_bundle
        self.explicit_driver = explicit_driver
        self.verbose = verbose
        self._tempdir: tempfile.TemporaryDirectory[str] | None = None
        self.path: Path | None = None

    def __enter__(self) -> Path | None:
        if self.explicit_bundle is not None:
            bundle = self.explicit_bundle.resolve()
            if not bundle.exists():
                raise SystemExit(f"LLVM bundle `{bundle}` does not exist")
            self.path = bundle
            return self.path

        driver = discover_llvm_driver(self.explicit_driver)
        if driver is None:
            return None

        self._tempdir = tempfile.TemporaryDirectory(prefix="agam_sdk_bundle_")
        bundle_root = Path(self._tempdir.name)
        bin_dir = bundle_root / host_platform_dir() / "bin"
        bin_dir.mkdir(parents=True, exist_ok=True)

        copied = False
        for candidate in companion_llvm_drivers(driver):
            destination = bin_dir / candidate.name
            shutil.copy2(candidate, destination)
            copied = True
            if self.verbose:
                print(f"staged LLVM driver {candidate} -> {destination}")

        if not copied:
            raise SystemExit(f"failed to seed LLVM bundle from `{driver}`")

        self.path = bundle_root
        return self.path

    def __exit__(self, exc_type, exc, tb) -> None:
        if self._tempdir is not None:
            self._tempdir.cleanup()


def discover_llvm_driver(explicit_driver: Path | None) -> Path | None:
    if explicit_driver is not None:
        driver = explicit_driver.resolve()
        if not driver.is_file():
            raise SystemExit(f"LLVM driver `{driver}` does not exist")
        return driver

    for name in LLVM_DRIVER_NAMES:
        discovered = shutil.which(name)
        if discovered:
            return Path(discovered).resolve()

    if os.name == "nt":
        return discover_visual_studio_llvm_driver()

    return None


def discover_visual_studio_llvm_driver() -> Path | None:
    program_files_x86 = os.environ.get("ProgramFiles(x86)")
    if not program_files_x86:
        return None

    vswhere = Path(program_files_x86) / "Microsoft Visual Studio" / "Installer" / "vswhere.exe"
    if not vswhere.is_file():
        return None

    result = subprocess.run(
        [str(vswhere), "-latest", "-products", "*", "-property", "installationPath"],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    install_path = result.stdout.strip()
    if result.returncode != 0 or not install_path:
        return None

    candidates = [
        Path(install_path) / "VC" / "Tools" / "Llvm" / "x64" / "bin" / "clang.exe",
        Path(install_path) / "VC" / "Tools" / "Llvm" / "bin" / "clang.exe",
        Path(install_path) / "VC" / "Tools" / "Llvm" / "ARM64" / "bin" / "clang.exe",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate.resolve()
    return None


def companion_llvm_drivers(primary_driver: Path) -> list[Path]:
    candidates: list[Path] = []
    for name in LLVM_DRIVER_NAMES:
        candidate = primary_driver.with_name(name)
        if candidate.is_file():
            candidates.append(candidate.resolve())

    if primary_driver.is_file() and primary_driver.resolve() not in candidates:
        candidates.insert(0, primary_driver.resolve())

    deduped: list[Path] = []
    seen: set[Path] = set()
    for candidate in candidates:
        if candidate in seen:
            continue
        seen.add(candidate)
        deduped.append(candidate)
    return deduped


def validate_sdk(output: Path, expect_llvm: bool) -> None:
    manifest_path = output / "sdk-manifest.json"
    if not manifest_path.is_file():
        raise SystemExit(f"expected SDK manifest at `{manifest_path}`")

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    expected_host_platform = host_platform_dir()
    if manifest.get("host_platform") != expected_host_platform:
        raise SystemExit(
            f"manifest host_platform `{manifest.get('host_platform')}` did not match `{expected_host_platform}`"
        )

    compiler_binary = output / manifest["compiler_binary"]
    if not compiler_binary.is_file():
        raise SystemExit(f"packaged compiler `{compiler_binary}` is missing")

    supported_targets = manifest.get("supported_targets", [])
    if not supported_targets:
        raise SystemExit("SDK manifest did not record any supported targets")
    if supported_targets[0].get("name") != "host-native":
        raise SystemExit("first SDK supported target must be `host-native`")

    llvm_bundle_root = manifest.get("llvm_bundle_root")
    preferred_driver = manifest.get("preferred_llvm_driver")
    if expect_llvm:
        if not llvm_bundle_root:
            raise SystemExit("SDK manifest did not record an LLVM bundle root")
        if not preferred_driver:
            raise SystemExit("SDK manifest did not record a preferred LLVM driver")
        if not (output / llvm_bundle_root).is_dir():
            raise SystemExit(f"packaged LLVM bundle `{output / llvm_bundle_root}` is missing")
        if not (output / preferred_driver).is_file():
            raise SystemExit(f"packaged LLVM driver `{output / preferred_driver}` is missing")


def run(command: list[str], verbose: bool) -> None:
    if verbose:
        print("+", " ".join(command))
    subprocess.run(command, cwd=REPO_ROOT, check=True)


if __name__ == "__main__":
    sys.exit(main())
