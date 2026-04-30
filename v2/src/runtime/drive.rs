//! Drive trait + 3 baseline implementations + DriveMix A/B
//! weight controller. ADR 0063 / Phase H2.0.

use crate::{R, RSet};
use super::action::ActionKind;
use super::Memory;
use std::collections::{HashMap, HashSet};

pub trait Drive {
    /// Stable identifier. Used as the key in `DriveMix` weight
    /// tables. Must be unique across registered drives.
    fn id(&self) -> &'static str;

    /// Compute the drive's current signal strength. Convention:
    /// non-negative scalar; 0 means "nothing for this drive to
    /// say"; magnitude scales with urgency. Implementations should
    /// be pure functions of (rset, memory, tick) — no internal
    /// state — so signals are reproducible across runs.
    fn evaluate(
        &self,
        rset: &RSet,
        memory: &Memory,
        tick: u64,
    ) -> f64;

    /// Whether this drive contributes a *penalty* to the blended
    /// signal rather than positive activity. Default: `false`
    /// (positive contribution). ADR 0063 OQ #4 resolution
    /// (option c — mathematical handling).
    ///
    /// When `is_penalty()` returns `true`:
    /// - `combined_drive_signal` *subtracts* this drive's
    ///   weight × evaluate from the total.
    /// - `normalized_drive_signal` excludes this drive's weight
    ///   from the denominator (so the divisor reflects only
    ///   positive drives' weight contribution).
    ///
    /// The conceptual semantic this fixes: a drive like
    /// `ModeThrashPenalty` reports "how much the runtime is
    /// thrashing"; high values should *reduce* the perceived
    /// activity signal, not increase it. Pre-OQ-#4 the
    /// implementation treated all drives as positive,
    /// causing step 3b's gate to fail on real substrates
    /// (long-run regression: 268 → 1000+ episodes).
    ///
    /// **BOOT-ONLY scope (ADR 0064 H2.1.1 cleanup):** This method
    /// is consulted ONLY at runtime construction time, inside
    /// `register_drives_in_rset()`, to decide whether to write
    /// the `R(PENALTY_MARKER, drive_<id>)` edge. After boot, the
    /// canonical source of penalty status is meta-R itself —
    /// `is_drive_penalty_via_meta_r()` queries the rset edge,
    /// not this method. This means **penalty status is mutable at
    /// runtime**: retracting the PENALTY_MARKER edge converts a
    /// drive from penalty to positive contribution, even though
    /// the trait method still returns `true`. The `Drive` trait
    /// provides the BOOT default; meta-R holds the LIVE state.
    ///
    /// Tests in `runtime::tests` exercise the trait directly to
    /// verify the boot default; production decision paths
    /// (`combined_drive_signal`, `normalized_drive_signal`) MUST
    /// query meta-R, not this method, for runtime correctness.
    fn is_penalty(&self) -> bool {
        false
    }
}

/// Compression drive — recent positive abstraction-score delta.
/// Saturates as the rset reaches compression equilibrium (the
/// original G0 problem). Mirror of the implicit signal that
/// already gates the scheduler's productive-vs-stagnant decision.
/// ADR 0063 / Phase H2.0.
pub struct CompressionDrive;

impl Drive for CompressionDrive {
    fn id(&self) -> &'static str {
        "compression"
    }

    fn evaluate(
        &self,
        _rset: &RSet,
        memory: &Memory,
        _tick: u64,
    ) -> f64 {
        const K: usize = 10;
        let mut sum = 0.0;
        let mut count = 0usize;
        for ep in memory.episodes.iter().rev().take(K) {
            if ep.delta > 0.0 {
                sum += ep.delta;
            }
            count += 1;
        }
        if count == 0 {
            0.0
        } else {
            sum / count as f64
        }
    }
}

/// Prediction-error drive — sum of |hit_rate_now - hit_rate_prev|
/// across named axioms. Mirrors `predictions_have_pending_delta`
/// but returns a scalar instead of a boolean. The G1.5 outward
/// drive uses the boolean form; H2.0 exposes the underlying
/// magnitude for blending. ADR 0063 / Phase H2.0.
pub struct PredictionErrorDrive;

impl Drive for PredictionErrorDrive {
    fn id(&self) -> &'static str {
        "prediction_error"
    }

    fn evaluate(
        &self,
        rset: &RSet,
        memory: &Memory,
        _tick: u64,
    ) -> f64 {
        // ADR 0066 Addendum 4 perf fix: amortize collect_meta_ids +
        // data_ids across all axioms.
        let meta = rset.collect_meta_ids();
        let data_ids = rset.compute_data_ids(&meta);
        let data_edges: HashSet<R> = rset
            .iter()
            .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
            .cloned()
            .collect();
        let ps = &memory.prediction_state;
        let mut total: f64 = 0.0;
        for ax in rset.axioms() {
            let pred = rset.forward_apply_axiom_with_data_ids(ax, &data_ids);
            if pred.is_empty() {
                continue;
            }
            let verified = pred.intersection(&data_edges).count();
            let now = verified as f64 / pred.len() as f64;
            let prev = ps
                .last_reflect_hit_rate_per_axiom
                .get(ax)
                .copied()
                .unwrap_or(0.0);
            total += (now - prev).abs();
        }
        total
    }
}

/// Mode-thrash penalty — count of mode transitions in the recent
/// window. Higher = more thrash = more penalty (caller weighs
/// negatively in the blended signal). The existing mode-thrash
/// gate inspects this implicitly; H2.0 exposes it as a first-
/// class drive. ADR 0063 / Phase H2.0.
pub struct ModeThrashPenalty;

