//! # agam_profile
//!
//! Lightweight profiling models for adaptive runtime decisions.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const CALL_CACHE_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const ADAPTIVE_ADMISSION_MIN_CANDIDATE_HITS: u64 = 2;
pub const ADAPTIVE_ADMISSION_HOT_CANDIDATE_HITS: u64 = 3;
pub const ADAPTIVE_ADMISSION_SHORT_REUSE_WINDOW_MULTIPLIER: u64 = 2;
pub const ADAPTIVE_ADMISSION_MEDIUM_REUSE_WINDOW_MULTIPLIER: u64 = 4;
pub const ADAPTIVE_ADMISSION_GROWING_HIT_RATE_PER_THOUSAND: u64 = 100;
pub const ADAPTIVE_ADMISSION_STRONG_HIT_RATE_PER_THOUSAND: u64 = 250;
pub const ADAPTIVE_ADMISSION_MIN_CALLS_FOR_DOMINANT_HOT_KEY: u64 = 8;
pub const ADAPTIVE_ADMISSION_REPEATED_ARGUMENTS_SCORE: u32 = 40;
pub const ADAPTIVE_ADMISSION_ALREADY_HOT_SCORE: u32 = 12;
pub const ADAPTIVE_ADMISSION_SHORT_REUSE_SCORE: u32 = 24;
pub const ADAPTIVE_ADMISSION_MEDIUM_REUSE_SCORE: u32 = 12;
pub const ADAPTIVE_ADMISSION_GROWING_HIT_RATE_SCORE: u32 = 8;
pub const ADAPTIVE_ADMISSION_STRONG_HIT_RATE_SCORE: u32 = 18;
pub const ADAPTIVE_ADMISSION_DOMINANT_HOT_KEY_SCORE: u32 = 18;
pub const ADAPTIVE_ADMISSION_STABLE_ARGUMENT_SLOT_SCORE: u32 = 10;
pub const ADAPTIVE_ADMISSION_BROAD_KEY_SPREAD_PENALTY: u32 = 20;
pub const ADAPTIVE_ADMISSION_CACHE_FULL_PENALTY: u32 = 6;
pub const ADAPTIVE_ADMISSION_BASIC_THRESHOLD: u32 = 40;
pub const ADAPTIVE_ADMISSION_OPTIMIZE_THRESHOLD: u32 = 52;
pub const SPECIALIZATION_FEEDBACK_MIN_ATTEMPTS: u64 = 8;
pub const SPECIALIZATION_FEEDBACK_FAVORABLE_SCORE_BONUS: u32 = 16;
pub const SPECIALIZATION_FEEDBACK_UNFAVORABLE_SCORE_PENALTY: u32 = 18;
pub const SPECIALIZATION_FEEDBACK_FAVORABLE_MULTIPLIER: u64 = 2;
pub const SPECIALIZATION_FEEDBACK_UNFAVORABLE_MULTIPLIER: u64 = 4;
pub const SPECIALIZATION_PLAN_MIN_MATCHES: u64 = 8;

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
    #[serde(default)]
    pub specialization_profiles: Vec<CallCacheSpecializationFeedbackProfile>,
    pub specialization_hint: CallCacheSpecializationHint,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallCacheSpecializationFeedbackProfile {
    pub stable_values: Vec<StableScalarValueProfile>,
    #[serde(default)]
    pub guard_hits: u64,
    #[serde(default)]
    pub guard_fallbacks: u64,
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
    #[serde(default)]
    pub specialization_guard_hits: u64,
    #[serde(default)]
    pub specialization_guard_fallbacks: u64,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpecializationFeedbackSignal {
    Favorable,
    Neutral,
    Unfavorable,
}

pub fn adaptive_admission_decision(
    observation: &AdaptiveAdmissionObservation,
) -> AdaptiveAdmissionDecision {
    let mut payoff_score = 0u32;
    let mut reasons = Vec::new();

    if observation.candidate_hits >= ADAPTIVE_ADMISSION_MIN_CANDIDATE_HITS {
        payoff_score += ADAPTIVE_ADMISSION_REPEATED_ARGUMENTS_SCORE;
        reasons.push("repeated arguments".into());
    }
    if observation.candidate_hits >= ADAPTIVE_ADMISSION_HOT_CANDIDATE_HITS {
        payoff_score += ADAPTIVE_ADMISSION_ALREADY_HOT_SCORE;
        reasons.push("candidate is already hot".into());
    }
    if let Some(distance) = observation.candidate_reuse_distance {
        let short =
            (observation.capacity.max(2) as u64) * ADAPTIVE_ADMISSION_SHORT_REUSE_WINDOW_MULTIPLIER;
        let medium = (observation.capacity.max(2) as u64)
            * ADAPTIVE_ADMISSION_MEDIUM_REUSE_WINDOW_MULTIPLIER;
        if distance <= short {
            payoff_score += ADAPTIVE_ADMISSION_SHORT_REUSE_SCORE;
            reasons.push("short reuse distance".into());
        } else if distance <= medium {
            payoff_score += ADAPTIVE_ADMISSION_MEDIUM_REUSE_SCORE;
            reasons.push("moderate reuse distance".into());
        }
    }
    if observation.total_calls > 0 {
        let hit_per_thousand =
            observation.total_hits.saturating_mul(1000) / observation.total_calls;
        if hit_per_thousand >= ADAPTIVE_ADMISSION_STRONG_HIT_RATE_PER_THOUSAND {
            payoff_score += ADAPTIVE_ADMISSION_STRONG_HIT_RATE_SCORE;
            reasons.push("strong hit rate".into());
        } else if hit_per_thousand >= ADAPTIVE_ADMISSION_GROWING_HIT_RATE_PER_THOUSAND {
            payoff_score += ADAPTIVE_ADMISSION_GROWING_HIT_RATE_SCORE;
            reasons.push("growing hit rate".into());
        }
    }
    if observation.total_calls >= ADAPTIVE_ADMISSION_MIN_CALLS_FOR_DOMINANT_HOT_KEY
        && observation.hottest_key_hits.saturating_mul(2) >= observation.total_calls
    {
        payoff_score += ADAPTIVE_ADMISSION_DOMINANT_HOT_KEY_SCORE;
        reasons.push("dominant hot key".into());
    }
    if observation.stable_argument_slots > 0 {
        payoff_score += ADAPTIVE_ADMISSION_STABLE_ARGUMENT_SLOT_SCORE
            * observation.stable_argument_slots.min(2) as u32;
        reasons.push("stable scalar values".into());
    }
    if observation.unique_keys > observation.capacity.saturating_mul(2) {
        payoff_score = payoff_score.saturating_sub(ADAPTIVE_ADMISSION_BROAD_KEY_SPREAD_PENALTY);
        reasons.push("broad unique-key spread".into());
    }
    if observation.cached_entries >= observation.capacity && observation.capacity > 0 {
        payoff_score = payoff_score.saturating_sub(ADAPTIVE_ADMISSION_CACHE_FULL_PENALTY);
        reasons.push("cache is full".into());
    }
    match specialization_feedback_signal_from_counts(
        observation.specialization_guard_hits,
        observation.specialization_guard_fallbacks,
    ) {
        Some(SpecializationFeedbackSignal::Favorable) => {
            payoff_score =
                payoff_score.saturating_add(SPECIALIZATION_FEEDBACK_FAVORABLE_SCORE_BONUS);
            reasons.push("specialization guard matches".into());
        }
        Some(SpecializationFeedbackSignal::Unfavorable) => {
            payoff_score =
                payoff_score.saturating_sub(SPECIALIZATION_FEEDBACK_UNFAVORABLE_SCORE_PENALTY);
            reasons.push("specialization guard mismatches".into());
        }
        Some(SpecializationFeedbackSignal::Neutral) | None => {}
    }

    let threshold = if observation.optimize_mode {
        ADAPTIVE_ADMISSION_OPTIMIZE_THRESHOLD
    } else {
        ADAPTIVE_ADMISSION_BASIC_THRESHOLD
    };
    let admit = observation.candidate_hits >= ADAPTIVE_ADMISSION_MIN_CANDIDATE_HITS
        && payoff_score >= threshold;

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
            let specialization_feedback_is_favorable =
                specialization_feedback_is_favorable(&function.profile);
            hit_rate_per_thousand >= 200
                || reuse_is_favorable
                || specialization_feedback_is_favorable
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
        .flat_map(|function| {
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
            let function_specialization_feedback_is_favorable =
                specialization_feedback_is_favorable(&function.profile);
            let plan_feedback_is_favorable =
                function
                    .profile
                    .specialization_profiles
                    .iter()
                    .any(|specialization| {
                        matches!(
                            specialization_feedback_signal_from_counts(
                                specialization.guard_hits,
                                specialization.guard_fallbacks,
                            ),
                            Some(SpecializationFeedbackSignal::Favorable)
                        )
                    });
            if hinted
                && (stable_enough
                    || function_specialization_feedback_is_favorable
                    || plan_feedback_is_favorable)
                && !reuse_is_unfavorable
            {
                specialization_plan_candidates(
                    &function.name,
                    &stable_values,
                    function_specialization_feedback_is_favorable || plan_feedback_is_favorable,
                )
                .into_iter()
                .filter(|plan| {
                    !specialization_feedback_is_unfavorable_for_values(
                        &function.profile,
                        &plan.stable_values,
                    )
                })
                .collect::<Vec<_>>()
            } else {
                Vec::new()
            }
        })
        .collect()
}

fn specialization_plan_candidates(
    function_name: &str,
    stable_values: &[StableScalarValueProfile],
    specialization_feedback_is_favorable: bool,
) -> Vec<CallCacheSpecializationPlan> {
    let mut ranked_values = stable_values.to_vec();
    ranked_values.sort_by(|left, right| {
        right
            .matches
            .cmp(&left.matches)
            .then_with(|| left.index.cmp(&right.index))
    });

    let mut candidate_values: Vec<_> = ranked_values
        .iter()
        .filter(|value| value.matches >= SPECIALIZATION_PLAN_MIN_MATCHES)
        .cloned()
        .collect();
    if candidate_values.is_empty() && specialization_feedback_is_favorable {
        candidate_values = ranked_values;
    }
    if candidate_values.is_empty() {
        return Vec::new();
    }

    let mut plans = Vec::with_capacity(candidate_values.len());
    for prefix_len in (1..=candidate_values.len()).rev() {
        let mut plan_values = candidate_values[..prefix_len].to_vec();
        plan_values.sort_by_key(|value| value.index);
        plans.push(CallCacheSpecializationPlan {
            name: function_name.to_string(),
            stable_values: plan_values,
        });
    }
    plans
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

pub fn apply_specialization_feedback_payoff(
    payoff_score: u32,
    specialization_guard_hits: u64,
    specialization_guard_fallbacks: u64,
) -> u32 {
    match specialization_feedback_signal_from_counts(
        specialization_guard_hits,
        specialization_guard_fallbacks,
    ) {
        Some(SpecializationFeedbackSignal::Favorable) => {
            payoff_score.saturating_add(SPECIALIZATION_FEEDBACK_FAVORABLE_SCORE_BONUS)
        }
        Some(SpecializationFeedbackSignal::Unfavorable) => {
            payoff_score.saturating_sub(SPECIALIZATION_FEEDBACK_UNFAVORABLE_SCORE_PENALTY)
        }
        Some(SpecializationFeedbackSignal::Neutral) | None => payoff_score,
    }
}

fn specialization_feedback_signal_from_counts(
    specialization_guard_hits: u64,
    specialization_guard_fallbacks: u64,
) -> Option<SpecializationFeedbackSignal> {
    let attempts = specialization_guard_hits.saturating_add(specialization_guard_fallbacks);
    if attempts < SPECIALIZATION_FEEDBACK_MIN_ATTEMPTS {
        return None;
    }
    if specialization_guard_hits == 0
        || specialization_guard_hits.saturating_mul(SPECIALIZATION_FEEDBACK_UNFAVORABLE_MULTIPLIER)
            < attempts
    {
        return Some(SpecializationFeedbackSignal::Unfavorable);
    }
    if specialization_guard_hits.saturating_mul(SPECIALIZATION_FEEDBACK_FAVORABLE_MULTIPLIER)
        >= attempts
    {
        return Some(SpecializationFeedbackSignal::Favorable);
    }
    Some(SpecializationFeedbackSignal::Neutral)
}

fn specialization_feedback_signal(
    profile: &CallCacheFunctionProfile,
) -> Option<SpecializationFeedbackSignal> {
    specialization_feedback_signal_from_counts(
        profile.specialization_guard_hits,
        profile.specialization_guard_fallbacks,
    )
}

fn specialization_feedback_is_favorable(profile: &CallCacheFunctionProfile) -> bool {
    matches!(
        specialization_feedback_signal(profile),
        Some(SpecializationFeedbackSignal::Favorable)
    )
}

fn specialization_feedback_is_unfavorable_for_values(
    profile: &CallCacheFunctionProfile,
    stable_values: &[StableScalarValueProfile],
) -> bool {
    matches!(
        specialization_feedback_signal_for_values(profile, stable_values),
        Some(SpecializationFeedbackSignal::Unfavorable)
    )
}

fn specialization_feedback_signal_for_values(
    profile: &CallCacheFunctionProfile,
    stable_values: &[StableScalarValueProfile],
) -> Option<SpecializationFeedbackSignal> {
    let key = specialization_feedback_key(stable_values);
    profile
        .specialization_profiles
        .iter()
        .find(|specialization| specialization_feedback_key(&specialization.stable_values) == key)
        .and_then(|specialization| {
            specialization_feedback_signal_from_counts(
                specialization.guard_hits,
                specialization.guard_fallbacks,
            )
        })
        .or_else(|| specialization_feedback_signal(profile))
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
    let mut specialization_profiles = incoming
        .specialization_profiles
        .iter()
        .filter(|specialization| {
            specialization
                .guard_hits
                .saturating_add(specialization.guard_fallbacks)
                > 0
        })
        .cloned()
        .collect::<Vec<_>>();
    specialization_profiles.sort_by(|left, right| {
        specialization_feedback_key(&left.stable_values)
            .cmp(&specialization_feedback_key(&right.stable_values))
    });
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
        specialization_profiles,
        specialization_hint: CallCacheSpecializationHint::None,
    };
    merged.specialization_hint = specialization_hint(total_calls_after_merge, &merged);
    merged
}

fn specialization_feedback_key(stable_values: &[StableScalarValueProfile]) -> Vec<(usize, u64)> {
    let mut key = stable_values
        .iter()
        .map(|value| (value.index, value.raw_bits))
        .collect::<Vec<_>>();
    key.sort_unstable();
    key.dedup();
    key
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
            specialization_guard_hits: 0,
            specialization_guard_fallbacks: 0,
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
            specialization_guard_hits: 0,
            specialization_guard_fallbacks: 0,
            optimize_mode: false,
        });
        assert!(decision.admit);
        assert!(decision.payoff_score >= 40);
    }

    #[test]
    fn adaptive_admission_uses_favorable_specialization_feedback_in_optimize_mode() {
        let decision = adaptive_admission_decision(&AdaptiveAdmissionObservation {
            total_calls: 16,
            total_hits: 0,
            unique_keys: 2,
            cached_entries: 0,
            capacity: 8,
            candidate_hits: 2,
            candidate_reuse_distance: None,
            hottest_key_hits: 4,
            stable_argument_slots: 0,
            specialization_guard_hits: 8,
            specialization_guard_fallbacks: 0,
            optimize_mode: true,
        });
        assert!(decision.admit);
        assert!(decision.payoff_score >= 52);
    }

    #[test]
    fn adaptive_admission_penalizes_unfavorable_specialization_feedback() {
        let decision = adaptive_admission_decision(&AdaptiveAdmissionObservation {
            total_calls: 16,
            total_hits: 0,
            unique_keys: 2,
            cached_entries: 0,
            capacity: 8,
            candidate_hits: 2,
            candidate_reuse_distance: None,
            hottest_key_hits: 4,
            stable_argument_slots: 0,
            specialization_guard_hits: 0,
            specialization_guard_fallbacks: 8,
            optimize_mode: false,
        });
        assert!(!decision.admit);
        assert!(decision.payoff_score < 40);
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
                        stable_values: vec![
                            StableScalarValueProfile {
                                index: 0,
                                raw_bits: 33,
                                matches: 24,
                            },
                            StableScalarValueProfile {
                                index: 1,
                                raw_bits: 7,
                                matches: 18,
                            },
                        ],
                        specialization_hint: CallCacheSpecializationHint::StableArguments {
                            slots: vec![0, 1],
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
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].name, "hot");
        assert_eq!(plans[0].stable_values.len(), 2);
        assert_eq!(plans[0].stable_values[0].index, 0);
        assert_eq!(plans[0].stable_values[1].index, 1);
        assert_eq!(plans[1].name, "hot");
        assert_eq!(plans[1].stable_values.len(), 1);
        assert_eq!(plans[1].stable_values[0].raw_bits, 33);
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
    fn recommended_optimize_functions_skip_unfavorable_specialization_only_signal() {
        let profile = PersistentCallCacheProfile {
            schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 6,
            total_stores: 2,
            functions: vec![PersistentCallCacheFunctionProfile {
                name: "thrashy".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 3,
                total_stores: 1,
                last_entries: 8,
                profile: CallCacheFunctionProfile {
                    unique_keys: 8,
                    hottest_key_hits: 6,
                    avg_reuse_distance: None,
                    max_reuse_distance: None,
                    stable_values: vec![StableScalarValueProfile {
                        index: 0,
                        raw_bits: 33,
                        matches: 12,
                    }],
                    specialization_guard_hits: 1,
                    specialization_guard_fallbacks: 15,
                    specialization_profiles: Vec::new(),
                    specialization_hint: CallCacheSpecializationHint::StableArguments {
                        slots: vec![0],
                    },
                },
            }],
        };

        let recommended = recommended_optimize_functions(&profile);
        assert!(recommended.is_empty());
    }

    #[test]
    fn recommended_optimize_functions_skip_neutral_specialization_only_signal() {
        let profile = PersistentCallCacheProfile {
            schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 0,
            total_stores: 2,
            functions: vec![PersistentCallCacheFunctionProfile {
                name: "steady".into(),
                runs: 2,
                total_calls: 32,
                total_hits: 0,
                total_stores: 1,
                last_entries: 1,
                profile: CallCacheFunctionProfile {
                    unique_keys: 1,
                    hottest_key_hits: 24,
                    avg_reuse_distance: None,
                    max_reuse_distance: None,
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

        let recommended = recommended_optimize_functions(&profile);
        assert!(recommended.is_empty());
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
                    specialization_profiles: Vec::new(),
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
    fn recommended_specializations_skip_only_unfavorable_plan_feedback() {
        let profile = PersistentCallCacheProfile {
            schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "jit".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 48,
            total_stores: 2,
            functions: vec![PersistentCallCacheFunctionProfile {
                name: "mixed".into(),
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
                    stable_values: vec![
                        StableScalarValueProfile {
                            index: 0,
                            raw_bits: 33,
                            matches: 24,
                        },
                        StableScalarValueProfile {
                            index: 1,
                            raw_bits: 7,
                            matches: 20,
                        },
                    ],
                    specialization_guard_hits: 8,
                    specialization_guard_fallbacks: 25,
                    specialization_profiles: vec![
                        CallCacheSpecializationFeedbackProfile {
                            stable_values: vec![
                                StableScalarValueProfile {
                                    index: 0,
                                    raw_bits: 33,
                                    matches: 24,
                                },
                                StableScalarValueProfile {
                                    index: 1,
                                    raw_bits: 7,
                                    matches: 20,
                                },
                            ],
                            guard_hits: 0,
                            guard_fallbacks: 25,
                        },
                        CallCacheSpecializationFeedbackProfile {
                            stable_values: vec![StableScalarValueProfile {
                                index: 0,
                                raw_bits: 33,
                                matches: 24,
                            }],
                            guard_hits: 8,
                            guard_fallbacks: 0,
                        },
                    ],
                    specialization_hint: CallCacheSpecializationHint::StableArguments {
                        slots: vec![0, 1],
                    },
                },
            }],
        };

        let plans = recommended_specializations(&profile);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "mixed");
        assert_eq!(plans[0].stable_values.len(), 1);
        assert_eq!(plans[0].stable_values[0].index, 0);
        assert_eq!(plans[0].stable_values[0].raw_bits, 33);
    }

    #[test]
    fn recommended_specializations_retain_favorable_specialization_feedback() {
        let profile = PersistentCallCacheProfile {
            schema_version: CALL_CACHE_PROFILE_SCHEMA_VERSION,
            backend: "llvm".into(),
            runs: 2,
            total_calls: 64,
            total_hits: 48,
            total_stores: 2,
            functions: vec![PersistentCallCacheFunctionProfile {
                name: "retained".into(),
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
                    stable_values: vec![
                        StableScalarValueProfile {
                            index: 0,
                            raw_bits: 33,
                            matches: 4,
                        },
                        StableScalarValueProfile {
                            index: 1,
                            raw_bits: 7,
                            matches: 3,
                        },
                    ],
                    specialization_guard_hits: 12,
                    specialization_guard_fallbacks: 4,
                    specialization_profiles: Vec::new(),
                    specialization_hint: CallCacheSpecializationHint::StableArguments {
                        slots: vec![0, 1],
                    },
                },
            }],
        };

        let plans = recommended_specializations(&profile);
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].name, "retained");
        assert_eq!(plans[0].stable_values.len(), 2);
        assert_eq!(plans[1].stable_values.len(), 1);
        assert_eq!(plans[1].stable_values[0].raw_bits, 33);
    }

    #[test]
    fn recommended_specializations_rank_prefix_plans_by_match_strength() {
        let plans = specialization_plan_candidates(
            "hot",
            &[
                StableScalarValueProfile {
                    index: 1,
                    raw_bits: 7,
                    matches: 14,
                },
                StableScalarValueProfile {
                    index: 0,
                    raw_bits: 33,
                    matches: 20,
                },
                StableScalarValueProfile {
                    index: 2,
                    raw_bits: 11,
                    matches: 9,
                },
            ],
            false,
        );

        assert_eq!(plans.len(), 3);
        assert_eq!(
            plans[0]
                .stable_values
                .iter()
                .map(|value| value.index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            plans[1]
                .stable_values
                .iter()
                .map(|value| value.index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            plans[2]
                .stable_values
                .iter()
                .map(|value| value.index)
                .collect::<Vec<_>>(),
            vec![0]
        );
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
                        specialization_profiles: vec![CallCacheSpecializationFeedbackProfile {
                            stable_values: vec![StableScalarValueProfile {
                                index: 0,
                                raw_bits: 33,
                                matches: 24,
                            }],
                            guard_hits: 8,
                            guard_fallbacks: 2,
                        }],
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
        assert!(function.profile.specialization_profiles.is_empty());
    }
}
