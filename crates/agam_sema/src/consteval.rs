//! Compile-time constant evaluation (comptime).
//!
//! Evaluates expressions at compile time when they are known to be constant.
//! Used for:
//! 1. `const` declarations — values computed at compile time.
//! 2. Array sizes — `[T; N]` where N must be a comptime value.
//! 3. Generic parameters — `Vec<N>` where N is a const generic.
//! 4. `@requires` / `@ensures` contract expressions.

use agam_ast::expr::{Expr, ExprKind, BinOp, UnaryOp};
use agam_errors::Span;

/// A compile-time evaluated value.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
    /// Could not evaluate at compile time.
    Unknown,
}

/// Error during const evaluation.
#[derive(Debug, Clone)]
pub struct ConstEvalError {
    pub message: String,
    pub span: Span,
}

/// The const evaluator.
pub struct ConstEvaluator {
    pub errors: Vec<ConstEvalError>,
}

impl ConstEvaluator {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Attempt to evaluate an expression at compile time.
    pub fn eval(&mut self, expr: &Expr) -> ConstValue {
        match &expr.kind {
            ExprKind::IntLiteral(v) => ConstValue::Int(*v),
            ExprKind::FloatLiteral(v) => ConstValue::Float(*v),
            ExprKind::BoolLiteral(v) => ConstValue::Bool(*v),
            ExprKind::StringLiteral(v) => ConstValue::Str(v.clone()),

            ExprKind::Unary { op, operand } => {
                let val = self.eval(operand);
                match (op, val) {
                    (UnaryOp::Neg, ConstValue::Int(v)) => ConstValue::Int(-v),
                    (UnaryOp::Neg, ConstValue::Float(v)) => ConstValue::Float(-v),
                    (UnaryOp::Not, ConstValue::Bool(v)) => ConstValue::Bool(!v),
                    _ => ConstValue::Unknown,
                }
            }

            ExprKind::Binary { op, left, right } => {
                let lv = self.eval(left);
                let rv = self.eval(right);
                self.eval_binary(*op, lv, rv)
            }

            ExprKind::If { condition, then_branch, else_branch } => {
                let cond = self.eval(condition);
                match cond {
                    ConstValue::Bool(true) => self.eval(then_branch),
                    ConstValue::Bool(false) => {
                        if let Some(eb) = else_branch {
                            self.eval(eb)
                        } else {
                            ConstValue::Unit
                        }
                    }
                    _ => ConstValue::Unknown,
                }
            }

            // Anything else is not const-evaluable.
            _ => ConstValue::Unknown,
        }
    }

