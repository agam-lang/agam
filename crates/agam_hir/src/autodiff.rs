//! Automatic Differentiation engine.
//!
//! Implements both forward-mode and reverse-mode automatic differentiation
//! directly in the compiler's HIR phase. The chain rule is applied at the
//! instruction level: `∂z/∂x = ∂z/∂y · ∂y/∂x`
//!
//! ## Forward Mode (Dual Numbers)
//! Each value carries its derivative alongside: `(value, derivative)`.
//! Best for functions with few inputs, many outputs.
//!
//! ## Reverse Mode (Backpropagation)
//! Records a computation graph (tape), then walks backward to compute
//! all partial derivatives in a single pass. Best for many inputs, few outputs
//! (i.e., neural networks).

/// A dual number for forward-mode AD: carries a value and its derivative.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dual {
    pub val: f64,
    pub grad: f64,
}

impl Dual {
    /// Create a constant (derivative = 0).
    pub fn constant(val: f64) -> Self {
        Self { val, grad: 0.0 }
    }

    /// Create a variable (derivative = 1 w.r.t. itself).
    pub fn variable(val: f64) -> Self {
        Self { val, grad: 1.0 }
    }

    /// Addition: d(a+b) = da + db
    pub fn add(self, other: Dual) -> Dual {
        Dual {
            val: self.val + other.val,
            grad: self.grad + other.grad,
        }
    }

    /// Subtraction: d(a-b) = da - db
    pub fn sub(self, other: Dual) -> Dual {
        Dual {
            val: self.val - other.val,
            grad: self.grad - other.grad,
        }
    }

    /// Multiplication (product rule): d(a*b) = a*db + b*da
    pub fn mul(self, other: Dual) -> Dual {
        Dual {
            val: self.val * other.val,
            grad: self.val * other.grad + self.grad * other.val,
        }
    }

    /// Division (quotient rule): d(a/b) = (da*b - a*db) / b²
    pub fn div(self, other: Dual) -> Dual {
        let b2 = other.val * other.val;
        Dual {
            val: self.val / other.val,
            grad: (self.grad * other.val - self.val * other.grad) / b2,
        }
    }

    /// Power: d(a^n) = n * a^(n-1) * da
    pub fn pow(self, n: f64) -> Dual {
        Dual {
            val: self.val.powf(n),
            grad: n * self.val.powf(n - 1.0) * self.grad,
        }
    }

    /// sin: d(sin(a)) = cos(a) * da
    pub fn sin(self) -> Dual {
        Dual {
            val: self.val.sin(),
            grad: self.val.cos() * self.grad,
        }
    }

    /// cos: d(cos(a)) = -sin(a) * da
    pub fn cos(self) -> Dual {
        Dual {
            val: self.val.cos(),
            grad: -self.val.sin() * self.grad,
        }
    }

    /// exp: d(e^a) = e^a * da
    pub fn exp(self) -> Dual {
        let e = self.val.exp();
        Dual {
            val: e,
            grad: e * self.grad,
        }
    }

    /// ln: d(ln(a)) = da / a
    pub fn ln(self) -> Dual {
        Dual {
            val: self.val.ln(),
            grad: self.grad / self.val,
        }
    }

    /// Negation: d(-a) = -da
    pub fn neg(self) -> Dual {
        Dual {
            val: -self.val,
            grad: -self.grad,
        }
    }
}

// ────────────────────────────────────────────────
// Reverse-mode AD (Tape-based backpropagation)
// ────────────────────────────────────────────────

/// A node in the reverse-mode computation graph.
#[derive(Debug, Clone)]
pub struct TapeNode {
    /// The computed value at this node.
    pub value: f64,
    /// Partial derivatives w.r.t. each parent: (parent_index, ∂self/∂parent).
    pub parents: Vec<(usize, f64)>,
}

/// A gradient tape for reverse-mode automatic differentiation.
#[derive(Debug)]
pub struct GradTape {
    nodes: Vec<TapeNode>,
}

