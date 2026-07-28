# Phase T1-doc-generation � Documentation Generation

**Status:** open (upgrading existing `agam_doc` stub)
**Tier:** 1 (Developer Experience Excellence)

## Scope

Implement doc-comment syntax, `agamc doc` command, HTML documentation generation, cross-reference linking, and doctest compilation. An undocumented language is an unusable language.

## Deliverables

### Doc Comment Syntax
- [ ] `///` for item-level doc comments (advance mode)
- [ ] `//!` for module-level doc comments (advance mode)
- [ ] `##` for item-level doc comments (base mode)
- [ ] `#!` for module-level doc comments (base mode)
- [ ] Markdown supported in doc comments
- [ ] Code examples in doc comments with ` ```agam ` fencing

### Doc Generation Command
- [ ] `agamc doc` generates HTML documentation for the current package
- [ ] `agamc doc --open` generates and opens in browser
- [ ] `agamc doc --json` outputs documentation as structured JSON
- [ ] Output directory configurable, default `target/doc/`

### HTML Documentation
- [ ] Module hierarchy navigation sidebar
- [ ] Type/function/trait index pages
- [ ] Cross-reference linking between types, functions, modules
- [ ] Source code links (click to see implementation)
- [ ] Search functionality (client-side JavaScript search)
- [ ] Responsive design for mobile viewing
- [ ] Dark mode support

### Doctests
- [ ] `agamc doctest` extracts and compiles code examples from doc comments
- [ ] Code examples that fail to compile are reported as documentation errors
- [ ] `# ` prefix hides setup lines from rendered docs but includes them in compilation
- [ ] CI integration: doctests run as part of `cargo test` / `agamc test`

### Standard Library Documentation
- [ ] All `agam_std` public API documented with doc comments
- [ ] Usage examples for every public function and type
- [ ] Module-level overview documentation

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
