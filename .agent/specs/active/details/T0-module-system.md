# Phase T0-module-system � Module System and Visibility

**Status:** open
**Tier:** 0 (Foundation Completion)
**Priority:** Required for multi-file projects, stdlib organization, and package ecosystem

## Scope

Formalize Agam's module system with explicit file-to-module mapping, visibility rules, qualified imports, and re-exports. Without this, real multi-crate projects and a proper standard library cannot be organized.

## Deliverables

### Module Declarations
- [ ] File-to-module mapping convention (e.g., `src/collections/hashmap.agam` → `collections::hashmap`)
- [ ] Explicit `module` declarations for inline sub-modules
- [ ] Module-level doc comments

### Import System
- [ ] Qualified imports: `import agam_std.collections.HashMap`
- [ ] Wildcard imports: `import agam_std.collections.*` (with lint warning)
- [ ] Selective imports: `import agam_std.collections.{HashMap, BTreeMap}`
- [ ] Aliased imports: `import agam_std.collections.HashMap as Map`
- [ ] Relative imports within the same package

### Re-exports
- [ ] `pub use` to re-export items from sub-modules
- [ ] Facade modules that collect and re-export public API
- [ ] Standard library uses re-exports for ergonomic top-level access

### Visibility Rules
- [ ] Private by default — items are module-private unless marked
- [ ] `pub` — visible to all external consumers
- [ ] `pub(crate)` — visible within the current package but not externally
- [ ] `pub(module)` — visible within the parent module (stretch)
- [ ] Visibility checking integrated into `agam_sema`
- [ ] Clear error messages for visibility violations

### Circular Dependency Handling
- [ ] Detect and reject circular module imports at compile time
- [ ] Clear diagnostic showing the cycle path

## Responsible Crates

- `agam_parser` — module declaration syntax, import syntax extensions
- `agam_sema` — name resolution through module hierarchy, visibility enforcement
- `agam_pkg` — file-to-module mapping, package boundary definition

## Dependencies

- Partially independent of F2/F3 — can be developed in parallel
- F3 (object model) needs module boundaries for visibility semantics

## Test Strategy

- Multi-file compilation tests with cross-module imports
- Visibility enforcement tests (private access from outside module = error)
- Circular dependency detection tests
- Re-export resolution tests