    /// Evaluate a binary operation on two const values.
    fn eval_binary(&self, op: BinOp, lv: ConstValue, rv: ConstValue) -> ConstValue {
        match (lv, rv) {
            // Integer arithmetic
            (ConstValue::Int(a), ConstValue::Int(b)) => match op {
                BinOp::Add => ConstValue::Int(a.wrapping_add(b)),
                BinOp::Sub => ConstValue::Int(a.wrapping_sub(b)),
                BinOp::Mul => ConstValue::Int(a.wrapping_mul(b)),
                BinOp::Div => {
                    if b == 0 { return ConstValue::Unknown; }
                    ConstValue::Int(a / b)
                }
                BinOp::Mod => {
                    if b == 0 { return ConstValue::Unknown; }
                    ConstValue::Int(a % b)
                }
                BinOp::Pow => ConstValue::Int(a.wrapping_pow(b as u32)),
                BinOp::Eq => ConstValue::Bool(a == b),
                BinOp::NotEq => ConstValue::Bool(a != b),
                BinOp::Lt => ConstValue::Bool(a < b),
                BinOp::LtEq => ConstValue::Bool(a <= b),
                BinOp::Gt => ConstValue::Bool(a > b),
                BinOp::GtEq => ConstValue::Bool(a >= b),
                BinOp::BitAnd => ConstValue::Int(a & b),
                BinOp::BitOr => ConstValue::Int(a | b),
                BinOp::BitXor => ConstValue::Int(a ^ b),
                BinOp::Shl => ConstValue::Int(a << b),
                BinOp::Shr => ConstValue::Int(a >> b),
                _ => ConstValue::Unknown,
            },

            // Float arithmetic
            (ConstValue::Float(a), ConstValue::Float(b)) => match op {
                BinOp::Add => ConstValue::Float(a + b),
                BinOp::Sub => ConstValue::Float(a - b),
                BinOp::Mul => ConstValue::Float(a * b),
                BinOp::Div => ConstValue::Float(a / b),
                BinOp::Eq => ConstValue::Bool(a == b),
                BinOp::NotEq => ConstValue::Bool(a != b),
                BinOp::Lt => ConstValue::Bool(a < b),
                BinOp::LtEq => ConstValue::Bool(a <= b),
                BinOp::Gt => ConstValue::Bool(a > b),
                BinOp::GtEq => ConstValue::Bool(a >= b),
                _ => ConstValue::Unknown,
            },

            // Boolean logic
            (ConstValue::Bool(a), ConstValue::Bool(b)) => match op {
                BinOp::And => ConstValue::Bool(a && b),
                BinOp::Or => ConstValue::Bool(a || b),
                BinOp::Eq => ConstValue::Bool(a == b),
                BinOp::NotEq => ConstValue::Bool(a != b),
                _ => ConstValue::Unknown,
            },

            // String concatenation
            (ConstValue::Str(a), ConstValue::Str(b)) => match op {
                BinOp::Add => ConstValue::Str(format!("{}{}", a, b)),
                BinOp::Eq => ConstValue::Bool(a == b),
                BinOp::NotEq => ConstValue::Bool(a != b),
                _ => ConstValue::Unknown,
            },

            _ => ConstValue::Unknown,
        }
    }

    /// Evaluate and expect a specific type, emitting an error otherwise.
    pub fn eval_expect_int(&mut self, expr: &Expr) -> Option<i64> {
        match self.eval(expr) {
            ConstValue::Int(v) => Some(v),
            ConstValue::Unknown => None,
            other => {
                self.errors.push(ConstEvalError {
                    message: format!("expected integer constant, found {:?}", other),
                    span: expr.span,
                });
                None
            }
        }
    }

