# Phase T1-testing-framework � Testing Framework Maturity

**Status:** open (upgrading existing `agam_test` + `@test` annotation)
**Tier:** 1 (Developer Experience Excellence)

## Scope

Evolve the current `@test` annotation and `agam_test` crate into a full-featured testing framework with filtering, parallel execution, assertions, fixtures, coverage, and benchmark testing.

## Deliverables

### Test Runner
- [ ] `agamc test` discovers and runs all `@test` functions in a package
- [ ] `agamc test --filter pattern` to run specific tests
- [ ] Parallel test execution with configurable thread count
- [ ] Structured output: pass/fail counts, timing, failure details
- [ ] `--format json` for CI integration
- [ ] `--format pretty` with color-coded terminal output
- [ ] Test timeout enforcement

### Assertions
- [ ] `assert(condition)` with rich failure messages showing the expression
- [ ] `assert_eq(left, right)` showing both values on failure
- [ ] `assert_ne(left, right)` showing both values on failure
- [ ] `assert_approx(left, right, epsilon)` for floating-point comparisons
- [ ] Custom failure messages: `assert(x > 0, "x must be positive, got {x}")`

### Test Organization
- [ ] `@test` annotation on functions (existing)
- [ ] `@test.setup` and `@test.teardown` for fixtures
- [ ] `@test.ignore` to skip tests
- [ ] `@test.should_panic` for expected-failure tests
- [ ] Test modules / test files discovery convention

### Code Coverage
- [ ] `agamc test --coverage` generates coverage report
- [ ] Line coverage, branch coverage, function coverage
- [ ] HTML coverage report output
- [ ] Coverage threshold enforcement for CI
- [ ] LLVM source-based coverage integration

### Benchmark Testing
- [ ] `@bench` annotation for benchmark functions
- [ ] Statistical analysis: median, mean, stddev, percentiles
- [ ] Warm-up iterations
- [ ] `agamc bench` command separate from `agamc test`
- [ ] Baseline comparison (detect regressions)

### Property-Based Testing (stretch)
- [ ] `@test.property` with random input generation
- [ ] Shrinking on failure to find minimal counterexample
- [ ] Built-in generators for common types

## Responsible Crates

- `agam_test` — test runner, assertion library, coverage integration
- `agam_driver` — `agamc test` and `agamc bench` commands
- `agam_runtime` — assertion panic handling

## Dependencies

- Phase T0-type-system (type system) — generics needed for typed assertions
- Phase T0-object-model (object model) — trait-based assertion comparison

## Test Strategy

- Meta-testing: test the test runner itself with known-pass and known-fail programs
- Coverage accuracy validation against manual line counting
- Benchmark stability tests (repeated runs produce consistent results)
