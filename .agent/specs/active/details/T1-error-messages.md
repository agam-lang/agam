# Phase T1-error-messages � Elite Error Messages and Recovery

**Status:** open
**Tier:** 1 (Developer Experience Excellence)
**Priority:** Rust proved that error message quality is a top adoption driver

## Scope

Transform Agam's compiler diagnostics from basic error reporting into a world-class developer experience with error recovery, rich source annotations, actionable suggestions, and machine-readable output for IDE consumption.

## Deliverables

### Parser Error Recovery
- [ ] Continue parsing after syntax errors to report multiple issues per compilation
- [ ] Synchronization points: recover at statement/block/function boundaries
- [ ] Track error recovery state to avoid cascading false-positive errors
- [ ] Report the most useful errors first, suppress noise

### Rich Diagnostic Rendering
- [ ] Multi-line source display with line numbers and column markers
- [ ] Underline spans with `^^^` markers pointing to the exact error location
- [ ] Color-coded terminal output: error (red), warning (yellow), note (blue), help (green)
- [ ] `--color auto/always/never` flag
- [ ] Unicode box-drawing for diagnostic frames (like Rust's `rustc`)

### Actionable Suggestions
- [ ] "Did you mean?" for misspelled identifiers (Levenshtein distance)
- [ ] "Did you mean?" for wrong keyword usage
- [ ] Suggested fixes with concrete code snippets
- [ ] "Help:" annotations explaining why an error occurs
- [ ] "Note:" annotations showing relevant declaration sites

### Multi-Span Diagnostics
- [ ] Show both error site and relevant declaration/definition
- [ ] Type mismatch errors show both the expected and actual types with their origins
- [ ] Import errors show the module being imported and available alternatives

### Machine-Readable Output
- [ ] `--error-format json` for IDE and CI consumption
- [ ] JSON diagnostic format compatible with LSP diagnostic protocol
- [ ] Structured error codes (e.g., `E0001`, `E0002`) with documentation
- [ ] `agamc explain E0001` command to show detailed error documentation

### Warning Infrastructure
- [ ] `#[allow(warning_name)]` to suppress specific warnings
- [ ] `#[deny(warning_name)]` to promote warnings to errors
- [ ] `--warn-as-error` / `-Werror` flag
- [ ] Unused variable, unused import, unreachable code warnings
- [ ] Deprecated item warnings

## Responsible Crates

- `agam_errors` — diagnostic data model, rendering engine, suggestion infrastructure
- `agam_parser` — error recovery, synchronization points
- `agam_sema` — type error diagnostics, suggestion generation
- `agam_driver` — `--error-format`, `--color`, `explain` command

## Dependencies

- Independent of other Tier 0 work — can start during F2 implementation
- Error recovery in parser is orthogonal to type system design

## Test Strategy

- Snapshot tests: compile known-bad programs, compare diagnostic output against golden files
- Error recovery tests: programs with multiple errors produce multiple diagnostics
- JSON format tests: parse diagnostic JSON, validate structure
- Suggestion accuracy tests: misspelled identifiers produce correct suggestions