    pub fn eval_expect_bool(&mut self, expr: &Expr) -> Option<bool> {
        match self.eval(expr) {
            ConstValue::Bool(v) => Some(v),
            ConstValue::Unknown => None,
            other => {
                self.errors.push(ConstEvalError {
                    message: format!("expected boolean constant, found {:?}", other),
                    span: expr.span,
                });
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agam_ast::NodeId;

    fn int_expr(v: i64) -> Expr {
        Expr { id: NodeId(0), span: Span::dummy(), kind: ExprKind::IntLiteral(v) }
    }

    fn float_expr(v: f64) -> Expr {
        Expr { id: NodeId(0), span: Span::dummy(), kind: ExprKind::FloatLiteral(v) }
    }

    fn bool_expr(v: bool) -> Expr {
        Expr { id: NodeId(0), span: Span::dummy(), kind: ExprKind::BoolLiteral(v) }
    }

    fn str_expr(v: &str) -> Expr {
        Expr { id: NodeId(0), span: Span::dummy(), kind: ExprKind::StringLiteral(v.into()) }
    }

    fn binop(op: BinOp, left: Expr, right: Expr) -> Expr {
        Expr {
            id: NodeId(0), span: Span::dummy(),
            kind: ExprKind::Binary { op, left: Box::new(left), right: Box::new(right) },
        }
    }

    fn unary(op: UnaryOp, operand: Expr) -> Expr {
        Expr {
            id: NodeId(0), span: Span::dummy(),
            kind: ExprKind::Unary { op, operand: Box::new(operand) },
        }
    }

    #[test]
    fn test_int_literal() {
        let mut eval = ConstEvaluator::new();
        assert_eq!(eval.eval(&int_expr(42)), ConstValue::Int(42));
    }

    #[test]
    fn test_float_literal() {
        let mut eval = ConstEvaluator::new();
        assert_eq!(eval.eval(&float_expr(3.14)), ConstValue::Float(3.14));
    }

    #[test]
    fn test_bool_literal() {
        let mut eval = ConstEvaluator::new();
        assert_eq!(eval.eval(&bool_expr(true)), ConstValue::Bool(true));
    }

    #[test]
    fn test_string_literal() {
        let mut eval = ConstEvaluator::new();
        assert_eq!(eval.eval(&str_expr("hello")), ConstValue::Str("hello".into()));
    }

    #[test]
    fn test_int_addition() {
        let mut eval = ConstEvaluator::new();
        let expr = binop(BinOp::Add, int_expr(10), int_expr(32));
        assert_eq!(eval.eval(&expr), ConstValue::Int(42));
    }

    #[test]
    fn test_int_multiplication() {
        let mut eval = ConstEvaluator::new();
        let expr = binop(BinOp::Mul, int_expr(6), int_expr(7));
        assert_eq!(eval.eval(&expr), ConstValue::Int(42));
    }

    #[test]
    fn test_nested_arithmetic() {
        let mut eval = ConstEvaluator::new();
        // (2 + 3) * 4 = 20
        let add = binop(BinOp::Add, int_expr(2), int_expr(3));
        let expr = binop(BinOp::Mul, add, int_expr(4));
        assert_eq!(eval.eval(&expr), ConstValue::Int(20));
    }

    #[test]
    fn test_comparison() {
        let mut eval = ConstEvaluator::new();
        let expr = binop(BinOp::Lt, int_expr(3), int_expr(5));
        assert_eq!(eval.eval(&expr), ConstValue::Bool(true));
    }

    #[test]
    fn test_boolean_and() {
        let mut eval = ConstEvaluator::new();
        let expr = binop(BinOp::And, bool_expr(true), bool_expr(false));
        assert_eq!(eval.eval(&expr), ConstValue::Bool(false));
    }

    #[test]
    fn test_negation() {
        let mut eval = ConstEvaluator::new();
        let expr = unary(UnaryOp::Neg, int_expr(42));
        assert_eq!(eval.eval(&expr), ConstValue::Int(-42));
    }

    #[test]
    fn test_not() {
        let mut eval = ConstEvaluator::new();
        let expr = unary(UnaryOp::Not, bool_expr(true));
        assert_eq!(eval.eval(&expr), ConstValue::Bool(false));
    }

    #[test]
    fn test_string_concat() {
        let mut eval = ConstEvaluator::new();
        let expr = binop(BinOp::Add, str_expr("hello "), str_expr("world"));
        assert_eq!(eval.eval(&expr), ConstValue::Str("hello world".into()));
    }

    #[test]
    fn test_division_by_zero_returns_unknown() {
        let mut eval = ConstEvaluator::new();
        let expr = binop(BinOp::Div, int_expr(42), int_expr(0));
        assert_eq!(eval.eval(&expr), ConstValue::Unknown);
    }

    #[test]
    fn test_const_if_true() {
        let mut eval = ConstEvaluator::new();
        let expr = Expr {
            id: NodeId(0), span: Span::dummy(),
            kind: ExprKind::If {
                condition: Box::new(bool_expr(true)),
                then_branch: Box::new(int_expr(1)),
                else_branch: Some(Box::new(int_expr(2))),
            },
        };
        assert_eq!(eval.eval(&expr), ConstValue::Int(1));
    }

    #[test]
    fn test_const_if_false() {
        let mut eval = ConstEvaluator::new();
        let expr = Expr {
            id: NodeId(0), span: Span::dummy(),
            kind: ExprKind::If {
                condition: Box::new(bool_expr(false)),
                then_branch: Box::new(int_expr(1)),
                else_branch: Some(Box::new(int_expr(2))),
            },
        };
        assert_eq!(eval.eval(&expr), ConstValue::Int(2));
    }

    #[test]
    fn test_float_arithmetic() {
        let mut eval = ConstEvaluator::new();
        let expr = binop(BinOp::Add, float_expr(1.5), float_expr(2.5));
        assert_eq!(eval.eval(&expr), ConstValue::Float(4.0));
    }

    #[test]
    fn test_bitwise_ops() {
        let mut eval = ConstEvaluator::new();
        let expr = binop(BinOp::BitAnd, int_expr(0xFF), int_expr(0x0F));
        assert_eq!(eval.eval(&expr), ConstValue::Int(0x0F));
    }
}
