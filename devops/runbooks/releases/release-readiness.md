# Release Readiness

Use this runbook before cutting a release, publishing SDK assets, or shipping the Python integration package.

## Pre-Flight

- confirm the branch or tag being released
- review the user-facing scope and non-goals
- confirm docs and runbooks are updated for any changed contract
- confirm generated junk is not mixed into the review

## Required Verification

Run the local baseline:

```powershell
just doctor
just ci-local
```

If release scope touches packaging or SDK output:

```powershell
just sdk-package
just sdk-validate
```

If release scope touches Python integration:

- verify `integrations/python` tests and packaging flow
- confirm `.github/workflows/agam-ffi-python.yml` still matches the published package contract

If release scope touches benchmarks or performance claims:

- run the relevant benchmark slice
- update any published narrative that would otherwise become stale or misleading

## Documentation Gate

Update the right surfaces before release:

- `README.md` for public-facing workflow or status changes
- `docs/architecture/project-brief.md` for repo-structure or engineering-contract changes
- `devops/runbooks/` for operational changes
- `info.md` if the document map changed
- `SECURITY.md`, `SUPPORT.md`, `CONTRIBUTING.md`, or `ROADMAP.md` if project-health contracts changed

## Graph/Artifact Hygiene

- keep `graphify-out/GRAPH_REPORT.md` current if code changed materially
- do not ship `graphify-out/cache/` or `graphify-out/graph.json` as source artifacts
- keep smoke binaries and generated outputs under `fixtures/` or outside source control

## Release Checklist

- local verification complete
- CI workflows green or intentionally waived with justification
- docs and runbooks updated
- release notes drafted
- GitHub release-note categories still match the current label taxonomy in `.github/release.yml`
- security-sensitive changes reviewed
- benchmark-backed claims have evidence

## Post-Release

- confirm SDK artifacts and checksums are reachable
- confirm the Python package workflow completed if applicable
- monitor initial bug reports and rollback signals
