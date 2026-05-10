//! Rule-based scheduler with mode-aware filtering and Expand /
//! Consolidate / Reflect transitions. ADR 0052 / A1 + A2.

use crate::R;
use std::collections::HashSet;

use super::action::{ActionKind, ActionPlan, FrontierTarget, SchedulerDecision};
use super::lifecycle::RuntimeMode;
use super::scheduler::{Scheduler, SchedulerContext};
use super::{
    action_kind_to_str, parse_action_kind, FrontierItem, FrontierKind, PolicyStats,
};

/// Rule-based scheduler with mode-aware filtering and Expand /
/// Consolidate / Reflect transitions. ADR 0052 / A1 + A2.
///
/// Mode policy:
/// - **Expand**: pick TheoryCandidate / PatternCandidate items.
///   Switch to Consolidate when recent expansion produced multiple
///   gains AND consolidate work exists. Switch to Reflect on
///   stagnation.
/// - **Consolidate**: pick LowValueObjectForPrune /
///   TheoryNeedsRelations items. Switch to Reflect when consolidate
///   work is empty.
/// - **Reflect**: pure state-machine mode — no Execute, only
///   SwitchMode or Sleep. Decides Expand if fresh discovery work
///   exists, else Sleep.
///
/// Stagnation falls back to Sleep after `max_zero_streak`
/// non-positive episodes regardless of mode.
pub struct RuleBasedScheduler {
    pub max_zero_streak: usize,
    /// Window over which `should_enter_consolidate` looks for
    /// recent positive-delta Discover episodes.
    pub recent_window: usize,
    /// Minimum positive-delta Discovers in `recent_window` to
    /// consider a switch to Consolidate.
    pub min_recent_gains: usize,
    /// Anti-thrash gate. ADR 0052 / B1.
    ///
    /// If two modes A↔B together account for at least this many
    /// transitions in `policy_stats.mode_transition_counts`, refuse
    /// further A↔B switches and Sleep instead. Prevents the
    /// scheduler from oscillating forever between Expand and
    /// Consolidate (or any other pair) when the rset has nothing
    /// new to offer either side.
    pub max_mode_oscillations: u64,
    /// Cooldown threshold for PatternCandidate selection.
    /// ADR 0052 / B1+.
    ///
    /// If `DiscoverPatterns` has been attempted at least
    /// `min_pattern_attempts_before_cooldown` times AND the rate
    /// `action_positive_delta_counts / action_counts` is below
    /// `min_pattern_hit_rate`, skip PatternCandidate items. The
    /// scheduler falls back to TheoryCandidate; if neither has
    /// work, walks the normal mode chain (Consolidate / Reflect /
    /// Sleep). Prevents the runtime from burning ticks on
    /// pattern-discovery passes that consistently produce nothing.
    pub min_pattern_hit_rate: f64,
    pub min_pattern_attempts_before_cooldown: u64,
    /// Cooldown threshold for `MetaMetaCandidate` selection.
    /// ADR 0054 / open question #2.
    ///
    /// Symmetric to the pattern-cooldown gate, but tracks
    /// `DiscoverMetaMetaPatterns` independently so an unproductive
    /// meta-meta pass does not burn the regular pattern-discovery
    /// budget. Default `min_meta_meta_hit_rate = 0.05` (5%, more
    /// permissive than pattern's 10%) — meta-meta is exploratory
    /// and the runtime should give it more attempts before giving
    /// up; raising the floor too aggressively defeats Phase D's
    /// purpose. `min_meta_meta_attempts_before_cooldown = 5` matches
    /// the pattern gate's floor.
    pub min_meta_meta_hit_rate: f64,
    pub min_meta_meta_attempts_before_cooldown: u64,
    /// Anomaly-coverage drive thresholds. ADR 0057 / Phase G0,
    /// signal tightened in ADR 0059 / Phase G1.4.
    ///
    /// When `rset.unexplained_data_edges().len() >=
    /// anomaly_pressure_threshold`, two scheduler hooks fire:
    /// (1) the B1+ pattern-cooldown hit-rate floor is multiplied by
    /// `anomaly_relaxation` (default 0.5 → effective floor drops
    /// from 10% to 5%), giving more room for exploratory pattern
    /// passes; (2) the Reflect → Sleep transition is replaced with
    /// Reflect → Expand so the runtime keeps trying while there is
    /// unexplained data. The mode-thrash gate still bounds the
    /// suppression so the runtime can't loop forever.
    ///
    /// "Unexplained" = data edges not in any named pattern's
    /// Layer B coverage AND not predicted by any axiom's
    /// forward-apply (the latter is the G1.4 strengthening).
    pub anomaly_pressure_threshold: usize,
    pub anomaly_relaxation: f64,
}

