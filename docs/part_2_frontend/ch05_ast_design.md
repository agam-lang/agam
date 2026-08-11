# Chapter 5: Abstract Syntax Trees & Grammar Representation

> **Core Literature Grounding**: *Language Implementation Patterns* (Chapter 3) by Terence Parr  
> **Compiler Module Focus**: [`agam_ast`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/core/agam_ast)

---

## 5.1 Concrete vs. Abstract Syntax Trees

- **Concrete Syntax Tree (CST / Parse Tree)**: Preserves every single token from input source text, including grouping parentheses, commas, semicolons, and comments. CSTs are essential for tools like formatters (`agam_fmt`) and language servers (`agam_lsp`).
- **Abstract Syntax Tree (AST)**: Discards redundant syntactic noise (e.g., matching parentheses, semicolons), preserving only semantic hierarchy. ASTs are optimized for type checking (`agam_sema`) and IR lowering (`agam_hir`).

```text
    Concrete Parse Tree (CST)                    Abstract Syntax Tree (AST)
           Expr                                         BinaryExpr(+)
        ┌───┼───┐                                          ┌───┴───┐
      Expr  +  Expr                                    Literal(1) Literal(2)
       (1)      (2)
```

---

## 5.2 AST Node Architecture in Rust (`agam_ast`)

In `agam_ast`, nodes are represented using algebraic data types (`enum` and `struct` definitions):

```rust
pub struct Module {
    pub name: String,
    pub items: Vec<Item>,
    pub span: Span,
}

pub enum Item {
    Fn(Function),
    Struct(StructDecl),
    Enum(EnumDecl),
    Effect(EffectDecl),
}

pub struct Function {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub span: Span,
}

pub enum Stmt {
    Let { name: Ident, ty: Option<TypeAnnotation>, init: Expr, span: Span },
    Expr(Expr),
    Return(Option<Expr>, Span),
}

pub enum Expr {
    Literal(Literal),
    Ident(Ident),
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr>, span: Span },
    Call { callee: Box<Expr>, args: Vec<Expr>, span: Span },
    Perform { effect_name: Ident, payload: Box<Expr>, span: Span },
    Handle { body: Box<Expr>, handlers: Vec<HandlerClause>, span: Span },
    Match { target: Box<Expr>, arms: Vec<MatchArm>, span: Span },
}
```

---

## 5.3 AST Visitor & Transformer Patterns

Following Terence Parr's *Language Implementation Patterns*, tree manipulation operations (semantic checking, AST rewrites, diagnostic validation) are decoupled from AST definitions using **Visitor** or **Folder** traits:

```rust
pub trait AstVisitor {
    fn visit_expr(&mut self, expr: &Expr) {
        walk_expr(self, expr);
    }
    fn visit_stmt(&mut self, stmt: &Stmt) {
        walk_stmt(self, stmt);
    }
}
```

This pattern guarantees clean separation of concerns: syntax trees remain lightweight data structures while passes implement specific compiler operations.
