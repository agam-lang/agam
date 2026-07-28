# Phase T0-effects-depth � Ergonomics and Syntax Cohesion

**Status:** open (was Phase 28, promoted to Tier 0)
**Tier:** 0 (Foundation Completion)
**Priority:** The difference between a language people try and a language people choose

## Scope

Add named arguments, default parameters, string interpolation, destructuring, range expressions, operator overloading, and expression-oriented blocks. Feed these into the formatter and LSP so the language teaches and preserves its premium style.

## Why This Was Promoted

Phase 28 was the last phase in the original roadmap. But ergonomics determine whether developers *choose* Agam over alternatives. Python's success is 80% ergonomics. These features must ship alongside the type system and object model, not years later.

## Deliverables

### Named Arguments and Defaults
- [ ] Named arguments at call sites: `connect(host: "localhost", port: 8080)`
- [ ] Default parameter values: `fn connect(host: String, port: i32 = 80)`
- [ ] Positional and named arguments can mix (positional first)

### String Interpolation
- [ ] Interpolation syntax: `f"Hello {name}, you are {age} years old"`
- [ ] Expression interpolation: `f"Result: {compute(x) + 1}"`
- [ ] Format specifiers (stretch): `f"{value:.2f}"`
- [ ] Multi-line strings with interpolation

### Destructuring
- [ ] Destructuring in `let` bindings: `let (x, y) = get_point()`
- [ ] Struct destructuring: `let Point { x, y } = point`
- [ ] Function parameter destructuring
- [ ] Nested destructuring

### Range Expressions
- [ ] Exclusive range: `0..n`
- [ ] Inclusive range: `0..=n`
- [ ] Range in `for` loops: `for i in 0..n`
- [ ] Range as slice index: `array[1..4]`

### Closures and Lambdas
- [ ] Closure syntax: `|x, y| x + y` or `(x, y) => x + y`
- [ ] Trailing closure syntax for higher-order functions
- [ ] Closure capture semantics (move vs reference)
- [ ] Type inference for closure parameters

### Operator Overloading
- [ ] Via trait implementation: `impl Add for Vector { ... }`
- [ ] Standard operator traits: `Add`, `Sub`, `Mul`, `Div`, `Eq`, `Ord`, `Index`
- [ ] No custom operators — only overload existing ones

### Expression-Oriented Blocks
- [ ] Last expression in a block is the return value
- [ ] `if`/`match` as expressions: `let x = if cond { a } else { b }`
- [ ] Block expressions in variable bindings

### Formatter and LSP Alignment
- [ ] `agam_fmt` understands and preserves all new syntax forms
- [ ] `agam_lsp` provides completion for named arguments, default values
- [ ] Consistent style across base and advance modes

## Responsible Crates

- `agam_parser` — all new syntax forms
- `agam_ast` — AST nodes for closures, ranges, destructuring, interpolation
- `agam_sema` — type checking for new constructs, closure capture analysis
- `agam_hir` — typed representation of closures, ranges
- `agam_fmt` — formatting rules for new syntax
- `agam_lsp` — completion and hover for new constructs

## Dependencies

- Phase T0-type-system (type system) — closures need type inference, operators need traits
- Phase T0-object-model (object model) — operator overloading needs trait system
- Phase T0-stdlib-completion (design principles) — informed by vibhakti (semantic role labels) and dhātu (naming conventions)

## Test Strategy

- Parser tests for every new syntax form in both base and advance modes
- Sema tests for type checking closures, destructuring, ranges
- End-to-end tests: compile and run programs using all new features
- Formatter roundtrip tests: format → parse → format = stable output
