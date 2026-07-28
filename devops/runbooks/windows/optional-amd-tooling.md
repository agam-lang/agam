# Optional AMD Tooling

Agam does not require AMD workstation utilities or AMD vendor SDK bundles for the default local
developer setup.

## Default Project Contract

- Windows host toolchain: Visual Studio Community 2026 plus the repo's documented LLVM workflow
- Linux and Android support: follow the first-party SDK and toolchain runbooks already in `devops/`
- Benchmarks and performance claims: keep the environment reproducible and documented

## Optional, Not Required

These tools may be useful for local experimentation on AMD hardware, but they are not part of
Agam's default build, test, packaging, or CI contract:

- AMD Zen Software Studio
  - optional Linux/HPC bundle with AOCC, AOCL, uProf, and benchmark payloads
- AMD Ryzen Master
  - optional machine-tuning utility; do not treat tuned results as baseline benchmark evidence
- AMD Ryzen AI Software
  - optional Windows NPU stack for future research; not part of the current Agam toolchain flow

## Project Stance

- Do not make Zen Software Studio, AOCC, AOCL, uProf, Ryzen Master, or Ryzen AI a prerequisite for
  routine Agam development.
- Do not add these tools to CI or release-readiness requirements unless a future phase explicitly
  adopts them.
- If you use optional AMD tooling for local experiments, record that fact next to any benchmark or
  hardware-acceleration claim.
