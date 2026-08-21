//! Tier-2 Optimizing JIT Execution Engine & On-Stack Replacement (OSR).
//!
//! Implements adaptive tiered execution (Tier-1 Baseline vs. Tier-2 Optimizing),
//! hot loop back-edge execution counters, mid-loop On-Stack Replacement (OSR),
//! and speculative deoptimization bailouts.

use std::collections::HashMap;

/// Execution tier of a compiled JIT function or loop body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JitTier {
    /// Tier-1 Baseline: Fast compilation with invocation and backedge profiling counters.
    Tier1Baseline,
    /// Tier-2 Optimized: Heavy optimizations (E-Graph, Polyhedral Tiling, Vectorization, Inlining).
    Tier2Optimized,
}

/// Representation of a scalar value stored in an active stack frame local slot.
#[derive(Clone, Debug, PartialEq)]
pub enum StackSlotValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Pointer(usize),
}

/// Active execution state of a stack frame captured at an OSR transition point.
#[derive(Clone, Debug, PartialEq)]
pub struct OsrFrameState {
    pub function_name: String,
    pub block_id: usize,
    pub loop_nest_depth: usize,
    /// Local variable index -> slot value.
    pub locals: HashMap<usize, StackSlotValue>,
    /// Loop induction variable values.
    pub induction_vars: HashMap<String, i64>,
}

impl OsrFrameState {
    pub fn new(function_name: impl Into<String>, block_id: usize) -> Self {
        Self {
            function_name: function_name.into(),
            block_id,
            loop_nest_depth: 0,
            locals: HashMap::new(),
            induction_vars: HashMap::new(),
        }
    }

    pub fn set_local_int(&mut self, local_idx: usize, val: i64) {
        self.locals.insert(local_idx, StackSlotValue::Int(val));
    }

    pub fn set_local_float(&mut self, local_idx: usize, val: f64) {
        self.locals.insert(local_idx, StackSlotValue::Float(val));
    }

    pub fn set_local_bool(&mut self, local_idx: usize, val: bool) {
        self.locals.insert(local_idx, StackSlotValue::Bool(val));
    }

    pub fn set_induction_var(&mut self, name: impl Into<String>, val: i64) {
        self.induction_vars.insert(name.into(), val);
    }
}

/// Reason for a speculative Tier-2 optimization bailout (deoptimization).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DeoptReason {
    TypeGuardFailed {
        expected_type: String,
        actual_type: String,
    },
    ArrayBoundsCheckFailed {
        index: i64,
        length: usize,
    },
    IntegerOverflow,
    DivisionByZero,
    SpeculationInvalidated {
        hint: String,
    },
}

/// Recorded deoptimization event transferring execution back to Tier-1 baseline.
#[derive(Clone, Debug, PartialEq)]
pub struct DeoptBailout {
    pub reason: DeoptReason,
    pub target_block_id: usize,
    pub restored_frame: OsrFrameState,
}

/// OSR execution engine managing loop counters, tiered transitions, and deoptimization.
#[derive(Debug)]
pub struct OsrEngine {
    /// Threshold of loop back-edge iterations before triggering Tier-2 OSR compilation.
    pub hot_loop_threshold: u64,
    /// Loop identifier -> execution iteration count.
    loop_backedge_counters: HashMap<String, u64>,
    /// Function name -> current active tier.
    active_tiers: HashMap<String, JitTier>,
    /// History of deoptimization events.
    deopt_history: Vec<DeoptBailout>,
}

impl Default for OsrEngine {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl OsrEngine {
    pub fn new(hot_loop_threshold: u64) -> Self {
        Self {
            hot_loop_threshold,
            loop_backedge_counters: HashMap::new(),
            active_tiers: HashMap::new(),
            deopt_history: Vec::new(),
        }
    }

