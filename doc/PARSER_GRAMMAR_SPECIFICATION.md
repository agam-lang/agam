# Agam Parser & Grammar Formalization Specification

> **Document Status:** Active Standard  
> **Crates:** `agam_lexer`, `agam_parser`, `agam_ast`  
> **Test Suite:** `agam_test::unit_passes`, `agam_test::error_reporting`

---

## 1. Executive Summary

Agam uses a hand-written hybrid **Recursive Descent + Pratt Precedence Climbing** parser capable of parsing dual syntax modes:
- **`@lang.base`**: Significant off-side Pythonic indentation rules with synthetic `Indent`/`Dedent` tokens.
- **`@lang.advance`**: C/Rust-style explicit curly braces and semicolons.

```
                          Token Stream (from agam_lexer)
                                       │
                                       ▼
                       ┌───────────────────────────────┐
                       │     Parser State & Stream     │
                       │  - tokens: Vec<Token>         │
                       │  - pos: usize                 │
                       │  - NodeId generator           │
                       │  - Error accumulator (Vec)    │
                       └───────────────┬───────────────┘
                                       │
                ┌──────────────────────┴──────────────────────┐
                ▼                                             ▼
┌──────────────────────────────┐              ┌───────────────────────────────┐
│ Recursive Descent Grammar    │              │     Pratt Expression Engine   │
│  - parse_module()            │              │  - parse_expr_with_precedence │
│  - parse_decl() (fn, struct) │              │  - Prefix binding powers      │
│  - parse_stmt() (let, if)    │              │  - Infix binding powers       │
│  - parse_pattern() (match)   │              │  - Postfix calls & indexing   │
└───────────────┬──────────────┘              └───────────────┬───────────────┘
                │                                             │
                └──────────────────────┬──────────────────────┘
                                       │
                                       ▼
                         Abstract Syntax Tree (AST)
```

---

## 2. Operator Precedence & Binding Power Table

| Category | Operators | Associativity | Left Power | Right Power |
|---|---|---|---|---|
| **Assignment** | `=`, `+=`, `-=`, `*=`, `/=`, `%=`, `&=`, `\|=`, `^=`, `<<=`, `>>=` | Right | 10 | 9 |
| **Logical OR** | `\|\|` | Left | 20 | 21 |
| **Logical AND** | `&&` | Left | 30 | 31 |
| **Bitwise OR / XOR** | `\|`, `^` | Left | 40 | 41 |
| **Bitwise AND** | `&` | Left | 50 | 51 |
| **Equality** | `==`, `!=` | Left | 60 | 61 |
| **Comparison** | `<`, `<=`, `>`, `>=` | Left | 70 | 71 |
| **Range Slices** | `..`, `..=` | Left | 75 | 76 |
| **Bit Shifts** | `<<`, `>>` | Left | 80 | 81 |
| **Additive** | `+`, `-` | Left | 90 | 91 |
| **Multiplicative** | `*`, `/`, `%` | Left | 100 | 101 |
| **Prefix Unary** | `-`, `!`, `~`, `*`, `&`, `&mut` | Prefix | - | 110 |
| **Postfix / Call** | `()`, `[]`, `.`, `::`, `?` | Postfix | 120 | - |

---

## 3. Core Grammar Constructs

### 3.1 Declarations
- **Functions:** `[pub] [async] fn name[T, U](param: Type) -> RetType: body`
- **Structs:** `[pub] struct Name { field: Type, ... }`
- **Enums:** `[pub] enum Name { Variant1, Variant2(Type), ... }`
- **Traits:** `[pub] trait Name: SuperTrait { fn method(&self); }`
- **Implementations:** `impl Name:` or `impl Trait for Name:`

### 3.2 Statements & Control Flow
- **Variables:** `let [mut] name: Type = expr`
- **Conditionals:** `if cond { ... } else if cond { ... } else { ... }`
- **Loops:** `while cond { ... }`, `for item in iter { ... }`, `loop { ... }`
- **Pattern Matching:** `match expr { Pattern => expr, ... }`

---

## 4. Error Recovery & Synchronization

The parser features panic-mode error recovery at statement and declaration boundaries:
1. When encountering a parse failure, a `ParseError` is recorded into the error list.
2. The parser advances tokens until reaching a synchronization delimiter: `;`, `\n`, `}`, `fn`, `let`, `struct`, `enum`, `trait`, `impl`.
3. Parsing resumes cleanly without cascading false-positive syntax errors.
