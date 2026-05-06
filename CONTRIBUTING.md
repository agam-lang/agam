# Contributing to Agam

Agam is a compiler and language project. That means review quality, reproducibility, and technical
evidence matter more than raw change volume.

## Before You Start

Read the project surfaces in this order:

1. [`README.md`](./README.md)
2. [`docs/architecture/project-brief.md`](./docs/architecture/project-brief.md)
3. [`devops/README.md`](./devops/README.md)
4. [`ROADMAP.md`](./ROADMAP.md)

For compiler or repo-local AI work, also follow [`AGENTS.md`](./AGENTS.md) and [`CLAUDE.md`](./CLAUDE.md).

## Local Setup

The repo now treats the local development environment as a checked-in contract:

- `rust-toolchain.toml`
- `.python-version`
- `.editorconfig`
- `.gitattributes`
- `justfile`

Recommended first commands on a workstation:

```powershell
just vs-status
just doctor
just ci-local
```

If you are working on packaging, benchmarks, or Python integration:

```powershell
just sdk-package
just sdk-validate
just bench-smoke
```

## Contribution Standards

### 1. Keep Scope Tight

- change the smallest responsible crate or document
- avoid mixing structural refactors, feature work, and formatting churn in one change
- keep generated artifacts out of source reviews unless the repo explicitly tracks them

### 2. Preserve Project Contracts

- `devops/` is the canonical automation surface
- root `scripts/` are compatibility shims
- `README.md` is the public overview
- `docs/architecture/` owns engineering design and repo structure
- `devops/runbooks/` owns operational runbooks
- `AGENTS.md` and `CLAUDE.md` are agent workflow entrypoints

### 3. Bring Evidence

For non-trivial changes, include:

- the problem being solved
- the scope and non-goals
- verification commands you ran
- benchmark evidence if you touch performance-sensitive code
- compatibility notes if you change CLI, packaging, SDK, or runtime behavior

### 4. Update the Right Docs

Update docs when you change contracts, not later.

Examples:

- CLI, SDK, toolchain, or packaging changes:
  update `README.md`, `docs/architecture/project-brief.md`, `info.md`, and matching `devops/runbooks/`
- contributor or governance changes:
  update the root governance docs and README links
- cross-cutting compiler/runtime/repo decisions:
  add or update an ADR under `docs/architecture/decisions/`
- agent workflow changes:
  update `.agent/`, `AGENTS.md`, or `CLAUDE.md` as appropriate

## Pull Request Expectations

A strong PR should answer these questions directly:

- What changed?
- Why now?
- What did you verify?
- What risks remain?

Use the PR template and keep the summary concrete. If a change is speculative or exploratory, say so.

Review routing is handled through [`.github/CODEOWNERS`](./.github/CODEOWNERS) where applicable.

## Performance and Compiler Changes

For optimization, backend, or runtime work:

- do not rely on intuition alone
- prefer measured benchmark data
- use the performance regression issue template when you are reporting a measurable slowdown
- preserve diagnostics and source traceability
- avoid changing semantics to chase benchmark numbers

## Working With Generated Output

- stable smoke artifacts belong in `fixtures/`
- temporary output belongs in `scratch/`
- only `graphify-out/GRAPH_REPORT.md` is intended to stay reviewable
- large graph cache/JSON artifacts should be generated on demand

After modifying code files in this repo, refresh the lightweight graph report:

```powershell
graphify update .
```

## How To Propose Larger Changes

Open an issue or draft PR first if the change affects:

- language surface or syntax
- runtime or ABI contracts
- packaging, registry, or SDK distribution behavior
- benchmark methodology or published performance narratives
- repository structure or contributor workflow

For large design changes, prefer an explicit design note or RFC-style write-up over an implementation-first surprise.
When the change sets a cross-cutting project contract, record the final decision under
`docs/architecture/decisions/`.

## License and Contribution Terms

Agam is dual-licensed under MIT or Apache-2.0.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this
repo is understood to be available under the same dual-license terms as the project itself.
