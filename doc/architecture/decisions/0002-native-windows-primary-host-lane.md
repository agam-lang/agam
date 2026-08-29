# ADR 0002: Native Windows Is the Primary Host Development Lane

- Status: accepted
- Date: 2026-04-24
- Supersedes:
- Superseded by:

## Context

Agam's shipped backend story is centered on native LLVM support for Windows, Linux, and Android.
Historically, WSL could become an accidental crutch for host setup and local verification, which
blurs the actual product contract and weakens the Windows toolchain story.

## Decision

Agam treats native Windows as the primary day-to-day host development lane:

- Visual Studio Community 2026 is the canonical Windows host inventory
- repo setup and validation flow through `.vsconfig`, `tasks.vs.json`, `launch.vs.json`, and
  `devops/scripts/vs2026-dev.ps1`
- WSL remains a development and verification fallback, not the default host contract
- Linux remains an active runtime target and CI verification lane

## Consequences

- public docs and runbooks should describe Windows-native setup first
- backend and SDK work should be validated without assuming WSL is present
- Linux and Android support remain first-class product targets, but the workstation story is no
  longer allowed to collapse into "just use WSL"
- claims about supported host tooling should stay aligned with the Visual Studio and LLVM contract

## Notes

See `README.md`, `devops/runbooks/windows/visual-studio-2026.md`, `.vsconfig`, and
`devops/scripts/vs2026-dev.ps1`.
