//! Memory subsystem (M0): episode log, mode + lifecycle transition
//! traces, per-object history store, policy stats, sequence stats,
//! and prediction state. ADR 0052 / M0; ADR 0059 / G1; ADR 0061 / H1.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::R;
use super::action::{ActionKind, FrontierTarget};
use super::lifecycle::{LifecycleState, RuntimeMode};

pub struct Episode {
    pub id: u64,
    pub tick: u64,
    pub mode: RuntimeMode,
    pub action_kind: ActionKind,
    pub target: FrontierTarget,
    pub score_before: f64,
    pub score_after: f64,
    pub delta: f64,
}

/// Recorded mode transition. ADR 0052 / A2.
#[derive(Debug, Clone)]
pub struct ModeTransition {
    pub tick: u64,
    pub from: RuntimeMode,
    pub to: RuntimeMode,
    pub reason: String,
}

/// Recorded lifecycle transition (Running ↔ Sleeping / → Stopped).
/// ADR 0052 / A3.
#[derive(Debug, Clone)]
pub struct LifecycleTransition {
    pub tick: u64,
    pub from: LifecycleState,
    pub to: LifecycleState,
    pub reason: String,
}

/// Per-object run history. ADR 0052 § Phase B / B0.
///
/// Tracks when a named object (pattern / theory) was first observed,
/// when it was last seen by the runtime, when it last contributed to
/// a positive-delta episode, and how many times it was selected as
/// the action target or pruned. `stability_estimate` is reserved for
/// B1 (rolling EMA over delta contributions); it is `None` until then
/// to make the missing-mechanism explicit.
#[derive(Debug, Clone)]
pub struct ObjectHistory {
    pub first_seen_tick: u64,
    pub last_seen_tick: u64,
    pub last_improved_tick: Option<u64>,
    pub times_selected_as_focus: u32,
    pub times_pruned: u32,
    pub last_counterfactual_value: Option<f64>,
    pub stability_estimate: Option<f64>,
    /// Cumulative count of positive-delta episodes in which this
    /// object was present in `patterns_after` / `theories_after`.
    /// ADR 0053 / Phase C0+. Distinct from
    /// `times_selected_as_focus`, which counts how often the object
    /// was the explicit `plan.target` (mostly Prune-side). The
    /// contribution counter is the input to a real `M ≥ N`
    /// promotion gate, replacing C0/C1's binary
    /// `last_improved_tick.is_some()` check.
    pub times_contributed_positive: u32,
}

impl ObjectHistory {
    pub fn new_at(tick: u64) -> Self {
        Self {
            first_seen_tick: tick,
            last_seen_tick: tick,
            last_improved_tick: None,
            times_selected_as_focus: 0,
            times_pruned: 0,
            last_counterfactual_value: None,
            stability_estimate: None,
            times_contributed_positive: 0,
        }
    }
}

/// Per-namespace store of `ObjectHistory`. ADR 0052 § Phase B / B0.
#[derive(Debug, Clone, Default)]
pub struct ObjectHistoryStore {
    pub patterns: HashMap<String, ObjectHistory>,
    pub axioms: HashMap<String, ObjectHistory>,
    pub theories: HashMap<String, ObjectHistory>,
}

/// Aggregate scheduler / lifecycle counters. ADR 0052 § Phase B / B0.
///
/// Filled by the runtime as a side-effect of dispatch; queried by
/// future scheduler policies (B1+). The counts are intentionally
/// minimal — regime-aware bucketing is deferred until a regime
/// signal is wired in.
#[derive(Debug, Clone, Default)]
pub struct PolicyStats {
    pub action_counts: HashMap<ActionKind, u64>,
    pub action_positive_delta_counts: HashMap<ActionKind, u64>,
    pub mode_transition_counts: HashMap<(RuntimeMode, RuntimeMode), u64>,
    pub wake_count: u64,
    pub sleep_count: u64,
    pub stop_count: u64,
}

