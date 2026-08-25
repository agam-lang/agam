# Phase T1-sdk-distribution — Native LLVM SDK Distribution and Toolchain Bundles

**Status:** complete
**Tier:** 1 (Developer Experience Excellence)

## Goal

- Ship native LLVM SDK outputs that keep `agamc`, bundled LLVM, target metadata, and first-party readiness checks under one supportable contract.
- Make Windows and Linux SDK distribution real, repeatable, and CI-verifiable before expanding target-pack coverage.

## Responsible Crates

- `agam_driver`
- `agam_pkg`
- `agam_runtime`

## Deliverables

- [x] Add `agamc package sdk` and the first SDK distribution manifest/layout contract.
- [x] Keep bundled LLVM layout and host-toolchain readiness checks wired into first-party tooling.
- [x] Add the CI workflow matrix for Windows and Linux SDK distribution (`.github/workflows/sdk-dist.yml`).
- [x] Emit release-ready SDK archives (`.zip` on Windows, `.tar.gz` on Linux) plus `.sha256` checksum metadata.
- [x] Validate end-to-end SDK builds, packaging, archive extraction, checksum matching, and manifest integrity (`devops/scripts/validate_sdk_e2e.py`).
- [x] Add Android target-pack packaging and validation on top of the host SDK flow (`target-packs/android-arm64/sysroot`).
- [x] Align `agamc doctor` with packaged SDK distribution and target packs.

## Test Results
- E2E local validation passes cleanly (`devops/scripts/validate_sdk_e2e.py`)
- 100% test pass rate across all 27 crates in workspace
- 0 Clippy warnings (`-D warnings`)
- 100% formatting compliance (`cargo fmt --check`)
