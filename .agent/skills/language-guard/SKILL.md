---
name: language-guard
description: Use when a task touches Agam source syntax, parser behavior, examples, benchmarks, or when there is risk of treating `.agam` like Python or Rust instead of Agam's own language.
---

# Language Guard

Use this skill whenever syntax, parser, examples, or code-generation expectations could drift toward Python or Rust assumptions.

## Workflow

1. Read `.agent/rules/language-guardrails.md`.
2. Inspect the closest real `.agam` examples under `benchmarks/benchmarks/`, `.agent/test/`, and `examples/`.
3. Base syntax decisions on repo reality, not on generic language priors.
4. Keep ML, tensor, dataframe, and effect features grounded in Agam's own compiler/runtime model.
5. If docs or examples drift away from real syntax, fix them in the same change.

## Best Sources

- `benchmarks/benchmarks/**/*.agam`
- `.agent/test/*.agam`
- `examples/*.agam`
- `README.md`
- parser and AST crates under `crates/agam_parser` and `crates/agam_ast`

## Success Criteria

- `.agam` examples remain believable and repo-grounded
- No Python-wrapper framing sneaks into Agam-facing work
