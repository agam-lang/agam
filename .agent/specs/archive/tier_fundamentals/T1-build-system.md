# Phase T1-build-system � Unified Build System

**Status:** partial (agamc already exists as unified CLI, needs completion)
**Tier:** 1 (Developer Experience Excellence)
**Pillar:** 18 — The "One Tool" Pillar

## Vision

**The compiler IS the package manager IS the test runner IS the formatter.** One binary (`agamc`), one tool. No `pip`, no `conda`, no `venv`, no separate `pytest`. Every development task is a subcommand of `agamc`.

## Current State

Agam already has a unified CLI. What's missing is completeness and polish:

| Command | Status | Purpose |
|---------|--------|---------|
| `agamc build` | ✅ | Compile project |
| `agamc run` | ✅ | Build and execute |
| `agamc check` | ✅ | Type-check without codegen |
| `agamc test` | Partial | Run tests |
| `agamc fmt` | Partial | Format code |
| `agamc lint` | Stub | Lint rules |
| `agamc doc` | Stub | Generate docs |
| `agamc repl` | ✅ | Interactive REPL |
| `agamc new` | ✅ | Create new project |
| `agamc add <pkg>` | ❌ | Add dependency |
| `agamc remove <pkg>` | ❌ | Remove dependency |
| `agamc update` | ❌ | Update dependencies |
| `agamc publish` | ❌ | Publish to registry |
| `agamc install` | ❌ | Install global tool |
| `agamc bench` | ❌ | Run benchmarks |
| `agamc doctor` | ✅ | Environment diagnostics |
| `agamc dev` | ✅ | Watch mode with hot-reload |
| `agamc exec` | ✅ | Headless execution |
| `agamc daemon` | ✅ | Background compilation daemon |
| `agamc cache` | ✅ | Cache management |
| `agamc env` | Partial | Environment/toolchain management |
| `agamc audit` | ❌ | Security audit |
| `agamc sbom` | ❌ | Software bill of materials |
| `agamc preview` | ❌ | GUI live preview |
| `agamc architect` | ❌ | Visual UI builder |

## Deliverables

### Package Management (built into agamc)
- [ ] `agamc add <package>` — add dependency, auto-update `agam.toml` and `agam.lock`
- [ ] `agamc add <package> --dev` — add dev-only dependency
- [ ] `agamc remove <package>` — remove dependency and clean lockfile
- [ ] `agamc update` — update all dependencies within semver constraints
- [ ] `agamc update <package>` — update specific dependency
- [ ] `agamc publish` — publish package to registry
- [ ] `agamc install <package>` — install as global CLI tool
- [ ] `agamc search <query>` — search package registry

### Environment Management
- [ ] `agamc env list` — list installed toolchain versions
- [ ] `agamc env install <version>` — install specific Agam version
- [ ] `agamc env use <version>` — set project toolchain version
- [ ] Toolchain pinning in `agam.toml`: `[toolchain] version = "0.2.0"`
- [ ] Auto-download correct toolchain version on `agamc build`

### Zero External Dependencies
- [ ] Single self-contained binary — no Python, no Node, no Ruby required
- [ ] Bundled LLVM (already done via SDK packaging)
- [ ] No `PATH` manipulation needed beyond adding `agamc` itself
- [ ] `agamc doctor` validates everything is correctly set up

## Responsible Crates

- `agam_driver` — all subcommands
- `agam_pkg` — package resolution, registry client

## Dependencies

- Phase T3-pkg-manager-maturity (package manager) — registry protocol
- Phase T1-testing-framework (testing) — `agamc test` completion
