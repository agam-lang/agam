//! Ownership and borrow analysis.
//!
//! Tracks value ownership, move semantics, and borrow rules:
//!
//! ## Dual Memory Mode
//! - **ARC mode** (default): Automatic Reference Counting. No move errors,
//!   no borrow conflicts. Values are refcounted at runtime.
//! - **Strict mode** (opt-in via `strict { }` blocks): Full Rust-like
//!   ownership — use-after-move, borrow exclusivity, mutability enforcement.
//!
//! ## Rules (Strict mode only)
//! 1. **Move tracking** — values are moved on assignment; use-after-move is an error.
//! 2. **Borrow tracking** — shared (`&T`) and exclusive (`&mut T`) borrows.
//! 3. **Mutability checking** — ensures only `mut` bindings are assigned to.
//! 4. **Drop analysis** — values are dropped at scope exit.

use crate::symbol::SymbolId;
use agam_errors::Span;
use std::collections::HashMap;

/// The memory management mode for a scope or module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMode {
    /// Automatic Reference Counting — default, no borrow checker.
    /// Values are retained/released at runtime. Moves become copies.
    ARC,
    /// Strict ownership — zero-cost, Rust-like lifetimes and borrow rules.
    /// Opt-in via `strict { }` blocks for performance-critical code.
    Strict,
}

impl Default for MemoryMode {
    fn default() -> Self {
        MemoryMode::ARC
    }
}

/// The ownership state of a binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipState {
    /// The binding owns the value and it is live.
    Owned,
    /// The value has been moved out of this binding.
    Moved,
    /// The value has been dropped (scope exit).
    Dropped,
}

/// The borrow state of a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    /// Shared / immutable borrow: `&T`
    Shared,
    /// Exclusive / mutable borrow: `&mut T`
    Exclusive,
}

/// An active borrow on a variable.
#[derive(Debug, Clone)]
pub struct ActiveBorrow {
    /// The variable being borrowed.
    pub target: SymbolId,
    /// What kind of borrow.
    pub kind: BorrowKind,
    /// Where the borrow was created.
    pub span: Span,
}

/// An error from ownership/borrow analysis.
#[derive(Debug, Clone)]
pub struct OwnershipError {
    pub message: String,
    pub span: Span,
}

/// Ownership and borrow tracker with dual memory mode support.
pub struct OwnershipTracker {
    /// Current memory mode.
    pub mode: MemoryMode,
    /// Current ownership state of each symbol.
    states: HashMap<SymbolId, OwnershipState>,
    /// Whether each symbol is declared mutable.
    mutability: HashMap<SymbolId, bool>,
    /// Active borrows on symbols.
    borrows: Vec<ActiveBorrow>,
    /// Errors accumulated during analysis.
    pub errors: Vec<OwnershipError>,
    /// ARC retain count (for diagnostics/codegen hints).
    arc_retains: HashMap<SymbolId, u32>,
}

impl OwnershipTracker {
    pub fn new() -> Self {
        Self::with_mode(MemoryMode::ARC)
    }

    pub fn with_mode(mode: MemoryMode) -> Self {
        Self {
            mode,
            states: HashMap::new(),
            mutability: HashMap::new(),
            borrows: Vec::new(),
            errors: Vec::new(),
            arc_retains: HashMap::new(),
        }
    }

    /// Enter a strict block — switch to strict ownership mode.
    pub fn enter_strict(&mut self) -> MemoryMode {
        let prev = self.mode;
        self.mode = MemoryMode::Strict;
        prev
    }

    /// Exit a strict block — restore the previous mode.
    pub fn exit_strict(&mut self, prev: MemoryMode) {
        self.mode = prev;
    }

    /// Register a new binding.
    pub fn declare(&mut self, sym: SymbolId, mutable: bool) {
        self.states.insert(sym, OwnershipState::Owned);
        self.mutability.insert(sym, mutable);
        if self.mode == MemoryMode::ARC {
            self.arc_retains.insert(sym, 1);
        }
    }

    /// Record a move of a value out of a binding.
    pub fn record_move(&mut self, sym: SymbolId, span: Span) {
        match self.mode {
            MemoryMode::ARC => {
                // In ARC mode, "moves" become retain (copy the reference).
                *self.arc_retains.entry(sym).or_insert(1) += 1;
                // Value stays Owned — no use-after-move.
            }
            MemoryMode::Strict => match self.states.get(&sym) {
                Some(OwnershipState::Moved) => {
                    self.errors.push(OwnershipError {
                        message: format!("use of moved value"),
                        span,
                    });
                }
                Some(OwnershipState::Dropped) => {
                    self.errors.push(OwnershipError {
                        message: format!("use of dropped value"),
                        span,
                    });
                }
                _ => {
                    self.states.insert(sym, OwnershipState::Moved);
                }
            },
        }
    }

