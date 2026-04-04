# Current Development

## Primary Program Goal

- Make native LLVM the first-class production backend for Windows, Linux, and Android.

## Active Workstreams

1. **Phase 15F: Incremental Daemon, Background Prewarm, and Parallel Compilation**
   - Status: partial
   - Done: shared workspace snapshot and invalidation-diff contract in `agam_pkg`, a real foreground warm-state daemon/status loop in `agam_driver`, intra-invocation warm-state reuse for `agamc dev`, a semantic-only `agamc dev --no-run` entry path that skips unnecessary lowering, deterministic multi-input `agamc build` request planning that stops dropping every file after the first one, deterministic multi-input `agamc build` worker scheduling that runs independent single-file builds in parallel while replaying diagnostics in request order, daemon-side entry-file prewarm that fills package/build caches from warm MIR without polluting the workspace, and cross-process reuse of daemon-prewarmed entry packages in `agamc build`, `agamc run`, and `agamc package pack`
   - Next work: connect build/run/dev flows to daemon-owned warm state and parallelize independent work deterministically
   - Detail file: `details/15F.md`
2. **Phase 15G: First-Party Premium Experience Layer**
   - Status: partial
   - Done: `agamc doctor`, `agamc new`, `agamc dev`, `agamc cache status`, workspace-aware `agamc check` input expansion, shared entry-file resolution for workspace-root build/run/package commands, and a broadened benchmark workspace with 38 Agam workloads plus dedicated call-cache locality/miss study cases and a result-root README that explains the checked-in measured snapshot versus later dry-run validation artifacts
   - Next work: unify tooling, package/runtime/cache metadata, and workspace conventions behind one stable contract
   - Detail file: `details/15G.md`
3. **Phase 15H: Native LLVM SDK Distribution and Toolchain Bundles**
   - Status: partial
   - Done: `agamc package sdk`, bundled LLVM layout, initial CI workflow skeleton
   - Next work: validate hosted-runner SDK builds, publish real Windows/Linux SDK artifacts, and add Android target-pack validation
   - Detail file: `details/15H.md`

## Current Decision Rules

- Prefer native host LLVM over WSL fallback.
- Keep `agamc doctor` and SDK packaging aligned with the real readiness contract.
- Treat VS Community 2026 as the canonical Windows-side toolchain inventory.
- Do not claim macOS or iOS backend support beyond planning and toolchain prep until native validation hardware exists.
- If a phase decision needs more than this summary, open `details/`.