impl Default for RuleBasedScheduler {
    fn default() -> Self {
        Self {
            max_zero_streak: 3,
            recent_window: 5,
            min_recent_gains: 2,
            max_mode_oscillations: 4,
            min_pattern_hit_rate: 0.1,
            // ADR 0075 piece 2 — pattern attempts threshold raised
            // from 5 to 30. Early DP dispatches (tick ~30 on OQ#1
            // / long5k / narrow_a) fire when the rset has only a
            // handful of stream events; sampling can't find
            // recurring substructure that early, so the first
            // ~5-10 dispatches are doomed to be unproductive
            // regardless of dispatch parameters. With the original
            // threshold of 5, 5 early failures forever locked DP
            // out of the run, even after the rset matured into a
            // pattern-rich state. Raising the threshold to 30
            // gives DP ~30 attempts before the cooldown gate
            // engages, which is enough for at least the
            // mid-Phase-0 attempts to succeed and accumulate a
            // hit-rate above 10%.
            min_pattern_attempts_before_cooldown: 30,
            min_meta_meta_hit_rate: 0.05,
            min_meta_meta_attempts_before_cooldown: 5,
            anomaly_pressure_threshold: 3,
            anomaly_relaxation: 0.5,
        }
    }
}

/// Reserved threshold from ADR 0063 step 3b's first/second
/// AND-on-EP-gate attempts. Both reverted (long-run regressed
/// each time). Retained as a constant so future refined gates
/// can reference it; not currently used. ADR 0063 Addendum 4.
const STEP3B_NORMALIZED_SIGNAL_THRESHOLD: f64 = 0.3;

/// Threshold for the (α) shape — OR semantics on EP gate. When
/// `normalized_drive_signal < STEP3B_ALPHA_LOW_SIGNAL_THRESHOLD`
/// AND axioms exist AND predictions have pending delta, fire EP
/// even when zero_streak hasn't accumulated yet. ADR 0063
/// Addendum 5 / shape (α). Strictly more conservative than (AND)
/// — adds firing conditions, never blocks them.
///
/// Calibration (post-OQ-#4 long-run baseline): hand-tuned signal
/// range -0.65 to -1.24, never crosses -2.0 → baseline is
/// preserved. Equal-weighted signal range -2.83 to -3.33 → the
/// new path fires extra EPs there, demonstrating wiring works.
const STEP3B_ALPHA_LOW_SIGNAL_THRESHOLD: f64 = -2.0;