/// Pair-frequency + post-EP-delta correlation accounting over the
/// episode log. ADR 0061 / Phase H1.0.
///
/// `pair_counts[(A, B)]` is the cumulative number of consecutive
/// episode pairs `(prev=A, curr=B)` observed in `Memory::record`.
/// `pair_post_ep_count[(A, B)]` and `pair_post_ep_delta_sum[(A, B)]`
/// accumulate per-occurrence credit when a positive-delta
/// `EvaluatePredictions` episode follows the pair within
/// `H1_LOOKAHEAD_K` (= 5) steps. The mean post-EP delta for a pair
/// is `sum / count`, useful for H1.1 promotion gating.
///
/// Pure observation: tracking happens as a side-effect of episode
/// recording; the scheduler does not consult these stats yet.
#[derive(Debug, Clone, Default)]
pub struct SequenceStats {
    pub pair_counts: HashMap<(ActionKind, ActionKind), u64>,
    pub pair_post_ep_count: HashMap<(ActionKind, ActionKind), u64>,
    pub pair_post_ep_delta_sum: HashMap<(ActionKind, ActionKind), f64>,
    /// Recent-window post-EP-delta counters. ADR 0062 / Phase H1.3.
    /// Reset every `H1_3_RECENT_WINDOW_TICKS` so the demotion sweep
    /// sees only fresh evidence; cumulative counters above stay
    /// monotonic for long-run reporting. The reset is tick-based,
    /// recorded against `last_recent_reset_tick`.
    pub pair_recent_post_ep_count:
        HashMap<(ActionKind, ActionKind), u64>,
    pub pair_recent_post_ep_delta_sum:
        HashMap<(ActionKind, ActionKind), f64>,
    pub last_recent_reset_tick: u64,
    /// Triple (length-3) sequence counters. ADR 0062 / Phase H1.4.
    /// Triple `(A, B, C)` increments when three consecutive
    /// episodes have those action_kinds. Post-EP credit follows
    /// the same K-lookahead semantics as pairs.
    pub triple_counts:
        HashMap<(ActionKind, ActionKind, ActionKind), u64>,
    pub triple_post_ep_count:
        HashMap<(ActionKind, ActionKind, ActionKind), u64>,
    pub triple_post_ep_delta_sum:
        HashMap<(ActionKind, ActionKind, ActionKind), f64>,
    /// Recent-window triple counters. Mirror of pair recent fields
    /// for triple demotion. ADR 0062 retrospective #2 (triple
    /// demotion). Reset on the same `H1_3_RECENT_WINDOW_TICKS`
    /// boundary as pairs.
    pub triple_recent_post_ep_count:
        HashMap<(ActionKind, ActionKind, ActionKind), u64>,
    pub triple_recent_post_ep_delta_sum:
        HashMap<(ActionKind, ActionKind, ActionKind), f64>,
}

const H1_LOOKAHEAD_K: usize = 5;
const H1_3_RECENT_WINDOW_TICKS: u64 = 50;

impl SequenceStats {
    /// Mean post-EP delta for a pair, or `None` when no positive-EP
    /// episode has followed an occurrence within the lookahead.
    pub fn pair_mean_post_ep_delta(
        &self,
        pair: (ActionKind, ActionKind),
    ) -> Option<f64> {
        let count = self.pair_post_ep_count.get(&pair).copied().unwrap_or(0);
        if count == 0 {
            return None;
        }
        let sum = self
            .pair_post_ep_delta_sum
            .get(&pair)
            .copied()
            .unwrap_or(0.0);
        Some(sum / count as f64)
    }

    /// Recent-window mean post-EP delta. Returns `None` when the
    /// recent window has accumulated zero post-EP credits for this
    /// pair. ADR 0062 / Phase H1.3.
    pub fn pair_recent_mean_post_ep_delta(
        &self,
        pair: (ActionKind, ActionKind),
    ) -> Option<f64> {
        let count = self
            .pair_recent_post_ep_count
            .get(&pair)
            .copied()
            .unwrap_or(0);
        if count == 0 {
            return None;
        }
        let sum = self
            .pair_recent_post_ep_delta_sum
            .get(&pair)
            .copied()
            .unwrap_or(0.0);
        Some(sum / count as f64)
    }

    /// Reset the recent-window counters. Called from
    /// `Memory::record` whenever the elapsed-tick budget has
    /// crossed `H1_3_RECENT_WINDOW_TICKS`. ADR 0062 / Phase H1.3.
    pub fn reset_recent_window(&mut self, current_tick: u64) {
        self.pair_recent_post_ep_count.clear();
        self.pair_recent_post_ep_delta_sum.clear();
        self.triple_recent_post_ep_count.clear();
        self.triple_recent_post_ep_delta_sum.clear();
        self.last_recent_reset_tick = current_tick;
    }

    /// Recent-window mean post-EP delta for a triple. ADR 0062
    /// retrospective #2 (triple demotion). Mirror of
    /// `pair_recent_mean_post_ep_delta`.
    pub fn triple_recent_mean_post_ep_delta(
        &self,
        triple: (ActionKind, ActionKind, ActionKind),
    ) -> Option<f64> {
        let count = self
            .triple_recent_post_ep_count
            .get(&triple)
            .copied()
            .unwrap_or(0);
        if count == 0 {
            return None;
        }
        let sum = self
            .triple_recent_post_ep_delta_sum
            .get(&triple)
            .copied()
            .unwrap_or(0.0);
        Some(sum / count as f64)
    }

