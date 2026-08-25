# Backup Preparation and Git Push

Clean up build directories, compiler caches, and SDK packages to minimize backup size before laptop reset, and commit/push uncommitted workspace changes to GitHub.

## User Review Required

> [!IMPORTANT]
> - There are substantial uncommitted changes (2100+ lines changed) in the `agam` compiler sub-repository and workspace configurations. We will commit these under an automated backup commit message (e.g., "backup: WIP changes before laptop reset") and push to remote.
> - The parent repo and `agam` sub-repository will both be pushed to GitHub to guarantee no code is lost.
> - We will clean up `target`, `dist`, `build`, and `.agam_cache` directories. This will reduce the workspace backup size from **~10 GB down to ~350 MB** (96% space savings), removing only generated files that can be rebuilt later.

## Open Questions

> [!NOTE]
> Are you comfortable with us committing all current modifications in `agam` and the parent repository under a WIP/backup branch or master/main branch directly? (We will commit directly to the current branches, `main` for `agam` and `master` for parent, unless you request otherwise).

---

## Proposed Changes

### Build Artifact Cleanup

#### [DELETE] [agam/target](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/target)
- Cargo build artifacts (~9.4 GB). Removed via `cargo clean`.

#### [DELETE] [agam/dist](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/dist)
- Packaged LLVM SDK bundle (~295 MB). Can be re-built via `package_sdk.py`.

#### [DELETE] [benchmarks/build](file:///c:/Users/ksvik/Projects/Agam-Lang/benchmarks/build)
- Benchmark compilation PDBs/binaries (~157 MB).

#### [DELETE] [agam/.agam_cache](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/.agam_cache)
- Compiler daemon caches (~1.5 MB).

#### [DELETE] [agam/graphify-out](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/graphify-out)
- Generated graph output files (~19.7 MB).

---

### Workspace Git Changes (Parent & Subrepos)

#### [MODIFY] [agam files](file:///c:/Users/ksvik/Projects/Agam-Lang/agam)
- Commit and push modified files in `agam` compiler crate (AST, Semantics, JIT, Codegen).

#### [MODIFY] [parent files](file:///c:/Users/ksvik/Projects/Agam-Lang)
- Commit and push active parent repository specifications, indexes, and checklists.

---

## Verification Plan

### Automated Tests
- Run `cargo check` and `cargo test` in the `agam` crate (already verified and passed).
- Verify directory size after cleanup to ensure size is below 500 MB.

### Manual Verification
- Check `git status` in parent and `agam` repositories to verify they are clean.
- Verify `git push` was successful.
