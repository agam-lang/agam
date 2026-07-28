# Phase T6-self-hosting � Self-Hosting

**Status:** open
**Tier:** 6 (Frontier and Long-Horizon)

## Scope

Bootstrap the Agam compiler to be self-hosting — the Agam compiler written in Agam. This is a maturity milestone that signals the language is capable enough for complex, real-world software.

## Approach

1. **Identify minimum subset:** Determine the minimal Agam language surface needed to write a lexer, parser, and compiler pass
2. **Lexer first:** Rewrite the lexer in Agam (simplest component, string processing heavy)
3. **Parser second:** Rewrite the parser in Agam (recursive descent, tree construction)
4. **Gradual migration:** One compiler pass at a time, maintaining the Rust version as reference
5. **Dual compilation:** Build Agam compiler with both Rust and Agam versions, compare output

## Deliverables

- [ ] Language subset analysis: what's needed for compiler-writing
- [ ] `agam_lexer` rewritten in Agam
- [ ] `agam_parser` rewritten in Agam
- [ ] At least one sema/lowering pass rewritten in Agam
- [ ] Bootstrap chain documented and reproducible
- [ ] CI job that builds the Agam compiler using the previous version

## Dependencies

- Tiers 0–2 must be substantially complete (type system, object model, modules, memory model)
- Standard library must support file I/O, string manipulation, collections
