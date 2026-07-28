# Phase T3-wasm-backend � WASM Backend (Component Model)

**Status:** open
**Tier:** 3 (Platform and Ecosystem Breadth)

## Scope

Add WebAssembly as a first-class compilation target, targeting the **WASI Component Model** (WASI 0.2+) for modular, cross-language, sandboxed execution. This enables browser-based execution (web playground), edge computing, secure plugin architectures, and universal cross-language interop.

## WASM Component Model Strategy

> **Updated for 2026**: WASI 0.2 (Component Model) is stable. WASI 0.3 adds native async. WASI 1.0 expected late 2026. Agam should target the **Component Model** from the start, not just flat WASM output.
>
> Components enable: typed cross-language interfaces (WIT), capability-based security, modular composition, and component registries — all aligning with Agam's pillar goals.

## Deliverables

### WASM Codegen
- [ ] `wasm32-wasip2` target in `agam_codegen` (WASI 0.2 Component Model)
- [ ] `wasm32-unknown-unknown` for browser embedding (MVP)
- [ ] `agamc build --target wasm32-wasip2` integration
- [ ] WASM-compatible subset of runtime (no OS-level syscalls in browser)
- [ ] Linear memory management for WASM target

### WASI Component Model
- [ ] WIT (WebAssembly Interface Types) generation from Agam module interfaces
- [ ] Component composition: Agam components can import/export typed interfaces
- [ ] WASI 0.2 world targeting: `wasi-cli`, `wasi-http`, `wasi-filesystem`
- [ ] Capability-based security: components declare required permissions
- [ ] Server-side WASM execution (Wasmtime, Wasmer compatibility)
- [ ] `agamc run --target wasm32-wasip2` via bundled WASM runtime

### WASI 0.3 Async (stretch — depends on R2)
- [ ] Native `Future` and `Stream` types across component boundaries
- [ ] Non-blocking I/O in WASM components
- [ ] Async component interfaces for server-side use

### Browser Integration
- [ ] JavaScript interop layer for calling JS from Agam and vice versa
- [ ] DOM access utilities (stretch — may be library-level)
- [ ] WASM binary size optimization (`wasm-opt` integration)

### Size Optimization
- [ ] Dead code elimination for WASM output
- [ ] `--release-size` profile for minimal WASM binaries
- [ ] Tree-shaking at the function level

## Design References

- **WASI Component Model (2026)**: Industry standard for modular, sandboxed WASM execution. Typed interfaces via WIT, capability-based security.
- **Rust `wasm32-wasip2` target**: Tier 2 support in rustc. Direct compilation to WASI 0.2 components.
- **Component Registries**: Emerging package managers for WASM components. Agam's PKG5 (Decentralized Registry) could integrate.
- **Edge/Serverless**: SpinKube, wasmCloud, Cloudflare Workers — all consume WASM components.

## Responsible Crates

- `agam_codegen` — WASM target codegen, WIT generation
- `agam_runtime` — WASM-compatible runtime subset
- `agam_driver` — `--target wasm32-*` flag handling
- `agam_pkg` — component manifest and interface declarations

## Dependencies

- Phase T0-type-system/F3 (type system, object model) — WASM output needs stable type layouts
- Phase T0-module-system (module system) — module interfaces map to WIT exports
- Phase T2-async-concurrency (async) — needed for WASI 0.3 async components
- LLVM WASM backend (already mature in LLVM)
