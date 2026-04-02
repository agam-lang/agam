# Package Ecosystem

This file is the canonical package and environment architecture direction for Agam.

## Design Goals

- Keep Agam's package story coherent instead of mixing source dependencies, portable runtime packages, SDK packs, and local environments into one format.
- Make reproducibility the default through stable manifests, lockfiles, content-addressable metadata, and explicit environment selection.
- Keep the base install small while still supporting curated first-party distributions and strong interoperability.
- Make official package governance clear without forcing every package into one repository layout.

## Layered Model

Agam should ship as a layered product, not as one giant package manager that tries to own every concern.

1. `agamc` core
   - compiler, formatter, test runner, doctor, build/run, cache inspection
2. portable runtime packages
   - verified MIR plus runtime metadata for cross-host execution
   - current artifact shape: `.agpkg.json`
3. SDK and target packs
   - LLVM bundles, sysroots, target-pack metadata, and host-toolchain integration
   - current distribution manifest shape: `sdk-manifest.json`
4. source packages
   - dependency metadata, versioning, resolution, publish/install/update flows
   - primary project manifest: `agam.toml`
   - primary lockfile: `agam.lock`
5. environments
   - named, reproducible selections of compiler version, SDK packs, target packs, dependency graph, and backend intent

If these layers are blurred, the ecosystem becomes hard to reason about and hard to support.

## Artifact Taxonomy

### `agam.toml`

- Defines the source package or workspace contract.
- Owns package identity, version, workspace members, dependency declarations, target intent, and toolchain expectations.
- Must stay tool-agnostic enough that CLI, formatter, LSP, tests, and the future daemon share one contract.

### `agam.lock`

- Defines the fully resolved dependency graph for a workspace.
- Must record enough metadata to make dependency reuse, verification, and cache addressing deterministic.
- Should be generated from explicit resolution, not hand-edited.

### `.agpkg.json`

- Defines a portable runtime package artifact.
- Carries verified MIR plus runtime metadata for execution.
- Is not the same thing as a source package release.

### `sdk-manifest.json`

- Defines a host-native SDK or target-pack distribution.
- Tracks LLVM bundle layout, supported targets, and toolchain expectations.
- Is not the same thing as a source dependency manifest.

### local environments

- Bind compiler version, SDK packs, target packs, backend choice, and dependency locks into named project-level environments.
- Must stay explicit and reproducible rather than relying on shell-global mutation.

## CLI Direction

Keep CLI surfaces separated by contract.

- `agamc package ...`
  - keep this for portable runtime packages and SDK distribution workflows
- `agamc deps ...`
  - add, remove, inspect, resolve, and update source dependencies
- `agamc env ...`
  - create, list, select, inspect, and diagnose named environments
- `agamc publish ...`
  - validate and publish source package releases to a registry

Do not overload `agamc package` so heavily that users cannot tell whether they are dealing with source dependencies, runtime artifacts, or SDK packs.

## Registry Model

- Use one thin central index repository for package metadata and release discovery.
- Package identity should be registry-based, not repository-name-based.
- Official packages may live in:
  - dedicated single-package repositories
  - monorepos
  - organization-managed infrastructure repos
- Publication should produce immutable release metadata plus checksums and provenance data.
- Private registries and mirrors should be possible later without redesigning the package identity model.

Recommended organization shape:

- `agam-lang/registry-index`
- `agam-lang/sdk-packs`
- `agam-lang/official-*` or `agam-lang/std-*`
- `agam-lang/interop-*`

Community packages should be able to live in any repository and still publish cleanly into the registry.

## Environment Model

Agam environments should be Agam-native, not a clone of Conda.

- An environment should pin:
  - compiler version
  - host SDK pack
  - target pack or target triple
  - runtime/backend preference
  - resolved dependency graph
- Environments should be named and project-local.
- Environments should support common profiles such as:
  - `dev`
  - `test`
  - `release`
  - `android-arm64`
  - `data-ai`

Agam should interoperate with foreign toolchains and libraries, but it should not absorb foreign package managers into the base environment contract.

## First-Party Distribution Direction

Agam should offer curated first-party profiles instead of one universal installer.

- base language support
- systems/native interop stack
- data/AI stack
- game/graphics stack

These profiles should resolve through the same registry and environment model as everything else.
They should not bypass the package manager with hidden bundle logic.

## Anti-Goals

- Do not merge source package manifests with portable runtime package manifests.
- Do not merge SDK distribution metadata with dependency metadata.
- Do not force one repository per package.
- Do not make global mutable environments the default.
- Do not try to replace `pip`, Cargo, Maven, or system package managers inside Agam's base contract.

## Priority Order

1. Phase 17A: stable `agam.toml` source-package and workspace contract
2. Phase 17B: deterministic resolver and `agam.lock`
3. Phase 17C: registry-index protocol and publish/install contract
4. Phase 17D: named environments and SDK linking
5. Phase 17E: curated first-party base distributions and official package governance
6. Phase 17F: standard-library growth on top of the package ecosystem

The package ecosystem should be built in that order.
