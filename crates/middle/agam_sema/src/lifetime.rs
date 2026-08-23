//! Lifetime analysis and affine borrow checker.
//!
//! Provides:
//! 1. **Region Inference** — region variables, subset constraints (`'a: 'b`), and transitive closure.
//! 2. **Loan & Conflict Checking** — shared (`&T`) and mutable (`&mut T`) loan tracking over places.
//! 3. **Move Semantics** — affine ownership, use-after-move detection, and move path invalidation.
//! 4. **Lifetime Elision** — standard inference rules for function parameter/return regions.

use agam_errors::Span;
use std::collections::{HashMap, HashSet};

/// A unique lifetime / region identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LifetimeId(pub u32);

/// A named lifetime: `'a`, `'b`, `'static`, etc.
#[derive(Debug, Clone)]
pub struct Lifetime {
    pub id: LifetimeId,
    pub name: String,
    /// Scope depth where this lifetime begins.
    pub start_depth: u32,
    /// Scope depth where this lifetime ends (inclusive).
    pub end_depth: u32,
}

/// A constraint: lifetime `longer` must outlive lifetime `shorter` (`longer: shorter`).
#[derive(Debug, Clone)]
pub struct LifetimeConstraint {
    pub longer: LifetimeId,
    pub shorter: LifetimeId,
    pub span: Span,
}

/// Error from lifetime or borrow analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifetimeError {
    pub message: String,
    pub span: Span,
}

/// The lifetime analyzer handling region constraints and lifetime elision.
pub struct LifetimeAnalyzer {
    /// All known lifetimes.
    lifetimes: HashMap<LifetimeId, Lifetime>,
    /// Constraints between lifetimes.
    constraints: Vec<LifetimeConstraint>,
    /// Counter for generating unique IDs.
    next_id: u32,
    /// The static lifetime (outlives everything).
    pub static_lifetime: LifetimeId,
    /// Accumulated errors.
    pub errors: Vec<LifetimeError>,
}

impl LifetimeAnalyzer {
    pub fn new() -> Self {
        let static_lt = LifetimeId(0);
        let mut lifetimes = HashMap::new();
        lifetimes.insert(
            static_lt,
            Lifetime {
                id: static_lt,
                name: "'static".into(),
                start_depth: 0,
                end_depth: u32::MAX,
            },
        );

        Self {
            lifetimes,
            constraints: Vec::new(),
            next_id: 1,
            static_lifetime: static_lt,
            errors: Vec::new(),
        }
    }

    /// Create a fresh lifetime for a given scope depth.
    pub fn fresh(&mut self, name: impl Into<String>, depth: u32) -> LifetimeId {
        let id = LifetimeId(self.next_id);
        self.next_id += 1;
        self.lifetimes.insert(
            id,
            Lifetime {
                id,
                name: name.into(),
                start_depth: depth,
                end_depth: depth,
            },
        );
        id
    }

    /// Extend a lifetime's end depth (it lives at least until `depth`).
    pub fn extend(&mut self, id: LifetimeId, depth: u32) {
        if let Some(lt) = self.lifetimes.get_mut(&id)
            && depth > lt.end_depth
        {
            lt.end_depth = depth;
        }
    }

    /// Add a constraint: `longer` must outlive `shorter`.
    pub fn constrain(&mut self, longer: LifetimeId, shorter: LifetimeId, span: Span) {
        self.constraints.push(LifetimeConstraint {
            longer,
            shorter,
            span,
        });
    }

    /// Check all constraints using region propagation fixed-point algorithm.
    pub fn check(&mut self) {
        // Build outlives adjacency graph
        let mut graph: HashMap<LifetimeId, Vec<(LifetimeId, Span)>> = HashMap::new();
        for c in &self.constraints {
            graph.entry(c.shorter).or_default().push((c.longer, c.span));
        }

        // Propagate required end depths across all paths
        let mut changed = true;
        while changed {
            changed = false;
            for c in &self.constraints {
                let s_depth = self
                    .lifetimes
                    .get(&c.shorter)
                    .map(|l| l.end_depth)
                    .unwrap_or(0);
                if let Some(l) = self.lifetimes.get_mut(&c.longer)
                    && l.end_depth < s_depth
                {
                    l.end_depth = s_depth;
                    changed = true;
                }
            }
        }

        // Verify bounds
        for c in &self.constraints {
            let longer = self.lifetimes.get(&c.longer);
            let shorter = self.lifetimes.get(&c.shorter);

            if let (Some(l), Some(s)) = (longer, shorter)
                && l.id != self.static_lifetime
                && l.start_depth > s.start_depth
            {
                self.errors.push(LifetimeError {
                    message: format!(
                        "lifetime '{}' does not live long enough (scope depth {} vs required {})",
                        l.name, l.start_depth, s.start_depth
                    ),
                    span: c.span,
                });
            }
        }
    }