impl Drive for ModeThrashPenalty {
    fn id(&self) -> &'static str {
        "mode_thrash"
    }

    fn evaluate(
        &self,
        _rset: &RSet,
        memory: &Memory,
        _tick: u64,
    ) -> f64 {
        const K: usize = 20;
        memory
            .mode_transitions
            .iter()
            .rev()
            .take(K)
            .count() as f64
    }

    /// `mode_thrash` is a penalty drive — high churn-count
    /// should *reduce* the perceived activity signal, not raise
    /// it. ADR 0063 OQ #4 resolution.
    fn is_penalty(&self) -> bool {
        true
    }
}

// ─── ADR 0063 / Phase H2.0 step 2 — DriveMix (A/B over weights) ──
//
// Weight blending across the registered drives, with an A/B
// mutation cycle keyed off mean EP delta per window. Mirrors the
// `MetaScheduler` design (windows, candidates, mutate-loser) but
// operates on weight maps instead of scheduler config knobs.
//
// Step 2 wires DriveMix into the runtime as a *shadow* layer:
// `maybe_advance` runs each tick and updates state, but no caller
// yet reads `active_weights()` to drive the wake/mode/sleep gate.
// That integration is step 3.

/// Which candidate is currently active in the DriveMix A/B cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveABState {
    TestingA,
    TestingB,
}

/// A/B-tuned weight blend across the registered drives. Each
/// candidate is a `drive_id → weight` map; the active candidate's
/// weights are blended into the per-tick combined signal (step 3,
/// not yet wired). Mutates the loser at each window boundary by
/// perturbing one randomly chosen weight by ×0.8 or ×1.25, clamped
/// to [0, 1]. ADR 0063 / Phase H2.0 step 2.
#[derive(Debug, Clone)]
pub struct DriveMix {
    pub candidate_a: HashMap<String, f64>,
    pub candidate_b: HashMap<String, f64>,
    pub state: DriveABState,
    pub window_size: u64,
    pub stage_start_episode_count: u64,
    pub last_completed_a_mean: Option<f64>,
    pub rng_state: u64,
}

impl Default for DriveMix {
    fn default() -> Self {
        Self::baseline()
    }
}

impl DriveMix {
    /// Baseline weights — hand-tuned mix that approximates the
    /// existing scheduler's effective blend. Matches the ADR 0063
    /// open-question #1 recommendation.
    pub fn baseline() -> Self {
        let mut weights: HashMap<String, f64> = HashMap::new();
        weights.insert("compression".to_string(), 0.5);
        weights.insert("prediction_error".to_string(), 0.4);
        weights.insert("mode_thrash".to_string(), 0.1);
        Self::with_weights(weights)
    }

    pub fn with_weights(weights: HashMap<String, f64>) -> Self {
        Self {
            candidate_a: weights.clone(),
            candidate_b: weights,
            state: DriveABState::TestingA,
            window_size: 50,
            stage_start_episode_count: 0,
            last_completed_a_mean: None,
            rng_state: 0xc0ffee_dead_beef_u64,
        }
    }

    /// Active candidate's weights. Step 3 will read this for
    /// blending; step 2 exposes it for observability.
    pub fn active_weights(&self) -> &HashMap<String, f64> {
        match self.state {
            DriveABState::TestingA => &self.candidate_a,
            DriveABState::TestingB => &self.candidate_b,
        }
    }

    fn ep_mean_in_range(
        memory: &Memory,
        start: usize,
        end: usize,
    ) -> f64 {
        if end <= start {
            return 0.0;
        }
        let mut sum: f64 = 0.0;
        let mut count: u64 = 0;
        for ep in memory.episodes.iter().skip(start).take(end - start) {
            if ep.action_kind == ActionKind::EvaluatePredictions {
                sum += ep.delta;
                count += 1;
            }
        }
        if count == 0 {
            0.0
        } else {
            sum / count as f64
        }
    }

    pub(crate) fn mutate(
        weights: &mut HashMap<String, f64>,
        rng_state: &mut u64,
    ) {
        if weights.is_empty() {
            return;
        }
        let step = |s: &mut u64| -> u64 {
            *s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *s >> 32
        };
        // Deterministic key order — sort so mutation across
        // serialized restores is reproducible.
        let mut keys: Vec<String> =
            weights.keys().cloned().collect();
        keys.sort();
        let idx = (step(rng_state) as usize) % keys.len();
        let dir_up = step(rng_state) & 1 == 1;
        let factor: f64 = if dir_up { 1.25 } else { 0.8 };
        let key = &keys[idx];
        if let Some(w) = weights.get_mut(key) {
            *w = (*w * factor).clamp(0.0, 1.0);
        }
    }

    /// Advance the A/B state machine if the current window's
    /// episode budget is exhausted. Called from the runtime each
    /// tick. ADR 0063 / Phase H2.0 step 2.
    pub fn maybe_advance(&mut self, memory: &Memory) {
        let now = memory.episodes.len() as u64;
        let elapsed = now.saturating_sub(self.stage_start_episode_count);
        if elapsed < self.window_size {
            return;
        }
        let stage_start = self.stage_start_episode_count as usize;
        let mean = Self::ep_mean_in_range(
            memory,
            stage_start,
            now as usize,
        );
        match self.state {
            DriveABState::TestingA => {
                self.last_completed_a_mean = Some(mean);
                self.state = DriveABState::TestingB;
            }
            DriveABState::TestingB => {
                let a_mean = self
                    .last_completed_a_mean
                    .take()
                    .unwrap_or(0.0);
                let b_mean = mean;
                if a_mean >= b_mean {
                    Self::mutate(
                        &mut self.candidate_b,
                        &mut self.rng_state,
                    );
                } else {
                    Self::mutate(
                        &mut self.candidate_a,
                        &mut self.rng_state,
                    );
                }
                self.state = DriveABState::TestingA;
            }
        }
        self.stage_start_episode_count = now;
    }
}
