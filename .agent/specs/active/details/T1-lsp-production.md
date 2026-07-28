# Phase T1-lsp-production � LSP Production Quality

**Status:** open (upgrading existing `agam_lsp` stub)
**Tier:** 1 (Developer Experience Excellence)

## Scope

Transform the existing `agam_lsp` crate from a stub into a production-quality Language Server Protocol implementation that provides real-time IDE features: completion, hover, go-to-definition, find references, rename, diagnostics, and semantic highlighting.

## Deliverables

### Core Navigation
- [ ] Go-to-definition for functions, types, traits, variables, imports
- [ ] Find all references
- [ ] Rename symbol (across files in workspace)
- [ ] Document symbol outline (file-level function/type list)
- [ ] Workspace symbol search

### Completion
- [ ] Identifier completion (local variables, functions, types)
- [ ] Keyword completion (context-aware)
- [ ] Module member completion (after `.` or `::`)
- [ ] Import path completion
- [ ] Named argument completion at call sites (after F5)
- [ ] Snippet completion for common patterns (if/match/for templates)

### Hover and Signature Help
- [ ] Type information on hover for any expression
- [ ] Doc comment display on hover for documented items
- [ ] Signature help on function call (parameter names and types)
- [ ] Inlay hints for inferred types (after F2)

### Diagnostics
- [ ] Real-time error and warning display (leveraging DX1 infrastructure)
- [ ] Inline diagnostic rendering
- [ ] Quick-fix code actions from diagnostic suggestions

### Semantic Tokens
- [ ] Semantic token highlighting (types, functions, variables, keywords, macros)
- [ ] Modifier tokens (mutable, static, deprecated)

### Code Actions
- [ ] Auto-import for unresolved identifiers
- [ ] Extract function refactoring
- [ ] Organize imports
- [ ] Add missing trait methods

### AI Agent Grounding (NEW — 2026 industry alignment)
- [ ] LSIF (Language Server Index Format) export for static code intelligence
- [ ] Semantic verification endpoints for AI agent grounding (symbol existence, type queries)
- [ ] MCP (Model Context Protocol) bridge: LSP → MCP tool exposure (coordinates with AGENT1)
- [ ] AI-powered inline completions: expose AI suggestion API alongside traditional completions

## Design References

- **LSP in 2026**: LSP is now the "semantic backbone" for development environments, serving both traditional compiler analysis and AI-assisted coding tools.
- **LSIF**: Enables IDE features in static environments (GitHub code navigation). Critical for Agam discoverability.
- **JetBrains LSP expansion (2025)**: IntelliJ Platform now has open LSP API. Agam's LSP should target both VS Code and JetBrains IDEs.
- **AI grounding**: AI agents (Claude Code, Cursor) use LSP to verify that generated code is semantically correct. Agam's LSP should be designed for this use case from the start.

## Responsible Crates

- `agam_lsp` — LSP server implementation
- `agam_sema` — semantic information provider
- `agam_pkg` — workspace/multi-file resolution

## Dependencies

- Phase T0-type-system (type system) — type hover and inference hints require type information
- Phase T1-error-messages (error messages) — diagnostic infrastructure reused in LSP
- Phase T1-compiler-agent-tool (Compiler-as-Agent-Tool) — MCP server coordinates with LSP

## Test Strategy

- LSP protocol conformance tests using the official LSP test framework
- Integration tests with VS Code extension
- Response time benchmarks (completion should feel instant, <100ms)
- LSIF output validation against GitHub code navigation
