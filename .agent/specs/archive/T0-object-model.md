# Phase T0-object-model � Object Model Completion

**Status:** open (was Phase 27, promoted to Tier 0)
**Tier:** 0 (Foundation Completion)
**Priority:** Core language mechanics — not polish, not optional

## Scope

Complete the struct/trait/impl pipeline end-to-end from parser through codegen. Add constructors, `self`, visibility, trait bounds, and method dispatch. Agam's object model should favor composition over inheritance.

## Why This Was Promoted

Phase 27 was originally the second-to-last phase in the roadmap. But an object model is not a luxury feature — it is core language mechanics. No language can claim to be production-ready without:
- Methods on types
- Interface/trait abstraction
- Visibility control
- Constructor semantics

## Deliverables

### Struct Completion
- [ ] Struct fields with visibility modifiers (`pub` field, private-by-default)
- [ ] Struct constructors: `Type { field: value }` syntax or `Type::new()` convention
- [ ] `Self` type alias within `impl` blocks
- [ ] Struct update syntax: `Type { field: new_value, ..existing }`

### Trait System
- [ ] Trait declarations with method signatures
- [ ] Associated types on traits: `trait Iterator { type Item; fn next(self) -> Option<Self::Item>; }`
- [ ] Default method implementations in traits
- [ ] Trait bounds on generic parameters (depends on F2)
- [ ] Multiple trait implementations per type
- [ ] Trait objects for dynamic dispatch: `dyn Trait`

### Impl Blocks
- [ ] `impl Type { ... }` for inherent methods
- [ ] `impl Trait for Type { ... }` for trait implementations
- [ ] `self`, `&self`, `&mut self` receiver syntax (or Agam-specific equivalent)
- [ ] Static methods (no `self` parameter): `Type::from_string(s)`
- [ ] Method resolution order for inherent vs trait methods

### Visibility
- [ ] `pub` — visible everywhere
- [ ] `pub(crate)` — visible within the current crate/package
- [ ] Private by default — visible only within the defining module
- [ ] Visibility applies to: struct fields, functions, methods, types, modules

### Method Dispatch
- [ ] Static dispatch by default (monomorphized, zero-cost)
- [ ] Dynamic dispatch via `dyn Trait` (vtable-based)
- [ ] Method call syntax: `object.method(args)`
- [ ] Uniform Function Call Syntax consideration

### Design Decisions
- [ ] **No class inheritance hierarchy.** Agam uses traits + composition, not `class Foo extends Bar`
- [ ] **No implicit constructors.** Explicit construction through struct literals or named constructors
- [ ] **Value semantics by default.** Structs are values unless wrapped in a reference type

## Responsible Crates

- `agam_parser` — method call syntax, impl blocks, trait declarations, visibility keywords
- `agam_ast` — AST nodes for impl, trait, method, visibility
- `agam_sema` — trait resolution, method lookup, visibility checking, coherence rules
- `agam_hir` — typed methods, trait implementations, vtable layout
- `agam_mir` — monomorphized methods, devirtualization opportunities
- `agam_codegen` — emit methods, vtables, dynamic dispatch

## Dependencies

- Phase T0-type-system (type system) — generics needed for trait bounds and associated types
- Phase T0-module-system (module system) — visibility rules depend on module boundaries
- Phase T0-stdlib-completion (design principles) — informed by anuvṛtti (contextual defaults) and samāsa (compound patterns)

## Test Strategy

- Parser tests for all new syntax (impl blocks, trait decls, method calls)
- Sema tests for trait resolution, coherence, visibility enforcement
- End-to-end tests: define types, implement traits, call methods, compile and run
- Negative tests: orphan rule violations, missing trait methods, visibility errors
