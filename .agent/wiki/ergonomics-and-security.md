# Agam Ergonomics vs. Security Strategy

This document tracks future improvements and design rules regarding the tension between developer ergonomics (ease of use) and strict security.

## Core Principle: Security is Non-Negotiable
Security must always remain the highest priority. Any ergonomic improvements in `@lang.base` must be implemented via compiler-level inference or intelligent tooling, **never** by relaxing backend type-safety, memory-safety, or execution sandboxing.

## Feature Tracking: Role Labels (Vibhakti)

### Current State
Function signatures use strict role labels (e.g., `copy(from: a, to: b, count: 10)`) to enforce semantic clarity and prevent argument-swapping vulnerabilities.

### Future Improvement: `@lang.base` Ergonomics
Data scientists and scripters find explicit role labels tedious. To improve the Base variant experience without compromising security:

1. **Optional Inference in Base:** In `@lang.base`, the compiler will allow omitting labels if the argument order is unambiguous (e.g., `copy(a, b, 10)`).
2. **Compiler Verification:** The compiler must internally map these positional arguments to the strict signature and run identical security checks.
3. **IDE Tooling:** The Language Server (LSP) should auto-insert or render ghost-text labels in the editor, ensuring code remains self-documenting.
4. **Mandatory in Advance:** In `@lang.advance`, strict label usage remains mandatory for critical systems programming to prevent human error.

*Status: Logged for future prioritization in the Language Surface Expansion phase.*