    /// Record a use (read) of a binding — checks that it hasn't been moved.
    pub fn record_use(&mut self, sym: SymbolId, span: Span) {
        if self.mode == MemoryMode::ARC {
            return; // ARC mode: always valid (refcounted).
        }
        match self.states.get(&sym) {
            Some(OwnershipState::Moved) => {
                self.errors.push(OwnershipError {
                    message: format!("use of moved value"),
                    span,
                });
            }
            Some(OwnershipState::Dropped) => {
                self.errors.push(OwnershipError {
                    message: format!("use of dropped value"),
                    span,
                });
            }
            _ => {}
        }
    }

    /// Record an assignment to a binding — checks mutability.
    pub fn record_assign(&mut self, sym: SymbolId, span: Span) {
        if self.mode == MemoryMode::ARC {
            // ARC mode: assignment always works (old value is released).
            self.states.insert(sym, OwnershipState::Owned);
            return;
        }
        match self.mutability.get(&sym) {
            Some(false) => {
                self.errors.push(OwnershipError {
                    message: format!("cannot assign to immutable binding"),
                    span,
                });
            }
            _ => {
                // Re-owning: a moved value can be reassigned.
                self.states.insert(sym, OwnershipState::Owned);
            }
        }
    }

    /// Create a shared borrow on a symbol.
    pub fn borrow_shared(&mut self, target: SymbolId, span: Span) {
        if self.mode == MemoryMode::ARC {
            return; // ARC mode: borrows are unrestricted.
        }
        // Check for existing exclusive borrow.
        if self.has_exclusive_borrow(target) {
            self.errors.push(OwnershipError {
                message: format!("cannot borrow as shared: already borrowed as mutable"),
                span,
            });
            return;
        }
        self.borrows.push(ActiveBorrow {
            target,
            kind: BorrowKind::Shared,
            span,
        });
    }

    /// Create an exclusive borrow on a symbol.
    pub fn borrow_exclusive(&mut self, target: SymbolId, span: Span) {
        if self.mode == MemoryMode::ARC {
            return; // ARC mode: borrows are unrestricted.
        }
        // Check mutability.
        if self.mutability.get(&target) == Some(&false) {
            self.errors.push(OwnershipError {
                message: format!("cannot borrow as mutable: binding is not declared `mut`"),
                span,
            });
            return;
        }
        // Check for any active borrows.
        if self.has_any_borrow(target) {
            self.errors.push(OwnershipError {
                message: format!("cannot borrow as mutable: already borrowed"),
                span,
            });
            return;
        }
        self.borrows.push(ActiveBorrow {
            target,
            kind: BorrowKind::Exclusive,
            span,
        });
    }

    /// Release all borrows on a symbol (e.g. when the borrow goes out of scope).
    pub fn release_borrows(&mut self, target: SymbolId) {
        self.borrows.retain(|b| b.target != target);
    }

    /// Drop all symbols from a list (scope exit).
    pub fn drop_scope(&mut self, symbols: &[SymbolId]) {
        for &sym in symbols {
            if self.mode == MemoryMode::ARC {
                // ARC mode: decrement refcount instead of hard drop.
                if let Some(rc) = self.arc_retains.get_mut(&sym) {
                    *rc = rc.saturating_sub(1);
                }
            }
            self.states.insert(sym, OwnershipState::Dropped);
            self.release_borrows(sym);
        }
    }

    /// Get the ARC retain count for a symbol (for codegen).
    pub fn arc_retain_count(&self, sym: SymbolId) -> u32 {
        self.arc_retains.get(&sym).copied().unwrap_or(0)
    }

    /// Check if a symbol has an active exclusive borrow.
    fn has_exclusive_borrow(&self, target: SymbolId) -> bool {
        self.borrows
            .iter()
            .any(|b| b.target == target && b.kind == BorrowKind::Exclusive)
    }

    /// Check if a symbol has any active borrow.
    fn has_any_borrow(&self, target: SymbolId) -> bool {
        self.borrows.iter().any(|b| b.target == target)
    }

    /// Get the current state of a symbol.
    pub fn state(&self, sym: SymbolId) -> Option<OwnershipState> {
        self.states.get(&sym).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_span() -> Span {
        Span::dummy()
    }

    // ── Strict mode tests (original) ──

    #[test]
    fn test_declare_and_use() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let sym = SymbolId(0);
        tracker.declare(sym, false);
        tracker.record_use(sym, dummy_span());
        assert!(tracker.errors.is_empty());
    }

    #[test]
    fn test_use_after_move() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let sym = SymbolId(0);
        tracker.declare(sym, false);
        tracker.record_move(sym, dummy_span());
        tracker.record_use(sym, dummy_span());