impl RuleBasedScheduler {
    fn pick_top<'a, F: Fn(&FrontierItem) -> bool>(
        ctx: &'a SchedulerContext<'_>,
        accept: F,
    ) -> Option<&'a FrontierItem> {
        ctx.frontier.items.iter().find(|it| accept(it))
    }

    /// Like `pick_top` but applies a per-`ActionKind` priority
    /// bonus to items whose `execute_for_kind(kind)` is in
    /// `bonus_kinds`. Used by Expand mode to bias selection
    /// toward action sequences promoted from
    /// `Memory::sequence_stats` (ADR 0061 / Phase H1.1).
    ///
    /// When `bonus_kinds` is empty the result equals `pick_top`
    /// (no resort cost). Otherwise items are scanned with a
    /// priority + bonus comparator; the highest effective
    /// priority among accepted items wins. Stable tie-break via
    /// item id.
    pub(crate) fn pick_top_biased<'a, F: Fn(&FrontierItem) -> bool>(
        ctx: &'a SchedulerContext<'_>,
        accept: F,
        bonus_kinds: &HashSet<ActionKind>,
    ) -> Option<&'a FrontierItem> {
        if bonus_kinds.is_empty() {
            return Self::pick_top(ctx, accept);
        }
        const BONUS: f64 = 1.0;
        let bonus_for = |kind: FrontierKind| -> f64 {
            if bonus_kinds.contains(&Self::execute_for_kind(kind)) {
                BONUS
            } else {
                0.0
            }
        };
        ctx.frontier
            .items
            .iter()
            .filter(|it| accept(it))
            .max_by(|a, b| {
                let pa = a.priority + bonus_for(a.kind);
                let pb = b.priority + bonus_for(b.kind);
                pa.partial_cmp(&pb)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.id.cmp(&a.id))
            })
    }

    /// Compute the set of suffix `ActionKind`s that should receive
    /// the H1.1 priority bonus given the previous episode's
    /// action_kind. Reads named action sequences from rset's meta-R
    /// chain. Empty set when no episodes exist or no matching
    /// sequence was promoted. ADR 0061 / Phase H1.1.
    pub(crate) fn h1_1_bonus_kinds(ctx: &SchedulerContext<'_>) -> HashSet<ActionKind> {
        let mut out: HashSet<ActionKind> = HashSet::new();
        let prev_kind = match ctx.memory.episodes.back() {
            Some(ep) => ep.action_kind,
            None => return out,
        };
        let prev_name = action_kind_to_str(prev_kind);
        for (_, prefix, suffix) in ctx.rset.action_sequence_pairs() {
            if prefix == prev_name {
                if let Ok(k) = parse_action_kind(&suffix) {
                    out.insert(k);
                }
            }
        }
        out
    }

    pub(crate) fn execute_for_kind(kind: FrontierKind) -> ActionKind {
        match kind {
            FrontierKind::TheoryCandidate => ActionKind::DiscoverTheory,
            FrontierKind::PatternCandidate => ActionKind::DiscoverPatterns,
            FrontierKind::LowValueObjectForPrune => {
                ActionKind::PruneLowValueObjects
            }
            FrontierKind::TheoryNeedsRelations => {
                ActionKind::UpdateTheoryRelations
            }
            FrontierKind::EstablishedPromotion => ActionKind::Declarativize,
            FrontierKind::MetaMetaCandidate => {
                ActionKind::DiscoverMetaMetaPatterns
            }
            FrontierKind::CompositeCandidate => {
                ActionKind::ExecuteComposite
            }
            FrontierKind::ShapeFamilyDiscoveryCandidate => {
                ActionKind::DiscoverAxiomShapeFamilies
            }
        }
    }

    fn has_expand_work(&self, ctx: &SchedulerContext<'_>) -> bool {
        let pattern_cool = self.pattern_cooldown_active(ctx);
        let meta_meta_cool = self.meta_meta_cooldown_active(ctx);
        ctx.frontier.items.iter().any(|it| match it.kind {
            FrontierKind::TheoryCandidate => true,
            FrontierKind::PatternCandidate => !pattern_cool,
            FrontierKind::MetaMetaCandidate => !meta_meta_cool,
            FrontierKind::CompositeCandidate => true,
            FrontierKind::ShapeFamilyDiscoveryCandidate => true,
            _ => false,
        })
    }

    /// Returns true if at least one named axiom has accumulated
    /// enough total predictions for `hit_rate(ax, 5)` to return
    /// `Some(_)`. Used by Reflect to decide whether dispatching
    /// `EvaluatePredictions` is even meaningful — without
    /// hit-rate data, the action would always emit delta = 0.
    /// ADR 0059 / Phase G1.5.
    pub(crate) fn any_axiom_has_hit_rate(ctx: &SchedulerContext<'_>) -> bool {
        const MIN_TOTAL_FOR_HIT_RATE: u64 = 5;
        for ax in ctx.rset.axioms() {
            if ctx
                .memory
                .prediction_state
                .hit_rate(ax, MIN_TOTAL_FOR_HIT_RATE)
                .is_some()
            {
                return true;
            }
        }
        false
    }

    /// Returns true if dispatching `EvaluatePredictions` *now*
    /// would produce a non-zero delta — i.e., at least one axiom's
    /// **fresh** forward-apply hit rate (recomputed from current
    /// rset state) differs from its stored
    /// `last_reflect_hit_rate_per_axiom`.
    ///
    /// Uses fresh forward-apply rather than the cumulative
    /// counters because the counters update only when verify runs
    /// against a prior snapshot, and verify is no-op while
    /// sleeping. With fresh forward-apply, the gate responds to
    /// any rset change (including events that arrive during sleep
    /// and are applied at wake time), which is the architectural
    /// point of the outward drive — react to environmental change.
    /// Cumulative counters are still maintained for future
    /// long-run reliability uses (e.g., axiom-trust promotion).
    /// ADR 0059 / Phase G1.5.
    fn predictions_have_pending_delta(
        ctx: &SchedulerContext<'_>,
    ) -> bool {
        // ADR 0066 Addendum 4 perf fix: amortize collect_meta_ids
        // + data_ids across all axioms.
        let meta = ctx.rset.collect_meta_ids();
        let data_ids = ctx.rset.compute_data_ids(&meta);
        let data_edges: HashSet<R> = ctx
            .rset
            .iter()
            .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
            .cloned()
            .collect();
        let ps = &ctx.memory.prediction_state;
        for ax in ctx.rset.axioms() {
            let pred = ctx
                .rset
                .forward_apply_axiom_with_data_ids(ax, &data_ids);
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
            if (now - prev).abs() > f64::EPSILON {
                return true;
            }
        }
        false
    }

    fn has_consolidate_work(ctx: &SchedulerContext<'_>) -> bool {
        ctx.frontier.items.iter().any(|it| {
            matches!(
                it.kind,
                FrontierKind::LowValueObjectForPrune
                    | FrontierKind::TheoryNeedsRelations
                    | FrontierKind::EstablishedPromotion
            )
        })
    }

    fn zero_streak(ctx: &SchedulerContext<'_>) -> usize {
        ctx.memory
            .episodes
            .iter()
            .rev()
            .take_while(|ep| ep.delta <= 0.0)
            .count()
    }

    fn recent_positive_discovers(&self, ctx: &SchedulerContext<'_>) -> usize {
        ctx.memory
            .episodes
            .iter()
            .rev()
            .take(self.recent_window)
            .filter(|ep| {
                ep.delta > 0.0
                    && matches!(
                        ep.action_kind,
                        ActionKind::DiscoverPatterns
                            | ActionKind::DiscoverTheory
                            | ActionKind::EvaluatePredictions
                    )
            })
            .count()
    }

    /// Anti-thrash gate. Returns true iff transitions in EITHER
    /// direction between `current` and `target` already total
    /// `max_mode_oscillations` or more in `policy_stats`.
    /// ADR 0052 / B1.
    pub(crate) fn would_thrash(
        &self,
        ctx: &SchedulerContext<'_>,
        current: RuntimeMode,
        target: RuntimeMode,
    ) -> bool {
        if current == target {
            return false;
        }
        let counts = &ctx.memory.policy_stats.mode_transition_counts;
        let forward = counts.get(&(current, target)).copied().unwrap_or(0);
        let back = counts.get(&(target, current)).copied().unwrap_or(0);
        forward + back >= self.max_mode_oscillations
    }

    /// Switch-or-sleep helper: returns SwitchMode(target) unless the
    /// pair already thrashed, in which case Sleep.
    fn switch_or_sleep(
        &self,
        ctx: &SchedulerContext<'_>,
        target: RuntimeMode,
    ) -> SchedulerDecision {
        if self.would_thrash(ctx, ctx.mode, target) {
            // ADR 0079.1 — drive-aware thrash bypass. Mirrors
            // ADR 0079's stagnation-gate bypass: when drive is
            // alive on a mature rset, mode oscillation is justified
            // by structural unexplored work, not by policy
            // thrashing. Without this, the OQ#2 long-horizon
            // observation (`phase_emergence_oq2_equilibrium`)
            // shows Phase 3 freeze: wake-on-drive triggers, but
            // every dispatch attempt routes through this gate and
            // returns Sleep because Reflect↔Expand transitions
            // accumulated during initialization already exceed
            // max_mode_oscillations.
            //
            // Pattern-cooldown remains the safety net: if DP
            // keeps failing post-bypass, cooldown blocks
            // PatternCandidate selection, has_expand_work returns
            // false, no more switches happen. The bypass is
            // therefore bounded by mintability of remaining drive
            // canonicals.
            const MATURE_DATA_EDGE_FLOOR: usize = 100;
            // ADR 0079 (caching, 2026-05-11) — prefer cached drive
            // signal from SchedulerContext when available.
            let drive_has_signal = match ctx.cached_drive {
                Some(d) => d.has_signal(),
                None => ctx.rset.unexplained_drive_signal().has_signal(),
            };
            let drive_alive = !ctx.rset.axioms().is_empty()
                && ctx.rset.iter().count() >= MATURE_DATA_EDGE_FLOOR
                && drive_has_signal;
            if drive_alive {
                return SchedulerDecision::SwitchMode(target);
            }
            SchedulerDecision::Sleep
        } else {
            SchedulerDecision::SwitchMode(target)
        }
    }

    /// Pattern-discovery cooldown gate. Returns true iff
    /// `DiscoverPatterns` has been attempted enough times to assess
    /// AND its positive-delta hit rate is below threshold. The
    /// effective hit-rate floor relaxes under anomaly pressure
    /// (ADR 0057 / Phase G0): when there are at least
    /// `anomaly_pressure_threshold` uncovered data edges, the floor
    /// drops to `min_pattern_hit_rate * anomaly_relaxation`. ADR 0052
    /// / B1+.
    pub(crate) fn pattern_cooldown_active(&self, ctx: &SchedulerContext<'_>) -> bool {
        let effective_floor = self.effective_pattern_hit_rate_floor(ctx);
        Self::action_kind_cooldown_active(
            &ctx.memory.policy_stats,
            ActionKind::DiscoverPatterns,
            self.min_pattern_attempts_before_cooldown,
            effective_floor,
        )
    }

    /// Effective hit-rate floor for `DiscoverPatterns` after the
    /// G0 anomaly-pressure relaxation. ADR 0057.
    fn effective_pattern_hit_rate_floor(
        &self,
        ctx: &SchedulerContext<'_>,
    ) -> f64 {
        let unexplained = ctx.rset.unexplained_data_edges().len();
        if unexplained >= self.anomaly_pressure_threshold {
            self.min_pattern_hit_rate * self.anomaly_relaxation
        } else {
            self.min_pattern_hit_rate
        }
    }

    /// Meta-meta-discovery cooldown gate. Symmetric to
    /// `pattern_cooldown_active` but reads the
    /// `ActionKind::DiscoverMetaMetaPatterns` slot of `policy_stats`
    /// — an unproductive D0 pass cools its own ActionKind without
    /// touching DiscoverPatterns' counter. ADR 0054 / OQ #2.
    pub(crate) fn meta_meta_cooldown_active(&self, ctx: &SchedulerContext<'_>) -> bool {
        Self::action_kind_cooldown_active(
            &ctx.memory.policy_stats,
            ActionKind::DiscoverMetaMetaPatterns,
            self.min_meta_meta_attempts_before_cooldown,
            self.min_meta_meta_hit_rate,
        )
    }

    /// Shared cooldown evaluator: an action is cooled iff
    /// `attempts >= min_attempts` and `hits / attempts < min_hit_rate`.
    /// Single source of truth for both pattern (B1+) and meta-meta
    /// (ADR 0054 OQ #2) cooldown gates.
    fn action_kind_cooldown_active(
        stats: &PolicyStats,
        kind: ActionKind,
        min_attempts: u64,
        min_hit_rate: f64,
    ) -> bool {
        let attempts =
            stats.action_counts.get(&kind).copied().unwrap_or(0);
        if attempts < min_attempts {
            return false;
        }
        let hits = stats
            .action_positive_delta_counts
            .get(&kind)
            .copied()
            .unwrap_or(0);
        (hits as f64 / attempts as f64) < min_hit_rate
    }
}

