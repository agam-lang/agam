# Phase T1-formatter-linter � Formatter and Linter

**Status:** open (upgrading existing `agam_debug` stub)
**Tier:** 1 (Developer Experience Excellence)

## Scope

Emit DWARF/CodeView debug information from the LLVM backend, implement a DAP (Debug Adapter Protocol) server for VS Code integration, and enable source-level debugging with breakpoints, stepping, and variable inspection.

## Deliverables

### Debug Info Emission
- [ ] DWARF debug info in LLVM backend for Linux/Android targets
- [ ] CodeView/PDB debug info for Windows/MSVC targets
- [ ] Source file and line number mapping through all lowering stages
- [ ] Function name preservation in debug info
- [ ] Variable location tracking (register, stack, optimized-out)
- [ ] Type information in debug info (struct layout, enum discriminants)
- [ ] `-g` flag for debug builds, stripped by default in release

### DAP Server
- [ ] Implement Debug Adapter Protocol server in `agam_debug`
- [ ] Launch and attach request handling
- [ ] Breakpoint management (line breakpoints, conditional breakpoints)
- [ ] Step over, step into, step out
- [ ] Continue and pause
- [ ] Stack frame inspection
- [ ] Variable inspection with Agam-aware formatting
- [ ] Watch expressions

### VS Code Extension
- [ ] `launch.json` configuration for Agam debugging
- [ ] Breakpoint UI integration
- [ ] Variable view with Agam type formatting
- [ ] Call stack with Agam function names and source locations

### Pretty Printing
- [ ] Custom formatters for Agam types in debugger (String, Vec, HashMap, etc.)
- [ ] Enum variant display with payload values
- [ ] Trait object display showing concrete type

## Responsible Crates

- `agam_debug` — DAP server, pretty printers
- `agam_codegen` — DWARF/CodeView emission
- `agam_hir` / `agam_mir` — source location propagation through lowering

## Dependencies

- Phase T0-type-system/F3 (types and objects) — debug info needs type layouts
- LLVM backend maturity (already strong)

## Test Strategy

- Debug info validation: compile with `-g`, inspect with `llvm-dwarfdump` or equivalent
- DAP protocol conformance tests
- Source-level stepping accuracy tests
- Variable inspection correctness tests
