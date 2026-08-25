# Phase T1-doc-generation — Documentation Generation

**Status:** complete
**Tier:** 1 (Developer Experience Excellence)

## Scope

Implement doc-comment syntax, `agamc doc` command, HTML documentation generation, cross-reference linking, and doctest compilation. An undocumented language is an unusable language.

## Deliverables

### Doc Comment Syntax
- [x] `///` for item-level doc comments (advance mode)
- [x] `//!` for module-level doc comments (advance mode)
- [x] `##` for item-level doc comments (base mode)
- [x] `#!` for module-level doc comments (base mode)
- [x] Markdown supported in doc comments
- [x] Code examples in doc comments with ` ```agam ` fencing

### Doc Generation Command
- [x] `agamc doc` generates HTML documentation for the current package
- [x] `agamc doc --open` generates and opens in browser
- [x] `agamc doc --json` outputs documentation as structured JSON
- [x] Output directory configurable, default `target/doc/`

### HTML Documentation
- [x] Module hierarchy navigation sidebar
- [x] Type/function/trait index pages
- [x] Cross-reference linking between types, functions, modules
- [x] Source code links (click to see implementation)
- [x] Search functionality (client-side JavaScript search)
- [x] Responsive design for mobile viewing
- [x] Dark mode support

### Doctests
- [x] `agamc doctest` extracts and compiles code examples from doc comments
- [x] Code examples that fail to compile are reported as documentation errors
- [x] `# ` prefix hides setup lines from rendered docs but includes them in compilation
- [x] CI integration: doctests run as part of `cargo test` / `agamc test`

### Standard Library Documentation
- [x] All `agam_std` public API documented with doc comments
- [x] Usage examples for every public function and type
- [x] Module-level overview documentation

## Responsible Crates

- `agam_doc` — documentation extraction, HTML generation, doctest runner
- `agam_parser` — doc comment parsing and attachment to AST nodes
- `agam_ast` — doc comment storage on AST items

## Dependencies

- Phase T0-type-system/F3 (types and objects) — documentation needs types to document
- Phase T0-module-system (modules) — module hierarchy drives documentation structure

## Test Strategy

- Doc comment parsing tests
- HTML generation snapshot tests
- Doctest compilation and execution tests
- Cross-reference link validity tests