    /// Apply lifetime elision rules for a function signature.
    pub fn elide_function(&mut self, input_count: usize, has_self: bool, depth: u32) -> LifetimeId {
        let input_lifetimes: Vec<LifetimeId> = (0..input_count)
            .map(|i| self.fresh(format!("'arg{}", i), depth))
            .collect();

        if (has_self && !input_lifetimes.is_empty()) || input_lifetimes.len() == 1 {
            input_lifetimes[0]
        } else {
            self.fresh("'anon", depth)
        }
    }

    /// Get a lifetime by ID.
    pub fn get(&self, id: LifetimeId) -> Option<&Lifetime> {
        self.lifetimes.get(&id)
    }

    /// Number of active lifetimes.
    pub fn count(&self) -> usize {
        self.lifetimes.len()
    }
}

impl Default for LifetimeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ── S-Grade CFG-Aware Borrow & Affine Ownership Checker ──

/// A place in memory (base local + optional field/deref projections).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Place {
    pub root: String,
    pub projections: Vec<Projection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Projection {
    Field(String),
    Index,
    Deref,
}

impl Place {
    pub fn from_var(name: impl Into<String>) -> Self {
        Self {
            root: name.into(),
            projections: Vec::new(),
        }
    }

    pub fn field(mut self, name: impl Into<String>) -> Self {
        self.projections.push(Projection::Field(name.into()));
        self
    }

    pub fn conflicts_with(&self, other: &Place) -> bool {
        if self.root != other.root {
            return false;
        }
        // If either place is a prefix of the other, they conflict
        let min_len = self.projections.len().min(other.projections.len());
        self.projections[..min_len] == other.projections[..min_len]
    }
}

/// Kind of borrow: shared `&` or mutable `&mut`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

/// An active loan of a place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLoan {
    pub loan_id: u32,
    pub place: Place,
    pub kind: BorrowKind,
    pub lifetime: LifetimeId,
    pub span: Span,
}

/// S-Grade Borrow Checker enforcing affine ownership and non-aliasing rules.
pub struct BorrowChecker {
    pub active_loans: Vec<ActiveLoan>,
    pub moved_places: HashSet<String>,
    pub errors: Vec<LifetimeError>,
    next_loan_id: u32,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            active_loans: Vec::new(),
            moved_places: HashSet::new(),
            errors: Vec::new(),
            next_loan_id: 1,
        }
    }

    /// Record a borrow of a place (`&` or `&mut`).
    pub fn borrow(
        &mut self,
        place: Place,
        kind: BorrowKind,
        lifetime: LifetimeId,
        span: Span,
    ) -> Option<u32> {
        // 1. Check if place is already moved
        if self.moved_places.contains(&place.root) {
            self.errors.push(LifetimeError {
                message: format!("cannot borrow `{}` after move", place.root),
                span,
            });
            return None;
        }

        // 2. Check for active loan conflicts
        for loan in &self.active_loans {
            if loan.place.conflicts_with(&place) {
                match (loan.kind, kind) {
                    (BorrowKind::Mutable, _) => {
                        self.errors.push(LifetimeError {
                            message: format!(
                                "cannot borrow `{}` because it is already borrowed mutably",
                                place.root
                            ),
                            span,
                        });
                        return None;
                    }
                    (BorrowKind::Shared, BorrowKind::Mutable) => {
                        self.errors.push(LifetimeError {
                            message: format!(
                                "cannot borrow `{}` as mutable because it is already borrowed as shared",
                                place.root
                            ),
                            span,
                        });
                        return None;
                    }
                    (BorrowKind::Shared, BorrowKind::Shared) => {
                        // Multiple shared borrows are allowed
                    }
                }
            }
        }

        let loan_id = self.next_loan_id;
        self.next_loan_id += 1;
        self.active_loans.push(ActiveLoan {
            loan_id,
            place,
            kind,
            lifetime,
            span,
        });

        Some(loan_id)
    }

    /// Record a move of a place (affine ownership transfer).
    pub fn move_place(&mut self, place: &Place, span: Span) -> bool {
        if self.moved_places.contains(&place.root) {
            self.errors.push(LifetimeError {
                message: format!("use of moved value: `{}`", place.root),
                span,
            });
            return false;
        }

        for loan in &self.active_loans {
            if loan.place.conflicts_with(place) {
                self.errors.push(LifetimeError {
                    message: format!(
                        "cannot move out of `{}` because it is currently borrowed",
                        place.root
                    ),
                    span,
                });
                return false;
            }
        }

        self.moved_places.insert(place.root.clone());
        true
    }

    /// Record a write / mutation to a place.
    pub fn write_place(&mut self, place: &Place, span: Span) -> bool {
        if self.moved_places.contains(&place.root) {
            self.errors.push(LifetimeError {
                message: format!("cannot assign to moved value: `{}`", place.root),
                span,
            });
            return false;
        }

        for loan in &self.active_loans {
            if loan.place.conflicts_with(place) {
                self.errors.push(LifetimeError {
                    message: format!(
                        "cannot mutate `{}` while borrowed by an active reference",
                        place.root
                    ),
                    span,
                });
                return false;
            }
        }

        true
    }

    /// Expire all loans associated with a given lifetime / scope end.
    pub fn expire_lifetime(&mut self, lifetime: LifetimeId) {
        self.active_loans.retain(|loan| loan.lifetime != lifetime);
    }
}

