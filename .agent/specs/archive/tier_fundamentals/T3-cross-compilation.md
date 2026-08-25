# Phase T3-cross-compilation � Cross-Compilation Matrix

**Status:** open (extends Phase T1-sdk-distribution)
**Tier:** 3 (Platform and Ecosystem Breadth)

## Scope

Enable cross-compilation from any supported host to any supported target without requiring external SDKs. Build on the existing SDK distribution infrastructure from Phase T1-sdk-distribution. Aspire to **Zig-style zero-config cross-compilation**.

## Zero-Config Cross-Compilation Vision

> **Inspired by Zig (2026)**: Zig's built-in cross-compiler is the industry gold standard — zero configuration, any target, drop-in use. Rust developers use `cargo-zigbuild` to leverage this. Agam should aspire to the same: `agamc build --target riscv64-linux` works without any extra toolchain installation.
>
> Key: Bundle target sysroots in SDK packs (`agamc package sdk`), auto-download on first cross-compile.

## Target Matrix

| Target | Priority | Notes |
|--------|----------|-------|
| `x86_64-windows-msvc` | ✅ Shipped | Primary development host |
| `x86_64-linux-gnu` | ✅ Shipped | Primary Linux target |
| `aarch64-linux-gnu` | High | ARM Linux (Raspberry Pi, servers) |
| `aarch64-android` | High | Android NDK (extends 15H) |
| `wasm32-wasip2` | High | WASM Component Model (Phase T3-wasm-backend) |
| `riscv64-linux-gnu` | Medium | RISC-V (Phase T3-riscv-backend) |
| `riscv32-none-eabi` | Medium | RISC-V embedded/bare-metal |
| `aarch64-apple-darwin` | Low | macOS (no validation hardware yet) |
| `aarch64-apple-ios` | Low | iOS (no validation hardware yet) |

## Deliverables

### Core Cross-Compilation
- [ ] `agamc build --target <triple>` cross-compilation from any host
- [ ] Sysroot management with automatic download and caching
- [ ] Target feature detection and CPU-specific codegen flags
- [ ] Bundled cross-compilation toolchains (Zig-style, no external SDKs needed)

### SDK Distribution
- [ ] Android NDK sysroot integration (extends existing 15H work)
- [ ] RISC-V sysroot packaging and validation
- [ ] Exercise hosted-runner SDK builds on real GitHub runners (15H remaining)
- [ ] Validate release-uploaded Windows/Linux SDK artifacts (15H remaining)

### Testing
- [ ] CI matrix testing across all host×target combinations
- [ ] QEMU-based validation for ARM and RISC-V targets
- [ ] Benchmark suite cross-target validation

## Design References

- **Zig cross-compilation**: Zero-config, built-in C/C++ cross-compiler. Industry benchmark for cross-compilation UX.
- **`cargo-zigbuild`**: Rust community tool leveraging Zig's cross-compilation for Rust projects.
- **SDK packs**: Agam's `agamc package sdk` already bundles LLVM; extend to bundle target sysroots.

## Responsible Crates

- `agam_driver` — target selection, sysroot management, auto-download
- `agam_codegen` — target-specific codegen
- `agam_pkg` — SDK packaging for cross targets
- `agam_runtime` — target-specific runtime adaptations
