# Chapter 29: Source Code Formatting Engine Architecture (`agam_fmt`)

> **Part VII: Advanced Tooling, Testing & Ecosystem Engineering**  
> **Compiler Module Focus**: [`agam_fmt`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/tooling/agam_fmt)

---

## 29.1 The Role of Code Formatters

Code formatters enforce a unified code style across codebases, eliminating style debates in pull requests and improving code readability.

Unlike compilers which discard white space and comments during AST parsing, a code formatter must operate on **Concrete Syntax Trees (CST)** or token streams that preserve comments, blank lines, and source layout.

---

## 29.2 Formatter Engine Pipeline

```text
Source Code Text (.agam)
           │
           ▼  Lexer / CST Builder
  Concrete Syntax Tree (CST)
           │
           ▼  Wadler-Style Pretty Printer
  Doc Abstraction Tree (Group, Nest, Line)
           │
           ▼  Line Width Layout Solver (max_width = 100)
  Formatted Source Code Text
```

### Formatter Algorithm Rules:
1. **Indentation**: Standard 4-space indent per block level.
2. **Line Breaking**: Wrap function arguments or struct fields across multiple indented lines when line length exceeds 100 characters.
3. **Comment Preservation**: Re-attach floating or inline comments (`//`, `/* */`) to their nearest sibling syntax nodes.
