# Chapter 30: Cross-Compilation, Target Triplets & Target Packs

> **Part VII: Advanced Tooling, Testing & Ecosystem Engineering**  
> **Compiler Module Focus**: [`agam_pkg`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_pkg), [`agam_codegen`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/backends/agam_codegen)

---

## 30.1 Target Triplets & Cross-Compilation

Agam supports compiling native binaries for cross-platform architectures. Targets are identified using **LLVM Target Triplets**:

```text
  x86_64-pc-windows-msvc      (Windows x64 Native)
  x86_64-unknown-linux-gnu    (Linux x64 Native)
  aarch64-linux-android       (Android ARM64 Target)
```

---

## 30.2 Target Packs & SDK Staging (`agamc package sdk`)

`agam_pkg` manages modular **Target Packs** (`Phase 15H`) containing sysroots, pre-compiled runtime static libraries (`libagam_runtime.a`), and LLVM target description files:

```bash
# Building an Android ARM64 target binary from a Windows host machine
agamc build --target aarch64-linux-android main.agam
```

The driver configures LLVM target machine triples, configures cross-linker parameters, and packages output binaries into release-ready `.apk` or `.agpkg` archives.
