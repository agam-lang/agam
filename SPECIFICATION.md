# Agam Language Specification (v0.1.0-alpha)

> Formal specification of the syntax, type system, algebraic effect model, memory model, and code generation semantics for the Agam Programming Language.

---

## 1. Syntax & Grammar (EBNF)

### 1.1 Declarations
```ebnf
Module        ::= Declaration*
Declaration   ::= FunctionDecl | StructDecl | EnumDecl | TraitDecl | ImplDecl | EffectDecl
FunctionDecl  ::= Annotation* "fn" Identifier Generics? "(" ParamList? ")" ("->" TypeExpr)? Block
StructDecl    ::= "struct" Identifier Generics? "{" (Identifier ":" TypeExpr ("," Identifier ":" TypeExpr)*)? "}"
EnumDecl      ::= "enum" Identifier Generics? "{" EnumVariant ("," EnumVariant)* "}"
EnumVariant   ::= Identifier ("(" TypeExprList ")")?
EffectDecl    ::= "effect" Identifier "{" (Identifier "(" ParamList? ")" ("->" TypeExpr)? ";")* "}"
```

### 1.2 Statements & Expressions
```ebnf
Block         ::= "{" Statement* Expr? "}"
Statement     ::= LetStmt | ReturnStmt | WhileStmt | ForStmt | ExprStmt
LetStmt       ::= "let" ("mut")? Identifier (":" TypeExpr)? ("=" Expr)? ";"
ReturnStmt    ::= "return" Expr? ";"
Expr          ::= BinaryExpr | UnaryExpr | CallExpr | MethodCall | MatchExpr | TryExpr | Literal | Var
MatchExpr     ::= "match" Expr "{" MatchArm ("," MatchArm)* "}"
MatchArm      ::= Pattern ("if" Expr)? "=>" (Expr | Block)
```

---

## 2. Type System

* **Interned Type Arena (`TypeStore`)**: All resolved types (`i8`..`i512`, `f32`, `f64`, `bool`, `str`, tuples, arrays, structs, enums, functions) are deduplicated with `$O(1)$` `TypeId` comparison.
* **Hindley-Milner Type Inference**: Constraint-based unification using path-compressed disjoint sets (Union-Find).
* **Sandhi Monomorphization**: Static specialization of generic functions and compound structures into specialized IR units.

---

## 3. Algebraic Effects System

* **Effect Perform**: `perform Effect::Operation(args...)` yields control to the dynamically enclosing handler.
* **Effect Handlers**: `with Handler handle Effect { body }` establishes a scoped delimited continuation boundary without colored functions.

---

## 4. Dual Memory Model

* **ARC Mode (Default)**: Value semantics for primitive structures and Automatic Reference Counting for heap structures, augmented by compile-time escape analysis to elide retain/release pairs.
* **Strict Mode (`strict { }`)**: Opt-in zero-overhead ownership semantics featuring affine moves, shared (`&T`) and exclusive (`&mut T`) borrow checking, and scope drop guarantees.

---

## 5. Formal Verification & Diagnostics

* **SMT Contracts**: Function specifications (`requires` precondition and `ensures` postcondition) are solved via Z3/SMT solvers during type checking.
* **Nyāya Diagnostics**: Compiler diagnostics emit structured 4-part epistemological proofs:
  1. **Pratijñā (Fact)**: Observed empirical condition.
  2. **Hetu (Reason)**: Invariant violation explanation.
  3. **Udāharaṇa (Fix)**: Actionable code suggestion.
  4. **Nigamana (Law)**: Governing language specification rule.