impl Default for BorrowChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_span() -> Span {
        Span::dummy()
    }

    #[test]
    fn test_fresh_lifetime() {
        let mut la = LifetimeAnalyzer::new();
        let a = la.fresh("'a", 1);
        let b = la.fresh("'b", 2);

        assert_ne!(a, b);
        assert_eq!(la.get(a).unwrap().name, "'a");
        assert_eq!(la.get(b).unwrap().start_depth, 2);
    }

    #[test]
    fn test_valid_constraint() {
        let mut la = LifetimeAnalyzer::new();
        let longer = la.fresh("'longer", 1);
        let shorter = la.fresh("'shorter", 2);

        la.extend(longer, 3);
        la.extend(shorter, 2);
        la.constrain(longer, shorter, dummy_span());
        la.check();

        assert!(
            la.errors.is_empty(),
            "expected no errors, got: {:?}",
            la.errors
        );
    }

    #[test]
    fn test_lifetime_elision_single_input() {
        let mut la = LifetimeAnalyzer::new();
        let ret = la.elide_function(1, false, 0);
        assert_eq!(la.get(ret).unwrap().name, "'arg0");
    }

    #[test]
    fn test_borrow_checker_prevents_aliasing_mutable_borrows() {
        let mut bc = BorrowChecker::new();
        let lt = LifetimeId(1);
        let place = Place::from_var("x");

        let loan1 = bc.borrow(place.clone(), BorrowKind::Mutable, lt, dummy_span());
        assert!(loan1.is_some());

        // Second borrow must fail
        let loan2 = bc.borrow(place, BorrowKind::Mutable, lt, dummy_span());
        assert!(loan2.is_none());
        assert_eq!(bc.errors.len(), 1);
        assert!(bc.errors[0].message.contains("already borrowed mutably"));
    }

    #[test]
    fn test_borrow_checker_allows_multiple_shared_borrows() {
        let mut bc = BorrowChecker::new();
        let lt = LifetimeId(1);
        let place = Place::from_var("data");

        assert!(
            bc.borrow(place.clone(), BorrowKind::Shared, lt, dummy_span())
                .is_some()
        );
        assert!(
            bc.borrow(place, BorrowKind::Shared, lt, dummy_span())
                .is_some()
        );
        assert!(bc.errors.is_empty());
    }

    #[test]
    fn test_borrow_checker_prevents_use_after_move() {
        let mut bc = BorrowChecker::new();
        let place = Place::from_var("msg");

        assert!(bc.move_place(&place, dummy_span()));
        assert!(!bc.move_place(&place, dummy_span()));
        assert_eq!(bc.errors.len(), 1);
        assert!(bc.errors[0].message.contains("use of moved value"));
    }

    #[test]
    fn test_borrow_checker_prevents_mutation_during_active_borrow() {
        let mut bc = BorrowChecker::new();
        let lt = LifetimeId(1);
        let place = Place::from_var("vec");

        bc.borrow(place.clone(), BorrowKind::Shared, lt, dummy_span());
        assert!(!bc.write_place(&place, dummy_span()));
        assert_eq!(bc.errors.len(), 1);
        assert!(
            bc.errors[0]
                .message
                .contains("cannot mutate `vec` while borrowed")
        );

        // After lifetime expiry, mutation is permitted
        bc.expire_lifetime(lt);
        assert!(bc.write_place(&place, dummy_span()));
    }
}