    /// Mean post-EP delta for a triple, or `None` when no
    /// positive-EP episode has followed an occurrence within
    /// the lookahead. ADR 0062 / Phase H1.4.
    pub fn triple_mean_post_ep_delta(
        &self,
        triple: (ActionKind, ActionKind, ActionKind),
    ) -> Option<f64> {
        let count = self
            .triple_post_ep_count
            .get(&triple)
            .copied()
            .unwrap_or(0);
        if count == 0 {
            return None;
        }
        let sum = self
            .triple_post_ep_delta_sum
            .get(&triple)
            .copied()
            .unwrap_or(0.0);
        Some(sum / count as f64)
    }
}

/// Per-axiom prediction-error accounting. ADR 0059 / Phase G1.3.
///
/// At end of each tick: `last_predicted_per_axiom[a] =
/// rset.forward_apply_axiom(a)` for every named axiom.
/// At start of next tick (after env events applied):
/// `verified = last_predicted_per_axiom[a] ∩ data_edges`.
/// Increment `total_predictions_per_axiom[a] += predicted.len()`
/// and `verified_predictions_per_axiom[a] += verified.len()`.
///
/// `last_predicted_at_tick = None` means "no snapshot taken yet";
/// the verify step skips when None.
///
/// `last_predicted_per_axiom` is intentionally NOT round-tripped
/// through the B2 checkpoint — it's an in-flight scratchpad that
/// regenerates from rset state on the first post-restore tick.
/// The cumulative counters DO round-trip.
#[derive(Debug, Clone, Default)]
pub struct PredictionState {
    pub last_predicted_at_tick: Option<u64>,
    pub last_predicted_per_axiom: HashMap<String, HashSet<R>>,
    pub total_predictions_per_axiom: HashMap<String, u64>,
    pub verified_predictions_per_axiom: HashMap<String, u64>,
    /// Per-axiom hit rate observed at the last `EvaluatePredictions`
    /// dispatch. Compared against the current hit rate to compute
    /// the per-axiom delta that's summed into the episode's overall
    /// `delta` field. ADR 0059 / Phase G1.5.
    pub last_reflect_hit_rate_per_axiom: HashMap<String, f64>,
    /// Per-axiom forward_apply result cache. Filled by
    /// `snapshot_predictions`; invalidated wholesale when
    /// `rset.version()` differs from `forward_apply_cache_version`.
    /// ADR 0066 Addendum 5+ — perf path.
    ///
    /// Not part of identity; not round-tripped through the B2
    /// checkpoint (rebuilt on first snapshot post-restore).
    pub forward_apply_cache: HashMap<String, HashSet<R>>,
    pub forward_apply_cache_version: Option<u64>,
}

impl PredictionState {
    /// Per-axiom verified hit rate, returned only when the axiom
    /// has accumulated at least `min_total` total predictions
    /// (default callers pass 5 — see ADR 0059 § G1.3).
    pub fn hit_rate(&self, axiom_id: &str, min_total: u64) -> Option<f64> {
        let total = self
            .total_predictions_per_axiom
            .get(axiom_id)
            .copied()
            .unwrap_or(0);
        if total < min_total {
            return None;
        }
        let verified = self
            .verified_predictions_per_axiom
            .get(axiom_id)
            .copied()
            .unwrap_or(0);
        Some(verified as f64 / total as f64)
    }
}

pub struct Memory {
    pub episodes: VecDeque<Episode>,
    pub mode_transitions: VecDeque<ModeTransition>,
    pub lifecycle_transitions: VecDeque<LifecycleTransition>,
    pub max_episodes: usize,
    pub max_mode_transitions: usize,
    pub max_lifecycle_transitions: usize,
    /// Phase B / B0.
    pub object_history: ObjectHistoryStore,
    /// Phase B / B0.
    pub policy_stats: PolicyStats,
    /// Phase G1.3.
    pub prediction_state: PredictionState,
    /// Phase H1.0.
    pub sequence_stats: SequenceStats,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            episodes: VecDeque::new(),
            mode_transitions: VecDeque::new(),
            lifecycle_transitions: VecDeque::new(),
            max_episodes: 1000,
            max_mode_transitions: 200,
            max_lifecycle_transitions: 200,
            object_history: ObjectHistoryStore::default(),
            policy_stats: PolicyStats::default(),
            prediction_state: PredictionState::default(),
            sequence_stats: SequenceStats::default(),
        }
    }
}

