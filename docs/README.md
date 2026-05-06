# Docs Map

This directory owns human-facing repo documentation that is not part of the `.agent/` program board
or the `devops/` operational runbooks.

## Ownership

- `README.md`
  - public overview, usage, current capabilities, and benchmark narrative
- `docs/architecture/`
  - engineering structure, repository layout, and compiler/platform briefings
- `docs/architecture/decisions/`
  - architecture decision records for cross-cutting technical contracts
- `docs/benchmarks/`
  - checked-in benchmark image assets referenced by the main README
- `devops/runbooks/`
  - operational runbooks, platform setup, release validation, and toolchain setup
- `info.md`
  - short index into the canonical docs above
- `rust-toolchain.toml`, `.python-version`, `.editorconfig`, `.gitattributes`
  - machine-readable development-environment contract at the repo root
- root governance docs
  - `CONTRIBUTING.md`, `ROADMAP.md`, `GOVERNANCE.md`, `SECURITY.md`, `SUPPORT.md`, and `CODE_OF_CONDUCT.md`

When architecture or repo structure changes, update `docs/architecture/` first and keep `README.md`
and `info.md` aligned at the summary level.