impl Scheduler for RuleBasedScheduler {
    fn choose(&mut self, ctx: &SchedulerContext<'_>) -> SchedulerDecision {
        // Global stagnation: would otherwise force Sleep, but
        // ADR 0059 / Phase G1.5 lets EvaluatePredictions run as
        // an "anti-stagnation" alternative when there's actually
        // pending hit-rate delta to record. This narrow placement
        // keeps EP from displacing normal Discover work — EP only
        // fires when the runtime would otherwise sleep AND there
        // is delta to capture. Stable hit rates skip EP and Sleep
        // proceeds.
        //
        // ADR 0063 / Phase H2.0 step 3b — refined shape (α):
        // OR semantics on EP firing. Two conditions independently
        // route to the gate body:
        //   (1) zero_streak >= max_zero_streak — original
        //       stagnation criterion. Sleep if EP can't fire,
        //       fire EP otherwise.
        //   (2) normalized_drive_signal < ALPHA_LOW threshold
        //       — drives report the runtime is in a deeply
        //       unproductive state. Fire EP for additional
        //       observation, but DON'T sleep (don't add a sleep
        //       opportunity beyond what (1) already provides).
        //
        // Condition (2) is strictly additive: it adds EP-firing
        // opportunities, never removes any. The two prior step 3b
        // attempts (AND semantics) failed because they removed
        // EP firings; (α) adds them. Threshold -2.0 calibrated
        // against post-OQ-#4 long-run hand-tuned signal range
        // (-0.65 to -1.24): never crosses → baseline preserved.
        // Equal-weighted signal (-2.83 to -3.33) does cross →
        // path is empirically load-bearing on that mix.
        //
        // Why no sleep on signal-low alone: the original Sleep
        // path triggers when EP can't fire OR when zero_streak
        // is high. Sleep semantics are unchanged here because
        // step 3b's scope is "EP anti-stagnation gate" only.
        if Self::zero_streak(ctx) >= self.max_zero_streak {
            // ADR 0079 — drive bypass. The stagnation gate
            // would normally either fire EP or sleep. But if the
            // drive signal is non-empty on a mature rset, drive-
            // proposed work exists in the frontier that
            // zero_streak doesn't account for. Fall through to
            // the frontier-selection path so the drive-driven
            // PatternCandidate gets a chance to be picked.
            //
            // Without this bypass, ADR 0079's drive-driven
            // frontier item was never reached: scheduler returned
            // Sleep here, runtime entered LifecycleState::Sleeping,
            // wake-on-drive logic then woke it the next tick, but
            // scheduler again hit this gate on the same stale
            // zero_streak and again returned Sleep — a wake/sleep
            // ping-pong with no dispatch progress (observed in
            // 2026-05-08 long-horizon re-run before this fix).
            const MATURE_DATA_EDGE_FLOOR: usize = 100;
            // ADR 0079 (caching, 2026-05-11) — prefer cached drive
            // signal from SchedulerContext to avoid recomputation.
            // Fallback path still works (legacy callers / tests).
            let drive_has_signal = match ctx.cached_drive {
                Some(d) => d.has_signal(),
                None => ctx.rset.unexplained_drive_signal().has_signal(),
            };
            let drive_alive = !ctx.rset.axioms().is_empty()
                && ctx.rset.iter().count() >= MATURE_DATA_EDGE_FLOOR
                && drive_has_signal;
            if !drive_alive {
                if !ctx.rset.axioms().is_empty()
                    && Self::predictions_have_pending_delta(ctx)
                {
                    return SchedulerDecision::Execute(ActionPlan {
                        action_kind: ActionKind::EvaluatePredictions,
                        target: FrontierTarget::WholeRSet,
                    });
                }
                return SchedulerDecision::Sleep;
            }
            // Drive alive: do not short-circuit; fall through to
            // mode-aware frontier selection below. Drive-driven
            // PatternCandidate will be picked there if priority
            // ranks it above other items.
        }
        // Shape (α) extra path — fire EP if drive signal is
        // deeply negative AND EP would have something to report.
        if ctx.normalized_drive_signal < STEP3B_ALPHA_LOW_SIGNAL_THRESHOLD
            && !ctx.rset.axioms().is_empty()
            && Self::predictions_have_pending_delta(ctx)
        {
            return SchedulerDecision::Execute(ActionPlan {
                action_kind: ActionKind::EvaluatePredictions,
                target: FrontierTarget::WholeRSet,
            });
        }
        let _ = STEP3B_NORMALIZED_SIGNAL_THRESHOLD;

        match ctx.mode {
            RuntimeMode::Expand => {
                // Should we transition to Consolidate?
                if self.recent_positive_discovers(ctx) >= self.min_recent_gains
                    && Self::has_consolidate_work(ctx)
                {
                    return self.switch_or_sleep(ctx, RuntimeMode::Consolidate);
                }
                // Pick an Expand-shaped action. Pattern-cooldown
                // gate: when DiscoverPatterns is consistently
                // unproductive, skip those items and prefer
                // TheoryCandidate. ADR 0052 / B1+.
                let pattern_cool = self.pattern_cooldown_active(ctx);
                let meta_meta_cool = self.meta_meta_cooldown_active(ctx);
                // ADR 0061 / Phase H1.1 — promoted-pair priority bias.
                let bonus_kinds = Self::h1_1_bonus_kinds(ctx);
                if let Some(item) = Self::pick_top_biased(
                    ctx,
                    |it| match it.kind {
                        FrontierKind::TheoryCandidate => true,
                        FrontierKind::PatternCandidate => !pattern_cool,
                        FrontierKind::MetaMetaCandidate => !meta_meta_cool,
                        FrontierKind::CompositeCandidate => true,
                        FrontierKind::ShapeFamilyDiscoveryCandidate => true,
                        _ => false,
                    },
                    &bonus_kinds,
                ) {
                    return SchedulerDecision::Execute(ActionPlan {
                        action_kind: Self::execute_for_kind(item.kind),
                        target: item.target.clone(),
                    });
                }
                // No expand work. Try consolidate or reflect.
                if Self::has_consolidate_work(ctx) {
                    self.switch_or_sleep(ctx, RuntimeMode::Consolidate)
                } else {
                    self.switch_or_sleep(ctx, RuntimeMode::Reflect)
                }
            }

            RuntimeMode::Consolidate => {
                if !Self::has_consolidate_work(ctx) {
                    return self.switch_or_sleep(ctx, RuntimeMode::Reflect);
                }
                if let Some(item) = Self::pick_top(ctx, |it| {
                    matches!(
                        it.kind,
                        FrontierKind::LowValueObjectForPrune
                            | FrontierKind::TheoryNeedsRelations
                            | FrontierKind::EstablishedPromotion
                    )
                }) {
                    return SchedulerDecision::Execute(ActionPlan {
                        action_kind: Self::execute_for_kind(item.kind),
                        target: item.target.clone(),
                    });
                }
                self.switch_or_sleep(ctx, RuntimeMode::Reflect)
            }

            RuntimeMode::Reflect => {
                // ADR 0059 / Phase G1.5's EvaluatePredictions check
                // moved to top-level `choose`. Reflect arm reverts
                // to the B-line + G0 fallback chain.
                if self.has_expand_work(ctx) {
                    self.switch_or_sleep(ctx, RuntimeMode::Expand)
                } else if Self::has_consolidate_work(ctx) {
                    self.switch_or_sleep(ctx, RuntimeMode::Consolidate)
                } else {
                    // ADR 0057 / Phase G0: sleep suppression under
                    // anomaly pressure (kept as fallback).
                    if !ctx.rset.unexplained_data_edges().is_empty()
                        && !self.would_thrash(
                            ctx,
                            ctx.mode,
                            RuntimeMode::Expand,
                        )
                    {
                        SchedulerDecision::SwitchMode(RuntimeMode::Expand)
                    } else {
                        SchedulerDecision::Sleep
                    }
                }
            }
        }
    }
}
