# Next Implementation Order

Use this as the default answer to "what should Agam agents build next?"

## Recommended Order

1. **Start Phase 15F**
   - Add a persistent daemon that keeps parsed, typed, and lowered state warm
   - Add deterministic parallel compilation and background prewarm
   - Detail file: `details/15F.md`
2. **Finish Phase 15G**
   - Unify workspace conventions, package/runtime/cache metadata, and tool contracts
   - Reduce per-command discovery drift across tooling
   - Detail file: `details/15G.md`
3. **Finish Phase 15H**
   - Validate real Windows/Linux hosted-runner SDK outputs with bundled LLVM
   - Add Android target-pack packaging and validation
   - Detail file: `details/15H.md`
4. **Finish Phase 17A**
   - Reuse the parsed manifest contract across formatter, tests, LSP, and later daemon surfaces
   - Strengthen diagnostics for malformed dependency, environment, and workspace-member metadata
   - Detail file: `details/17A.md`
5. **Start Phase 17B**
   - Land deterministic dependency resolution and `agam.lock`
   - Make reproducibility and content addressing the default instead of an optional add-on
   - Detail file: `details/17B.md`
6. **Start Phase 17C**
   - Define the thin central registry-index protocol and immutable publish contract
   - Keep package identity registry-based rather than repo-name-based
   - Detail file: `details/17C.md`
7. **Start Phase 17D**
   - Build named Agam environments that pin compiler, SDK packs, target packs, and dependencies together
   - Make environment selection explicit and reproducible instead of shell-global
   - Detail file: `details/17D.md`
8. **Start Phase 17E**
   - Ship curated first-party base distributions and official package governance
   - Keep foreign-language interop as layered packs, not the base package manager contract
   - Detail file: `details/17E.md`
9. **Start Phase 17F**
   - Expand `agam_std` and native I/O once the package and environment layers stop drifting
   - Keep standard library growth aligned with the new package ecosystem instead of bypassing it
   - Detail file: `details/17F.md`

## What Not To Prioritize First

- macOS/iOS backend bring-up beyond planning and driver hooks
- broad new language-surface expansion that distracts from the native LLVM product path
- REPL, agent-wrapper, and model-facing ecosystem phases ahead of the package, lockfile, registry, and environment foundations from 17A through 17D
- WSL-only shortcuts that weaken the real host-toolchain story
