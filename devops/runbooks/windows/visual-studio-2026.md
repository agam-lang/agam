# Visual Studio Community 2026 Setup

Agam treats Visual Studio Community 2026 as the canonical Windows-side host toolchain inventory for
native LLVM work. This repo now ships the Visual Studio-side setup artifacts directly:

- `.vsconfig`
  - curated workload and component selection for Agam Windows development
- `tasks.vs.json`
  - Open Folder build/test/check tasks for the Rust workspace and `.agam` files
- `launch.vs.json`
  - starter debug targets for `agamc`
- `devops/scripts/vs2026-dev.ps1`
  - imports the VS developer environment, exposes the VS LLVM toolchain, and runs Agam commands

## What `.vsconfig` installs

The repo config pins the Visual Studio surfaces Agam currently cares about:

- Desktop C++ with MSVC, Windows 11 SDK 26100, Clang, CMake, AddressSanitizer, Build Insights, and vcpkg
- Linux/Mac C++ cross-workflow support for remote and CMake-oriented validation paths
- Python IDE tooling for the benchmark and packaging scripts
- Visual Studio extension tooling for first-party editor integration work
- Game tooling relevant to `agam_game`, HLSL, Unreal-side workflows, and future GPU work
- Incredibuild integration as an optional acceleration layer for Visual Studio-native C/C++/game builds
- Android NDK tooling for the active Android sysroot packaging direction

The config intentionally does not pin an old CPython runtime from the Visual Studio catalog. Install a
current Python separately if `python` is not already available on the machine. The repo's local
baseline is now pinned in `.python-version` and currently expects Python `3.12`. The `justfile`
uses `devops/scripts/invoke-python.ps1` so the common local tasks can resolve that baseline through
either `python`, the `py` launcher, or a local uv-managed interpreter.

## Install or update the local VS instance

From the `agam` repo root:

```powershell
powershell.exe -ExecutionPolicy Bypass -File .\devops\scripts\vs2026-dev.ps1 -Task install
```

That runs the Visual Studio Installer `modify --config` flow against the detected Visual Studio 2026
instance. If you prefer the GUI, import `.vsconfig` from the Visual Studio Installer. The legacy
`scripts\vs2026-dev.ps1` path remains as a compatibility shim.

## Validate the Windows-side toolchain

```powershell
powershell.exe -ExecutionPolicy Bypass -File .\devops\scripts\vs2026-dev.ps1 -Task status
powershell.exe -ExecutionPolicy Bypass -File .\devops\scripts\vs2026-dev.ps1 -Task doctor
```

`status` checks the repo config against the installed VS instance, imports `vcvars64.bat`, exposes the
VS-hosted LLVM tools on `PATH`, and reports whether Python and Incredibuild are actually reachable.
`doctor` runs the same setup and then delegates to `agamc doctor`.

For the normal human-facing inner loop, prefer the root `justfile` verbs after the toolchain is
installed:

- `just vs-status`
- `just doctor`
- `just ci-local`
- `just sdk-package`
- `just sdk-validate`

## Visual Studio Open Folder workflow

Open the `agam` folder in Visual Studio, not the organization root.

After that:

- right-click [Cargo.toml](/C:/Users/ksvik/IdeaProjects/Agam-Lang/agam/Cargo.toml) for Build, Rebuild, Clean, `cargo check`, `cargo test`, `cargo fmt --check`, `doctor`, and LLVM smoke tasks
- right-click any `.agam` file for build and run tasks
- use the debug target dropdown from [launch.vs.json](/C:/Users/ksvik/IdeaProjects/Agam-Lang/agam/launch.vs.json) to launch `agamc doctor`, `agamc repl`, or the LLVM smoke build

## Incredibuild scope

Incredibuild is included because it integrates directly with Visual Studio and is relevant for Agam's
Windows-side C/C++, shader, Android, and future game-tooling surfaces.

Be precise about the benefit:

- it helps Visual Studio-native solution, project, C++, Android NDK, and game-oriented build graphs
- it does not automatically accelerate plain Cargo Rust compilation in the same way

So treat Incredibuild as part of the Windows host-toolchain inventory, not as a magic speedup switch
for every Rust command in the workspace.