        assert_eq!(tracker.errors.len(), 1);
        assert!(tracker.errors[0].message.contains("moved value"));
    }

    #[test]
    fn test_double_move() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let sym = SymbolId(0);
        tracker.declare(sym, false);
        tracker.record_move(sym, dummy_span());
        tracker.record_move(sym, dummy_span());

        assert_eq!(tracker.errors.len(), 1);
        assert!(tracker.errors[0].message.contains("moved value"));
    }

    #[test]
    fn test_assign_to_immutable() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let sym = SymbolId(0);
        tracker.declare(sym, false); // immutable
        tracker.record_assign(sym, dummy_span());

        assert_eq!(tracker.errors.len(), 1);
        assert!(tracker.errors[0].message.contains("immutable"));
    }

    #[test]
    fn test_assign_to_mutable() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let sym = SymbolId(0);
        tracker.declare(sym, true); // mutable
        tracker.record_assign(sym, dummy_span());

        assert!(tracker.errors.is_empty());
    }

    #[test]
    fn test_reassign_moved_value() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let sym = SymbolId(0);
        tracker.declare(sym, true);
        tracker.record_move(sym, dummy_span());
        // Reassigning should re-own the value
        tracker.record_assign(sym, dummy_span());
        tracker.record_use(sym, dummy_span());

        assert!(tracker.errors.is_empty());
        assert_eq!(tracker.state(sym), Some(OwnershipState::Owned));
    }

    #[test]
    fn test_shared_borrow_ok() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let sym = SymbolId(0);
        tracker.declare(sym, false);
        tracker.borrow_shared(sym, dummy_span());
        tracker.borrow_shared(sym, dummy_span()); // multiple shared borrows OK

        assert!(tracker.errors.is_empty());
    }

    #[test]
    fn test_exclusive_borrow_conflicts_with_shared() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let sym = SymbolId(0);
        tracker.declare(sym, true);
        tracker.borrow_shared(sym, dummy_span());
        tracker.borrow_exclusive(sym, dummy_span()); // conflict!

        assert_eq!(tracker.errors.len(), 1);
        assert!(tracker.errors[0].message.contains("already borrowed"));
    }

    #[test]
    fn test_exclusive_borrow_requires_mut() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let sym = SymbolId(0);
        tracker.declare(sym, false); // immutable
        tracker.borrow_exclusive(sym, dummy_span());

        assert_eq!(tracker.errors.len(), 1);
        assert!(tracker.errors[0].message.contains("not declared `mut`"));
    }

    #[test]
    fn test_drop_scope() {
        let mut tracker = OwnershipTracker::with_mode(MemoryMode::Strict);
        let s1 = SymbolId(0);
        let s2 = SymbolId(1);
        tracker.declare(s1, false);
        tracker.declare(s2, false);

        tracker.drop_scope(&[s1, s2]);

        assert_eq!(tracker.state(s1), Some(OwnershipState::Dropped));
        assert_eq!(tracker.state(s2), Some(OwnershipState::Dropped));

        // Use after drop should error
        tracker.record_use(s1, dummy_span());
        assert_eq!(tracker.errors.len(), 1);
        assert!(tracker.errors[0].message.contains("dropped"));
    }

    // ── ARC mode tests ──

    #[test]
    fn test_arc_no_use_after_move_error() {
        let mut tracker = OwnershipTracker::new(); // default = ARC
        assert_eq!(tracker.mode, MemoryMode::ARC);

        let sym = SymbolId(0);
        tracker.declare(sym, false);
        tracker.record_move(sym, dummy_span());
        tracker.record_use(sym, dummy_span());

        // ARC mode: no errors — value is refcounted
        assert!(tracker.errors.is_empty());
    }

    #[test]
    fn test_arc_assign_to_immutable_ok() {
        let mut tracker = OwnershipTracker::new(); // ARC
        let sym = SymbolId(0);
        tracker.declare(sym, false); // immutable is fine in ARC mode
        tracker.record_assign(sym, dummy_span());

        assert!(tracker.errors.is_empty());
    }

    #[test]
    fn test_arc_borrows_unrestricted() {
        let mut tracker = OwnershipTracker::new(); // ARC
        let sym = SymbolId(0);
        tracker.declare(sym, false);
        tracker.borrow_shared(sym, dummy_span());
        tracker.borrow_exclusive(sym, dummy_span()); // No conflict in ARC mode!

        assert!(tracker.errors.is_empty());
    }

    #[test]
    fn test_arc_retain_count() {
        let mut tracker = OwnershipTracker::new(); // ARC
        let sym = SymbolId(0);
        tracker.declare(sym, false);
        assert_eq!(tracker.arc_retain_count(sym), 1);

        tracker.record_move(sym, dummy_span()); // retain
        assert_eq!(tracker.arc_retain_count(sym), 2);

        tracker.record_move(sym, dummy_span()); // retain again
        assert_eq!(tracker.arc_retain_count(sym), 3);
    }

    #[test]
    fn test_strict_block_toggle() {
        let mut tracker = OwnershipTracker::new(); // starts ARC
        assert_eq!(tracker.mode, MemoryMode::ARC);

        let prev = tracker.enter_strict();
        assert_eq!(tracker.mode, MemoryMode::Strict);

        // Now strict rules apply
        let sym = SymbolId(0);
        tracker.declare(sym, false);
        tracker.record_move(sym, dummy_span());
        tracker.record_use(sym, dummy_span()); // ERROR in strict mode

        assert_eq!(tracker.errors.len(), 1);

        tracker.exit_strict(prev);
        assert_eq!(tracker.mode, MemoryMode::ARC);
    }
}
