# Phase T2-sandboxing � Multi-Tier Sandboxing and Isolation

**Status:** open
**Tier:** 2 (Runtime, Concurrency, Security)
**Pillar:** 4 (Security)

## Vision

Provide **multi-tier isolation** for Agam program execution — from language-level capability permissions through OS sandboxing to hardware-enforced MicroVM isolation. Make Agam the safest language for running untrusted code, AI-generated code, and multi-tenant workloads.

## Motivation

In 2026, the explosion of AI coding agents means programs are increasingly written by machines, not humans. Executing untrusted AI-generated code safely is a critical industry need:
- **Firecracker MicroVMs**: AWS's Rust-based VMM is now the standard for untrusted execution (~125ms startup, <5MB overhead)
- **Deno's permission model**: `--allow-read`, `--allow-net` flags are the UX gold standard
- **WASI capabilities**: Capability-based security at the WASM module level

Agam already has OS-level sandboxing (Phase T2-os-sandbox — Win32 JobObject, Linux prctl/setrlimit). This phase adds language-level and container-level isolation tiers.

## Deliverables

### Language-Level Permissions (Deno-style)
- [ ] `@capability(fs.read, net.connect("api.example.com"))` — declare required permissions per module/package
- [ ] Runtime capability enforcement: unauthorized access → panic with clear diagnostic
- [ ] `agamc run --allow-fs-read --allow-net` CLI permission flags
- [ ] `agamc run --deny-all` strict mode for maximum sandboxing
- [ ] Capability manifest in `agam.toml`: `[permissions]` section for package-level declarations
- [ ] Transitive permission auditing: `agamc audit` shows what permissions dependencies require

### Container/MicroVM Isolation
- [ ] `agamc run --isolate=process` — OS-level sandbox (extends Phase 21)
- [ ] `agamc run --isolate=container` — OCI container isolation
- [ ] `agamc run --isolate=microvm` — Firecracker MicroVM for untrusted code
- [ ] `agamc exec --isolate=microvm` — agent execution with hardware isolation
- [ ] Automatic isolation level selection based on trust context

### WASM Capability Bridge
- [ ] WASM Component Model capability mapping (coordinates with Phase T3-wasm-backend)
- [ ] WIT-defined capability interfaces for cross-component security
- [ ] Capability attenuation: host can filter/restrict module capabilities

### Audit and Compliance
- [ ] `agamc audit` — report all capabilities used by workspace and dependencies
- [ ] Capability diff on dependency updates (alert on new permission requests)
- [ ] SBOM (Software Bill of Materials) generation with capability annotations

## Design References

- **Deno**: Secure-by-default with explicit permission flags. Industry-leading UX.
- **WASI capabilities**: No ambient authority, explicit resource grants, unforgeable handles.
- **Firecracker**: Hardware-enforced isolation with near-container performance.
- **Agam Phase 21**: OS-level JobObject/prctl sandboxing already implemented.

## Responsible Crates

- `agam_runtime` — capability enforcement, isolation mode selection
- `agam_sema` — `@capability` annotation validation
- `agam_driver` — `--allow-*`, `--deny-*`, `--isolate` CLI flags
- `agam_pkg` — capability manifest parsing, dependency auditing

## Dependencies

- Phase T2-os-sandbox (Runtime Hardening) — extends existing OS-level sandbox (complete ✅)
- Phase T2-cybersecurity (Cybersecurity) — broader security framework
- Phase T3-wasm-backend (WASM) — WASI capability bridge
- Phase T1-supply-chain-sec (Supply Chain Fortress) — SBOM generation
