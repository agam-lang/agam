# Next Implementation Order

Use this as the default answer to "what should Agam agents build next?"

## Recommended Order

1. **Finish Phase 15H**
   - Exercise the hosted-runner Windows/Linux SDK flow on GitHub with bundled LLVM and the new post-download archive validation
   - Confirm one end-to-end release publication and Android target-pack packaging path on hosted infrastructure
   - Detail file: `details/15H.md`
2. **Continue Phase 18**
   - Extend the execution-policy contract beyond source/arg limits and native-backend gating
   - Add stronger OS-level isolation for filesystem, network, process, and runtime resource usage
   - Detail file: `details/18.md`
3. **Continue Phase 19**
   - Validate the packaged LangChain/LlamaIndex adapter hooks against live framework releases
   - Publish the optional-extras integration story beyond the repo-local test doubles
   - Detail file: `details/19.md`
4. **Continue Phase 17F**
   - Build on the new `agam_std::io` file/path helpers and move native I/O toward an effects-aware language contract
   - Keep standard-library growth aligned with the effects model and official package governance
   - Detail file: `details/17F.md`

## What Not To Prioritize First

- macOS/iOS backend bring-up beyond planning and driver hooks
- broad new language-surface expansion that distracts from the native LLVM product path
- long-horizon model-training phases ahead of the hosted SDK proof, execution sandbox hardening, and wrapper validation now in 15H/18/19
- WSL-only shortcuts that weaken the real host-toolchain story
