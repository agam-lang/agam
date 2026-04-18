#!/usr/bin/env python3
"""Build, package, and validate a host-native Agam SDK distribution."""

from __future__ import annotations

import argparse
import hashlib
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
LLVM_SYSROOT_ENV = "AGAM_LLVM_SYSROOT"


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
        "--release",
        action="store_true",
        help="build the compiler in release mode for optimized distribution",
    )
    parser.add_argument(
        "--skip-build",
        action="store_true",
        help="skip the cargo build step; requires --compiler to be set",
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
        "--android-sysroot",
        type=Path,
        help="Android sysroot directory to stage as an SDK target pack",
    )
    parser.add_argument(
        "--require-android-target-pack",
        action="store_true",
        help="fail if no Android sysroot target pack can be staged",
    )
    parser.add_argument(
        "--archive-format",
        choices=["none", "auto", "zip", "tar.gz"],
        default="none",
        help="optionally archive the validated SDK output for distribution",
    )
    parser.add_argument(
        "--archive-output",
        type=Path,
        help="explicit archive path; defaults to dist/sdk/agam-sdk-<platform>.<ext>",
    )
    parser.add_argument(
        "--checksum",
        action="store_true",
        help="write a sha256 checksum file next to the generated archive",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="print command execution details",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.skip_build and args.compiler is None:
        raise SystemExit("--skip-build requires --compiler to be set")
    compiler = resolve_compiler(args.compiler, args.release, args.skip_build, args.verbose)
    android_sysroot = discover_android_sysroot(args.android_sysroot)

    args.output = args.output.resolve()
    if args.output.exists():
        shutil.rmtree(args.output)

    with temporary_llvm_bundle(args.llvm_bundle, args.llvm_driver, args.verbose) as bundle_root:
        if args.require_llvm_bundle and bundle_root is None:
            raise SystemExit("no LLVM bundle source was found; pass --llvm-bundle or --llvm-driver")
        if args.require_android_target_pack and android_sysroot is None:
            raise SystemExit(
                "no Android sysroot was found; pass --android-sysroot or set "
                f"{LLVM_SYSROOT_ENV}/ANDROID_NDK_HOME/ANDROID_NDK_ROOT"
            )

        package_sdk(compiler, args.output, bundle_root, android_sysroot, args.verbose)
        validate_sdk(
            args.output,
            expect_llvm=bundle_root is not None,
            expect_android=android_sysroot is not None,
        )
        archive_path = maybe_archive_sdk(
            args.output,
            args.archive_format,
            args.archive_output,
            args.checksum,
            args.verbose,
        )

    print(f"SDK packaged and validated at {args.output}")
    if archive_path is not None:
        print(f"SDK archive written to {archive_path}")
    print_manifest_summary(args.output)
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


def resolve_compiler(
    explicit: Path | None,
    release: bool,
    skip_build: bool,
    verbose: bool,
) -> Path:
    if explicit is not None:
        compiler = explicit.resolve()
        if not compiler.is_file():
            raise SystemExit(f"compiler `{compiler}` does not exist")
        return compiler

    if skip_build:
        raise SystemExit("--skip-build requires --compiler to be set")

    build_cmd = ["cargo", "build", "-p", "agam_driver", "--bin", "agamc"]
    profile_dir = "debug"
    if release:
        build_cmd.append("--release")
        profile_dir = "release"
    run(build_cmd, verbose=verbose)
    compiler = REPO_ROOT / "target" / profile_dir / COMPILER_NAME
    if not compiler.is_file():
        raise SystemExit(f"expected compiler binary at `{compiler}` after cargo build")
    return compiler


def package_sdk(
    compiler: Path,
    output: Path,
    llvm_bundle: Path | None,
    android_sysroot: Path | None,
    verbose: bool,
) -> None:
    command = [str(compiler), "package", "sdk", "--output", str(output)]
    if llvm_bundle is not None:
        command.extend(["--llvm-bundle", str(llvm_bundle)])
    if android_sysroot is not None:
        command.extend(["--android-sysroot", str(android_sysroot)])
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

    # Also discover clang-cl on Windows (useful for MSVC-compatible compilation)
    if os.name == "nt":
        clang_cl = primary_driver.with_name("clang-cl.exe")
        if clang_cl.is_file():
            candidates.append(clang_cl.resolve())

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


def discover_android_sysroot(explicit: Path | None) -> Path | None:
    if explicit is not None:
        sysroot = explicit.resolve()
        if not sysroot.is_dir():
            raise SystemExit(f"Android sysroot `{sysroot}` does not exist")
        return sysroot

    configured = os.environ.get(LLVM_SYSROOT_ENV)
    if configured:
        sysroot = Path(configured).resolve()
        if sysroot.is_dir():
            return sysroot

    for env_name in ("ANDROID_NDK_HOME", "ANDROID_NDK_ROOT"):
        ndk_root = os.environ.get(env_name)
        if not ndk_root:
            continue
        sysroot = (
            Path(ndk_root).resolve()
            / "toolchains"
            / "llvm"
            / "prebuilt"
            / host_platform_dir()
            / "sysroot"
        )
        if sysroot.is_dir():
            return sysroot

    return None


def validate_sdk(output: Path, expect_llvm: bool, expect_android: bool) -> None:
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

    packaged_android_targets = [
        target
        for target in supported_targets
        if "android" in target.get("target_triple", "") and target.get("packaged_sysroot")
    ]
    if expect_android and not packaged_android_targets:
        raise SystemExit("SDK manifest did not record a packaged Android target pack")
    for target in packaged_android_targets:
        sysroot_path = output / target["packaged_sysroot"]
        if not sysroot_path.is_dir():
            raise SystemExit(f"packaged Android sysroot `{sysroot_path}` is missing")
        if not (sysroot_path / "usr").is_dir():
            raise SystemExit(f"packaged Android sysroot `{sysroot_path}` is missing `usr/`")


def maybe_archive_sdk(
    output: Path,
    requested_format: str,
    archive_output: Path | None,
    checksum: bool,
    verbose: bool,
) -> Path | None:
    archive_format = resolve_archive_format(requested_format)
    if archive_format is None:
        return None

    archive_path = create_archive(output, archive_format, archive_output, verbose)
    if checksum:
        checksum_path = write_sha256_file(archive_path)
        if verbose:
            print(f"wrote sha256 checksum {checksum_path}")
    return archive_path


def resolve_archive_format(requested_format: str) -> str | None:
    if requested_format == "none":
        return None
    if requested_format == "auto":
        return "zip" if os.name == "nt" else "tar.gz"
    return requested_format


def create_archive(
    output: Path,
    archive_format: str,
    explicit_output: Path | None,
    verbose: bool,
) -> Path:
    archive_path = resolve_archive_output(output, archive_format, explicit_output)
    if archive_path.exists():
        archive_path.unlink()

    if archive_format == "zip":
        base_name = archive_path.with_suffix("")
        generated = shutil.make_archive(
            str(base_name),
            "zip",
            root_dir=output.parent,
            base_dir=output.name,
        )
    elif archive_format == "tar.gz":
        base_name = archive_path.parent / archive_path.name.removesuffix(".tar.gz")
        generated = shutil.make_archive(
            str(base_name),
            "gztar",
            root_dir=output.parent,
            base_dir=output.name,
        )
    else:
        raise SystemExit(f"unsupported archive format `{archive_format}`")

    generated_path = Path(generated).resolve()
    if verbose:
        print(f"created archive {generated_path}")
    return generated_path


def resolve_archive_output(
    output: Path,
    archive_format: str,
    explicit_output: Path | None,
) -> Path:
    if explicit_output is not None:
        archive_path = explicit_output.resolve()
    else:
        archive_path = default_archive_output(output, archive_format)

    archive_path.parent.mkdir(parents=True, exist_ok=True)
    expected_suffix = archive_suffix(archive_format)
    if not str(archive_path).endswith(expected_suffix):
        raise SystemExit(
            f"archive output `{archive_path}` must end with `{expected_suffix}` for format `{archive_format}`"
        )
    return archive_path


def default_archive_output(output: Path, archive_format: str) -> Path:
    platform_dir = output.name or host_platform_dir()
    return output.parent / f"agam-sdk-{platform_dir}{archive_suffix(archive_format)}"


def archive_suffix(archive_format: str) -> str:
    if archive_format == "zip":
        return ".zip"
    if archive_format == "tar.gz":
        return ".tar.gz"
    raise SystemExit(f"unsupported archive format `{archive_format}`")


def write_sha256_file(path: Path) -> Path:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)

    checksum_path = Path(f"{path}.sha256")
    checksum_path.write_text(
        f"{digest.hexdigest()}  {path.name}\n",
        encoding="utf-8",
    )
    return checksum_path


