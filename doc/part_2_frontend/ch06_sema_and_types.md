# Chapter 6: Symbol Tables, Lexical Scopes & Type Inference Engine

> **Core Literature Grounding**: *Language Implementation Patterns* (Chapters 6–8) by Terence Parr  
> **Compiler Module Focus**: [`agam_sema`](file:///c:/Users/ksvik/Projects/Agam-Lang/agam/crates/middle/agam_sema)

---

## 6.1 Symbol Resolution & Lexical Scopes

Before type checking can evaluate expressions, the compiler must resolve every variable, function, or type identifier to its corresponding canonical definition.

### Scope Graph Architecture
A **Symbol Table** tracks symbol declarations across nested lexical scope blocks:

```text
 ┌──────────────────────────────────────────────────────────────┐
 │ Module Scope: fn main, struct Tensor, effect Logger          │
 └──────────────────────────────▲───────────────────────────────┘
                                │ Parent Scope Link
 ┌──────────────────────────────┴───────────────────────────────┐
 │ Function Scope (main): let x: Int, let weights: Tensor       │
 └──────────────────────────────▲───────────────────────────────┘
                                │ Parent Scope Link
 ┌──────────────────────────────┴───────────────────────────────┐
 │ Block Scope (if condition): let temp: Float                  │
 └──────────────────────────────────────────────────────────────┘
```

```rust
pub struct SymbolTable {
    scopes: Vec<Scope>,
    current_scope: ScopeId,
}

pub struct Scope {
    parent: Option<ScopeId>,
    symbols: HashMap<String, SymbolInfo>,
}

pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub ty: Type,
    pub span: Span,
}
```

---

## 6.2 Bidirectional Type Checking & Inference Engine

`agam_sema` enforces static type safety using a bidirectional type checking algorithm:

1. **Type Checking (Top-Down / Synthesize)**: Given an expected target type $T$, verify that expression $E$ evaluates to $T$.
2. **Type Inference (Bottom-Up / Analyze)**: Given an expression $E$ without explicit annotations, infer its primitive or composite type $T$.

$$\frac{\Gamma \vdash e_1 : \text{Int} \quad \Gamma \vdash e_2 : \text{Int}}{\Gamma \vdash e_1 + e_2 : \text{Int}}$$

```rust
pub fn check_expr(&mut self, expr: &Expr, expected: Option<&Type>) -> Result<Type, TypeError> {
    match expr {
        Expr::Literal(Literal::Int(_)) => Ok(Type::Int),
        Expr::Binary { op, lhs, rhs, .. } => {
            let lhs_ty = self.check_expr(lhs, None)?;
            let rhs_ty = self.check_expr(rhs, None)?;
            
            if lhs_ty != rhs_ty {
                return Err(TypeError::Mismatch { expected: lhs_ty, found: rhs_ty });
            }
            Ok(lhs_ty)
        }
        Expr::Ident(ident) => {
            let symbol = self.symbol_table.lookup(&ident.name)
                .ok_or(TypeError::UndeclaredIdentifier(ident.name.clone()))?;
            Ok(symbol.ty.clone())
        }
        _ => ...
    }
}
```

---

## 6.3 Algebraic Effect Checking & Verification

Agam verifies side-effects statically during semantic analysis. If a function contains a `perform EffectName(...)` expression, `agam_sema` verifies that:
1. The effect is declared in the function's signature (`fn run() -> Int ! Logger`), OR
2. The `perform` expression occurs inside an enclosing `handle` block that intercepts `Logger`.

Uncaught or undeclared effects raise compile-time semantic errors (`UnhandledEffect`).
