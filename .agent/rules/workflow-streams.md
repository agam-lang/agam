# Rule: Workflow Streams & Continuous Verification

All AI assistants working on Agam-Lang MUST adhere to the Operational Streams framework:

1. **Stream 1 (Construction):** Write clean, verified code following pipeline discipline (AST → HIR → MIR → Codegen).
2. **Stream 0 (Assurance & Push):** After completing ANY feature or fix, immediately:
   - Run `cargo check --manifest-path agam/Cargo.toml`
   - Run unit/integration tests
   - Verify error diagnostics and SSA invariants
   - Update `task.md` / `walkthrough.md`
   - Execute `git commit` and `git push origin main`
3. **Stream 2 (Frontier Horizon):** On weekends, synthesize industry breakthroughs (e.g. MiniTriton, Kimi K3, Tensor Core lowering) and update master specs.
