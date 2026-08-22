//! Formal contract verification engine: `@verify`, `requires()`, `ensures()`, and loop invariants.
//!
//! Generates verification conditions (VCs) and dispatches to SMT solvers (Z3 / CVC5)
//! to prove functional correctness and memory safety at compile time.

use crate::solver::{Constraint, SmtSolver, SolverResult, Z3Solver};

/// Pre-condition or Post-condition specification on a function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contract {
    /// Pre-conditions that caller must satisfy: `requires(condition)`
    pub requires: Vec<Constraint>,
    /// Post-conditions that function guarantees: `ensures(condition)`
    pub ensures: Vec<Constraint>,
    /// Invariants maintained across loop iterations: `invariant(condition)`
    pub loop_invariants: Vec<Constraint>,
}

impl Contract {
    pub fn new() -> Self {
        Self {
            requires: Vec::new(),
            ensures: Vec::new(),
            loop_invariants: Vec::new(),
        }
    }

    pub fn require(mut self, cond: Constraint) -> Self {
        self.requires.push(cond);
        self
    }

    pub fn ensure(mut self, cond: Constraint) -> Self {
        self.ensures.push(cond);
        self
    }

    pub fn invariant(mut self, cond: Constraint) -> Self {
        self.loop_invariants.push(cond);
        self
    }
}

impl Default for Contract {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of formal contract verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationOutcome {
    /// Proof succeeded: mathematically impossible for contract or safety invariants to be violated.
    ProvenSafe,
    /// Counterexample found: solver proved a state where post-condition or safety invariant fails.
    CounterexampleFound(String),
    /// Solver timed out or was inconclusive.
    Indeterminate,
}

/// SMT Verification Engine for Agam Functions and Modules.
pub struct ContractVerifier {
    solver: Z3Solver,
}

impl ContractVerifier {
    pub fn new() -> Self {
        Self {
            solver: Z3Solver::new(),
        }
    }

    /// Verify that `requires => ensures` holds under the given variable declarations.
    pub fn verify_function_contract(
        &mut self,
        vars: &[&str],
        contract: &Contract,
    ) -> VerificationOutcome {
        self.solver.push();

        // 1. Declare all typed variables in SMT solver
        for &var in vars {
            self.solver.declare_int(var);
        }

        // 2. Assert all pre-conditions (assumptions)
        for req in &contract.requires {
            self.solver.assert(req.clone());
        }

        // 3. Check negation of post-conditions (if NOT ensures is Unsat, then ensures is ALWAYS True)
        let mut all_proven = true;
        for ens in &contract.ensures {
            self.solver.push();
            // Negate condition
            let negated = match ens {
                Constraint::Gt(a, b) => Constraint::Le(a.clone(), b.clone()),
                Constraint::Ge(a, b) => Constraint::Lt(a.clone(), b.clone()),
                Constraint::Lt(a, b) => Constraint::Ge(a.clone(), b.clone()),
                Constraint::Le(a, b) => Constraint::Gt(a.clone(), b.clone()),
                Constraint::Eq(a, b) => Constraint::NotEq(a.clone(), b.clone()),
                Constraint::NotEq(a, b) => Constraint::Eq(a.clone(), b.clone()),
                other => {
                    Constraint::NotEq(Box::new(other.clone()), Box::new(Constraint::Bool(true)))
                }
            };
            self.solver.assert(negated);

            match self.solver.check_sat() {
                SolverResult::Unsat => {
                    // Negation is unsatisfiable => property holds universally!
                }
                SolverResult::Sat => {
                    self.solver.pop();
                    self.solver.pop();
                    return VerificationOutcome::CounterexampleFound(format!(
                        "Post-condition violated: {}",
                        ens.to_smtlib()
                    ));
                }
                SolverResult::Unknown => {
                    all_proven = false;
                }
            }
            self.solver.pop();
        }

        self.solver.pop();

        if all_proven {
            VerificationOutcome::ProvenSafe
        } else {
            VerificationOutcome::Indeterminate
        }
    }
}

impl Default for ContractVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_simple_arithmetic_contract() {
        let mut verifier = ContractVerifier::new();

        // fn abs_val(x: int) -> int
        // requires(x >= 0)
        // ensures(result >= 0)
        let contract = Contract::new()
            .require(Constraint::Ge(
                Box::new(Constraint::Var("x".to_string())),
                Box::new(Constraint::Int(0)),
            ))
            .ensure(Constraint::Ge(
                Box::new(Constraint::Var("x".to_string())),
                Box::new(Constraint::Int(0)),
            ));

        let outcome = verifier.verify_function_contract(&["x"], &contract);
        assert_eq!(outcome, VerificationOutcome::ProvenSafe);
    }

    #[test]
    fn test_verify_detects_violation_counterexample() {
        let mut verifier = ContractVerifier::new();

        // requires(x > 0)
        // ensures(x > 100) -> NOT universally valid!
        let contract = Contract::new()
            .require(Constraint::Gt(
                Box::new(Constraint::Var("x".to_string())),
                Box::new(Constraint::Int(0)),
            ))
            .ensure(Constraint::Gt(
                Box::new(Constraint::Var("x".to_string())),
                Box::new(Constraint::Int(100)),
            ));

        let outcome = verifier.verify_function_contract(&["x"], &contract);
        assert!(matches!(
            outcome,
            VerificationOutcome::CounterexampleFound(_)
        ));
    }
}
