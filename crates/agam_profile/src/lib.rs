//! # agam_profile
//!
//! Lightweight profiling models for adaptive runtime decisions.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const CALL_CACHE_PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableScalarValueProfile {
    pub index: usize,
    pub raw_bits: u64,
    pub matches: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallCacheFunctionProfile {
    pub unique_keys: usize,
    pub hottest_key_hits: u64,
    pub avg_reuse_distance: Option<u64>,
    pub max_reuse_distance: Option<u64>,
    pub stable_values: Vec<StableScalarValueProfile>,
    #[serde(default)]
    pub specialization_guard_hits: u64,
    #[serde(default)]
    pub specialization_guard_fallbacks: u64,
    pub specialization_hint: CallCacheSpecializationHint,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallCacheSpecializationHint {
    #[default]
    None,
    StableArguments {
        slots: Vec<usize>,
    },
    HotKey {
        hits: u64,
        unique_keys: usize,
    },
    StableArgumentsAndHotKey {
        slots: Vec<usize>,
        hits: u64,
        unique_keys: usize,
    },
}

impl fmt::Display for CallCacheSpecializationHint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallCacheSpecializationHint::None => write!(f, "none"),
            CallCacheSpecializationHint::StableArguments { slots } => {
                write!(f, "stable scalar arguments at slots {:?}", slots)
            }
            CallCacheSpecializationHint::HotKey { hits, unique_keys } => {
                write!(f, "single hot key ({hits} hits across {unique_keys} keys)")
            }
            CallCacheSpecializationHint::StableArgumentsAndHotKey {
                slots,
                hits,
                unique_keys,
            } => write!(
                f,
                "stable scalar arguments at slots {:?} plus a hot key ({hits} hits across {unique_keys} keys)",
                slots
            ),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptiveAdmissionObservation {
    pub total_calls: u64,
    pub total_hits: u64,
    pub unique_keys: usize,
    pub cached_entries: usize,
    pub capacity: usize,
    pub candidate_hits: u64,
    pub candidate_reuse_distance: Option<u64>,
    pub hottest_key_hits: u64,
    pub stable_argument_slots: usize,
    pub optimize_mode: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptiveAdmissionDecision {
    pub admit: bool,
    pub payoff_score: u32,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallCacheFunctionSnapshot {
    pub name: String,
    pub calls: u64,
    pub hits: u64,
    pub stores: u64,
    pub entries: usize,
    pub profile: CallCacheFunctionProfile,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallCacheRunProfile {
    pub backend: String,
    pub total_calls: u64,
    pub total_hits: u64,
    pub total_stores: u64,
    pub functions: Vec<CallCacheFunctionSnapshot>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentCallCacheFunctionProfile {
    pub name: String,
    pub runs: u64,
    pub total_calls: u64,
    pub total_hits: u64,
    pub total_stores: u64,
    pub last_entries: usize,
    pub profile: CallCacheFunctionProfile,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentCallCacheProfile {
    pub schema_version: u32,
    pub backend: String,
    pub runs: u64,
    pub total_calls: u64,
    pub total_hits: u64,
    pub total_stores: u64,
    pub functions: Vec<PersistentCallCacheFunctionProfile>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallCacheSpecializationPlan {
    pub name: String,
    pub stable_values: Vec<StableScalarValueProfile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReuseDistanceSignal {
    Favorable,
    Neutral,
    Unfavorable,
}

pub fn adaptive_admission_decision(
    observation: &AdaptiveAdmissionObservation,
) -> AdaptiveAdmissionDecision {
    let mut payoff_score = 0u32;
    let mut reasons = Vec::new();

    if observation.candidate_hits >= 2 {
        payoff_score += 40;
        reasons.push("repeated arguments".into());
    }
    if observation.candidate_hits >= 3 {
        payoff_score += 12;
        reasons.push("candidate is already hot".into());
    }
    if let Some(distance) = observation.candidate_reuse_distance {
        let short = (observation.capacity.max(2) as u64) * 2;
        let medium = (observation.capacity.max(2) as u64) * 4;
        if distance <= short {
            payoff_score += 24;
            reasons.push("short reuse distance".into());
        } else if distance <= medium {
            payoff_score += 12;
            reasons.push("moderate reuse distance".into());
        }
    }
    if observation.total_calls > 0 {
        let hit_per_thousand =
            observation.total_hits.saturating_mul(1000) / observation.total_calls;
        if hit_per_thousand >= 250 {
            payoff_score += 18;
            reasons.push("strong hit rate".into());
        } else if hit_per_thousand >= 100 {
            payoff_score += 8;
            reasons.push("growing hit rate".into());
        }
    }
    if observation.total_calls >= 8
        && observation.hottest_key_hits.saturating_mul(2) >= observation.total_calls
    {
        payoff_score += 18;
        reasons.push("dominant hot key".into());
    }
    if observation.stable_argument_slots > 0 {
        payoff_score += 10 * observation.stable_argument_slots.min(2) as u32;
        reasons.push("stable scalar values".into());
    }
    if observation.unique_keys > observation.capacity.saturating_mul(2) {
        payoff_score = payoff_score.saturating_sub(20);
        reasons.push("broad unique-key spread".into());
    }
    if observation.cached_entries >= observation.capacity && observation.capacity > 0 {
        payoff_score = payoff_score.saturating_sub(6);
        reasons.push("cache is full".into());
    }

    let threshold = if observation.optimize_mode { 52 } else { 40 };
    let admit = observation.candidate_hits >= 2 && payoff_score >= threshold;

    AdaptiveAdmissionDecision {
        admit,
        payoff_score,
        reasons,
    }
}

pub fn specialization_hint(
    calls: u64,
    profile: &CallCacheFunctionProfile,
) -> CallCacheSpecializationHint {
    let stable_slots: Vec<usize> = profile
        .stable_values
        .iter()
        .map(|value| value.index)
        .collect();
    let stable = !stable_slots.is_empty() && calls >= 8;
    let hot_key = calls >= 8
        && profile.hottest_key_hits > 0
        && profile.hottest_key_hits.saturating_mul(2) >= calls
        && profile.unique_keys <= 4;

    match (stable, hot_key) {
        (true, true) => CallCacheSpecializationHint::StableArgumentsAndHotKey {
            slots: stable_slots,
            hits: profile.hottest_key_hits,
            unique_keys: profile.unique_keys,
        },
        (true, false) => CallCacheSpecializationHint::StableArguments {
            slots: stable_slots,
        },
        (false, true) => CallCacheSpecializationHint::HotKey {
            hits: profile.hottest_key_hits,
            unique_keys: profile.unique_keys,
        },
        (false, false) => CallCacheSpecializationHint::None,
    }
}

pub fn merge_persistent_profile(
    existing: Option<PersistentCallCacheProfile>,
    run: &CallCacheRunProfile,
) -> PersistentCallCacheProfile {
    let mut merged = existing.unwrap_or_else(|| PersistentCallCacheProfile {
        schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
        backend: run.backend.clone(),
        ..Default::default()
    });

    merged.schema_version = CALL_CACHE_PROFILE_SCHEMA_VERSION;
    merged.backend = run.backend.clone();
    merged.runs = merged.runs.saturating_add(1);
    merged.total_calls = merged.total_calls.saturating_add(run.total_calls);
    merged.total_hits = merged.total_hits.saturating_add(run.total_hits);
    merged.total_stores = merged.total_stores.saturating_add(run.total_stores);

    let mut functions: BTreeMap<String, PersistentCallCacheFunctionProfile> = merged
        .functions
        .into_iter()
        .map(|function| (function.name.clone(), function))
        .collect();

    for snapshot in &run.functions {
        let entry = functions.entry(snapshot.name.clone()).or_insert_with(|| {
            PersistentCallCacheFunctionProfile {
                name: snapshot.name.clone(),
                ..Default::default()
            }
        });
        entry.runs = entry.runs.saturating_add(1);
        entry.total_calls = entry.total_calls.saturating_add(snapshot.calls);
        entry.total_hits = entry.total_hits.saturating_add(snapshot.hits);
        entry.total_stores = entry.total_stores.saturating_add(snapshot.stores);
        entry.last_entries = snapshot.entries;
        entry.profile = merge_function_profile(
            &entry.profile,
            entry.total_calls,
            &snapshot.profile,
            snapshot.calls,
        );
    }

    merged.functions = functions.into_values().collect();
    merged
}

pub fn recommended_optimize_functions(profile: &PersistentCallCacheProfile) -> Vec<String> {
    profile
        .functions
        .iter()
        .filter(|function| function.total_calls >= 16)
        .filter(|function| {
            let hit_rate_per_thousand = if function.total_calls == 0 {
                0
            } else {
                function.total_hits.saturating_mul(1000) / function.total_calls
            };
            let reuse_is_favorable = matches!(
                reuse_distance_signal(function),
                Some(ReuseDistanceSignal::Favorable)
            );
            hit_rate_per_thousand >= 200
                || reuse_is_favorable
                || !matches!(
                    function.profile.specialization_hint,
                    CallCacheSpecializationHint::None
                )
        })
        .map(|function| function.name.clone())
        .collect()
}

pub fn recommended_specializations(
    profile: &PersistentCallCacheProfile,
) -> Vec<CallCacheSpecializationPlan> {
    profile
        .functions
        .iter()
        .filter(|function| function.total_calls >= 16)
        .filter_map(|function| {
            let mut stable_values = function.profile.stable_values.clone();
            stable_values.sort_by_key(|value| value.index);
            stable_values.dedup_by_key(|value| value.index);

            let hinted = matches!(
                function.profile.specialization_hint,
                CallCacheSpecializationHint::StableArguments { .. }
                    | CallCacheSpecializationHint::StableArgumentsAndHotKey { .. }
            );
            let stable_enough = stable_values.iter().any(|value| value.matches >= 8);
            let reuse_is_unfavorable = matches!(
                reuse_distance_signal(function),
                Some(ReuseDistanceSignal::Unfavorable)
            );
            let specialization_feedback_is_unfavorable =
                specialization_feedback_is_unfavorable(&function.profile);
            if hinted
                && stable_enough
                && !reuse_is_unfavorable
                && !specialization_feedback_is_unfavorable
            {
                Some(CallCacheSpecializationPlan {
                    name: function.name.clone(),
                    stable_values,
                })
            } else {
                None
            }
        })
        .collect()
}

fn reuse_distance_signal(
    function: &PersistentCallCacheFunctionProfile,
) -> Option<ReuseDistanceSignal> {
    let working_set = function
        .last_entries
        .max(function.profile.unique_keys)
        .max(1) as u64;
    let avg_favorable_limit = working_set.saturating_mul(4);
    let avg_unfavorable_limit = working_set.saturating_mul(16);
    if let Some(distance) = function.profile.avg_reuse_distance {
        if distance <= avg_favorable_limit {
            return Some(ReuseDistanceSignal::Favorable);
        }
        if distance > avg_unfavorable_limit {
            return Some(ReuseDistanceSignal::Unfavorable);
        }
    }

    let max_favorable_limit = working_set.saturating_mul(8);
    let max_unfavorable_limit = working_set.saturating_mul(32);
    if let Some(distance) = function.profile.max_reuse_distance {
        if distance <= max_favorable_limit {
            return Some(ReuseDistanceSignal::Favorable);
        }
        if distance > max_unfavorable_limit {
            return Some(ReuseDistanceSignal::Unfavorable);
        }
        return Some(ReuseDistanceSignal::Neutral);
    }

    function
        .profile
        .avg_reuse_distance
        .map(|_| ReuseDistanceSignal::Neutral)
}

fn specialization_feedback_is_unfavorable(profile: &CallCacheFunctionProfile) -> bool {
    let attempts = profile
        .specialization_guard_hits
        .saturating_add(profile.specialization_guard_fallbacks);
    attempts >= 8
        && (profile.specialization_guard_hits == 0
            || profile.specialization_guard_hits.saturating_mul(4) < attempts)
}

fn merge_function_profile(
    existing: &CallCacheFunctionProfile,
    total_calls_after_merge: u64,
    incoming: &CallCacheFunctionProfile,
    incoming_calls: u64,
) -> CallCacheFunctionProfile {
    let unique_keys = existing.unique_keys.max(incoming.unique_keys);
    let hottest_key_hits = existing.hottest_key_hits.max(incoming.hottest_key_hits);
    let avg_reuse_distance = weighted_avg_option(
        existing.avg_reuse_distance,
        total_calls_after_merge.saturating_sub(incoming_calls),
        incoming.avg_reuse_distance,
        incoming_calls,
    );
    let max_reuse_distance = match (existing.max_reuse_distance, incoming.max_reuse_distance) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    };

    let mut stable_values: BTreeMap<usize, StableScalarValueProfile> = existing
        .stable_values
        .iter()
        .cloned()
        .map(|value| (value.index, value))
        .collect();
    for value in &incoming.stable_values {
        let replace = stable_values
            .get(&value.index)
            .map(|current| value.matches > current.matches)
            .unwrap_or(true);
        if replace {
            stable_values.insert(value.index, value.clone());
        }
    }

    let specialization_attempts = incoming
        .specialization_guard_hits
        .saturating_add(incoming.specialization_guard_fallbacks);
    let mut merged = CallCacheFunctionProfile {
        unique_keys,
        hottest_key_hits,
        avg_reuse_distance,
        max_reuse_distance,
        stable_values: stable_values.into_values().collect(),
        specialization_guard_hits: if specialization_attempts > 0 {
            incoming.specialization_guard_hits
        } else {
            0
        },
        specialization_guard_fallbacks: if specialization_attempts > 0 {
            incoming.specialization_guard_fallbacks
        } else {
            0
        },
        specialization_hint: CallCacheSpecializationHint::None,
    };
    merged.specialization_hint = specialization_hint(total_calls_after_merge, &merged);
    merged
}

fn weighted_avg_option(
    left: Option<u64>,
    left_weight: u64,
    right: Option<u64>,
    right_weight: u64,
) -> Option<u64> {
    match (left, right) {
        (Some(left_value), Some(right_value)) => {
            let total_weight = left_weight.saturating_add(right_weight);
            if total_weight == 0 {
                None
            } else {
                Some(
                    (left_value.saturating_mul(left_weight)
                        + right_value.saturating_mul(right_weight))
                        / total_weight,
                )
            }
        }
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_admission_rejects_unique_inputs() {
        let decision = adaptive_admission_decision(&AdaptiveAdmissionObservation {
            total_calls: 32,
            total_hits: 0,
            unique_keys: 32,
            cached_entries: 0,
            capacity: 8,
            candidate_hits: 1,
            candidate_reuse_distance: None,
            hottest_key_hits: 1,
            stable_argument_slots: 0,
            optimize_mode: false,
        });
        assert!(!decision.admit);
        assert!(decision.payoff_score < 40);
    }

    #[test]
    fn adaptive_admission_accepts_repeated_short_reuse() {
        let decision = adaptive_admission_decision(&AdaptiveAdmissionObservation {
            total_calls: 12,
            total_hits: 2,
            unique_keys: 2,
            cached_entries: 0,
            capacity: 8,
            candidate_hits: 2,
            candidate_reuse_distance: Some(1),
            hottest_key_hits: 8,
            stable_argument_slots: 1,
            optimize_mode: false,
        });
        assert!(decision.admit);
        assert!(decision.payoff_score >= 40);
    }

    #[test]
    fn specialization_hint_detects_stable_arguments_and_hot_key() {
        let mut profile = CallCacheFunctionProfile {
            unique_keys: 1,
            hottest_key_hits: 32,
            avg_reuse_distance: Some(1),
            max_reuse_distance: Some(1),
            stable_values: vec![StableScalarValueProfile {
                index: 0,
                raw_bits: 33,
                matches: 32,
            }],
            specialization_hint: CallCacheSpecializationHint::None,
            ..Default::default()
        };
        profile.specialization_hint = specialization_hint(32, &profile);
        assert!(matches!(
            profile.specialization_hint,
            CallCacheSpecializationHint::StableArgumentsAndHotKey { .. }
        ));
    }

    #[test]
    fn recommended_specializations_select_stable_argument_profiles() {
        let profile = PersistentCallCacheProfile {
            schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 48,
            total_stores: 2,
            functions: vec![
                PersistentCallCacheFunctionProfile {
                    name: "hot".into(),
                    runs: 2,
                    total_calls: 32,
                    total_hits: 24,
                    total_stores: 1,
                    last_entries: 1,
                    profile: CallCacheFunctionProfile {
                        unique_keys: 1,
                        hottest_key_hits: 24,
                        avg_reuse_distance: Some(1),
                        max_reuse_distance: Some(1),
                        stable_values: vec![StableScalarValueProfile {
                            index: 0,
                            raw_bits: 33,
                            matches: 24,
                        }],
                        specialization_hint: CallCacheSpecializationHint::StableArguments {
                            slots: vec![0],
                        },
                        ..Default::default()
                    },
                },
                PersistentCallCacheFunctionProfile {
                    name: "spread".into(),
                    runs: 2,
                    total_calls: 32,
                    total_hits: 24,
                    total_stores: 1,
                    last_entries: 1,
                    profile: CallCacheFunctionProfile {
                        unique_keys: 1,
                        hottest_key_hits: 24,
                        avg_reuse_distance: Some(1),
                        max_reuse_distance: Some(1),
                        stable_values: vec![],
                        specialization_hint: CallCacheSpecializationHint::HotKey {
                            hits: 24,
                            unique_keys: 1,
                        },
                        ..Default::default()
                    },
                },
            ],
        };

        let plans = recommended_specializations(&profile);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "hot");
        assert_eq!(plans[0].stable_values.len(), 1);
        assert_eq!(plans[0].stable_values[0].raw_bits, 33);
    }

    #[test]
    fn recommended_optimize_functions_use_favorable_reuse_distance() {
        let profile = PersistentCallCacheProfile {
            schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "llvm".into(),
            runs: 2,
            total_calls: 32,
            total_hits: 4,
            total_stores: 2,
            functions: vec![PersistentCallCacheFunctionProfile {
                name: "reusey".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 4,
                total_stores: 2,
                last_entries: 2,
                profile: CallCacheFunctionProfile {
                    unique_keys: 2,
                    hottest_key_hits: 4,
                    avg_reuse_distance: Some(2),
                    max_reuse_distance: Some(3),
                    stable_values: vec![],
                    specialization_hint: CallCacheSpecializationHint::None,
                    ..Default::default()
                },
            }],
        };

        let recommended = recommended_optimize_functions(&profile);
        assert_eq!(recommended, vec!["reusey".to_string()]);
    }

    #[test]
    fn recommended_specializations_skip_unfavorable_reuse_distance() {
        let profile = PersistentCallCacheProfile {
            schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "llvm".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 48,
            total_stores: 2,
            functions: vec![PersistentCallCacheFunctionProfile {
                name: "wide".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 24,
                total_stores: 1,
                last_entries: 1,
                profile: CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 24,
                    avg_reuse_distance: Some(64),
                    max_reuse_distance: Some(128),
                    stable_values: vec![StableScalarValueProfile {
                        index: 0,
                        raw_bits: 33,
                        matches: 24,
                    }],
                    specialization_hint: CallCacheSpecializationHint::StableArguments {
                        slots: vec![0],
                    },
                    ..Default::default()
                },
            }],
        };

        let plans = recommended_specializations(&profile);
        assert!(plans.is_empty());
    }

    #[test]
    fn recommended_specializations_skip_unfavorable_specialization_feedback() {
        let profile = PersistentCallCacheProfile {
            schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "llvm".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 48,
            total_stores: 2,
            functions: vec![PersistentCallCacheFunctionProfile {
                name: "thrashy".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 24,
                total_stores: 1,
                last_entries: 1,
                profile: CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 24,
                    avg_reuse_distance: Some(1),
                    max_reuse_distance: Some(1),
                    stable_values: vec![StableScalarValueProfile {
                        index: 0,
                        raw_bits: 33,
                        matches: 24,
                    }],
                    specialization_guard_hits: 1,
                    specialization_guard_fallbacks: 15,
                    specialization_hint: CallCacheSpecializationHint::StableArguments {
                        slots: vec![0],
                    },
                },
            }],
        };

        let plans = recommended_specializations(&profile);
        assert!(plans.is_empty());
    }

    #[test]
    fn merge_persistent_profile_clears_stale_specialization_feedback_without_attempts() {
        let merged = merge_persistent_profile(
            Some(PersistentCallCacheProfile {
                schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
                backend: "jit".into(),
                runs: 1,
                total_calls: 32,
                total_hits: 24,
                total_stores: 1,
                functions: vec![PersistentCallCacheFunctionProfile {
                    name: "hot".into(),
                    runs: 1,
                    total_calls: 32,
                    total_hits: 24,
                    total_stores: 1,
                    last_entries: 1,
                    profile: CallCacheFunctionProfile {
                        unique_keys: 1,
                        hottest_key_hits: 24,
                        avg_reuse_distance: Some(1),
                        max_reuse_distance: Some(1),
                        stable_values: vec![StableScalarValueProfile {
                            index: 0,
                            raw_bits: 33,
                            matches: 24,
                        }],
                        specialization_guard_hits: 8,
                        specialization_guard_fallbacks: 2,
                        specialization_hint: CallCacheSpecializationHint::StableArguments {
                            slots: vec![0],
                        },
                    },
                }],
            }),
            &CallCacheRunProfile {
                backend: "jit".into(),
                total_calls: 16,
                total_hits: 12,
                total_stores: 1,
                functions: vec![CallCacheFunctionSnapshot {
                    name: "hot".into(),
                    calls: 16,
                    hits: 12,
                    stores: 1,
                    entries: 1,
                    profile: CallCacheFunctionProfile {
                        unique_keys: 1,
                        hottest_key_hits: 12,
                        avg_reuse_distance: Some(1),
                        max_reuse_distance: Some(1),
                        stable_values: vec![StableScalarValueProfile {
                            index: 0,
                            raw_bits: 33,
                            matches: 12,
                        }],
                        specialization_hint: CallCacheSpecializationHint::StableArguments {
                            slots: vec![0],
                        },
                        ..Default::default()
                    },
                }],
            },
        );

        let function = merged
            .functions
            .iter()
            .find(|function| function.name == "hot")
            .expect("missing merged hot profile");
        assert_eq!(function.profile.specialization_guard_hits, 0);
        assert_eq!(function.profile.specialization_guard_fallbacks, 0);
    }
}