impl Memory {
    pub fn record(&mut self, ep: Episode) {
        // ADR 0061 / Phase H1.0: pair-frequency + post-EP-delta
        // accounting. Update BEFORE appending so the indices align
        // with the "previous" episode being the current tail.
        let prev_kind = self.episodes.back().map(|e| e.action_kind);
        let cur_kind = ep.action_kind;
        let cur_delta = ep.delta;

        self.episodes.push_back(ep);

        // Pair count: increment for (prev, cur) when prev exists.
        if let Some(p) = prev_kind {
            *self
                .sequence_stats
                .pair_counts
                .entry((p, cur_kind))
                .or_insert(0) += 1;
        }

        // Triple count: increment for (prev_prev, prev, cur). The
        // current episode is already pushed at episodes.back();
        // need len ≥ 3 to read both predecessors. ADR 0062 / H1.4.
        let n = self.episodes.len();
        if n >= 3 {
            let prev_prev = self.episodes[n - 3].action_kind;
            let prev_now = self.episodes[n - 2].action_kind;
            *self
                .sequence_stats
                .triple_counts
                .entry((prev_prev, prev_now, cur_kind))
                .or_insert(0) += 1;
        }

        // Post-EP-delta credit: when the new episode IS an EP with
        // positive delta, look back at the last K pair completions
        // and credit each for this EP's delta. Per ADR 0061 § H1.0.
        // Recent-window counters mirror cumulative ones for ADR
        // 0062 / Phase H1.3.
        if cur_kind == ActionKind::EvaluatePredictions && cur_delta > 0.0 {
            let n = self.episodes.len();
            let start = n.saturating_sub(H1_LOOKAHEAD_K + 1).max(1);
            let end = n.saturating_sub(1);
            let mut pairs_seen: Vec<(ActionKind, ActionKind)> =
                Vec::new();
            let mut triples_seen: Vec<(ActionKind, ActionKind, ActionKind)> =
                Vec::new();
            for i in start..end {
                let a = self.episodes[i - 1].action_kind;
                let b = self.episodes[i].action_kind;
                pairs_seen.push((a, b));
                // Triple completion at i requires i >= 2.
                if i >= 2 {
                    let t0 = self.episodes[i - 2].action_kind;
                    triples_seen.push((t0, a, b));
                }
            }
            for pair in pairs_seen {
                *self
                    .sequence_stats
                    .pair_post_ep_count
                    .entry(pair)
                    .or_insert(0) += 1;
                *self
                    .sequence_stats
                    .pair_post_ep_delta_sum
                    .entry(pair)
                    .or_insert(0.0) += cur_delta;
                *self
                    .sequence_stats
                    .pair_recent_post_ep_count
                    .entry(pair)
                    .or_insert(0) += 1;
                *self
                    .sequence_stats
                    .pair_recent_post_ep_delta_sum
                    .entry(pair)
                    .or_insert(0.0) += cur_delta;
            }
            // Triple credits — ADR 0062 / Phase H1.4. Recent-
            // window mirror added by retrospective #2 for triple
            // demotion.
            for triple in triples_seen {
                *self
                    .sequence_stats
                    .triple_post_ep_count
                    .entry(triple)
                    .or_insert(0) += 1;
                *self
                    .sequence_stats
                    .triple_post_ep_delta_sum
                    .entry(triple)
                    .or_insert(0.0) += cur_delta;
                *self
                    .sequence_stats
                    .triple_recent_post_ep_count
                    .entry(triple)
                    .or_insert(0) += 1;
                *self
                    .sequence_stats
                    .triple_recent_post_ep_delta_sum
                    .entry(triple)
                    .or_insert(0.0) += cur_delta;
            }
        }

        // ADR 0062 / Phase H1.3 — recent-window reset. When the
        // current episode's tick has advanced at least
        // `H1_3_RECENT_WINDOW_TICKS` past the last reset, clear
        // the recent counters so the next demotion sweep sees only
        // fresh evidence. Done after the credit step so the
        // current EP's contribution survives one tick (until the
        // next window boundary).
        let current_tick = self
            .episodes
            .back()
            .map(|e| e.tick)
            .unwrap_or(0);
        if current_tick
            >= self.sequence_stats.last_recent_reset_tick
                + H1_3_RECENT_WINDOW_TICKS
        {
            self.sequence_stats.reset_recent_window(current_tick);
        }

        while self.episodes.len() > self.max_episodes {
            self.episodes.pop_front();
        }
    }

    /// Append a mode-transition record. ADR 0052 / A2.
    pub fn record_mode_transition(&mut self, mt: ModeTransition) {
        self.mode_transitions.push_back(mt);
        while self.mode_transitions.len() > self.max_mode_transitions {
            self.mode_transitions.pop_front();
        }
    }

    /// Append a lifecycle-transition record. ADR 0052 / A3.
    pub fn record_lifecycle_transition(&mut self, lt: LifecycleTransition) {
        self.lifecycle_transitions.push_back(lt);
        while self.lifecycle_transitions.len() > self.max_lifecycle_transitions {
            self.lifecycle_transitions.pop_front();
        }
    }

    pub fn len(&self) -> usize {
        self.episodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.episodes.is_empty()
    }
}
