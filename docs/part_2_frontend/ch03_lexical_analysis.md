# Chapter 3: Lexical Analysis & Token Scanning

> **Core Literature Grounding**: *Crafting Interpreters* (Chapter 4) by Robert Nystrom  
> **Compiler Module Focus**: [`agam_lexer`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_lexer), [`agam_errors`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_errors)

---

## 3.1 Role of the Lexer

The **Lexer** (or Scanner) forms the first stage of the compiler frontend. It converts an unformatted stream of UTF-8 source text into a sequential stream of structured **Tokens**, stripping whitespace and comments while preserving positional metadata.

```text
Raw Source Code Stream ("let x: Int = 42;")
                    │
                    ▼
     ┌────────────────────────────┐
     │ Lexical Scanner (`agam_lexer`)│
     └──────────────┬─────────────┘
                    │
                    ▼
Token Stream: [ Let, Identifier("x"), Colon, Identifier("Int"), Equal, Number(42), Semicolon ]
```

---

## 3.2 Token Structure & Source Span Attribution

To generate diagnostic error reports with source code underlining, every token must record its physical location in the input source file.

In `agam_lexer`, tokens are paired with a `Span`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u32,       // Byte offset of first character
    pub end: u32,         // Byte offset past last character
    pub source_id: SourceId, // Unique ID of input source file
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}
```

---

## 3.3 Lexer Implementation Techniques

### State Machine Character Scanning
The scanner iterates through characters using a single lookahead pointer:

```rust
pub enum TokenKind {
    // Keywords
    Fn, Let, Perform, Handle, Effect, Match,
    // Literals
    Identifier(String), Integer(i64), Float(f64), StringLit(String),
    // Operators & Punctuation
    Plus, Minus, Star, Slash, Equal, EqualEqual, Colon, Arrow,
    // Control
    EOF, Error(String),
}
```

When scanning multi-character operators (e.g., `=` vs. `==`, `->`), the scanner inspects the lookahead character (`peek()`) to determine whether to advance:

```rust
match current_char {
    '=' => {
        if self.peek_char() == '=' {
            self.advance();
            TokenKind::EqualEqual
        } else {
            TokenKind::Equal
        }
    }
    '-' => {
        if self.peek_char() == '>' {
            self.advance();
            TokenKind::Arrow
        } else {
            TokenKind::Minus
        }
    }
    _ => ...
}
```

---

## 3.4 Resilient Diagnostic Error Recovery

If the lexer encounters invalid UTF-8 sequences or unrecognized characters, it does not abort immediately. Instead, it emits an `Error` token variant alongside an `agam_errors` diagnostic, allowing the scanner to continue processing subsequent tokens so the compiler can report multiple errors in a single build pass.
