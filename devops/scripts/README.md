# DevOps Scripts

Canonical automation entrypoints live here.

Use the root `justfile` first for the normal local workflow. Drop to the scripts here when you need
the underlying platform-specific entrypoints directly.

- `package_sdk.py`
  - package and validate an SDK distribution
- `validate_sdk_e2e.py`
  - local end-to-end SDK pipeline validation
- `vs2026-dev.ps1`
  - Visual Studio Community 2026 bootstrap and validation loop
- `invoke-python.ps1`
  - resolves a usable local Python runtime for packaging and benchmark scripts

Use the root `scripts/` directory only when you need backward-compatible entrypoints for older docs,
muscle memory, or external automation that still points there.