def run(command: list[str], verbose: bool) -> None:
    if verbose:
        print("+", " ".join(command))
    subprocess.run(command, cwd=REPO_ROOT, check=True)


def print_manifest_summary(output: Path) -> None:
    """Print a human-readable SDK manifest summary for CI log visibility."""
    manifest_path = output / "sdk-manifest.json"
    if not manifest_path.is_file():
        return
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    print("\n=== SDK Manifest Summary ===")
    print(f"  sdk_name:        {manifest.get('sdk_name', '?')}")
    print(f"  host_platform:   {manifest.get('host_platform', '?')}")
    print(f"  compiler_binary: {manifest.get('compiler_binary', '?')}")
    print(f"  llvm_bundle:     {manifest.get('llvm_bundle_root', 'none')}")
    print(f"  llvm_driver:     {manifest.get('preferred_llvm_driver', 'none')}")
    targets = manifest.get("supported_targets", [])
    print(f"  targets:         {len(targets)}")
    for target in targets:
        packaged = target.get("packaged_sysroot", "")
        suffix = f" (packaged: {packaged})" if packaged else ""
        print(f"    - {target.get('name', '?')}: {target.get('target_triple', '?')}{suffix}")
    notes = manifest.get("notes", [])
    if notes:
        print(f"  notes:")
        for note in notes:
            print(f"    - {note}")


if __name__ == "__main__":
    sys.exit(main())
