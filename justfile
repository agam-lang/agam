set shell := ["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

default:
    just --list

doctor:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\devops\scripts\vs2026-dev.ps1 -Task doctor

vs-status:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\devops\scripts\vs2026-dev.ps1 -Task status

vs-install:
    powershell.exe -ExecutionPolicy Bypass -File .\devops\scripts\vs2026-dev.ps1 -Task install

check:
    cargo check --manifest-path Cargo.toml

clippy:
    cargo clippy --manifest-path Cargo.toml --workspace --all-targets -- -D warnings

test:
    cargo test --manifest-path Cargo.toml

fmt-check:
    cargo fmt --manifest-path Cargo.toml -- --check

build-driver:
    cargo build --manifest-path Cargo.toml -p agam_driver

llvm-smoke:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\devops\scripts\vs2026-dev.ps1 -Task llvm-smoke

sdk-package:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\devops\scripts\invoke-python.ps1 devops/scripts/package_sdk.py --require-llvm-bundle

sdk-validate:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\devops\scripts\invoke-python.ps1 devops/scripts/validate_sdk_e2e.py

bench-smoke:
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\devops\scripts\invoke-python.ps1 -m benchmarks.infrastructure.benchmark_harness --suite 01_algorithms --max-benchmarks 1

ci-local: check clippy test fmt-check

graph-update:
    graphify update .
