//! Verification result caching for incremental compilation.
//!
//! SMT solving is slow. This module provides a cache to store verification
//! results (e.g. "function `foo` is completely safe") so we don't re-prove
//! properties unless the function's AST or type signature changes.

use agam_ast::NodeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A cache of verification results to speed up incremental compiles.
#[derive(Clone)]
pub struct VerificationCache {
    /// Maps function AST node IDs to their verification status.
    /// In a real compiler, this would map a content hash (BLAKE3) of the function.
    results: Arc<Mutex<HashMap<NodeId, VerificationStatus>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    /// All constraints in this scope were proven Unsat (safe).
    VerifiedSafe,
    /// At least one constraint was Sat (unsafe/potential panic).
    Failed,
    /// The solver couldn't determine safety.
    Unknown,
}

impl VerificationCache {
    pub fn new() -> Self {
        Self {
            results: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if a function's safety was already verified.
    pub fn get_status(&self, id: NodeId) -> Option<VerificationStatus> {
        let lock = self.results.lock().unwrap();
        lock.get(&id).copied()
    }

    /// Store a verification result.
    pub fn set_status(&self, id: NodeId, status: VerificationStatus) {
        let mut lock = self.results.lock().unwrap();
        lock.insert(id, status);
    }

    /// Clear the cache.
    pub fn clear(&self) {
        let mut lock = self.results.lock().unwrap();
        lock.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_cache() {
        let cache = VerificationCache::new();
        let id1 = NodeId(1);
        let id2 = NodeId(2);

        assert_eq!(cache.get_status(id1), None);

        cache.set_status(id1, VerificationStatus::VerifiedSafe);
        cache.set_status(id2, VerificationStatus::Failed);

        assert_eq!(
            cache.get_status(id1),
            Some(VerificationStatus::VerifiedSafe)
        );
        assert_eq!(cache.get_status(id2), Some(VerificationStatus::Failed));

        cache.clear();
        assert_eq!(cache.get_status(id1), None);
    }
}