impl GradTape {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Record a constant on the tape.
    pub fn constant(&mut self, val: f64) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(TapeNode {
            value: val,
            parents: vec![],
        });
        idx
    }

    /// Record a variable (same as constant, but represents an input).
    pub fn variable(&mut self, val: f64) -> usize {
        self.constant(val)
    }

    /// Record addition: z = a + b, ∂z/∂a = 1, ∂z/∂b = 1
    pub fn add(&mut self, a: usize, b: usize) -> usize {
        let val = self.nodes[a].value + self.nodes[b].value;
        let idx = self.nodes.len();
        self.nodes.push(TapeNode {
            value: val,
            parents: vec![(a, 1.0), (b, 1.0)],
        });
        idx
    }

    /// Record multiplication: z = a * b, ∂z/∂a = b, ∂z/∂b = a
    pub fn mul(&mut self, a: usize, b: usize) -> usize {
        let va = self.nodes[a].value;
        let vb = self.nodes[b].value;
        let idx = self.nodes.len();
        self.nodes.push(TapeNode {
            value: va * vb,
            parents: vec![(a, vb), (b, va)],
        });
        idx
    }

    /// Record subtraction: z = a - b, ∂z/∂a = 1, ∂z/∂b = -1
    pub fn sub(&mut self, a: usize, b: usize) -> usize {
        let val = self.nodes[a].value - self.nodes[b].value;
        let idx = self.nodes.len();
        self.nodes.push(TapeNode {
            value: val,
            parents: vec![(a, 1.0), (b, -1.0)],
        });
        idx
    }

    /// Record sin: z = sin(a), ∂z/∂a = cos(a)
    pub fn sin(&mut self, a: usize) -> usize {
        let va = self.nodes[a].value;
        let idx = self.nodes.len();
        self.nodes.push(TapeNode {
            value: va.sin(),
            parents: vec![(a, va.cos())],
        });
        idx
    }

    /// Get the value at a tape node.
    pub fn value(&self, idx: usize) -> f64 {
        self.nodes[idx].value
    }

    /// Run backward pass: compute ∂output/∂(every input).
    /// Returns a gradient vector indexed by tape node index.
    pub fn backward(&self, output: usize) -> Vec<f64> {
        let mut grads = vec![0.0; self.nodes.len()];
        grads[output] = 1.0; // ∂output/∂output = 1

        // Walk the tape in reverse order (topological sort by construction)
        for i in (0..=output).rev() {
            let g = grads[i];
            for &(parent, local_grad) in &self.nodes[i].parents {
                grads[parent] += g * local_grad; // Chain rule
            }
        }
        grads
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Forward-mode tests ──

    #[test]
    fn test_dual_constant() {
        let c = Dual::constant(5.0);
        assert_eq!(c.val, 5.0);
        assert_eq!(c.grad, 0.0);
    }

    #[test]
    fn test_dual_variable() {
        let x = Dual::variable(3.0);
        assert_eq!(x.val, 3.0);
        assert_eq!(x.grad, 1.0);
    }

    #[test]
    fn test_dual_add() {
        // f(x) = x + 5, f'(x) = 1
        let x = Dual::variable(3.0);
        let c = Dual::constant(5.0);
        let result = x.add(c);
        assert_eq!(result.val, 8.0);
        assert_eq!(result.grad, 1.0);
    }

    #[test]
    fn test_dual_mul() {
        // f(x) = x * 3, f'(x) = 3
        let x = Dual::variable(4.0);
        let c = Dual::constant(3.0);
        let result = x.mul(c);
        assert_eq!(result.val, 12.0);
        assert_eq!(result.grad, 3.0);
    }

    #[test]
    fn test_dual_x_squared() {
        // f(x) = x², f'(x) = 2x. At x=3: f=9, f'=6
        let x = Dual::variable(3.0);
        let result = x.mul(x);
        assert_eq!(result.val, 9.0);
        assert_eq!(result.grad, 6.0);
    }

    #[test]
    fn test_dual_power() {
        // f(x) = x³, f'(x) = 3x². At x=2: f=8, f'=12
        let x = Dual::variable(2.0);
        let result = x.pow(3.0);
        assert_eq!(result.val, 8.0);
        assert_eq!(result.grad, 12.0);
    }

    #[test]
    fn test_dual_chain_rule() {
        // f(x) = (x+1)², f'(x) = 2(x+1). At x=2: f=9, f'=6
        let x = Dual::variable(2.0);
        let one = Dual::constant(1.0);
        let sum = x.add(one);
        let result = sum.mul(sum);
        assert_eq!(result.val, 9.0);
        assert_eq!(result.grad, 6.0);
    }

    #[test]
    fn test_dual_exp() {
        // f(x) = e^x, f'(x) = e^x. At x=0: f=1, f'=1
        let x = Dual::variable(0.0);
        let result = x.exp();
        assert!((result.val - 1.0).abs() < 1e-10);
        assert!((result.grad - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_dual_sin_cos() {
        // f(x) = sin(x), f'(x) = cos(x). At x=0: f=0, f'=1
        let x = Dual::variable(0.0);
        let result = x.sin();
        assert!((result.val).abs() < 1e-10);
        assert!((result.grad - 1.0).abs() < 1e-10);
    }

    // ── Reverse-mode tests ──

    #[test]
    fn test_tape_basic() {
        let mut tape = GradTape::new();
        let x = tape.variable(3.0);
        assert_eq!(tape.value(x), 3.0);
    }

    #[test]
    fn test_tape_add_grad() {
        // f(x,y) = x + y. ∂f/∂x = 1, ∂f/∂y = 1
        let mut tape = GradTape::new();
        let x = tape.variable(3.0);
        let y = tape.variable(5.0);
        let z = tape.add(x, y);
        let grads = tape.backward(z);
        assert_eq!(grads[x], 1.0);
        assert_eq!(grads[y], 1.0);
    }

    #[test]
    fn test_tape_mul_grad() {
        // f(x,y) = x * y. ∂f/∂x = y, ∂f/∂y = x
        let mut tape = GradTape::new();
        let x = tape.variable(3.0);
        let y = tape.variable(5.0);
        let z = tape.mul(x, y);
        let grads = tape.backward(z);
        assert_eq!(grads[x], 5.0); // ∂f/∂x = y = 5
        assert_eq!(grads[y], 3.0); // ∂f/∂y = x = 3
    }

    #[test]
    fn test_tape_x_squared() {
        // f(x) = x², ∂f/∂x = 2x. At x=4: ∂f/∂x = 8
        let mut tape = GradTape::new();
        let x = tape.variable(4.0);
        let z = tape.mul(x, x);
        let grads = tape.backward(z);
        assert_eq!(grads[x], 8.0);
    }

    #[test]
    fn test_tape_chain_f_of_g() {
        // f(x) = (x+1)*x = x²+x, ∂f/∂x = 2x+1. At x=3: ∂f/∂x = 7
        let mut tape = GradTape::new();
        let x = tape.variable(3.0);
        let one = tape.constant(1.0);
        let xp1 = tape.add(x, one);
        let z = tape.mul(xp1, x);
        let grads = tape.backward(z);
        assert_eq!(grads[x], 7.0);
    }

    #[test]
    fn test_tape_complex_expression() {
        // f(x,y) = x*y + x. ∂f/∂x = y+1 = 6, ∂f/∂y = x = 3
        let mut tape = GradTape::new();
        let x = tape.variable(3.0);
        let y = tape.variable(5.0);
        let xy = tape.mul(x, y);
        let z = tape.add(xy, x);
        let grads = tape.backward(z);
        assert_eq!(grads[x], 6.0); // y+1
        assert_eq!(grads[y], 3.0); // x
    }
}
