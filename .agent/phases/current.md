# Current Development

## Primary Program Goal

- Make native LLVM the first-class production backend for Windows, Linux, and Android.

## Active Workstreams

1. **Phase 15D: Value Profiling, Adaptive Admission, and Specialization**
   - Status: partial
   - Done: runtime-side value profiling, specialization guard reporting, and persisted optimize/specialization decisions that now react to measured guard feedback
   - Next work: feed measured payoff and specialization guard evidence into runtime admission and eviction policy
   - Detail file: `details/15D.md`
2. **Phase 15F: Incremental Daemon, Background Prewarm, and Parallel Compilation**
   - Status: not started as a completed slice
   - Next work: keep parsed/typed/lowered state warm across edits and parallelize independent work deterministically
   - Detail file: `details/15F.md`
3. **Phase 15G: First-Party Premium Experience Layer**
   - Status: partial
   - Done: `agamc doctor`, `agamc new`, `agamc dev`, and `agamc cache status`
   - Next work: unify tooling, package/runtime/cache metadata, and workspace conventions behind one stable contract
   - Detail file: `details/15G.md`
4. **Phase 15H: Native LLVM SDK Distribution and Toolchain Bundles**
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
