//! E-Graph Equality Saturation Optimization Engine powered by `egg`.
//!
//! Applies algebraic rewrite rules, identity simplifications, and constant
//! foldings on Agam MIR expressions using equality saturation.

#![deny(clippy::unwrap_used)]

use egg::{define_language, rewrite, Id, RecExpr, Rewrite, Runner, Symbol};

define_language! {
    pub enum AgamLanguage {
        Num(i64),
        Symbol(Symbol),
        "+" = Add([Id; 2]),
        "-" = Sub([Id; 2]),
        "*" = Mul([Id; 2]),
        "/" = Div([Id; 2]),
        "%" = Rem([Id; 2]),
        "&" = BitAnd([Id; 2]),
        "|" = BitOr([Id; 2]),
        "^" = BitXor([Id; 2]),
        "<<" = Shl([Id; 2]),
        ">>" = Shr([Id; 2]),
        "neg" = Neg(Id),
        "not" = Not(Id),
    }
}

/// Construct standard algebraic rewrite rules for scalar operations.
pub fn algebraic_rules() -> Vec<Rewrite<AgamLanguage, ()>> {
    vec![
        // Additive identities
        rewrite!("add-zero-right"; "(+ ?a 0)" => "?a"),
        rewrite!("add-zero-left"; "(+ 0 ?a)" => "?a"),
        rewrite!("add-comm"; "(+ ?a ?b)" => "(+ ?b ?a)"),
        rewrite!("add-assoc"; "(+ (+ ?a ?b) ?c)" => "(+ ?a (+ ?b ?c))"),

        // Subtractive identities
        rewrite!("sub-zero"; "(- ?a 0)" => "?a"),
        rewrite!("sub-self"; "(- ?a ?a)" => "0"),
        rewrite!("sub-cancel"; "(- (+ ?a ?b) ?b)" => "?a"),

        // Multiplicative identities
        rewrite!("mul-one-right"; "(* ?a 1)" => "?a"),
        rewrite!("mul-one-left"; "(* 1 ?a)" => "?a"),
        rewrite!("mul-zero-right"; "(* ?a 0)" => "0"),
        rewrite!("mul-zero-left"; "(* 0 ?a)" => "0"),
        rewrite!("mul-comm"; "(* ?a ?b)" => "(* ?b ?a)"),
        rewrite!("mul-assoc"; "(* (* ?a ?b) ?c)" => "(* ?a (* ?b ?c))"),

        // Distributivity
        rewrite!("distribute-mul-add"; "(* ?a (+ ?b ?c))" => "(+ (* ?a ?b) (* ?a ?c))"),

        // Bitwise identities
        rewrite!("and-self"; "(& ?a ?a)" => "?a"),
        rewrite!("or-self"; "(| ?a ?a)" => "?a"),
        rewrite!("xor-self"; "(^ ?a ?a)" => "0"),
        rewrite!("and-zero"; "(& ?a 0)" => "0"),
        rewrite!("or-zero"; "(| ?a 0)" => "?a"),
        rewrite!("xor-zero"; "(^ ?a 0)" => "?a"),

        // Double negation
        rewrite!("double-neg"; "(neg (neg ?a))" => "?a"),
    ]
}

/// Simplify an algebraic expression string using equality saturation.
pub fn simplify_expr(expr_str: &str) -> Result<String, String> {
    let expr: RecExpr<AgamLanguage> = expr_str
        .parse()
        .map_err(|e| format!("Failed to parse expression '{expr_str}': {e}"))?;

    let rules = algebraic_rules();
    let runner = Runner::default()
        .with_expr(&expr)
        .with_node_limit(10_000)
        .with_iter_limit(30)
        .run(&rules);

    let root = runner.roots[0];
    let extractor = egg::Extractor::new(&runner.egraph, egg::AstSize);
    let (_best_cost, best_expr) = extractor.find_best(root);

    Ok(best_expr.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_egg_additive_identity() {
        let res = simplify_expr("(+ x 0)").unwrap_or_default();
        assert_eq!(res, "x");

        let res2 = simplify_expr("(+ 0 y)").unwrap_or_default();
        assert_eq!(res2, "y");
    }

    #[test]
    fn test_egg_multiplicative_identity_and_zero() {
        let res = simplify_expr("(* x 1)").unwrap_or_default();
        assert_eq!(res, "x");

        let res2 = simplify_expr("(* x 0)").unwrap_or_default();
        assert_eq!(res2, "0");
    }

    #[test]
    fn test_egg_subtraction_cancellation() {
        let res = simplify_expr("(- x x)").unwrap_or_default();
        assert_eq!(res, "0");

        let res2 = simplify_expr("(- (+ a b) b)").unwrap_or_default();
        assert_eq!(res2, "a");
    }

    #[test]
    fn test_egg_bitwise_identities() {
        let res = simplify_expr("(^ val val)").unwrap_or_default();
        assert_eq!(res, "0");

        let res2 = simplify_expr("(& val val)").unwrap_or_default();
        assert_eq!(res2, "val");
    }

    #[test]
    fn test_egg_double_negation() {
        let res = simplify_expr("(neg (neg z))").unwrap_or_default();
        assert_eq!(res, "z");
    }
}
