# Phase T1-plugin-system � Plugin and Extension System

**Status:** open
**Tier:** 3 (Platform and Ecosystem Breadth)
**Pillar:** 21 — The Zero-Configuration Interop Pillar

## Vision

Drop a `.c` file, a `.py` script, or a `.rs` crate into your Agam project, and the build system **automatically detects, compiles, and links it** — zero CMake, zero Gradle, zero configuration. Foreign code integration should be as effortless as importing an Agam module.

## Deliverables

### Auto-Detection Build System
- [ ] `agamc build` scans project for foreign source files:
  - `.c` / `.h` → compile with bundled C compiler (zig cc or clang)
  - `.cpp` / `.hpp` → compile with bundled C++ compiler
  - `.rs` → compile with cargo (if installed), link as static library
  - `.py` → provision isolated Python environment, generate FFI bridge
  - `.js` → bundle for WASM target, generate interop glue
- [ ] Foreign files placed in `foreign/` directory are auto-discovered
- [ ] No `build.rs` or `CMakeLists.txt` needed for simple cases

### Isolated Build Environments
- [ ] Each foreign language gets its own isolated build environment
- [ ] Python: auto-create virtualenv, install requirements from `requirements.txt`
- [ ] C/C++: use bundled compiler, no system dependency on MSVC/GCC
- [ ] Foreign build artifacts cached in `.agam_cache/foreign/`

### Automatic FFI Bridge Generation
- [ ] C header → Agam bindings generated at build time (extends FFI1)
- [ ] Python module → typed Agam wrapper generated at build time
- [ ] `agam.toml` configuration for complex cases:
  ```toml
  [foreign.mylib]
  type = "c"
  sources = ["lib/*.c"]
  include = ["include/"]
  cflags = ["-O2"]
  ```

### Pre-built Binary Integration
- [ ] `agamc add --binary <url>` — link pre-compiled shared/static library
- [ ] System library detection: `agamc build` auto-detects `libssl`, `libcurl`, etc.
- [ ] `pkg-config` integration on Linux, `vcpkg` on Windows

## Responsible Crates

- `agam_driver` — foreign source detection, build orchestration
- `agam_ffi` — automatic binding generation
- `agam_pkg` — foreign dependency management

## Dependencies

- Phase T3-universal-ffi (universal FFI) — binding generation infrastructure
- Phase T1-build-system (one tool) — unified CLI integration