    /// Record a loop iteration back-edge.
    ///
    /// Returns `Some(JitTier::Tier2Optimized)` if the loop has crossed the hot threshold
    /// and warrants immediate On-Stack Replacement.
    pub fn record_loop_iteration(&mut self, loop_id: &str, function_name: &str) -> Option<JitTier> {
        let count = self
            .loop_backedge_counters
            .entry(loop_id.to_string())
            .or_insert(0);
        *count += 1;

        if *count >= self.hot_loop_threshold {
            let current_tier = self
                .active_tiers
                .get(function_name)
                .copied()
                .unwrap_or(JitTier::Tier1Baseline);
            if current_tier == JitTier::Tier1Baseline {
                self.active_tiers
                    .insert(function_name.to_string(), JitTier::Tier2Optimized);
                return Some(JitTier::Tier2Optimized);
            }
        }
        None
    }

    /// Perform On-Stack Replacement migration of a running frame to Tier-2.
    pub fn migrate_to_tier2(&self, frame: &OsrFrameState) -> OsrFrameState {
        // Clone frame state with preserved variable slots for Tier-2 entry
        frame.clone()
    }

    /// Trigger a speculative deoptimization bailout from Tier-2 back to Tier-1.
    pub fn trigger_deopt(
        &mut self,
        function_name: &str,
        reason: DeoptReason,
        target_block_id: usize,
        frame: OsrFrameState,
    ) -> DeoptBailout {
        // Downgrade active tier to Tier-1 Baseline
        self.active_tiers
            .insert(function_name.to_string(), JitTier::Tier1Baseline);

        let bailout = DeoptBailout {
            reason,
            target_block_id,
            restored_frame: frame,
        };

        self.deopt_history.push(bailout.clone());
        bailout
    }

    /// Return the current execution tier for a function.
    pub fn get_tier(&self, function_name: &str) -> JitTier {
        self.active_tiers
            .get(function_name)
            .copied()
            .unwrap_or(JitTier::Tier1Baseline)
    }

    /// Total number of deoptimizations that have occurred.
    pub fn deopt_count(&self) -> usize {
        self.deopt_history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osr_threshold_and_tier_upgrade() {
        let mut engine = OsrEngine::new(500);
        assert_eq!(engine.get_tier("matrix_mul"), JitTier::Tier1Baseline);

        // Run 499 iterations: still Tier-1
        for _ in 0..499 {
            assert!(
                engine
                    .record_loop_iteration("matrix_mul_loop1", "matrix_mul")
                    .is_none()
            );
        }
        assert_eq!(engine.get_tier("matrix_mul"), JitTier::Tier1Baseline);

        // 500th iteration: triggers Tier-2 OSR upgrade
        let upgrade = engine.record_loop_iteration("matrix_mul_loop1", "matrix_mul");
        assert_eq!(upgrade, Some(JitTier::Tier2Optimized));
        assert_eq!(engine.get_tier("matrix_mul"), JitTier::Tier2Optimized);
    }

    #[test]
    fn test_osr_frame_state_migration() {
        let engine = OsrEngine::new(100);
        let mut frame = OsrFrameState::new("saxpy", 2);
        frame.set_local_float(0, 3.14159);
        frame.set_local_int(1, 1024);
        frame.set_induction_var("i", 42);

        let migrated = engine.migrate_to_tier2(&frame);
        assert_eq!(migrated.function_name, "saxpy");
        assert_eq!(
            migrated.locals.get(&0),
            Some(&StackSlotValue::Float(3.14159))
        );
        assert_eq!(migrated.locals.get(&1), Some(&StackSlotValue::Int(1024)));
        assert_eq!(migrated.induction_vars.get("i"), Some(&42));
    }

    #[test]
    fn test_speculative_deoptimization_bailout() {
        let mut engine = OsrEngine::new(100);
        // Force Tier-2
        engine
            .active_tiers
            .insert("filter".into(), JitTier::Tier2Optimized);

        let mut frame = OsrFrameState::new("filter", 4);
        frame.set_local_int(0, 9999);

        // Trigger deopt due to failed array bounds check
        let bailout = engine.trigger_deopt(
            "filter",
            DeoptReason::ArrayBoundsCheckFailed {
                index: 100,
                length: 50,
            },
            4,
            frame,
        );

        assert_eq!(engine.get_tier("filter"), JitTier::Tier1Baseline);
        assert_eq!(engine.deopt_count(), 1);
        assert_eq!(bailout.target_block_id, 4);
        assert_eq!(
            bailout.restored_frame.locals.get(&0),
            Some(&StackSlotValue::Int(9999))
        );
    }
}
