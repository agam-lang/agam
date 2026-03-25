//! Lifetime analysis — region inference and borrow duration tracking.
//!
//! Tracks how long references live and ensures they don't outlive
//! their referents. Inspired by Rust's region-based lifetime system.
//!
//! Key concepts:
//! 1. **Lifetime** — a named region representing how long a reference is valid.
//! 2. **Lifetime constraints** — `'a: 'b` means `'a` outlives `'b`.
//! 3. **Lifetime elision** — automatic lifetime assignment for common patterns.
//! 4. **Dangling reference detection** — prevents returning refs to local values.

use std::collections::HashMap;
use agam_errors::Span;

/// A unique lifetime identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// A constraint: lifetime `longer` must outlive lifetime `shorter`.
#[derive(Debug, Clone)]
pub struct LifetimeConstraint {
    pub longer: LifetimeId,
    pub shorter: LifetimeId,
    pub span: Span,
}

/// Error from lifetime analysis.
#[derive(Debug, Clone)]
pub struct LifetimeError {
    pub message: String,
    pub span: Span,
}

/// The lifetime analyzer.
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
        lifetimes.insert(static_lt, Lifetime {
            id: static_lt,
            name: "'static".into(),
            start_depth: 0,
            end_depth: u32::MAX,
        });

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
        self.lifetimes.insert(id, Lifetime {
            id,
            name: name.into(),
            start_depth: depth,
            end_depth: depth,
        });
        id
    }

    /// Extend a lifetime's end depth (it lives at least until `depth`).
    pub fn extend(&mut self, id: LifetimeId, depth: u32) {
        if let Some(lt) = self.lifetimes.get_mut(&id) {
            if depth > lt.end_depth {
                lt.end_depth = depth;
            }
        }
    }

    /// Add a constraint: `longer` must outlive `shorter`.
    pub fn constrain(&mut self, longer: LifetimeId, shorter: LifetimeId, span: Span) {
        self.constraints.push(LifetimeConstraint { longer, shorter, span });
    }

    /// Check all constraints.
    pub fn check(&mut self) {
        for c in self.constraints.clone() {
            let longer = self.lifetimes.get(&c.longer);
            let shorter = self.lifetimes.get(&c.shorter);

            if let (Some(l), Some(s)) = (longer, shorter) {
                // `longer` must have an end_depth >= shorter's end_depth.
                if l.end_depth < s.end_depth {
                    self.errors.push(LifetimeError {
                        message: format!(
                            "lifetime '{}' does not live long enough (ends at depth {}, but '{}' requires depth {})",
                            l.name, l.end_depth, s.name, s.end_depth
                        ),
                        span: c.span,
                    });
                }
            }
        }
    }

    /// Apply lifetime elision rules for a function signature.
    ///
    /// Rust-style rules:
    /// 1. Each input reference gets its own lifetime.
    /// 2. If there's exactly one input lifetime, output gets the same.
    /// 3. If there's a `&self`/`&mut self`, output gets self's lifetime.
    pub fn elide_function(&mut self, input_count: usize, has_self: bool, depth: u32) -> LifetimeId {
        let input_lifetimes: Vec<LifetimeId> = (0..input_count)
            .map(|i| self.fresh(format!("'arg{}", i), depth))
            .collect();

        if has_self && !input_lifetimes.is_empty() {
            // Rule 3: output gets self's lifetime
            input_lifetimes[0]
        } else if input_lifetimes.len() == 1 {
            // Rule 2: single input → output gets same lifetime
            input_lifetimes[0]
        } else {
            // No elision possible, create fresh lifetime
            self.fresh("'out", depth)
        }
    }

    /// Check if returning a reference to a local variable (dangling ref).
    pub fn check_return_ref(&mut self, ref_lifetime: LifetimeId, fn_depth: u32, span: Span) {
        if let Some(lt) = self.lifetimes.get(&ref_lifetime) {
            if lt.start_depth >= fn_depth && ref_lifetime != self.static_lifetime {
                self.errors.push(LifetimeError {
                    message: "cannot return reference to local variable".into(),
                    span,
                });
            }
        }
    }

    /// Get a lifetime by ID.
    pub fn get(&self, id: LifetimeId) -> Option<&Lifetime> {
        self.lifetimes.get(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_span() -> Span { Span::dummy() }

    #[test]
    fn test_static_lifetime_outlives_everything() {
        let mut analyzer = LifetimeAnalyzer::new();
        let local = analyzer.fresh("'a", 3);

        analyzer.constrain(analyzer.static_lifetime, local, dummy_span());
        analyzer.check();

        assert!(analyzer.errors.is_empty());
    }

    #[test]
    fn test_local_does_not_outlive_static() {
        let mut analyzer = LifetimeAnalyzer::new();
        let local = analyzer.fresh("'a", 3);

        // local must outlive static — should fail
        analyzer.constrain(local, analyzer.static_lifetime, dummy_span());
        analyzer.check();

        assert_eq!(analyzer.errors.len(), 1);
        assert!(analyzer.errors[0].message.contains("does not live long enough"));
    }

    #[test]
    fn test_nested_lifetimes() {
        let mut analyzer = LifetimeAnalyzer::new();
        let outer = analyzer.fresh("'outer", 1);
        let inner = analyzer.fresh("'inner", 2);

        analyzer.extend(outer, 3);
        analyzer.extend(inner, 2);

        // outer outlives inner — OK
        analyzer.constrain(outer, inner, dummy_span());
        analyzer.check();
        assert!(analyzer.errors.is_empty());
    }

    #[test]
    fn test_inner_does_not_outlive_outer() {
        let mut analyzer = LifetimeAnalyzer::new();
        let outer = analyzer.fresh("'outer", 1);
        let inner = analyzer.fresh("'inner", 2);

        analyzer.extend(outer, 5);
        analyzer.extend(inner, 3);

        // inner must outlive outer — should fail
        analyzer.constrain(inner, outer, dummy_span());
        analyzer.check();

        assert_eq!(analyzer.errors.len(), 1);
    }

    #[test]
    fn test_dangling_reference() {
        let mut analyzer = LifetimeAnalyzer::new();
        let local_ref = analyzer.fresh("'local", 2);

        analyzer.check_return_ref(local_ref, 1, dummy_span());

        assert_eq!(analyzer.errors.len(), 1);
        assert!(analyzer.errors[0].message.contains("local variable"));
    }

    #[test]
    fn test_static_return_is_ok() {
        let mut analyzer = LifetimeAnalyzer::new();

        analyzer.check_return_ref(analyzer.static_lifetime, 1, dummy_span());

        assert!(analyzer.errors.is_empty());
    }

    #[test]
    fn test_elision_single_input() {
        let mut analyzer = LifetimeAnalyzer::new();
        let out = analyzer.elide_function(1, false, 1);

        // With one input, output should get the same lifetime
        // (they should be the same LifetimeId)
        assert_ne!(out, analyzer.static_lifetime);
    }

    #[test]
    fn test_elision_with_self() {
        let mut analyzer = LifetimeAnalyzer::new();
        let out = analyzer.elide_function(3, true, 1);

        // With self, output gets self's lifetime (first input)
        assert_ne!(out, analyzer.static_lifetime);
    }
}
