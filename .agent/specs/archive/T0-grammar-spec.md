# Phase T0-grammar-spec � Formal Grammar Specification

**Status:** complete
**Tier:** 0 (Foundation Completion)
**Priority:** Highest — no world-class language ships without a formal syntax specification

## Scope

Write a complete, mechanically testable formal grammar for Agam covering all three syntax modes (`@lang.base`, `@lang.base.dynamic`, `@lang.advance`). This is the canonical syntax reference that parser implementors, tool authors, and language evaluators depend on.

## Why This Is Credibility-Critical

- Every serious language (Rust, Go, Swift, Zig, Python, C++) publishes a formal grammar
- Without it, third-party tools (editors, formatters, static analyzers) cannot be built reliably
- Language evaluators look for a grammar spec as the first sign of maturity
- Ambiguities in the current parser can only be surfaced and resolved through formal specification

## Deliverables

### Grammar Specification
- [ ] Complete EBNF or PEG grammar document at `docs/specification/grammar.ebnf`
- [ ] Cover all three syntax modes with shared and mode-specific productions
- [ ] Document operator precedence and associativity table
- [ ] Document keyword reservation table
- [ ] Document whitespace and indentation significance rules for base mode

### Rendered Reference
- [ ] HTML-rendered grammar reference at `docs/specification/grammar.html` or equivalent
- [ ] Cross-linked from `README.md` and `docs/`

### Mechanical Validation
- [ ] CI job that parses all `examples/*.agam` and `benchmarks/benchmarks/**/*.agam` against the grammar spec
- [ ] Any file that parses successfully with `agam_parser` must also be accepted by the grammar spec
- [ ] Any file rejected by the grammar spec must also be rejected by `agam_parser`
- [ ] Roundtrip consistency test: grammar → generated parser → real sources

### Parser Alignment
- [ ] Audit `agam_parser` for any behavior not captured in the grammar
- [ ] File issues for any parser/grammar mismatches discovered during specification

## Responsible Crates

- `agam_parser` (reference implementation)
- `docs/specification/` (new directory)

## Dependencies

- None — this phase has zero code dependencies and can start immediately

## Test Strategy

- Grammar validation against all checked-in `.agam` sources
- Fuzzing: generate random valid programs from grammar, verify parser accepts them
- Negative testing: generate invalid programs, verify parser rejects them
