# CI/CD Map

GitHub Actions definitions must stay in `.github/workflows/`, but this document is the operational
index for what each workflow owns.

## Workflow Inventory

- `.github/workflows/ci.yml`
  - shared Rust CI entrypoint from the organization workflow repo
- `.github/workflows/benchmarks.yml`
  - shared benchmark workflow entrypoint
- `.github/workflows/sdk-dist.yml`
  - release-grade SDK packaging and artifact revalidation
  - canonical script dependency: `devops/scripts/package_sdk.py`
- `.github/workflows/agam-ffi-python.yml`
  - Python package build and publish path for `integrations/python`
- `.github/workflows/publish-ghcr.yml`
  - container publication flow
- `.github/workflows/add-to-project.yml`
  - project automation
- `.github/workflows/auto-assign.yml`
  - triage automation

## DevOps Rules

- Keep business logic in repo scripts, not inline workflow shell blocks, unless the logic is trivial.
- Prefer one canonical script path per operational concern.
- Treat root `scripts/` entries as compatibility shims, not the long-term source of truth.
- When a workflow changes the release or packaging contract, update the matching runbook under
  `devops/runbooks/`.
