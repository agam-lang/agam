# Chapter 26: Diagnostic Engineering, Spans & Error Recovery

> **Part VII: Advanced Tooling, Testing & Ecosystem Engineering**  
> **Compiler Module Focus**: [`agam_errors`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_errors)

---

## 26.1 Diagnostic Architecture in Production Compilers

A compiler's diagnostic engine is often a developer's primary interface with the language. Cryptic or misaligned error reports slow development down significantly.

In the Agam Compiler, `agam_errors` provides a unified diagnostic infrastructure that:
- Captures exact source byte spans (`Span`, `SourceId`).
- Supports multi-line code snippet extraction and underline highlighting.
- Renders rich terminal output using ANSI colors and human-readable suggestions.

---

## 26.2 Diagnostic Data Models

```rust
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel, // Error, Warning, Note, Help
    pub code: Option<String>,   // e.g., "E0308"
    pub message: String,
    pub primary_label: Label,
    pub secondary_labels: Vec<Label>,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub span: Span,
    pub replacement: String,
    pub message: String,
}
```

---

## 26.3 Diagnostic Rendering Example

When `agam_sema` detects a type mismatch, `agam_errors` renders a detailed report:

```text
error[E0308]: mismatched types
 --> src/main.agam:12:21
   |
12 |     let count: Int = "forty-two";
   |                ---   ^^^^^^^^^^^ expected `Int`, found `String`
   |                |
   |                expected due to this type annotation
   |
help: to convert a String to an Int, use `String.parse_int()`
   |
12 |     let count: Int = "forty-two".parse_int();
   |                                 ++++++++++++
```
