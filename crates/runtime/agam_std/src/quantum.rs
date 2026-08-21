//! Quantum Correlated Sampling & Entangled Game Non-Local Value Verification.
//!
//! Implements quantum state representations, postselection-stable correlated sampling,
//! and parallel repetition non-local game value decay evaluations based on quantum info theory.

use crate::complex::Complex;

/// A normalized multi-qubit pure quantum state vector $|\psi\rangle = \sum_i c_i |i\rangle$.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantumState {
    pub num_qubits: usize,
    pub amplitudes: Vec<Complex>,
}

impl QuantumState {
    /// Create a zero-initialized computational basis state $|0\dots 0\rangle$.
    pub fn zero(num_qubits: usize) -> Self {
        let dim = 1usize << num_qubits;
        let mut amplitudes = vec![Complex::new(0.0, 0.0); dim];
        if dim > 0 {
            amplitudes[0] = Complex::new(1.0, 0.0);
        }
        Self {
            num_qubits,
            amplitudes,
        }
    }

    /// Create a quantum state from custom complex amplitudes, normalizing automatically.
    pub fn from_amplitudes(num_qubits: usize, amplitudes: Vec<Complex>) -> Self {
        let dim = 1usize << num_qubits;
        assert_eq!(amplitudes.len(), dim);
        let mut state = Self {
            num_qubits,
            amplitudes,
        };
        state.normalize();
        state
    }

    /// Compute the state vector Euclidean norm $\sqrt{\sum |c_i|^2}$.
    pub fn norm(&self) -> f64 {
        let sum_sq: f64 = self.amplitudes.iter().map(|c| c.magnitude_squared()).sum();
        sum_sq.sqrt()
    }

    /// Normalize the quantum state vector to unit length $\|\psi\|_2 = 1$.
    pub fn normalize(&mut self) {
        let n = self.norm();
        if n > 1e-15 {
            for c in &mut self.amplitudes {
                *c = Complex::new(c.re / n, c.im / n);
            }
        }
    }

    /// Compute the quantum inner product (overlap) $\langle \phi | \psi \rangle = \sum \phi_i^* \psi_i$.
    pub fn inner_product(&self, other: &QuantumState) -> Complex {
        assert_eq!(self.num_qubits, other.num_qubits);
        let mut sum = Complex::new(0.0, 0.0);
        for (a, b) in self.amplitudes.iter().zip(&other.amplitudes) {
            let conj_a = a.conjugate();
            sum = sum.add(conj_a.mul(*b));
        }
        sum
    }

    /// Compute the quantum fidelity $F(\psi, \phi) = |\langle \psi | \phi \rangle|^2$.
    pub fn fidelity(&self, other: &QuantumState) -> f64 {
        self.inner_product(other).magnitude_squared()
    }

    /// Compute the trace distance between two pure quantum states:
    ///
    /// $$D(\psi, \phi) = \sqrt{1 - F(\psi, \phi)}$$
    pub fn trace_distance(&self, other: &QuantumState) -> f64 {
        let f = self.fidelity(other);
        (1.0 - f).max(0.0).sqrt()
    }
}

/// Simulate postselection-stable quantum correlated sampling.
///
/// Smooths and aligns quantum amplitudes with regularizer $\alpha \in [0, 1]$:
///
/// $$|\psi'\rangle \propto (1 - \alpha)|\psi\rangle + \alpha |\text{target}\rangle$$
pub fn quantum_correlated_sampling(
    state: &QuantumState,
    target: &QuantumState,
    alpha: f64,
) -> QuantumState {
    assert_eq!(state.num_qubits, target.num_qubits);
    let alpha = alpha.clamp(0.0, 1.0);
    let one_minus_alpha = 1.0 - alpha;

    let mut new_amps = Vec::with_capacity(state.amplitudes.len());
    for (s, t) in state.amplitudes.iter().zip(&target.amplitudes) {
        let blended = Complex::new(
            one_minus_alpha * s.re + alpha * t.re,
            one_minus_alpha * s.im + alpha * t.im,
        );
        new_amps.push(blended);
    }

    QuantumState::from_amplitudes(state.num_qubits, new_amps)
}

/// Evaluate the parallel repetition entangled game value decay upper bound:
///
/// $$\omega^*(G^{\otimes n}) \le \exp\left(-c_{qs} \cdot \frac{\varepsilon^{13}}{\varepsilon + \log(|A| \cdot |B|)} \cdot n\right)$$
///
/// where $\varepsilon = 1 - \omega^*(G)$ is the game value deficit and $|A|, |B|$ are player answer alphabet sizes.
pub fn entangled_game_value_decay(
    eps: f64,
    alphabet_a: usize,
    alphabet_b: usize,
    n_rounds: usize,
) -> f64 {
    if eps <= 0.0 || n_rounds == 0 {
        return 1.0;
    }
    let eps = eps.min(1.0);
    let log_alphabet = ((alphabet_a.max(2) * alphabet_b.max(2)) as f64).ln();
    let c_qs = 0.005; // Universal quantum correlated sampling constant

    let exponent = -c_qs * (eps.powi(13) / (eps + log_alphabet)) * (n_rounds as f64);
    exponent.exp().min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_state_basis_and_normalization() {
        let q = QuantumState::zero(2);
        assert_eq!(q.num_qubits, 2);
        assert_eq!(q.amplitudes.len(), 4);
        assert!((q.norm() - 1.0).abs() < 1e-10);
        assert_eq!(q.amplitudes[0], Complex::new(1.0, 0.0));
    }

    #[test]
    fn test_quantum_inner_product_and_trace_distance() {
        let q0 = QuantumState::zero(1);
        let q1 =
            QuantumState::from_amplitudes(1, vec![Complex::new(0.0, 0.0), Complex::new(1.0, 0.0)]);

        // Orthogonal states
        assert_eq!(q0.inner_product(&q1), Complex::new(0.0, 0.0));
        assert_eq!(q0.fidelity(&q1), 0.0);
        assert!((q0.trace_distance(&q1) - 1.0).abs() < 1e-10);

        // Identical states
        assert!((q0.fidelity(&q0) - 1.0).abs() < 1e-10);
        assert_eq!(q0.trace_distance(&q0), 0.0);
    }

    #[test]
    fn test_entangled_game_decay_scaling() {
        let val_100 = entangled_game_value_decay(0.1, 2, 2, 100);
        let val_1000 = entangled_game_value_decay(0.1, 2, 2, 1000);
        assert!(
            val_1000 < val_100,
            "Decay must strictly decrease with rounds"
        );
    }
}
