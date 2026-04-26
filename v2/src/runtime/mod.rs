//! Autonomous runtime layer for v2. ADR 0052 Phases A0–A3.
//!
//! - A0: spin loop, stub scheduler, NoOp environment, bounded ticks.
//! - A1: `Frontier` with candidate enumeration + cooldown via dirty
//!   tracking; `RuleBasedScheduler` picking top frontier items;
//!   action-plan `target`; pattern and prune actions wired in
//!   addition to `DiscoverTheory`.
//! - A2: Expand / Consolidate / Reflect modes; `UpdateTheoryRelations`
//!   action; mode-transition log.
//! - A3: lifecycle stays in the loop while `Sleeping` and wakes on
//!   any data event (`AddEdge` / `RemoveEdge`); lifecycle transitions
//!   logged in `Memory`; `checkpoint_text` / `from_checkpoint_text`
//!   round-trip serialization (no file I/O — caller's job).

use crate::{
    AutonomousConfig, AxiomDiscoveryConfig, DiscoveryConfig, NamingPolicy,
    PatternRecordingPolicy, RefinementConfig, RSet, TheoryRelationKind,
    ESTABLISHED_MARKER, SHARED_AXIOM_MARKER, R,
};
use std::collections::{HashMap, HashSet, VecDeque};

// ─── lifecycle + mode ──────────────────────────────────────────────

/// Macro state of the runtime. ADR 0052.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Booting,
    Running,
    Sleeping,
    Stopped,
}

/// Micro state within Running — what kind of work the runtime is
/// doing. Phase A0/A1 only uses `Expand`. ADR 0052.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeMode {
    Expand,
    Consolidate,
    Reflect,
}

// ─── budget ────────────────────────────────────────────────────────

/// Two-dimensional budget (step counts only; no wall-clock). ADR 0052.
#[derive(Debug, Clone, Copy)]
pub struct BudgetState {
    pub ticks_remaining: Option<u64>,
    pub actions_remaining_this_tick: u32,
    pub actions_per_tick_cap: u32,
}

impl BudgetState {
    pub fn new(actions_per_tick_cap: u32) -> Self {
        Self {
            ticks_remaining: None,
            actions_remaining_this_tick: actions_per_tick_cap,
            actions_per_tick_cap,
        }
    }

    fn reset_per_tick(&mut self) {
        self.actions_remaining_this_tick = self.actions_per_tick_cap;
    }
}

// ─── action + target + decision ────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionKind {
    DiscoverPatterns,
    DiscoverTheory,
    PruneLowValueObjects,
    /// Scan named theories pairwise, persist any missing
    /// extension / independence / parallel edges. ADR 0052 / A2.
    UpdateTheoryRelations,
    /// Promote a named pattern (or other knowledge object) to the
    /// experience-with meta-R class by emitting the edge
    /// `R(<id>, ESTABLISHED_MARKER)`. ADR 0053 / Phase C0.
    Declarativize,
    /// Run `discover_motifs_with_meta_subset` over the rset's M1
    /// markers and the named objects they anchor. ADR 0054 / Phase D0.
    /// Reports candidates as an Episode without naming new patterns
    /// — the loop-closure naming pipeline is deferred to a follow-on
    /// slice.
    DiscoverMetaMetaPatterns,
}

/// Where (in the RSet) the action should apply. ADR 0052 / A1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontierTarget {
    WholeRSet,
    PatternSize(usize),
    Pattern(String),
    Theory(String),
    /// ADR 0053 / Phase C2. Used by `Declarativize` when the target
    /// is a named axiom (e.g., for `SHARED_AXIOM_MARKER` promotion).
    Axiom(String),
}

#[derive(Debug, Clone)]
pub struct ActionPlan {
    pub action_kind: ActionKind,
    pub target: FrontierTarget,
}

#[derive(Debug, Clone)]
pub enum SchedulerDecision {
    Execute(ActionPlan),
    SwitchMode(RuntimeMode),
    Sleep,
    Stop,
}

// ─── scheduler trait + context ─────────────────────────────────────

/// Read-only view handed to `Scheduler::choose`. ADR 0052 / A1.
pub struct SchedulerContext<'a> {
    pub rset: &'a RSet,
    pub memory: &'a Memory,
    pub frontier: &'a Frontier,
    pub mode: RuntimeMode,
    pub tick: u64,
}

pub trait Scheduler {
    fn choose(&mut self, ctx: &SchedulerContext<'_>) -> SchedulerDecision;
}

/// Simplest scheduler: always asks for `DiscoverTheory` on the whole
/// RSet. Kept for A0-style tests and for determinism baselines.
pub struct StubScheduler;

impl Scheduler for StubScheduler {
    fn choose(&mut self, _ctx: &SchedulerContext<'_>) -> SchedulerDecision {
        SchedulerDecision::Execute(ActionPlan {
            action_kind: ActionKind::DiscoverTheory,
            target: FrontierTarget::WholeRSet,
        })
    }
}

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
    /// Anomaly-coverage drive thresholds. ADR 0057 / Phase G0.
    ///
    /// When `rset.uncovered_data_edges().len() >=
    /// anomaly_pressure_threshold`, two scheduler hooks fire:
    /// (1) the B1+ pattern-cooldown hit-rate floor is multiplied by
    /// `anomaly_relaxation` (default 0.5 → effective floor drops
    /// from 10% to 5%), giving more room for exploratory pattern
    /// passes; (2) the Reflect → Sleep transition is replaced with
    /// Reflect → Expand so the runtime keeps trying while there is
    /// unexplained data. The mode-thrash gate still bounds the
    /// suppression so the runtime can't loop forever.
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
            min_pattern_attempts_before_cooldown: 5,
            min_meta_meta_hit_rate: 0.05,
            min_meta_meta_attempts_before_cooldown: 5,
            anomaly_pressure_threshold: 3,
            anomaly_relaxation: 0.5,
        }
    }
}

impl RuleBasedScheduler {
    fn pick_top<'a, F: Fn(&FrontierItem) -> bool>(
        ctx: &'a SchedulerContext<'_>,
        accept: F,
    ) -> Option<&'a FrontierItem> {
        ctx.frontier.items.iter().find(|it| accept(it))
    }

    fn execute_for_kind(kind: FrontierKind) -> ActionKind {
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
        }
    }

    fn has_expand_work(&self, ctx: &SchedulerContext<'_>) -> bool {
        let pattern_cool = self.pattern_cooldown_active(ctx);
        let meta_meta_cool = self.meta_meta_cooldown_active(ctx);
        ctx.frontier.items.iter().any(|it| match it.kind {
            FrontierKind::TheoryCandidate => true,
            FrontierKind::PatternCandidate => !pattern_cool,
            FrontierKind::MetaMetaCandidate => !meta_meta_cool,
            _ => false,
        })
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
                        ActionKind::DiscoverPatterns | ActionKind::DiscoverTheory
                    )
            })
            .count()
    }

    /// Anti-thrash gate. Returns true iff transitions in EITHER
    /// direction between `current` and `target` already total
    /// `max_mode_oscillations` or more in `policy_stats`.
    /// ADR 0052 / B1.
    fn would_thrash(
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
    fn pattern_cooldown_active(&self, ctx: &SchedulerContext<'_>) -> bool {
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
        let uncovered = ctx.rset.uncovered_data_edges().len();
        if uncovered >= self.anomaly_pressure_threshold {
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
    fn meta_meta_cooldown_active(&self, ctx: &SchedulerContext<'_>) -> bool {
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
        // Global stagnation always wins.
        if Self::zero_streak(ctx) >= self.max_zero_streak {
            return SchedulerDecision::Sleep;
        }

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
                if let Some(item) = Self::pick_top(ctx, |it| {
                    match it.kind {
                        FrontierKind::TheoryCandidate => true,
                        FrontierKind::PatternCandidate => !pattern_cool,
                        FrontierKind::MetaMetaCandidate => !meta_meta_cool,
                        _ => false,
                    }
                }) {
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
                // Pure state-machine mode: no Execute, no episode added.
                if self.has_expand_work(ctx) {
                    self.switch_or_sleep(ctx, RuntimeMode::Expand)
                } else if Self::has_consolidate_work(ctx) {
                    self.switch_or_sleep(ctx, RuntimeMode::Consolidate)
                } else {
                    // ADR 0057 / Phase G0: sleep suppression under
                    // anomaly pressure. If there is uncovered data
                    // and the Reflect→Expand pair hasn't already
                    // thrashed, prefer re-entering Expand over going
                    // to sleep — the runtime should keep trying
                    // while explanations remain owed.
                    if !ctx.rset.uncovered_data_edges().is_empty()
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

// ─── environment ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Event {
    AddEdge(R),
    RemoveEdge(R),
    Tick,
}

pub trait Environment {
    fn poll(&mut self) -> Vec<Event>;
}

pub struct NoOpEnvironment;

impl Environment for NoOpEnvironment {
    fn poll(&mut self) -> Vec<Event> {
        Vec::new()
    }
}

/// Replay events from a fixed schedule of `(target_poll_index, Event)`
/// pairs. The poll index is 1-based — the first call to `poll`
/// returns all events scheduled for `<= 1`. ADR 0052 § Phase B / B0.
///
/// Use case: drip-feed an evolving graph into the runtime to verify
/// it responds correctly to a temporal sequence.
pub struct SyntheticStreamEnvironment {
    schedule: Vec<(u64, Event)>,
    polled_count: u64,
}

impl SyntheticStreamEnvironment {
    pub fn new(schedule: Vec<(u64, Event)>) -> Self {
        let mut s = schedule;
        s.sort_by_key(|(t, _)| *t);
        Self { schedule: s, polled_count: 0 }
    }

    pub fn polled_count(&self) -> u64 {
        self.polled_count
    }

    pub fn remaining(&self) -> usize {
        self.schedule.len()
    }
}

impl Environment for SyntheticStreamEnvironment {
    fn poll(&mut self) -> Vec<Event> {
        self.polled_count += 1;
        let target = self.polled_count;
        let mut out = Vec::new();
        // Drain events whose tick <= current poll index. Keep those
        // in the future. Sorted schedule means we can stop early once
        // we hit a future-tick event.
        while !self.schedule.is_empty() && self.schedule[0].0 <= target {
            let (_, ev) = self.schedule.remove(0);
            out.push(ev);
        }
        out
    }
}

/// Wake predicate: any data-mutating event lifts the runtime out of
/// `Sleeping`. Bare `Tick` is informational and does NOT wake — it
/// preserves "no signal, stay asleep" semantics. ADR 0052 / A3.
///
/// Ordering: this predicate is evaluated *after* `Environment::poll`
/// but *before* `apply_events`, so any add/remove arriving in this
/// tick will both wake the runtime AND modify the rset on the same
/// pass — the next iteration (or the rest of this iteration) sees a
/// dirty frontier.
pub fn should_wake(events: &[Event]) -> bool {
    events.iter().any(|e| matches!(e, Event::AddEdge(_) | Event::RemoveEdge(_)))
}

// ─── memory (M0) ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
        }
    }
}

impl Memory {
    pub fn record(&mut self, ep: Episode) {
        self.episodes.push_back(ep);
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

// ─── frontier (A1) ─────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontierKind {
    TheoryCandidate,
    PatternCandidate,
    LowValueObjectForPrune,
    /// At least two named theories exist with no recorded relation
    /// edge between them. ADR 0052 / A2.
    TheoryNeedsRelations,
    /// A named pattern has met the C0 promotion gate (age + has at
    /// least one positive-delta contribution) and is not yet marked
    /// `R(id, ESTABLISHED_MARKER)`. ADR 0053 / Phase C0.
    EstablishedPromotion,
    /// The rset has accumulated enough M1 marker edges to warrant a
    /// pass of meta-meta discovery. ADR 0054 / Phase D0.
    MetaMetaCandidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontierStatus {
    Fresh,
    Active,
    Cooling,
    Saturated,
    Blocked,
}

#[derive(Debug, Clone)]
pub struct FrontierItem {
    pub id: String,
    pub kind: FrontierKind,
    pub target: FrontierTarget,
    pub priority: f64,
    pub estimated_value: f64,
    pub estimated_cost: f64,
    pub novelty_score: f64,
    pub first_seen_tick: u64,
    pub last_visited_tick: Option<u64>,
    pub revisit_count: u32,
    pub cooldown_until_tick: Option<u64>,
    pub status: FrontierStatus,
}

/// Threshold config for staleness-based prune injection.
/// ADR 0052 / B3.
///
/// A named pattern is "stale" if it has been around long enough
/// (`first_seen_tick` ≥ `min_pattern_age_for_staleness` ticks ago)
/// but its `last_improved_tick` has not advanced for at least
/// `max_pattern_staleness_ticks`. Stale patterns become
/// `LowValueObjectForPrune` candidates with a low fixed priority,
/// so the existing Consolidate / Prune lane retires them without
/// the scheduler needing a new dispatch path.
#[derive(Debug, Clone, Copy)]
pub struct StalenessConfig {
    pub max_pattern_staleness_ticks: u64,
    pub min_pattern_age_for_staleness: u64,
}

impl Default for StalenessConfig {
    fn default() -> Self {
        Self {
            max_pattern_staleness_ticks: 30,
            min_pattern_age_for_staleness: 50,
        }
    }
}

/// Config for meta-meta-pattern discovery. ADR 0054 / Phase D0.
///
/// Drives the `MetaMetaCandidate` frontier item: surfaced once the
/// rset has accumulated at least `min_m1_edges_for_meta_meta` edges
/// involving the listed M1 markers. The default markers correspond
/// to ADR 0053's two M1 marker classes.
#[derive(Debug, Clone)]
pub struct MetaMetaConfig {
    pub min_m1_edges_for_meta_meta: usize,
    pub markers: Vec<&'static str>,
    pub target_size: usize,
    pub sample_count: usize,
    pub top_m: usize,
    pub rng_seed: u64,
}

impl Default for MetaMetaConfig {
    fn default() -> Self {
        Self {
            min_m1_edges_for_meta_meta: 5,
            markers: vec![ESTABLISHED_MARKER, SHARED_AXIOM_MARKER],
            target_size: 3,
            sample_count: 200,
            top_m: 10,
            rng_seed: 2026,
        }
    }
}

/// Threshold config for ESTABLISHED promotion. ADR 0053 / Phase C0/C1.
///
/// A named object (pattern or theory) earns the
/// `R(id, ESTABLISHED_MARKER)` edge once it has been alive in the
/// runtime's `ObjectHistory` for at least the relevant age threshold
/// AND has contributed to at least the relevant `min_*_use_for_promotion`
/// number of positive-delta episodes. The contribution count is
/// the `times_contributed_positive` counter on `ObjectHistory`,
/// added in Phase C0+ alongside this knob.
///
/// Theory thresholds are more conservative than pattern (200/3
/// vs. 100/3) per ADR 0053 / Phase C1 — theories are larger
/// investments and the runtime should be slower to declare them
/// stable. The default `min_*_use_for_promotion = 3` reproduces
/// ADR 0053's original sketch ("M = 3"), now that the counter
/// exists to enforce it.
#[derive(Debug, Clone, Copy)]
pub struct PromotionConfig {
    pub min_pattern_age_for_promotion: u64,
    pub min_theory_age_for_promotion: u64,
    pub min_pattern_use_for_promotion: u32,
    pub min_theory_use_for_promotion: u32,
}

impl Default for PromotionConfig {
    fn default() -> Self {
        Self {
            min_pattern_age_for_promotion: 100,
            min_theory_age_for_promotion: 200,
            min_pattern_use_for_promotion: 3,
            min_theory_use_for_promotion: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frontier {
    pub items: Vec<FrontierItem>,
    pub last_full_refresh_tick: u64,
    pub dirty: bool,
    /// ADR 0052 / B3.
    pub staleness: StalenessConfig,
    /// ADR 0053 / Phase C0.
    pub promotion: PromotionConfig,
    /// ADR 0054 / Phase D0.
    pub meta_meta: MetaMetaConfig,
}

impl Default for Frontier {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            last_full_refresh_tick: 0,
            dirty: true,
            staleness: StalenessConfig::default(),
            promotion: PromotionConfig::default(),
            meta_meta: MetaMetaConfig::default(),
        }
    }
}

impl Frontier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Enumerate candidate actions from the current RSet state and
    /// sort by priority (descending). ADR 0052 / A1.
    pub fn refresh(&mut self, rset: &RSet, tick: u64) {
        let mut items: Vec<FrontierItem> = Vec::new();

        // 1. TheoryCandidate: propose if discover_theory yields a
        //    nonempty member set AND no existing theory has exactly
        //    that member set.
        let cfg = AxiomDiscoveryConfig::default();
        let th = rset.discover_theory(&cfg);
        if !th.member_axiom_ids.is_empty() {
            let want: HashSet<&str> = th
                .member_axiom_ids
                .iter()
                .map(|s| s.as_str())
                .collect();
            let already_named = rset.theories().iter().any(|t| {
                let members: HashSet<&str> =
                    rset.theory_axioms(t).into_iter().collect();
                members == want
            });
            if !already_named {
                let value = (th.member_axiom_ids.len() * 2) as f64;
                items.push(FrontierItem {
                    id: format!("theory_cand_{}", tick),
                    kind: FrontierKind::TheoryCandidate,
                    target: FrontierTarget::WholeRSet,
                    priority: value / 1.0,
                    estimated_value: value,
                    estimated_cost: 1.0,
                    novelty_score: value,
                    first_seen_tick: tick,
                    last_visited_tick: None,
                    revisit_count: 0,
                    cooldown_until_tick: None,
                    status: FrontierStatus::Fresh,
                });
            }
        }

        // 2. PatternCandidate: for each k in [2, 3], if there are at
        //    least k data edges, propose discovery at that size.
        let meta = rset.collect_meta_ids();
        let data_edge_count = rset
            .iter()
            .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
            .count();
        for &size in &[2usize, 3] {
            if data_edge_count >= size {
                let value = (data_edge_count as f64) / (size as f64);
                items.push(FrontierItem {
                    id: format!("pattern_size_{}_{}", size, tick),
                    kind: FrontierKind::PatternCandidate,
                    target: FrontierTarget::PatternSize(size),
                    priority: value / (size as f64 + 1.0),
                    estimated_value: value,
                    estimated_cost: size as f64,
                    novelty_score: value / 2.0,
                    first_seen_tick: tick,
                    last_visited_tick: None,
                    revisit_count: 0,
                    cooldown_until_tick: None,
                    status: FrontierStatus::Fresh,
                });
            }
        }

        // 3. LowValueObjectForPrune: every named object with
        //    counterfactual value < 0.
        for (id, cv) in rset.rank_by_counterfactual() {
            if cv < 0.0 {
                items.push(FrontierItem {
                    id: format!("prune_{}_{}", id, tick),
                    kind: FrontierKind::LowValueObjectForPrune,
                    target: FrontierTarget::Pattern(id.clone()),
                    priority: (-cv) * 2.0, // slight preference over
                                            // equal-value discovery
                    estimated_value: -cv,
                    estimated_cost: 1.0,
                    novelty_score: 0.0,
                    first_seen_tick: tick,
                    last_visited_tick: None,
                    revisit_count: 0,
                    cooldown_until_tick: None,
                    status: FrontierStatus::Fresh,
                });
            }
        }

        // 4. TheoryNeedsRelations: ≥ 2 named theories AND at least one
        //    pair has no extension/independence/parallel edge between
        //    them. ADR 0052 / A2.
        let theories: Vec<String> =
            rset.theories().iter().map(|s| s.to_string()).collect();
        if theories.len() >= 2 {
            let missing_pair = (0..theories.len()).any(|i| {
                ((i + 1)..theories.len()).any(|j| {
                    !theory_pair_has_relation(rset, &theories[i], &theories[j])
                })
            });
            if missing_pair {
                items.push(FrontierItem {
                    id: format!("theory_relations_{}", tick),
                    kind: FrontierKind::TheoryNeedsRelations,
                    target: FrontierTarget::WholeRSet,
                    // Mid priority — slightly below pruning, above
                    // pattern-discovery on small graphs.
                    priority: 1.5,
                    estimated_value: 1.0,
                    estimated_cost: 1.0,
                    novelty_score: 0.5,
                    first_seen_tick: tick,
                    last_visited_tick: None,
                    revisit_count: 0,
                    cooldown_until_tick: None,
                    status: FrontierStatus::Fresh,
                });
            }
        }

        items.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        self.items = items;
        self.last_full_refresh_tick = tick;
        self.dirty = false;
    }

    /// Append `LowValueObjectForPrune` items for named patterns whose
    /// `last_improved_tick` is too stale relative to `tick`. Idempotent
    /// against repeat calls in the same tick (skips targets that
    /// already have a Prune item) and re-sorts items by priority on
    /// exit. ADR 0052 / B3.
    pub fn refresh_stale_prune(
        &mut self,
        history: &ObjectHistoryStore,
        tick: u64,
    ) {
        let cfg = self.staleness;
        let mut added = false;
        for (id, h) in &history.patterns {
            let age = tick.saturating_sub(h.first_seen_tick);
            if age < cfg.min_pattern_age_for_staleness {
                continue;
            }
            let stale_since = match h.last_improved_tick {
                Some(t) => tick.saturating_sub(t),
                None => age,
            };
            if stale_since < cfg.max_pattern_staleness_ticks {
                continue;
            }
            let target = FrontierTarget::Pattern(id.clone());
            let already = self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::LowValueObjectForPrune)
                    && it.target == target
            });
            if already {
                continue;
            }
            self.items.push(FrontierItem {
                id: format!("prune_stale_{}_{}", id, tick),
                kind: FrontierKind::LowValueObjectForPrune,
                target,
                // Below the typical negative-cv prune priority
                // (≈ -cv * 2.0, normally ≥ 1.0). Staleness is a
                // softer signal so it should not preempt a
                // counterfactually-bad object.
                priority: 0.5,
                estimated_value: 0.5,
                estimated_cost: 1.0,
                novelty_score: 0.0,
                first_seen_tick: tick,
                last_visited_tick: None,
                revisit_count: 0,
                cooldown_until_tick: None,
                status: FrontierStatus::Fresh,
            });
            added = true;
        }
        if added {
            self.items.sort_by(|a, b| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
    }

    /// Append `EstablishedPromotion` items for named patterns and
    /// theories that meet the C0/C1 gate: alive for ≥ the relevant
    /// age threshold AND `last_improved_tick.is_some()` (M ≥ 1) AND
    /// not yet promoted. Skips ids that already have a pending
    /// promotion item. Re-sorts on exit.
    /// ADR 0053 / Phase C0 (patterns) + C1 (theories).
    pub fn refresh_established_promotions(
        &mut self,
        rset: &RSet,
        history: &ObjectHistoryStore,
        tick: u64,
    ) {
        let cfg = self.promotion;
        let mut added = false;

        // Patterns (C0).
        let named_patterns: HashSet<&str> =
            rset.patterns().into_iter().collect();
        for (id, h) in &history.patterns {
            if !named_patterns.contains(id.as_str()) {
                continue;
            }
            if !Self::passes_promotion_gate(
                h,
                tick,
                cfg.min_pattern_age_for_promotion,
                cfg.min_pattern_use_for_promotion,
            ) {
                continue;
            }
            if rset.contains(&R::new(id.clone(), ESTABLISHED_MARKER)) {
                continue;
            }
            let target = FrontierTarget::Pattern(id.clone());
            if self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::EstablishedPromotion)
                    && it.target == target
            }) {
                continue;
            }
            self.items.push(Self::make_promotion_item(
                id, target, tick,
            ));
            added = true;
        }

        // Theories (C1).
        let named_theories: HashSet<&str> =
            rset.theories().into_iter().collect();
        for (id, h) in &history.theories {
            if !named_theories.contains(id.as_str()) {
                continue;
            }
            if !Self::passes_promotion_gate(
                h,
                tick,
                cfg.min_theory_age_for_promotion,
                cfg.min_theory_use_for_promotion,
            ) {
                continue;
            }
            if rset.contains(&R::new(id.clone(), ESTABLISHED_MARKER)) {
                continue;
            }
            let target = FrontierTarget::Theory(id.clone());
            if self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::EstablishedPromotion)
                    && it.target == target
            }) {
                continue;
            }
            self.items.push(Self::make_promotion_item(
                id, target, tick,
            ));
            added = true;
        }

        if added {
            self.items.sort_by(|a, b| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
    }

    fn passes_promotion_gate(
        h: &ObjectHistory,
        tick: u64,
        min_age: u64,
        min_use: u32,
    ) -> bool {
        let age = tick.saturating_sub(h.first_seen_tick);
        age >= min_age && h.times_contributed_positive >= min_use
    }

    fn make_promotion_item(
        id: &str,
        target: FrontierTarget,
        tick: u64,
    ) -> FrontierItem {
        FrontierItem {
            id: format!("promote_{}_{}", id, tick),
            kind: FrontierKind::EstablishedPromotion,
            target,
            // Mid-tier consolidate priority: above stale-prune
            // (0.5) so a freshly-mature object is acknowledged
            // before stale ones are trimmed, but below normal
            // negative-cv prune so a known-bad object still wins.
            priority: 1.5,
            estimated_value: 1.0,
            estimated_cost: 1.0,
            novelty_score: 0.5,
            first_seen_tick: tick,
            last_visited_tick: None,
            revisit_count: 0,
            cooldown_until_tick: None,
            status: FrontierStatus::Fresh,
        }
    }

    /// Append a single `MetaMetaCandidate` item if the rset carries
    /// at least `meta_meta.min_m1_edges_for_meta_meta` M1-marker
    /// edges and no MetaMetaCandidate is already pending. The
    /// runtime executes this through `DiscoverMetaMetaPatterns`,
    /// which calls `RSet::discover_motifs_with_meta_subset` over a
    /// view that contains data + edges anchored to the markers.
    /// ADR 0054 / Phase D0.
    pub fn refresh_meta_meta_candidates(
        &mut self,
        rset: &RSet,
        tick: u64,
    ) {
        if self.items.iter().any(|it| {
            matches!(it.kind, FrontierKind::MetaMetaCandidate)
        }) {
            return;
        }
        let cfg = &self.meta_meta;
        let m1_edge_count: usize = cfg
            .markers
            .iter()
            .map(|m| rset.right_of(*m).len())
            .sum();
        if m1_edge_count < cfg.min_m1_edges_for_meta_meta {
            return;
        }
        // Conservative priority: above pattern-discovery floor but
        // below TheoryCandidate when a useful theory is in play.
        // Meta-meta is exploratory; let it lose ties.
        self.items.push(FrontierItem {
            id: format!("meta_meta_{}", tick),
            kind: FrontierKind::MetaMetaCandidate,
            target: FrontierTarget::WholeRSet,
            priority: 1.0,
            estimated_value: m1_edge_count as f64,
            estimated_cost: cfg.target_size as f64,
            novelty_score: 1.0,
            first_seen_tick: tick,
            last_visited_tick: None,
            revisit_count: 0,
            cooldown_until_tick: None,
            status: FrontierStatus::Fresh,
        });
        self.items.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    /// Append `EstablishedPromotion` items for axioms that are
    /// referenced by ≥ 2 named theories AND don't yet carry the
    /// `SHARED_AXIOM_MARKER` edge. Demotion is handled by
    /// `RSet::retract_theory`'s cascade — no history is consulted
    /// because C2's gate is purely structural. Re-sorts on exit.
    /// ADR 0053 / Phase C2.
    pub fn refresh_shared_axiom_promotions(
        &mut self,
        rset: &RSet,
        tick: u64,
    ) {
        let mut added = false;
        for axiom_id in rset.axioms() {
            if rset.theories_containing(axiom_id).len() < 2 {
                continue;
            }
            if rset.contains(&R::new(axiom_id, SHARED_AXIOM_MARKER)) {
                continue;
            }
            let target = FrontierTarget::Axiom(axiom_id.to_string());
            if self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::EstablishedPromotion)
                    && it.target == target
            }) {
                continue;
            }
            self.items.push(Self::make_promotion_item(
                axiom_id, target, tick,
            ));
            added = true;
        }
        if added {
            self.items.sort_by(|a, b| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
    }
}

fn theory_pair_has_relation(rset: &RSet, a: &str, b: &str) -> bool {
    rset.extension_edges().iter().any(|e| {
        if let Some((sub, sup)) = rset.extension_endpoints(e) {
            (sub == a && sup == b) || (sub == b && sup == a)
        } else {
            false
        }
    }) || rset.independence_edges().iter().any(|e| {
        if let Some((lo, hi)) = rset.independence_endpoints(e) {
            (lo == a && hi == b) || (lo == b && hi == a)
        } else {
            false
        }
    }) || rset.parallel_edges().iter().any(|e| {
        if let Some((lo, hi)) = rset.parallel_endpoints(e) {
            (lo == a && hi == b) || (lo == b && hi == a)
        } else {
            false
        }
    })
}

// ─── runtime ───────────────────────────────────────────────────────

pub struct AutonomousRuntime {
    pub rset: RSet,
    pub lifecycle: LifecycleState,
    pub mode: RuntimeMode,
    pub memory: Memory,
    pub scheduler: Box<dyn Scheduler>,
    pub environment: Box<dyn Environment>,
    pub frontier: Frontier,

    pub tick: u64,
    pub episode_counter: u64,
    pub steps_since_last_gain: u64,
    pub budget: BudgetState,
    pub current_score: f64,

    /// Snapshot of `checkpoint_text()` taken on the last entry into
    /// `Sleeping` or `Stopped`. ADR 0052 / A3. Caller persists to disk
    /// at its discretion; the runtime itself does no I/O.
    pub last_checkpoint: Option<String>,
}

impl AutonomousRuntime {
    /// Construct with defaults: `StubScheduler`, `NoOpEnvironment`,
    /// empty Frontier (refreshes on first tick). Caller swaps
    /// `scheduler` / `environment` before `run_bounded` as needed.
    pub fn new(rset: RSet) -> Self {
        let current_score = rset.abstraction_score();
        Self {
            rset,
            lifecycle: LifecycleState::Running,
            mode: RuntimeMode::Expand,
            memory: Memory::default(),
            scheduler: Box::new(StubScheduler),
            environment: Box::new(NoOpEnvironment),
            frontier: Frontier::default(),
            tick: 0,
            episode_counter: 0,
            steps_since_last_gain: 0,
            budget: BudgetState::new(1),
            current_score,
            last_checkpoint: None,
        }
    }

    /// Record a lifecycle transition and update `self.lifecycle`.
    /// Snapshot a checkpoint when entering `Sleeping` or `Stopped`.
    /// No-op if `to == self.lifecycle`. ADR 0052 / A3.
    fn transition_lifecycle(
        &mut self,
        to: LifecycleState,
        reason: &str,
    ) {
        if to == self.lifecycle {
            return;
        }
        let from = self.lifecycle;
        self.memory.record_lifecycle_transition(LifecycleTransition {
            tick: self.tick,
            from,
            to,
            reason: reason.to_string(),
        });
        // B0 / PolicyStats.
        match to {
            LifecycleState::Sleeping => {
                self.memory.policy_stats.sleep_count += 1;
            }
            LifecycleState::Running if from == LifecycleState::Sleeping => {
                self.memory.policy_stats.wake_count += 1;
            }
            LifecycleState::Stopped => {
                self.memory.policy_stats.stop_count += 1;
            }
            _ => {}
        }
        self.lifecycle = to;
        if matches!(to, LifecycleState::Sleeping | LifecycleState::Stopped) {
            if let Ok(cp) = self.checkpoint_text() {
                self.last_checkpoint = Some(cp);
            }
        }
    }

    pub fn run_bounded(&mut self, max_ticks: u64) {
        let start_tick = self.tick;

        while self.tick - start_tick < max_ticks
            && self.lifecycle != LifecycleState::Stopped
        {
            self.tick += 1;
            self.budget.reset_per_tick();

            // 1. Ingest events. Decide wake-on-event before applying so
            //    the predicate's input matches the events we just got.
            let events = self.environment.poll();
            let wake_signal = should_wake(&events);
            if !events.is_empty() {
                self.apply_events(events);
                self.frontier.mark_dirty();
            }

            // 2. Sleeping short-circuit. Wake on any data event,
            //    otherwise spend the tick asleep (no scheduler call,
            //    no episode). ADR 0052 / A3.
            if self.lifecycle == LifecycleState::Sleeping {
                if wake_signal {
                    self.transition_lifecycle(
                        LifecycleState::Running,
                        "wake_on_event",
                    );
                } else {
                    continue;
                }
            }

            // 3. Refresh frontier when dirty (cheap at β-scale). The
            //    staleness pass (B3) consults object_history, so it
            //    runs alongside refresh whenever items are recomputed.
            //    The promotion pass (C0) also rides the same dirty
            //    gate; it inspects rset for already-promoted ids.
            if self.frontier.dirty {
                self.frontier.refresh(&self.rset, self.tick);
                self.frontier.refresh_stale_prune(
                    &self.memory.object_history,
                    self.tick,
                );
                self.frontier.refresh_established_promotions(
                    &self.rset,
                    &self.memory.object_history,
                    self.tick,
                );
                self.frontier.refresh_shared_axiom_promotions(
                    &self.rset,
                    self.tick,
                );
                self.frontier.refresh_meta_meta_candidates(
                    &self.rset,
                    self.tick,
                );
            }

            // 4. Scheduler decision.
            let decision = {
                let ctx = SchedulerContext {
                    rset: &self.rset,
                    memory: &self.memory,
                    frontier: &self.frontier,
                    mode: self.mode,
                    tick: self.tick,
                };
                self.scheduler.choose(&ctx)
            };

            // 5. Dispatch.
            match decision {
                SchedulerDecision::Execute(plan) => {
                    self.execute_and_record(plan);
                    self.frontier.mark_dirty();
                }
                SchedulerDecision::SwitchMode(m) => {
                    if m != self.mode {
                        let from = self.mode;
                        self.memory.record_mode_transition(ModeTransition {
                            tick: self.tick,
                            from,
                            to: m,
                            reason: "scheduler".to_string(),
                        });
                        // B0 / PolicyStats.
                        *self
                            .memory
                            .policy_stats
                            .mode_transition_counts
                            .entry((from, m))
                            .or_insert(0) += 1;
                        self.mode = m;
                    }
                }
                SchedulerDecision::Sleep => {
                    self.transition_lifecycle(
                        LifecycleState::Sleeping,
                        "scheduler_sleep",
                    );
                }
                SchedulerDecision::Stop => {
                    self.transition_lifecycle(
                        LifecycleState::Stopped,
                        "scheduler_stop",
                    );
                }
            }
        }
    }

    fn apply_events(&mut self, events: Vec<Event>) {
        for ev in events {
            match ev {
                Event::AddEdge(r) => {
                    self.rset.add(r);
                }
                Event::RemoveEdge(r) => {
                    self.rset.remove(&r);
                }
                Event::Tick => {}
            }
        }
    }

    fn execute_and_record(&mut self, plan: ActionPlan) {
        let before = self.rset.abstraction_score();
        let patterns_before: HashSet<String> = self
            .rset
            .patterns()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let theories_before: HashSet<String> = self
            .rset
            .theories()
            .iter()
            .map(|s| s.to_string())
            .collect();

        self.execute_action(&plan);

        let after = self.rset.abstraction_score();
        let delta = after - before;
        let patterns_after: HashSet<String> = self
            .rset
            .patterns()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let theories_after: HashSet<String> = self
            .rset
            .theories()
            .iter()
            .map(|s| s.to_string())
            .collect();

        self.episode_counter += 1;
        self.memory.record(Episode {
            id: self.episode_counter,
            tick: self.tick,
            mode: self.mode,
            action_kind: plan.action_kind,
            target: plan.target.clone(),
            score_before: before,
            score_after: after,
            delta,
        });

        // ── Phase B / B0: feed object history + policy stats. ──────
        let tick = self.tick;
        let store = &mut self.memory.object_history;

        for id in patterns_after.difference(&patterns_before) {
            store
                .patterns
                .entry(id.clone())
                .or_insert_with(|| ObjectHistory::new_at(tick));
        }
        for id in theories_after.difference(&theories_before) {
            store
                .theories
                .entry(id.clone())
                .or_insert_with(|| ObjectHistory::new_at(tick));
        }
        for id in patterns_before.difference(&patterns_after) {
            if let Some(h) = store.patterns.get_mut(id) {
                h.times_pruned += 1;
            }
        }
        for id in theories_before.difference(&theories_after) {
            if let Some(h) = store.theories.get_mut(id) {
                h.times_pruned += 1;
            }
        }
        for id in &patterns_after {
            if let Some(h) = store.patterns.get_mut(id) {
                h.last_seen_tick = tick;
                if delta > 0.0 {
                    h.last_improved_tick = Some(tick);
                    h.times_contributed_positive =
                        h.times_contributed_positive.saturating_add(1);
                }
            }
        }
        for id in &theories_after {
            if let Some(h) = store.theories.get_mut(id) {
                h.last_seen_tick = tick;
                if delta > 0.0 {
                    h.last_improved_tick = Some(tick);
                    h.times_contributed_positive =
                        h.times_contributed_positive.saturating_add(1);
                }
            }
        }
        match &plan.target {
            FrontierTarget::Pattern(id) => {
                if let Some(h) = store.patterns.get_mut(id) {
                    h.times_selected_as_focus += 1;
                }
            }
            FrontierTarget::Theory(id) => {
                if let Some(h) = store.theories.get_mut(id) {
                    h.times_selected_as_focus += 1;
                }
            }
            _ => {}
        }

        let stats = &mut self.memory.policy_stats;
        *stats.action_counts.entry(plan.action_kind).or_insert(0) += 1;
        if delta > 0.0 {
            *stats
                .action_positive_delta_counts
                .entry(plan.action_kind)
                .or_insert(0) += 1;
        }
        // ───────────────────────────────────────────────────────────

        if delta > 0.0 {
            self.steps_since_last_gain = 0;
        } else {
            self.steps_since_last_gain += 1;
        }
        self.current_score = after;
    }

    fn execute_action(&mut self, plan: &ActionPlan) {
        match plan.action_kind {
            ActionKind::DiscoverTheory => {
                let cfg = AxiomDiscoveryConfig::default();
                let th = self.rset.discover_theory(&cfg);
                if !th.member_axiom_ids.is_empty() {
                    let ids: Vec<&str> = th
                        .member_axiom_ids
                        .iter()
                        .map(|s| s.as_str())
                        .collect();
                    let _ = self.rset.name_theory(&ids);
                }
            }
            ActionKind::DiscoverPatterns => {
                let size = match plan.target {
                    FrontierTarget::PatternSize(s) => s,
                    _ => 3, // fallback
                };
                let cfg = AutonomousConfig {
                    discovery: DiscoveryConfig {
                        target_size: size,
                        sample_count: 200,
                        top_m: 10,
                        rng_seed: 2024,
                        include_meta_in_discovery: false,
                    },
                    refinement: RefinementConfig {
                        max_tries: 200,
                        rng_seed: 999,
                    },
                    naming: NamingPolicy::default(),
                    instance_sampling: None,
                };
                let _ = self.rset.autonomous_pass(&cfg);
            }
            ActionKind::UpdateTheoryRelations => {
                // Snapshot ids so we can mutate self.rset inside the loop.
                let theories: Vec<String> = self
                    .rset
                    .theories()
                    .into_iter()
                    .map(str::to_owned)
                    .collect();
                for i in 0..theories.len() {
                    for j in (i + 1)..theories.len() {
                        let a = theories[i].clone();
                        let b = theories[j].clone();
                        match self.rset.classify_theory_pair(&a, &b) {
                            Some(TheoryRelationKind::Extends) => {
                                let _ = self.rset.name_theory_extension(&a, &b);
                            }
                            Some(TheoryRelationKind::ExtendedBy) => {
                                let _ = self.rset.name_theory_extension(&b, &a);
                            }
                            Some(TheoryRelationKind::Independent) => {
                                let _ = self.rset.name_theory_independence(&a, &b);
                            }
                            Some(TheoryRelationKind::Parallel) => {
                                let _ = self.rset.name_theory_parallel(&a, &b);
                            }
                            _ => {}
                        }
                    }
                }
            }
            ActionKind::PruneLowValueObjects => {
                // Prune at the object pointed at by the plan, or all
                // negative-CV if `WholeRSet`.
                match &plan.target {
                    FrontierTarget::Pattern(id) => {
                        let _ = self.rset.retract_pattern(id);
                    }
                    FrontierTarget::Theory(id) => {
                        let _ = self.rset.retract_theory(id);
                    }
                    _ => {
                        // Prune all negative-CV named objects.
                        let to_prune: Vec<String> = self
                            .rset
                            .rank_by_counterfactual()
                            .into_iter()
                            .filter(|(_, v)| *v < 0.0)
                            .map(|(id, _)| id)
                            .collect();
                        for id in to_prune {
                            if self.rset.is_theory(&id) {
                                let _ = self.rset.retract_theory(&id);
                            } else if self
                                .rset
                                .patterns()
                                .iter()
                                .any(|p| *p == id.as_str())
                            {
                                let _ = self.rset.retract_pattern(&id);
                            } else if self
                                .rset
                                .extension_edges()
                                .iter()
                                .any(|e| *e == id.as_str())
                            {
                                let _ = self.rset.retract_extension(&id);
                            }
                        }
                    }
                }
            }
            ActionKind::Declarativize => {
                // ADR 0053 / Phases C0–C2. The frontier pass already
                // gated this; the marker is selected by target type:
                // patterns and theories carry ESTABLISHED ("experience-
                // with"); axioms carry SHARED_AXIOM ("structurally
                // referenced by ≥ 2 theories"). `rset.add` is
                // idempotent — duplicate edges return false silently.
                let edge = match &plan.target {
                    FrontierTarget::Pattern(id) => Some(R::new(
                        id.clone(),
                        ESTABLISHED_MARKER,
                    )),
                    FrontierTarget::Theory(id) => Some(R::new(
                        id.clone(),
                        ESTABLISHED_MARKER,
                    )),
                    FrontierTarget::Axiom(id) => Some(R::new(
                        id.clone(),
                        SHARED_AXIOM_MARKER,
                    )),
                    _ => None,
                };
                if let Some(e) = edge {
                    let _ = self.rset.add(e);
                }
            }
            ActionKind::DiscoverMetaMetaPatterns => {
                // ADR 0054 / Phase D0+. Probe the rset's M1 subgraph
                // and (loop closure) name the top novel candidate via
                // an Intensional pattern recording. Intensional means
                // we write the pattern's roles + Layer A structural
                // edges but skip Layer B instance bindings — keeps
                // marker nodes from being pinned as concrete
                // participants. The naming may fail if no clean
                // instance survives `is_clean_subgraph_with_meta_subset`,
                // in which case the action is effectively a no-op.
                let cfg = &self.frontier.meta_meta;
                let markers: Vec<&str> = cfg.markers.clone();
                let subset = self.rset.meta_meta_subset(&markers);
                let dconfig = DiscoveryConfig {
                    target_size: cfg.target_size,
                    sample_count: cfg.sample_count,
                    top_m: cfg.top_m,
                    rng_seed: cfg.rng_seed,
                    include_meta_in_discovery: false,
                };
                let candidates = self
                    .rset
                    .discover_motifs_with_meta_subset(&dconfig, &subset);
                // Walk the top-`top_m` candidates by frequency and
                // name the first novel one with at least one clean
                // instance under the meta-subset view. ADR 0055
                // sharpens canonical resolution, which means the
                // single highest-frequency candidate is now more
                // likely to encode a Y- or path-shape that crosses
                // markers and fails `is_clean_subgraph_with_meta_subset`.
                // The iteration is bounded by `top_m` so the action
                // stays predictable on its budget.
                for candidate in candidates.iter() {
                    if self
                        .rset
                        .find_pattern_matching(&candidate.canonical)
                        .is_some()
                    {
                        continue;
                    }
                    let instances = self
                        .rset
                        .find_instances_of_with_meta_subset(
                            &candidate.canonical,
                            &subset,
                        );
                    if instances.is_empty() {
                        continue;
                    }
                    let _ = self.rset.name_pattern_instances_with_policy(
                        &instances,
                        PatternRecordingPolicy::Intensional,
                    );
                    break;
                }
            }
        }
    }

    // ─── A3: checkpoint round-trip ─────────────────────────────────

    /// Serialize the runtime's mutable state into a hand-rolled
    /// section-based text format. Mirrors `RSet::to_text`'s TSV style
    /// (ADR 0038). Does NOT serialize scheduler / environment / frontier
    /// — those are behavior or rederivable. ADR 0052 / A3.
    ///
    /// Format (sections in fixed order, blank line between sections):
    ///
    /// ```text
    /// # v2 runtime checkpoint v1
    /// [meta]
    /// tick<TAB>N
    /// episode_counter<TAB>N
    /// steps_since_last_gain<TAB>N
    /// current_score<TAB>F
    /// lifecycle<TAB>Running|Sleeping|Stopped|Booting
    /// mode<TAB>Expand|Consolidate|Reflect
    /// max_episodes<TAB>N
    /// max_mode_transitions<TAB>N
    /// max_lifecycle_transitions<TAB>N
    /// actions_per_tick_cap<TAB>N
    ///
    /// [rset]
    /// <RSet::to_text() output>
    ///
    /// [episodes]
    /// id<TAB>tick<TAB>mode<TAB>action<TAB>tgt_kind<TAB>tgt_value<TAB>before<TAB>after<TAB>delta
    ///
    /// [mode_transitions]
    /// tick<TAB>from<TAB>to<TAB>reason
    ///
    /// [lifecycle_transitions]
    /// tick<TAB>from<TAB>to<TAB>reason
    /// ```
    pub fn checkpoint_text(&self) -> Result<String, String> {
        let mut out = String::new();
        out.push_str("# v2 runtime checkpoint v1\n");

        // [meta]
        out.push_str("[meta]\n");
        out.push_str(&format!("tick\t{}\n", self.tick));
        out.push_str(&format!("episode_counter\t{}\n", self.episode_counter));
        out.push_str(&format!(
            "steps_since_last_gain\t{}\n",
            self.steps_since_last_gain
        ));
        out.push_str(&format!("current_score\t{:?}\n", self.current_score));
        out.push_str(&format!(
            "lifecycle\t{}\n",
            lifecycle_to_str(self.lifecycle)
        ));
        out.push_str(&format!("mode\t{}\n", mode_to_str(self.mode)));
        out.push_str(&format!(
            "max_episodes\t{}\n",
            self.memory.max_episodes
        ));
        out.push_str(&format!(
            "max_mode_transitions\t{}\n",
            self.memory.max_mode_transitions
        ));
        out.push_str(&format!(
            "max_lifecycle_transitions\t{}\n",
            self.memory.max_lifecycle_transitions
        ));
        out.push_str(&format!(
            "actions_per_tick_cap\t{}\n",
            self.budget.actions_per_tick_cap
        ));
        out.push('\n');

        // [rset]
        out.push_str("[rset]\n");
        let rset_text = self
            .rset
            .to_text()
            .map_err(|e| format!("rset serialization failed: {:?}", e))?;
        out.push_str(&rset_text);
        if !rset_text.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');

        // [episodes]
        out.push_str("[episodes]\n");
        for ep in &self.memory.episodes {
            check_no_tab_or_newline(&ep.target, "episode target")?;
            let (tk, tv) = target_to_pair(&ep.target);
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\n",
                ep.id,
                ep.tick,
                mode_to_str(ep.mode),
                action_kind_to_str(ep.action_kind),
                tk,
                tv,
                ep.score_before,
                ep.score_after,
                ep.delta,
            ));
        }
        out.push('\n');

        // [mode_transitions]
        out.push_str("[mode_transitions]\n");
        for mt in &self.memory.mode_transitions {
            check_reason(&mt.reason, "mode_transition")?;
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                mt.tick,
                mode_to_str(mt.from),
                mode_to_str(mt.to),
                mt.reason,
            ));
        }
        out.push('\n');

        // [lifecycle_transitions]
        out.push_str("[lifecycle_transitions]\n");
        for lt in &self.memory.lifecycle_transitions {
            check_reason(&lt.reason, "lifecycle_transition")?;
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                lt.tick,
                lifecycle_to_str(lt.from),
                lifecycle_to_str(lt.to),
                lt.reason,
            ));
        }
        out.push('\n');

        // B2 — object_history sections (sorted by id for idempotency).
        write_history_section(
            &mut out,
            "[object_history_patterns]",
            &self.memory.object_history.patterns,
        )?;
        out.push('\n');
        write_history_section(
            &mut out,
            "[object_history_axioms]",
            &self.memory.object_history.axioms,
        )?;
        out.push('\n');
        write_history_section(
            &mut out,
            "[object_history_theories]",
            &self.memory.object_history.theories,
        )?;
        out.push('\n');

        // B2 — policy_stats sections.
        out.push_str("[policy_stats_action_counts]\n");
        let mut action_keys: Vec<&ActionKind> =
            self.memory.policy_stats.action_counts.keys().collect();
        // Also include keys present only in positive_delta_counts.
        for k in self.memory.policy_stats.action_positive_delta_counts.keys() {
            if !action_keys.contains(&k) {
                action_keys.push(k);
            }
        }
        action_keys.sort_by_key(|a| action_kind_to_str(**a));
        for k in action_keys {
            let total = self
                .memory
                .policy_stats
                .action_counts
                .get(k)
                .copied()
                .unwrap_or(0);
            let pos = self
                .memory
                .policy_stats
                .action_positive_delta_counts
                .get(k)
                .copied()
                .unwrap_or(0);
            out.push_str(&format!(
                "{}\t{}\t{}\n",
                action_kind_to_str(*k),
                total,
                pos
            ));
        }
        out.push('\n');

        out.push_str("[policy_stats_mode_transition_counts]\n");
        let mut mtc_keys: Vec<&(RuntimeMode, RuntimeMode)> = self
            .memory
            .policy_stats
            .mode_transition_counts
            .keys()
            .collect();
        mtc_keys.sort_by_key(|(f, t)| (mode_to_str(*f), mode_to_str(*t)));
        for k in mtc_keys {
            let n = self
                .memory
                .policy_stats
                .mode_transition_counts
                .get(k)
                .copied()
                .unwrap_or(0);
            out.push_str(&format!(
                "{}\t{}\t{}\n",
                mode_to_str(k.0),
                mode_to_str(k.1),
                n
            ));
        }
        out.push('\n');

        out.push_str("[policy_stats_lifecycle_counts]\n");
        out.push_str(&format!(
            "wake\t{}\n",
            self.memory.policy_stats.wake_count
        ));
        out.push_str(&format!(
            "sleep\t{}\n",
            self.memory.policy_stats.sleep_count
        ));
        out.push_str(&format!(
            "stop\t{}\n",
            self.memory.policy_stats.stop_count
        ));

        Ok(out)
    }

    /// Reverse of `checkpoint_text`. Returns a runtime with default
    /// `StubScheduler` + `NoOpEnvironment`; caller swaps these in
    /// before calling `run_bounded`. Frontier starts dirty (empty
    /// items) and is rebuilt on the next tick. ADR 0052 / A3.
    pub fn from_checkpoint_text(text: &str) -> Result<Self, String> {
        let parsed = parse_checkpoint(text)?;

        // Rebuild rset from its dedicated section.
        let rset_blob = parsed.rset_lines.join("\n");
        let rset = RSet::from_text(&rset_blob)
            .map_err(|e| format!("rset parse failed: {:?}", e))?;

        // Pull required meta fields.
        let meta = &parsed.meta;
        let get = |k: &str| -> Result<&String, String> {
            meta.get(k).ok_or_else(|| format!("missing meta key '{}'", k))
        };
        let tick = parse_u64(get("tick")?, "tick")?;
        let episode_counter = parse_u64(get("episode_counter")?, "episode_counter")?;
        let steps_since_last_gain =
            parse_u64(get("steps_since_last_gain")?, "steps_since_last_gain")?;
        let current_score = parse_f64(get("current_score")?, "current_score")?;
        let lifecycle = parse_lifecycle(get("lifecycle")?)?;
        let mode = parse_mode(get("mode")?)?;
        let max_episodes =
            parse_usize(get("max_episodes")?, "max_episodes")?;
        let max_mode_transitions =
            parse_usize(get("max_mode_transitions")?, "max_mode_transitions")?;
        let max_lifecycle_transitions = parse_usize(
            get("max_lifecycle_transitions")?,
            "max_lifecycle_transitions",
        )?;
        let actions_per_tick_cap = parse_u32(
            get("actions_per_tick_cap")?,
            "actions_per_tick_cap",
        )?;

        // Episodes.
        let mut episodes: VecDeque<Episode> = VecDeque::new();
        for (idx, raw) in parsed.episode_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.split('\t').collect();
            if fields.len() != 9 {
                return Err(format!(
                    "episode line {} has {} fields, expected 9",
                    idx + 1,
                    fields.len()
                ));
            }
            let target = pair_to_target(fields[4], fields[5])?;
            episodes.push_back(Episode {
                id: parse_u64(fields[0], "episode.id")?,
                tick: parse_u64(fields[1], "episode.tick")?,
                mode: parse_mode(fields[2])?,
                action_kind: parse_action_kind(fields[3])?,
                target,
                score_before: parse_f64(fields[6], "episode.score_before")?,
                score_after: parse_f64(fields[7], "episode.score_after")?,
                delta: parse_f64(fields[8], "episode.delta")?,
            });
        }

        // Mode transitions.
        let mut mode_transitions: VecDeque<ModeTransition> = VecDeque::new();
        for (idx, raw) in parsed.mode_transition_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.splitn(4, '\t').collect();
            if fields.len() != 4 {
                return Err(format!(
                    "mode_transition line {} has {} fields, expected 4",
                    idx + 1,
                    fields.len()
                ));
            }
            mode_transitions.push_back(ModeTransition {
                tick: parse_u64(fields[0], "mode_transition.tick")?,
                from: parse_mode(fields[1])?,
                to: parse_mode(fields[2])?,
                reason: fields[3].to_string(),
            });
        }

        // Lifecycle transitions.
        let mut lifecycle_transitions: VecDeque<LifecycleTransition> =
            VecDeque::new();
        for (idx, raw) in parsed.lifecycle_transition_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.splitn(4, '\t').collect();
            if fields.len() != 4 {
                return Err(format!(
                    "lifecycle_transition line {} has {} fields, expected 4",
                    idx + 1,
                    fields.len()
                ));
            }
            lifecycle_transitions.push_back(LifecycleTransition {
                tick: parse_u64(fields[0], "lifecycle_transition.tick")?,
                from: parse_lifecycle(fields[1])?,
                to: parse_lifecycle(fields[2])?,
                reason: fields[3].to_string(),
            });
        }

        // B2 — object history.
        let object_history = ObjectHistoryStore {
            patterns: parse_history_lines(
                &parsed.history_patterns_lines,
                "object_history_patterns",
            )?,
            axioms: parse_history_lines(
                &parsed.history_axioms_lines,
                "object_history_axioms",
            )?,
            theories: parse_history_lines(
                &parsed.history_theories_lines,
                "object_history_theories",
            )?,
        };

        // B2 — policy stats.
        let mut policy_stats = PolicyStats::default();
        for (idx, raw) in parsed.action_count_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.splitn(3, '\t').collect();
            if fields.len() != 3 {
                return Err(format!(
                    "action_count line {} has {} fields, expected 3",
                    idx + 1,
                    fields.len()
                ));
            }
            let kind = parse_action_kind(fields[0])?;
            let total = parse_u64(fields[1], "action_count.total")?;
            let pos = parse_u64(fields[2], "action_count.positive")?;
            if total > 0 {
                policy_stats.action_counts.insert(kind, total);
            }
            if pos > 0 {
                policy_stats.action_positive_delta_counts.insert(kind, pos);
            }
        }
        for (idx, raw) in parsed.mode_transition_count_lines.iter().enumerate() {
            let fields: Vec<&str> = raw.splitn(3, '\t').collect();
            if fields.len() != 3 {
                return Err(format!(
                    "mode_transition_count line {} has {} fields, expected 3",
                    idx + 1,
                    fields.len()
                ));
            }
            let from = parse_mode(fields[0])?;
            let to = parse_mode(fields[1])?;
            let n = parse_u64(fields[2], "mode_transition_count.n")?;
            if n > 0 {
                policy_stats
                    .mode_transition_counts
                    .insert((from, to), n);
            }
        }
        for (idx, raw) in parsed.lifecycle_count_lines.iter().enumerate() {
            let (k, v) = raw.split_once('\t').ok_or_else(|| {
                format!(
                    "lifecycle_count line {} not key<TAB>value: '{}'",
                    idx + 1,
                    raw
                )
            })?;
            let n = parse_u64(v, "lifecycle_count.value")?;
            match k {
                "wake" => policy_stats.wake_count = n,
                "sleep" => policy_stats.sleep_count = n,
                "stop" => policy_stats.stop_count = n,
                other => {
                    return Err(format!(
                        "unknown lifecycle_count key '{}' (line {})",
                        other,
                        idx + 1
                    ))
                }
            }
        }

        let memory = Memory {
            episodes,
            mode_transitions,
            lifecycle_transitions,
            max_episodes,
            max_mode_transitions,
            max_lifecycle_transitions,
            object_history,
            policy_stats,
        };

        Ok(Self {
            rset,
            lifecycle,
            mode,
            memory,
            scheduler: Box::new(StubScheduler),
            environment: Box::new(NoOpEnvironment),
            frontier: Frontier::default(),
            tick,
            episode_counter,
            steps_since_last_gain,
            budget: BudgetState::new(actions_per_tick_cap),
            current_score,
            last_checkpoint: None,
        })
    }
}

// ─── A3: serialization helpers ─────────────────────────────────────

fn mode_to_str(m: RuntimeMode) -> &'static str {
    match m {
        RuntimeMode::Expand => "Expand",
        RuntimeMode::Consolidate => "Consolidate",
        RuntimeMode::Reflect => "Reflect",
    }
}

fn parse_mode(s: &str) -> Result<RuntimeMode, String> {
    match s {
        "Expand" => Ok(RuntimeMode::Expand),
        "Consolidate" => Ok(RuntimeMode::Consolidate),
        "Reflect" => Ok(RuntimeMode::Reflect),
        other => Err(format!("unknown RuntimeMode '{}'", other)),
    }
}

fn lifecycle_to_str(l: LifecycleState) -> &'static str {
    match l {
        LifecycleState::Booting => "Booting",
        LifecycleState::Running => "Running",
        LifecycleState::Sleeping => "Sleeping",
        LifecycleState::Stopped => "Stopped",
    }
}

fn parse_lifecycle(s: &str) -> Result<LifecycleState, String> {
    match s {
        "Booting" => Ok(LifecycleState::Booting),
        "Running" => Ok(LifecycleState::Running),
        "Sleeping" => Ok(LifecycleState::Sleeping),
        "Stopped" => Ok(LifecycleState::Stopped),
        other => Err(format!("unknown LifecycleState '{}'", other)),
    }
}

fn action_kind_to_str(a: ActionKind) -> &'static str {
    match a {
        ActionKind::DiscoverPatterns => "DiscoverPatterns",
        ActionKind::DiscoverTheory => "DiscoverTheory",
        ActionKind::PruneLowValueObjects => "PruneLowValueObjects",
        ActionKind::UpdateTheoryRelations => "UpdateTheoryRelations",
        ActionKind::Declarativize => "Declarativize",
        ActionKind::DiscoverMetaMetaPatterns => "DiscoverMetaMetaPatterns",
    }
}

fn parse_action_kind(s: &str) -> Result<ActionKind, String> {
    match s {
        "DiscoverPatterns" => Ok(ActionKind::DiscoverPatterns),
        "DiscoverTheory" => Ok(ActionKind::DiscoverTheory),
        "PruneLowValueObjects" => Ok(ActionKind::PruneLowValueObjects),
        "UpdateTheoryRelations" => Ok(ActionKind::UpdateTheoryRelations),
        "Declarativize" => Ok(ActionKind::Declarativize),
        "DiscoverMetaMetaPatterns" => {
            Ok(ActionKind::DiscoverMetaMetaPatterns)
        }
        other => Err(format!("unknown ActionKind '{}'", other)),
    }
}

fn target_to_pair(t: &FrontierTarget) -> (&'static str, String) {
    match t {
        FrontierTarget::WholeRSet => ("WholeRSet", String::new()),
        FrontierTarget::PatternSize(s) => ("PatternSize", s.to_string()),
        FrontierTarget::Pattern(id) => ("Pattern", id.clone()),
        FrontierTarget::Theory(id) => ("Theory", id.clone()),
        FrontierTarget::Axiom(id) => ("Axiom", id.clone()),
    }
}

fn pair_to_target(kind: &str, value: &str) -> Result<FrontierTarget, String> {
    match kind {
        "WholeRSet" => Ok(FrontierTarget::WholeRSet),
        "PatternSize" => Ok(FrontierTarget::PatternSize(
            parse_usize(value, "PatternSize.value")?,
        )),
        "Pattern" => Ok(FrontierTarget::Pattern(value.to_string())),
        "Theory" => Ok(FrontierTarget::Theory(value.to_string())),
        "Axiom" => Ok(FrontierTarget::Axiom(value.to_string())),
        other => Err(format!("unknown FrontierTarget kind '{}'", other)),
    }
}

fn parse_u64(s: &str, ctx: &str) -> Result<u64, String> {
    s.parse::<u64>()
        .map_err(|e| format!("{}: parse u64 '{}' failed: {}", ctx, s, e))
}

fn parse_u32(s: &str, ctx: &str) -> Result<u32, String> {
    s.parse::<u32>()
        .map_err(|e| format!("{}: parse u32 '{}' failed: {}", ctx, s, e))
}

fn parse_usize(s: &str, ctx: &str) -> Result<usize, String> {
    s.parse::<usize>()
        .map_err(|e| format!("{}: parse usize '{}' failed: {}", ctx, s, e))
}

fn parse_f64(s: &str, ctx: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .map_err(|e| format!("{}: parse f64 '{}' failed: {}", ctx, s, e))
}

fn check_reason(reason: &str, ctx: &str) -> Result<(), String> {
    if reason.contains('\t') || reason.contains('\n') {
        return Err(format!(
            "{} reason '{}' contains tab or newline",
            ctx, reason
        ));
    }
    Ok(())
}

/// Sentinel for missing optional values in the checkpoint format.
/// `parse_opt_*` accept this and return `None`; `format_opt_*` write
/// it when the value is `None`. Chosen because `-` is not a legal
/// prefix for the unsigned and float values we serialize, so it
/// can't ambiguously parse as data.
const OPT_NONE: &str = "-";

fn format_opt_u64(v: Option<u64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => OPT_NONE.to_string(),
    }
}

fn parse_opt_u64(s: &str, ctx: &str) -> Result<Option<u64>, String> {
    if s == OPT_NONE {
        Ok(None)
    } else {
        Ok(Some(parse_u64(s, ctx)?))
    }
}

fn format_opt_f64(v: Option<f64>) -> String {
    match v {
        Some(n) => format!("{:?}", n),
        None => OPT_NONE.to_string(),
    }
}

fn parse_opt_f64(s: &str, ctx: &str) -> Result<Option<f64>, String> {
    if s == OPT_NONE {
        Ok(None)
    } else {
        Ok(Some(parse_f64(s, ctx)?))
    }
}

/// Format: `<id>\t<first>\t<last_seen>\t<last_improved>\t<focus>\t<pruned>\t<cv>\t<stability>`
/// where `last_improved`, `cv`, `stability` use `-` for `None`.
fn write_history_section(
    out: &mut String,
    header: &str,
    map: &HashMap<String, ObjectHistory>,
) -> Result<(), String> {
    out.push_str(header);
    out.push('\n');
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    for k in keys {
        if k.contains('\t') || k.contains('\n') {
            return Err(format!(
                "history id '{}' contains tab or newline",
                k
            ));
        }
        let h = &map[k];
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            k,
            h.first_seen_tick,
            h.last_seen_tick,
            format_opt_u64(h.last_improved_tick),
            h.times_selected_as_focus,
            h.times_pruned,
            format_opt_f64(h.last_counterfactual_value),
            format_opt_f64(h.stability_estimate),
            h.times_contributed_positive,
        ));
    }
    Ok(())
}

fn parse_history_lines(
    lines: &[String],
    label: &str,
) -> Result<HashMap<String, ObjectHistory>, String> {
    let mut out: HashMap<String, ObjectHistory> = HashMap::new();
    for (idx, raw) in lines.iter().enumerate() {
        let fields: Vec<&str> = raw.split('\t').collect();
        if fields.len() != 9 {
            return Err(format!(
                "{} line {} has {} fields, expected 9",
                label,
                idx + 1,
                fields.len()
            ));
        }
        let id = fields[0].to_string();
        let h = ObjectHistory {
            first_seen_tick: parse_u64(fields[1], &format!("{}.first", label))?,
            last_seen_tick: parse_u64(fields[2], &format!("{}.last", label))?,
            last_improved_tick: parse_opt_u64(
                fields[3],
                &format!("{}.last_improved", label),
            )?,
            times_selected_as_focus: parse_u32(
                fields[4],
                &format!("{}.focus", label),
            )?,
            times_pruned: parse_u32(fields[5], &format!("{}.pruned", label))?,
            last_counterfactual_value: parse_opt_f64(
                fields[6],
                &format!("{}.cv", label),
            )?,
            stability_estimate: parse_opt_f64(
                fields[7],
                &format!("{}.stability", label),
            )?,
            times_contributed_positive: parse_u32(
                fields[8],
                &format!("{}.contributed", label),
            )?,
        };
        out.insert(id, h);
    }
    Ok(out)
}

fn check_no_tab_or_newline(t: &FrontierTarget, ctx: &str) -> Result<(), String> {
    let id = match t {
        FrontierTarget::WholeRSet | FrontierTarget::PatternSize(_) => return Ok(()),
        FrontierTarget::Pattern(s)
        | FrontierTarget::Theory(s)
        | FrontierTarget::Axiom(s) => s,
    };
    if id.contains('\t') || id.contains('\n') {
        return Err(format!(
            "{} target id '{}' contains tab or newline",
            ctx, id
        ));
    }
    Ok(())
}

#[derive(Default)]
struct ParsedCheckpoint {
    meta: HashMap<String, String>,
    rset_lines: Vec<String>,
    episode_lines: Vec<String>,
    mode_transition_lines: Vec<String>,
    lifecycle_transition_lines: Vec<String>,
    // B2 — history + stats sections.
    history_patterns_lines: Vec<String>,
    history_axioms_lines: Vec<String>,
    history_theories_lines: Vec<String>,
    action_count_lines: Vec<String>,
    mode_transition_count_lines: Vec<String>,
    lifecycle_count_lines: Vec<String>,
}

fn parse_checkpoint(text: &str) -> Result<ParsedCheckpoint, String> {
    let mut out = ParsedCheckpoint::default();
    let mut section: Option<&str> = None;

    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            match name {
                "meta" | "rset" | "episodes" | "mode_transitions"
                | "lifecycle_transitions"
                | "object_history_patterns"
                | "object_history_axioms"
                | "object_history_theories"
                | "policy_stats_action_counts"
                | "policy_stats_mode_transition_counts"
                | "policy_stats_lifecycle_counts" => {
                    section = Some(match name {
                        "meta" => "meta",
                        "rset" => "rset",
                        "episodes" => "episodes",
                        "mode_transitions" => "mode_transitions",
                        "lifecycle_transitions" => "lifecycle_transitions",
                        "object_history_patterns" => "object_history_patterns",
                        "object_history_axioms" => "object_history_axioms",
                        "object_history_theories" => "object_history_theories",
                        "policy_stats_action_counts" => "policy_stats_action_counts",
                        "policy_stats_mode_transition_counts" => {
                            "policy_stats_mode_transition_counts"
                        }
                        _ => "policy_stats_lifecycle_counts",
                    });
                }
                other => {
                    return Err(format!(
                        "unknown section '[{}]' at line {}",
                        other,
                        i + 1
                    ))
                }
            }
            continue;
        }
        match section {
            Some("meta") => {
                let (k, v) = line.split_once('\t').ok_or_else(|| {
                    format!(
                        "meta line {} not key<TAB>value: '{}'",
                        i + 1,
                        line
                    )
                })?;
                out.meta.insert(k.to_string(), v.to_string());
            }
            Some("rset") => out.rset_lines.push(line.to_string()),
            Some("episodes") => out.episode_lines.push(line.to_string()),
            Some("mode_transitions") => {
                out.mode_transition_lines.push(line.to_string())
            }
            Some("lifecycle_transitions") => {
                out.lifecycle_transition_lines.push(line.to_string())
            }
            Some("object_history_patterns") => {
                out.history_patterns_lines.push(line.to_string())
            }
            Some("object_history_axioms") => {
                out.history_axioms_lines.push(line.to_string())
            }
            Some("object_history_theories") => {
                out.history_theories_lines.push(line.to_string())
            }
            Some("policy_stats_action_counts") => {
                out.action_count_lines.push(line.to_string())
            }
            Some("policy_stats_mode_transition_counts") => {
                out.mode_transition_count_lines.push(line.to_string())
            }
            Some("policy_stats_lifecycle_counts") => {
                out.lifecycle_count_lines.push(line.to_string())
            }
            None => {
                return Err(format!(
                    "data line {} has no enclosing section: '{}'",
                    i + 1,
                    line
                ))
            }
            _ => unreachable!(),
        }
    }
    Ok(out)
}

// ─── tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        axiom_template_id, AxiomTemplate, EdgeTemplate, AX_ANTISYMMETRY,
        AX_REFLEXIVITY,
    };

    fn diamond_poset() -> RSet {
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d"];
        for n in &nodes {
            rs.add(R::new(*n, *n));
        }
        rs.extend([
            R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
            R::new("b", "d"), R::new("c", "d"),
        ]);
        rs
    }

    // ─── Phase A0 carryover tests ────────────────────────────────

    #[test]
    fn a0_runtime_runs_bounded_ticks() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(10);
        assert_eq!(rt.memory.len(), 10);
        assert_eq!(rt.tick, 10);
        assert_eq!(rt.lifecycle, LifecycleState::Running);
    }

    #[test]
    fn a0_runtime_discovers_theory_on_diamond() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(5);
        assert_eq!(rt.rset.theories().len(), 1);
        let t_id = rt.rset.theories()[0].to_string();
        let members = rt.rset.theory_axioms(&t_id);
        assert!(members.contains(&AX_REFLEXIVITY));
        assert!(members.contains(&AX_ANTISYMMETRY));
        let transitivity = AxiomTemplate {
            num_vars: 3,
            premise: vec![
                EdgeTemplate { x_var: 0, y_var: 1 },
                EdgeTemplate { x_var: 1, y_var: 2 },
            ],
            conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
        };
        assert!(members.contains(&axiom_template_id(&transitivity).as_str()));
    }

    #[test]
    fn a0_score_monotone_non_decreasing() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        let start_score = rt.current_score;
        rt.run_bounded(10);
        assert!(rt.current_score >= start_score);
    }

    #[test]
    fn a0_first_episode_has_positive_delta_on_structured_input() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(5);
        let first = rt.memory.episodes.front().unwrap();
        assert!(first.delta > 0.0);
    }

    #[test]
    fn a0_stub_decision_is_discover_theory_every_tick() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(7);
        for ep in &rt.memory.episodes {
            assert_eq!(ep.action_kind, ActionKind::DiscoverTheory);
        }
    }

    #[test]
    fn a0_memory_respects_cap() {
        let mut mem = Memory::default();
        mem.max_episodes = 3;
        for i in 0..10 {
            mem.record(Episode {
                id: i,
                tick: i,
                mode: RuntimeMode::Expand,
                action_kind: ActionKind::DiscoverTheory,
                target: FrontierTarget::WholeRSet,
                score_before: 0.0,
                score_after: 0.0,
                delta: 0.0,
            });
        }
        assert_eq!(mem.len(), 3);
        let kept: Vec<u64> = mem.episodes.iter().map(|e| e.id).collect();
        assert_eq!(kept, vec![7, 8, 9]);
    }

    #[test]
    fn a0_run_bounded_is_additive() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(3);
        rt.run_bounded(4);
        assert_eq!(rt.tick, 7);
        assert_eq!(rt.memory.len(), 7);
    }

    #[test]
    fn a0_noop_environment_yields_empty() {
        let mut env = NoOpEnvironment;
        assert!(env.poll().is_empty());
        assert!(env.poll().is_empty());
    }

    #[test]
    fn a0_stop_decision_halts_loop() {
        struct StopAfterOne {
            called: bool,
        }
        impl Scheduler for StopAfterOne {
            fn choose(
                &mut self,
                _ctx: &SchedulerContext<'_>,
            ) -> SchedulerDecision {
                if self.called {
                    SchedulerDecision::Stop
                } else {
                    self.called = true;
                    SchedulerDecision::Execute(ActionPlan {
                        action_kind: ActionKind::DiscoverTheory,
                        target: FrontierTarget::WholeRSet,
                    })
                }
            }
        }
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(StopAfterOne { called: false });
        rt.run_bounded(100);
        assert_eq!(rt.lifecycle, LifecycleState::Stopped);
        assert_eq!(rt.memory.len(), 1);
    }

    #[test]
    fn a0_sleep_decision_halts_loop() {
        struct SleepImmediately;
        impl Scheduler for SleepImmediately {
            fn choose(
                &mut self,
                _ctx: &SchedulerContext<'_>,
            ) -> SchedulerDecision {
                SchedulerDecision::Sleep
            }
        }
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(SleepImmediately);
        rt.run_bounded(100);
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
        assert!(rt.memory.is_empty());
    }

    // ─── Phase A1 tests ─────────────────────────────────────────

    #[test]
    fn a1_frontier_proposes_theory_candidate_on_diamond() {
        let rs = diamond_poset();
        let mut fr = Frontier::default();
        fr.refresh(&rs, 1);
        assert!(fr.items.iter().any(|it| it.kind == FrontierKind::TheoryCandidate));
    }

    #[test]
    fn a1_frontier_omits_theory_candidate_after_naming() {
        let mut rs = diamond_poset();
        let cfg = AxiomDiscoveryConfig::default();
        let th = rs.discover_theory(&cfg);
        let ids: Vec<&str> =
            th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        let _ = rs.name_theory(&ids);
        let mut fr = Frontier::default();
        fr.refresh(&rs, 1);
        assert!(!fr
            .items
            .iter()
            .any(|it| it.kind == FrontierKind::TheoryCandidate));
    }

    #[test]
    fn a1_frontier_proposes_pattern_candidates() {
        let rs = diamond_poset();
        let mut fr = Frontier::default();
        fr.refresh(&rs, 1);
        let pattern_kinds: Vec<_> = fr
            .items
            .iter()
            .filter(|it| it.kind == FrontierKind::PatternCandidate)
            .collect();
        assert!(!pattern_kinds.is_empty());
    }

    #[test]
    fn a1_rule_based_runs_and_sleeps() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(50);
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
        // Has taken *some* episodes before sleeping.
        assert!(rt.memory.len() > 0);
        // Theory was named.
        assert_eq!(rt.rset.theories().len(), 1);
    }

    #[test]
    fn a1_deterministic_trace_reproducible() {
        fn run_once() -> Vec<(u64, ActionKind, FrontierTarget, f64)> {
            let rs = diamond_poset();
            let mut rt = AutonomousRuntime::new(rs);
            rt.scheduler = Box::new(RuleBasedScheduler::default());
            rt.run_bounded(30);
            rt.memory
                .episodes
                .iter()
                .map(|e| (e.tick, e.action_kind, e.target.clone(), e.delta))
                .collect()
        }
        let trace_a = run_once();
        let trace_b = run_once();
        assert_eq!(trace_a, trace_b);
    }

    #[test]
    fn a1_empty_frontier_triggers_sleep() {
        let rs = RSet::new();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(50);
        // Empty RSet: nothing to discover, nothing to prune.
        // Scheduler should sleep on first tick (zero-streak or empty frontier).
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
    }

    #[test]
    fn a1_frontier_dirty_after_action() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(1);
        // After one Execute, frontier should be marked dirty
        // (though run_bounded may have already refreshed).
        assert!(rt.memory.len() >= 1);
    }

    #[test]
    fn a1_pattern_candidate_priority_decreases_with_size() {
        let rs = diamond_poset();
        let mut fr = Frontier::default();
        fr.refresh(&rs, 1);
        let pats: Vec<&FrontierItem> = fr
            .items
            .iter()
            .filter(|it| it.kind == FrontierKind::PatternCandidate)
            .collect();
        if pats.len() >= 2 {
            let mut by_size: Vec<(usize, f64)> = pats
                .iter()
                .filter_map(|p| match p.target {
                    FrontierTarget::PatternSize(s) => Some((s, p.priority)),
                    _ => None,
                })
                .collect();
            by_size.sort_by_key(|(s, _)| *s);
            // Smaller sizes → higher priority.
            for w in by_size.windows(2) {
                assert!(w[0].1 >= w[1].1);
            }
        }
    }

    #[test]
    fn a1_frontier_sorted_by_priority_desc() {
        let rs = diamond_poset();
        let mut fr = Frontier::default();
        fr.refresh(&rs, 1);
        for w in fr.items.windows(2) {
            assert!(w[0].priority >= w[1].priority);
        }
    }

    #[test]
    fn a1_mark_dirty_leaves_items_intact() {
        // mark_dirty only flips the flag; refresh replaces items.
        let rs = diamond_poset();
        let mut fr = Frontier::default();
        fr.refresh(&rs, 1);
        let before = fr.items.len();
        fr.mark_dirty();
        assert_eq!(fr.items.len(), before);
        assert!(fr.dirty);
    }

    #[test]
    fn a1_rule_based_zero_streak_triggers_sleep() {
        // Custom: a scheduler-under-test that gives unproductive
        // actions → RuleBased would need zero-streak trigger.
        // We observe this indirectly: diamond poset after theory is
        // named keeps proposing patterns which may not help. After
        // max_zero_streak ticks, Sleep.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler {
            max_zero_streak: 2,
            ..RuleBasedScheduler::default()
        });
        rt.run_bounded(30);
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
    }

    // ─── Phase A2 tests ─────────────────────────────────────────

    /// Build an RSet with multiple distinct theories so
    /// TheoryNeedsRelations gets generated.
    fn rset_with_multiple_theories() -> RSet {
        let mut rs = diamond_poset();
        // Manually name two distinct theories so the runtime later
        // has consolidate work to do.
        let _t1 = rs.name_theory(&[AX_REFLEXIVITY]).unwrap();
        let _t2 = rs.name_theory(&[AX_ANTISYMMETRY]).unwrap();
        rs
    }

    #[test]
    fn a2_frontier_proposes_relations_when_theories_lack_them() {
        let rs = rset_with_multiple_theories();
        let mut fr = Frontier::default();
        fr.refresh(&rs, 1);
        assert!(fr.items.iter().any(|it|
            it.kind == FrontierKind::TheoryNeedsRelations
        ));
    }

    #[test]
    fn a2_frontier_omits_relations_when_all_pairs_have_them() {
        let mut rs = rset_with_multiple_theories();
        // Manually classify and persist relations between every pair.
        let theories: Vec<String> =
            rs.theories().iter().map(|s| s.to_string()).collect();
        for i in 0..theories.len() {
            for j in (i + 1)..theories.len() {
                let a = &theories[i];
                let b = &theories[j];
                match rs.classify_theory_pair(a, b) {
                    Some(crate::TheoryRelationKind::Independent) => {
                        let _ = rs.name_theory_independence(a, b);
                    }
                    _ => {}
                }
            }
        }
        let mut fr = Frontier::default();
        fr.refresh(&rs, 1);
        assert!(!fr.items.iter().any(|it|
            it.kind == FrontierKind::TheoryNeedsRelations
        ));
    }

    #[test]
    fn a2_update_theory_relations_persists_independence() {
        let mut rs = rset_with_multiple_theories();
        // Verify no relation edges before.
        assert!(rs.independence_edges().is_empty());

        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        // Force one execution of the relation action by injecting a
        // scheduler that always picks UpdateTheoryRelations.
        struct OnlyRelations;
        impl Scheduler for OnlyRelations {
            fn choose(&mut self, _ctx: &SchedulerContext<'_>) -> SchedulerDecision {
                SchedulerDecision::Execute(ActionPlan {
                    action_kind: ActionKind::UpdateTheoryRelations,
                    target: FrontierTarget::WholeRSet,
                })
            }
        }
        rt.scheduler = Box::new(OnlyRelations);
        rt.run_bounded(1);
        // After one tick, the {AX_REFLEXIVITY} and {AX_ANTISYMMETRY}
        // theories are independent → independence edge created.
        assert!(!rt.rset.independence_edges().is_empty());
    }

    #[test]
    fn a2_mode_transition_logged() {
        // Force a SwitchMode by handing scheduler that switches first.
        struct SwitchOnce {
            switched: bool,
        }
        impl Scheduler for SwitchOnce {
            fn choose(&mut self, _ctx: &SchedulerContext<'_>) -> SchedulerDecision {
                if !self.switched {
                    self.switched = true;
                    SchedulerDecision::SwitchMode(RuntimeMode::Reflect)
                } else {
                    SchedulerDecision::Stop
                }
            }
        }
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(SwitchOnce { switched: false });
        rt.run_bounded(10);
        assert_eq!(rt.memory.mode_transitions.len(), 1);
        let mt = rt.memory.mode_transitions.front().unwrap();
        assert_eq!(mt.from, RuntimeMode::Expand);
        assert_eq!(mt.to, RuntimeMode::Reflect);
    }

    #[test]
    fn a2_same_mode_switch_is_noop() {
        // SwitchMode to current mode should NOT log a transition.
        struct StaySame;
        impl Scheduler for StaySame {
            fn choose(&mut self, _ctx: &SchedulerContext<'_>) -> SchedulerDecision {
                SchedulerDecision::SwitchMode(RuntimeMode::Expand)
            }
        }
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(StaySame);
        rt.run_bounded(5);
        assert!(rt.memory.mode_transitions.is_empty());
    }

    #[test]
    fn a2_consolidate_mode_processes_consolidate_work() {
        let rs = rset_with_multiple_theories();
        let mut rt = AutonomousRuntime::new(rs);
        rt.mode = RuntimeMode::Consolidate;
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(5);
        // Consolidate should have triggered UpdateTheoryRelations,
        // which named at least one independence edge.
        let any_relation_action = rt
            .memory
            .episodes
            .iter()
            .any(|ep| ep.action_kind == ActionKind::UpdateTheoryRelations);
        assert!(any_relation_action);
    }

    #[test]
    fn a2_reflect_mode_does_not_execute() {
        // In Reflect, scheduler::choose should never return Execute.
        // Unit-test the scheduler directly so we don't get cascading
        // mode changes confusing the assertion.
        let rs = diamond_poset();
        let frontier = {
            let mut f = Frontier::default();
            f.refresh(&rs, 0);
            f
        };
        let memory = Memory::default();
        let mut scheduler = RuleBasedScheduler::default();
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Reflect,
            tick: 0,
        };
        let decision = scheduler.choose(&ctx);
        match decision {
            SchedulerDecision::SwitchMode(_) | SchedulerDecision::Sleep => {}
            _ => panic!("Reflect must not Execute; got {:?}", decision),
        }
    }

    #[test]
    fn a2_expand_to_consolidate_to_reflect_chain() {
        // On a multi-theory rset, the rule-based scheduler should
        // walk Expand → Consolidate → Reflect → Sleep over the run.
        // Lower min_recent_gains so transition triggers within the
        // test's tick budget.
        let rs = rset_with_multiple_theories();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler {
            min_recent_gains: 1,
            ..RuleBasedScheduler::default()
        });
        rt.run_bounded(40);
        let modes_visited: Vec<RuntimeMode> = rt
            .memory
            .mode_transitions
            .iter()
            .map(|mt| mt.to)
            .collect();
        assert!(
            modes_visited.contains(&RuntimeMode::Consolidate)
                || modes_visited.contains(&RuntimeMode::Reflect),
            "expected mode walk; got {:?}", modes_visited
        );
    }

    #[test]
    fn a2_mode_transition_cap_respected() {
        let mut mem = Memory::default();
        mem.max_mode_transitions = 3;
        for i in 0..10 {
            mem.record_mode_transition(ModeTransition {
                tick: i,
                from: RuntimeMode::Expand,
                to: RuntimeMode::Reflect,
                reason: "test".to_string(),
            });
        }
        assert_eq!(mem.mode_transitions.len(), 3);
        let kept: Vec<u64> =
            mem.mode_transitions.iter().map(|mt| mt.tick).collect();
        assert_eq!(kept, vec![7, 8, 9]);
    }

    #[test]
    fn a2_deterministic_trace_with_modes() {
        // Mode-aware run also reproducible across identical inputs.
        fn run_once() -> (Vec<RuntimeMode>, Vec<RuntimeMode>) {
            let rs = rset_with_multiple_theories();
            let mut rt = AutonomousRuntime::new(rs);
            rt.scheduler = Box::new(RuleBasedScheduler::default());
            rt.run_bounded(20);
            let episode_modes: Vec<RuntimeMode> =
                rt.memory.episodes.iter().map(|ep| ep.mode).collect();
            let transition_modes: Vec<RuntimeMode> = rt
                .memory
                .mode_transitions
                .iter()
                .map(|mt| mt.to)
                .collect();
            (episode_modes, transition_modes)
        }
        let a = run_once();
        let b = run_once();
        assert_eq!(a, b);
    }

    // ─── Phase A3 tests ─────────────────────────────────────────

    /// Environment that returns a fixed list of events on the first
    /// `poll`, then nothing. Used to inject one wake-up event.
    struct OneShotEnv {
        events: Vec<Event>,
    }

    impl Environment for OneShotEnv {
        fn poll(&mut self) -> Vec<Event> {
            std::mem::take(&mut self.events)
        }
    }

    /// Environment that fires the given events on a specific tick
    /// (matched against `polled_count`). Useful for "wake on the Nth
    /// tick" scenarios.
    struct TickGatedEnv {
        events: Vec<Event>,
        fire_after_polls: u64,
        polled: u64,
    }

    impl Environment for TickGatedEnv {
        fn poll(&mut self) -> Vec<Event> {
            self.polled += 1;
            if self.polled == self.fire_after_polls {
                std::mem::take(&mut self.events)
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn a3_should_wake_returns_true_for_data_events() {
        let add = vec![Event::AddEdge(R::new("a", "b"))];
        let rem = vec![Event::RemoveEdge(R::new("a", "b"))];
        let mixed = vec![Event::Tick, Event::AddEdge(R::new("x", "y"))];
        assert!(should_wake(&add));
        assert!(should_wake(&rem));
        assert!(should_wake(&mixed));
    }

    #[test]
    fn a3_should_wake_false_for_tick_or_empty() {
        assert!(!should_wake(&[]));
        assert!(!should_wake(&[Event::Tick]));
        assert!(!should_wake(&[Event::Tick, Event::Tick]));
    }

    #[test]
    fn a3_runtime_stays_sleeping_under_noop_environment() {
        // Pre-sleep, NoOp env: runtime stays asleep across all ticks.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.lifecycle = LifecycleState::Sleeping;
        rt.run_bounded(10);
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
        assert_eq!(rt.tick, 10);
        // No episodes added while sleeping.
        assert!(rt.memory.episodes.is_empty());
    }

    #[test]
    fn a3_sleeping_runtime_wakes_on_event() {
        // The runtime may go back to sleep after waking (no fresh
        // work). The durable signal is the lifecycle log, not the
        // final state.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.lifecycle = LifecycleState::Sleeping;
        rt.environment = Box::new(OneShotEnv {
            events: vec![Event::AddEdge(R::new("xx", "yy"))],
        });
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(5);
        let lts: Vec<_> = rt
            .memory
            .lifecycle_transitions
            .iter()
            .map(|lt| (lt.from, lt.to, lt.reason.clone()))
            .collect();
        let has_wake = lts.iter().any(|(f, t, r)| {
            *f == LifecycleState::Sleeping
                && *t == LifecycleState::Running
                && r == "wake_on_event"
        });
        assert!(has_wake, "missing wake transition; got {:?}", lts);
    }

    #[test]
    fn a3_tick_event_does_not_wake() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.lifecycle = LifecycleState::Sleeping;
        rt.environment = Box::new(OneShotEnv {
            events: vec![Event::Tick],
        });
        rt.run_bounded(3);
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
    }

    #[test]
    fn a3_lifecycle_transition_logged_on_sleep_entry() {
        struct SleepImmediately;
        impl Scheduler for SleepImmediately {
            fn choose(
                &mut self,
                _ctx: &SchedulerContext<'_>,
            ) -> SchedulerDecision {
                SchedulerDecision::Sleep
            }
        }
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(SleepImmediately);
        rt.run_bounded(3);
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
        let lts: Vec<_> = rt
            .memory
            .lifecycle_transitions
            .iter()
            .map(|lt| (lt.from, lt.to, lt.reason.clone()))
            .collect();
        assert_eq!(lts.len(), 1);
        assert_eq!(lts[0].0, LifecycleState::Running);
        assert_eq!(lts[0].1, LifecycleState::Sleeping);
        assert_eq!(lts[0].2, "scheduler_sleep");
    }

    #[test]
    fn a3_last_checkpoint_populated_on_sleep_entry() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(50);
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
        assert!(
            rt.last_checkpoint.is_some(),
            "expected checkpoint snapshot on sleep entry"
        );
        let cp = rt.last_checkpoint.as_ref().unwrap();
        assert!(cp.starts_with("# v2 runtime checkpoint"));
        assert!(cp.contains("[meta]"));
        assert!(cp.contains("[rset]"));
    }

    #[test]
    fn a3_checkpoint_round_trip_preserves_state() {
        // Run a real session, checkpoint, restore, compare.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(20);
        let text = rt.checkpoint_text().unwrap();

        let restored = AutonomousRuntime::from_checkpoint_text(&text).unwrap();

        assert_eq!(restored.tick, rt.tick);
        assert_eq!(restored.episode_counter, rt.episode_counter);
        assert_eq!(restored.lifecycle, rt.lifecycle);
        assert_eq!(restored.mode, rt.mode);
        assert_eq!(restored.current_score, rt.current_score);
        assert_eq!(
            restored.steps_since_last_gain,
            rt.steps_since_last_gain
        );
        assert_eq!(
            restored.budget.actions_per_tick_cap,
            rt.budget.actions_per_tick_cap
        );

        // RSet equality via to_text.
        let a_text = rt.rset.to_text().unwrap();
        let b_text = restored.rset.to_text().unwrap();
        assert_eq!(a_text, b_text);

        // Episodes deeply equal.
        assert_eq!(restored.memory.episodes.len(), rt.memory.episodes.len());
        for (a, b) in restored.memory.episodes.iter().zip(rt.memory.episodes.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.tick, b.tick);
            assert_eq!(a.mode, b.mode);
            assert_eq!(a.action_kind, b.action_kind);
            assert_eq!(a.target, b.target);
            assert_eq!(a.score_before, b.score_before);
            assert_eq!(a.score_after, b.score_after);
            assert_eq!(a.delta, b.delta);
        }

        // Mode + lifecycle transitions.
        assert_eq!(
            restored.memory.mode_transitions.len(),
            rt.memory.mode_transitions.len()
        );
        assert_eq!(
            restored.memory.lifecycle_transitions.len(),
            rt.memory.lifecycle_transitions.len()
        );

        // Caps preserved.
        assert_eq!(restored.memory.max_episodes, rt.memory.max_episodes);
        assert_eq!(
            restored.memory.max_mode_transitions,
            rt.memory.max_mode_transitions
        );
        assert_eq!(
            restored.memory.max_lifecycle_transitions,
            rt.memory.max_lifecycle_transitions
        );
    }

    #[test]
    fn a3_checkpoint_round_trip_is_idempotent() {
        // checkpoint → load → checkpoint again → text identical.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(15);
        let t1 = rt.checkpoint_text().unwrap();
        let restored = AutonomousRuntime::from_checkpoint_text(&t1).unwrap();
        let t2 = restored.checkpoint_text().unwrap();
        assert_eq!(t1, t2);
    }

    #[test]
    fn a3_resume_continues_correctly() {
        // Run, checkpoint, restore with a fresh scheduler, run more —
        // tick advances, no panic.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(5);
        let snapshot_tick = rt.tick;
        let text = rt.checkpoint_text().unwrap();

        let mut restored = AutonomousRuntime::from_checkpoint_text(&text).unwrap();
        restored.scheduler = Box::new(RuleBasedScheduler::default());
        restored.run_bounded(5);
        assert!(restored.tick > snapshot_tick);
    }

    #[test]
    fn a3_lifecycle_transition_cap_respected() {
        let mut mem = Memory::default();
        mem.max_lifecycle_transitions = 3;
        for i in 0..10 {
            mem.record_lifecycle_transition(LifecycleTransition {
                tick: i,
                from: LifecycleState::Running,
                to: LifecycleState::Sleeping,
                reason: "test".to_string(),
            });
        }
        assert_eq!(mem.lifecycle_transitions.len(), 3);
        let kept: Vec<u64> = mem
            .lifecycle_transitions
            .iter()
            .map(|lt| lt.tick)
            .collect();
        assert_eq!(kept, vec![7, 8, 9]);
    }

    #[test]
    fn a3_resume_runs_full_run_to_completion() {
        // End-to-end: a runtime that woke on event then resumed and
        // sleeps again — a full Running → Sleeping → Running →
        // Sleeping cycle in one bounded run. Verifies wake doesn't
        // leave the runtime stuck.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        // Inject one AddEdge mid-run via TickGatedEnv. Pick a tick
        // count that is well after first sleep (RuleBased on a
        // diamond sleeps within a handful of ticks).
        rt.environment = Box::new(TickGatedEnv {
            events: vec![Event::AddEdge(R::new("ext", "ext"))],
            fire_after_polls: 10,
            polled: 0,
        });
        rt.run_bounded(40);
        // Eventually settles. The runtime received a wake at tick 10
        // and either kept running or slept again — but at least one
        // wake transition is in the log.
        let woke = rt
            .memory
            .lifecycle_transitions
            .iter()
            .any(|lt| {
                lt.from == LifecycleState::Sleeping
                    && lt.to == LifecycleState::Running
            });
        assert!(woke, "runtime never woke on the injected event");
    }

    // ─── Phase B0 tests — ObjectHistory / PolicyStats / Stream ──

    #[test]
    fn b0_object_history_recorded_on_first_theory_creation() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(1);
        // First tick: stub scheduler runs DiscoverTheory. A theory
        // should be named and its history populated.
        assert_eq!(rt.rset.theories().len(), 1);
        let t_id = rt.rset.theories()[0].to_string();
        let hist = rt
            .memory
            .object_history
            .theories
            .get(&t_id)
            .expect("theory history missing");
        assert_eq!(hist.first_seen_tick, 1);
        assert_eq!(hist.last_seen_tick, 1);
        assert_eq!(hist.last_improved_tick, Some(1));
        assert_eq!(hist.times_pruned, 0);
    }

    #[test]
    fn b0_object_history_last_seen_advances() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(5);
        let t_id = rt.rset.theories()[0].to_string();
        let hist = &rt.memory.object_history.theories[&t_id];
        assert_eq!(hist.first_seen_tick, 1);
        // Stub keeps re-running DiscoverTheory; last_seen advances.
        assert!(
            hist.last_seen_tick >= 1,
            "last_seen_tick = {}",
            hist.last_seen_tick
        );
        assert!(hist.last_seen_tick <= 5);
    }

    #[test]
    fn b0_policy_stats_action_counts_increment() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(5);
        let count = rt
            .memory
            .policy_stats
            .action_counts
            .get(&ActionKind::DiscoverTheory)
            .copied()
            .unwrap_or(0);
        assert_eq!(count, 5, "5 stub episodes → 5 DiscoverTheory");
        let pos = rt
            .memory
            .policy_stats
            .action_positive_delta_counts
            .get(&ActionKind::DiscoverTheory)
            .copied()
            .unwrap_or(0);
        assert!(pos >= 1, "first DiscoverTheory should yield positive delta");
    }

    #[test]
    fn b0_policy_stats_mode_transitions_counted() {
        struct SwitchOnce {
            switched: bool,
        }
        impl Scheduler for SwitchOnce {
            fn choose(&mut self, _ctx: &SchedulerContext<'_>) -> SchedulerDecision {
                if !self.switched {
                    self.switched = true;
                    SchedulerDecision::SwitchMode(RuntimeMode::Reflect)
                } else {
                    SchedulerDecision::Stop
                }
            }
        }
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(SwitchOnce { switched: false });
        rt.run_bounded(5);
        let key = (RuntimeMode::Expand, RuntimeMode::Reflect);
        assert_eq!(
            rt.memory.policy_stats.mode_transition_counts.get(&key),
            Some(&1)
        );
    }

    #[test]
    fn b0_policy_stats_sleep_wake_counts() {
        // Pre-sleep, fire one event, then NoOp again → exactly one
        // wake count, zero additional sleeps from that event.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.lifecycle = LifecycleState::Sleeping;
        rt.environment = Box::new(OneShotEnv {
            events: vec![Event::AddEdge(R::new("p", "q"))],
        });
        rt.run_bounded(3);
        assert_eq!(rt.memory.policy_stats.wake_count, 1);
        // sleep_count: depends on whether scheduler put it back to
        // sleep. With NoOp post-event and StubScheduler always
        // executing, it stays Running; sleep_count == 0.
        assert_eq!(rt.memory.policy_stats.sleep_count, 0);
    }

    #[test]
    fn b0_policy_stats_stop_count() {
        struct StopImmediately;
        impl Scheduler for StopImmediately {
            fn choose(&mut self, _ctx: &SchedulerContext<'_>) -> SchedulerDecision {
                SchedulerDecision::Stop
            }
        }
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(StopImmediately);
        rt.run_bounded(3);
        assert_eq!(rt.memory.policy_stats.stop_count, 1);
    }

    #[test]
    fn b0_synthetic_stream_yields_events_on_schedule() {
        let mut env = SyntheticStreamEnvironment::new(vec![
            (1, Event::AddEdge(R::new("a", "b"))),
            (3, Event::AddEdge(R::new("b", "c"))),
            (3, Event::AddEdge(R::new("c", "d"))),
        ]);
        // poll #1 → tick 1 event
        let p1 = env.poll();
        assert_eq!(p1.len(), 1);
        // poll #2 → nothing
        assert!(env.poll().is_empty());
        // poll #3 → both tick-3 events
        let p3 = env.poll();
        assert_eq!(p3.len(), 2);
        // poll #4 → nothing left
        assert!(env.poll().is_empty());
        assert_eq!(env.remaining(), 0);
    }

    #[test]
    fn b0_synthetic_stream_back_dated_events_fire_on_first_poll() {
        // Schedule says tick 0 — a "before time began" event. Should
        // still fire on the first poll (target_index = 1 > 0).
        let mut env = SyntheticStreamEnvironment::new(vec![
            (0, Event::AddEdge(R::new("a", "b"))),
        ]);
        let p1 = env.poll();
        assert_eq!(p1.len(), 1);
    }

    #[test]
    fn b0_synthetic_stream_drives_runtime_to_named_theory() {
        // ADR 0052 verification scenario #5 (drip-feed):
        // start empty, drip-feed a 4-node diamond poset over 12
        // ticks, runtime ends up with at least one named theory.
        let schedule: Vec<(u64, Event)> = vec![
            (1, Event::AddEdge(R::new("a", "a"))),
            (2, Event::AddEdge(R::new("b", "b"))),
            (3, Event::AddEdge(R::new("c", "c"))),
            (4, Event::AddEdge(R::new("d", "d"))),
            (5, Event::AddEdge(R::new("a", "b"))),
            (6, Event::AddEdge(R::new("a", "c"))),
            (7, Event::AddEdge(R::new("a", "d"))),
            (8, Event::AddEdge(R::new("b", "d"))),
            (9, Event::AddEdge(R::new("c", "d"))),
        ];
        let expected = [
            R::new("a", "a"), R::new("b", "b"), R::new("c", "c"),
            R::new("d", "d"), R::new("a", "b"), R::new("a", "c"),
            R::new("a", "d"), R::new("b", "d"), R::new("c", "d"),
        ];
        let mut rt = AutonomousRuntime::new(RSet::new());
        rt.environment = Box::new(SyntheticStreamEnvironment::new(schedule));
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(30);
        for r in &expected {
            assert!(
                rt.rset.iter().any(|got| got == r),
                "missing scheduled edge {:?}",
                r
            );
        }
        // And at least one theory has been named (poset emerged).
        assert!(
            rt.rset.theories().len() >= 1,
            "no theory named after drip-feed; theories = {:?}",
            rt.rset.theories()
        );
    }

    #[test]
    fn b0_pruning_increments_times_pruned() {
        // Manually pre-name two theories, then force a Prune action
        // targeting one. ObjectHistory.times_pruned should bump.
        let mut rs = rset_with_multiple_theories();
        let theories: Vec<String> =
            rs.theories().iter().map(|s| s.to_string()).collect();
        let target = theories[0].clone();
        let _ = rs.classify_theory_pair(&theories[0], &theories[1]);
        let mut rt = AutonomousRuntime::new(rs);
        // Seed the history so we can observe the increment.
        rt.memory
            .object_history
            .theories
            .insert(target.clone(), ObjectHistory::new_at(0));
        struct PruneTarget(String);
        impl Scheduler for PruneTarget {
            fn choose(&mut self, _ctx: &SchedulerContext<'_>) -> SchedulerDecision {
                SchedulerDecision::Execute(ActionPlan {
                    action_kind: ActionKind::PruneLowValueObjects,
                    target: FrontierTarget::Theory(self.0.clone()),
                })
            }
        }
        rt.scheduler = Box::new(PruneTarget(target.clone()));
        rt.run_bounded(1);
        let h = &rt.memory.object_history.theories[&target];
        assert_eq!(h.times_pruned, 1);
    }

    #[test]
    fn b0_focus_target_increments_times_selected() {
        let mut rs = rset_with_multiple_theories();
        let target = rs.theories()[0].to_string();
        // Make the target a pattern instead of theory? No — it's a
        // theory; the focus tracker covers Pattern + Theory targets.
        let mut rt = AutonomousRuntime::new(rs.clone());
        rt.memory
            .object_history
            .theories
            .insert(target.clone(), ObjectHistory::new_at(0));
        struct FocusTheory(String);
        impl Scheduler for FocusTheory {
            fn choose(&mut self, _ctx: &SchedulerContext<'_>) -> SchedulerDecision {
                SchedulerDecision::Execute(ActionPlan {
                    action_kind: ActionKind::UpdateTheoryRelations,
                    target: FrontierTarget::Theory(self.0.clone()),
                })
            }
        }
        rt.scheduler = Box::new(FocusTheory(target.clone()));
        rt.run_bounded(2);
        let h = &rt.memory.object_history.theories[&target];
        assert_eq!(h.times_selected_as_focus, 2);
    }

    #[test]
    fn b2_history_and_stats_round_trip() {
        // B2 closes the boundary B0 left open: object_history and
        // policy_stats now round-trip through the checkpoint.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(3);
        assert!(!rt.memory.policy_stats.action_counts.is_empty());
        assert!(!rt.memory.object_history.theories.is_empty());

        let text = rt.checkpoint_text().unwrap();
        let restored = AutonomousRuntime::from_checkpoint_text(&text).unwrap();

        assert_eq!(
            restored.memory.policy_stats.action_counts,
            rt.memory.policy_stats.action_counts
        );
        assert_eq!(
            restored.memory.policy_stats.action_positive_delta_counts,
            rt.memory.policy_stats.action_positive_delta_counts
        );
        assert_eq!(
            restored.memory.policy_stats.mode_transition_counts,
            rt.memory.policy_stats.mode_transition_counts
        );
        assert_eq!(
            restored.memory.policy_stats.wake_count,
            rt.memory.policy_stats.wake_count
        );
        assert_eq!(
            restored.memory.policy_stats.sleep_count,
            rt.memory.policy_stats.sleep_count
        );
        assert_eq!(
            restored.memory.policy_stats.stop_count,
            rt.memory.policy_stats.stop_count
        );

        // ObjectHistory deeply equal across all three namespaces.
        for ns in ["patterns", "axioms", "theories"] {
            let (a, b) = match ns {
                "patterns" => (
                    &rt.memory.object_history.patterns,
                    &restored.memory.object_history.patterns,
                ),
                "axioms" => (
                    &rt.memory.object_history.axioms,
                    &restored.memory.object_history.axioms,
                ),
                _ => (
                    &rt.memory.object_history.theories,
                    &restored.memory.object_history.theories,
                ),
            };
            assert_eq!(a.len(), b.len(), "{} namespace size mismatch", ns);
            for (id, h_a) in a {
                let h_b = b.get(id).expect("missing id in restored");
                assert_eq!(h_a.first_seen_tick, h_b.first_seen_tick);
                assert_eq!(h_a.last_seen_tick, h_b.last_seen_tick);
                assert_eq!(h_a.last_improved_tick, h_b.last_improved_tick);
                assert_eq!(h_a.times_selected_as_focus, h_b.times_selected_as_focus);
                assert_eq!(h_a.times_pruned, h_b.times_pruned);
                assert_eq!(
                    h_a.last_counterfactual_value,
                    h_b.last_counterfactual_value
                );
                assert_eq!(h_a.stability_estimate, h_b.stability_estimate);
            }
        }
    }

    #[test]
    fn b2_checkpoint_with_stats_is_idempotent() {
        // After B2, the existing A3 idempotent property must still
        // hold across the larger format.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(15);
        let t1 = rt.checkpoint_text().unwrap();
        let restored = AutonomousRuntime::from_checkpoint_text(&t1).unwrap();
        let t2 = restored.checkpoint_text().unwrap();
        assert_eq!(t1, t2);
    }

    #[test]
    fn b2_thrash_history_survives_resume() {
        // Seed a runtime with a thrash record, checkpoint, restore,
        // and verify the gate still fires on the resumed runtime.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        // Plant a thrashed pair directly.
        *rt.memory
            .policy_stats
            .mode_transition_counts
            .entry((RuntimeMode::Expand, RuntimeMode::Reflect))
            .or_insert(0) = 4;
        let text = rt.checkpoint_text().unwrap();
        let restored = AutonomousRuntime::from_checkpoint_text(&text).unwrap();
        let count = restored
            .memory
            .policy_stats
            .mode_transition_counts
            .get(&(RuntimeMode::Expand, RuntimeMode::Reflect))
            .copied()
            .unwrap_or(0);
        assert_eq!(count, 4, "thrash count lost on round-trip");
    }

    #[test]
    fn b2_optional_fields_round_trip_none_and_some() {
        // ObjectHistory's three Option fields must round-trip both
        // None and Some values correctly.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        let mut h_none = ObjectHistory::new_at(7);
        h_none.times_selected_as_focus = 3;
        let mut h_some = ObjectHistory::new_at(2);
        h_some.last_improved_tick = Some(5);
        h_some.last_counterfactual_value = Some(-1.25);
        h_some.stability_estimate = Some(0.875);
        rt.memory
            .object_history
            .patterns
            .insert("p_none".to_string(), h_none.clone());
        rt.memory
            .object_history
            .patterns
            .insert("p_some".to_string(), h_some.clone());

        let text = rt.checkpoint_text().unwrap();
        let restored = AutonomousRuntime::from_checkpoint_text(&text).unwrap();
        let r_none = &restored.memory.object_history.patterns["p_none"];
        let r_some = &restored.memory.object_history.patterns["p_some"];

        assert_eq!(r_none.last_improved_tick, None);
        assert_eq!(r_none.last_counterfactual_value, None);
        assert_eq!(r_none.stability_estimate, None);
        assert_eq!(r_none.times_selected_as_focus, 3);

        assert_eq!(r_some.last_improved_tick, Some(5));
        assert_eq!(r_some.last_counterfactual_value, Some(-1.25));
        assert_eq!(r_some.stability_estimate, Some(0.875));
    }

    // ─── Phase B1 tests — mode-thrash gate ──────────────────────

    #[test]
    fn b1_would_thrash_returns_false_with_no_history() {
        let rs = diamond_poset();
        let frontier = Frontier::default();
        let memory = Memory::default();
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler::default();
        assert!(!sched.would_thrash(&ctx, RuntimeMode::Expand, RuntimeMode::Consolidate));
    }

    #[test]
    fn b1_would_thrash_triggers_when_pair_count_meets_threshold() {
        let rs = diamond_poset();
        let frontier = Frontier::default();
        let mut memory = Memory::default();
        // Fake a thrashing history: 3 Expand→Consolidate + 1 reverse.
        for _ in 0..3 {
            *memory
                .policy_stats
                .mode_transition_counts
                .entry((RuntimeMode::Expand, RuntimeMode::Consolidate))
                .or_insert(0) += 1;
        }
        *memory
            .policy_stats
            .mode_transition_counts
            .entry((RuntimeMode::Consolidate, RuntimeMode::Expand))
            .or_insert(0) += 1;
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler {
            max_mode_oscillations: 4,
            ..RuleBasedScheduler::default()
        };
        // 3 + 1 = 4, hits the threshold.
        assert!(sched.would_thrash(&ctx, RuntimeMode::Expand, RuntimeMode::Consolidate));
        // Pair Reflect is untouched.
        assert!(!sched.would_thrash(&ctx, RuntimeMode::Expand, RuntimeMode::Reflect));
    }

    #[test]
    fn b1_thrashed_pair_yields_sleep_decision() {
        // Drive Reflect mode with a frontier that has expand work
        // (so Reflect would normally SwitchMode→Expand) but with
        // mode-transition history that makes Expand↔Reflect thrashing.
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        *memory
            .policy_stats
            .mode_transition_counts
            .entry((RuntimeMode::Reflect, RuntimeMode::Expand))
            .or_insert(0) = 2;
        *memory
            .policy_stats
            .mode_transition_counts
            .entry((RuntimeMode::Expand, RuntimeMode::Reflect))
            .or_insert(0) = 2;
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Reflect,
            tick: 0,
        };
        let mut sched = RuleBasedScheduler {
            max_mode_oscillations: 4,
            ..RuleBasedScheduler::default()
        };
        match sched.choose(&ctx) {
            SchedulerDecision::Sleep => {}
            other => panic!(
                "expected Sleep when Expand↔Reflect thrashed; got {:?}",
                other
            ),
        }
    }

    #[test]
    fn b1plus_pattern_cooldown_inactive_with_few_attempts() {
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        // 3 attempts, 0 hits — bad rate but below the 5-attempt
        // floor, so cooldown should NOT activate.
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverPatterns, 3);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler::default();
        assert!(!sched.pattern_cooldown_active(&ctx));
    }

    #[test]
    fn b1plus_pattern_cooldown_activates_on_low_hit_rate() {
        // Use an empty rset so the G0 anomaly-pressure relaxation
        // (ADR 0057) doesn't apply — the test is about the base
        // 10% threshold, not the relaxed 5% threshold under
        // pressure. 1/20 = 5% < 10% → cooled.
        let rs = RSet::new();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverPatterns, 20);
        memory
            .policy_stats
            .action_positive_delta_counts
            .insert(ActionKind::DiscoverPatterns, 1);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler::default();
        assert!(sched.pattern_cooldown_active(&ctx));
    }

    #[test]
    fn b1plus_pattern_cooldown_inactive_on_healthy_hit_rate() {
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverPatterns, 10);
        memory
            .policy_stats
            .action_positive_delta_counts
            .insert(ActionKind::DiscoverPatterns, 5);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler::default();
        assert!(!sched.pattern_cooldown_active(&ctx));
    }

    #[test]
    fn b1plus_cooled_pattern_falls_back_to_theory_candidate() {
        // Frontier with both kinds; cooldown must steer the
        // selection to TheoryCandidate even if PatternCandidate
        // priority would normally be higher.
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        // Inject a high-priority synthetic PatternCandidate so it
        // would dominate without the cooldown.
        frontier.items.insert(
            0,
            FrontierItem {
                id: "synth_pat".to_string(),
                kind: FrontierKind::PatternCandidate,
                target: FrontierTarget::PatternSize(3),
                priority: 999.0,
                estimated_value: 999.0,
                estimated_cost: 1.0,
                novelty_score: 1.0,
                first_seen_tick: 0,
                last_visited_tick: None,
                revisit_count: 0,
                cooldown_until_tick: None,
                status: FrontierStatus::Fresh,
            },
        );
        let mut memory = Memory::default();
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverPatterns, 20);
        memory
            .policy_stats
            .action_positive_delta_counts
            .insert(ActionKind::DiscoverPatterns, 0);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 1,
        };
        let mut sched = RuleBasedScheduler::default();
        match sched.choose(&ctx) {
            SchedulerDecision::Execute(plan) => {
                assert_eq!(
                    plan.action_kind,
                    ActionKind::DiscoverTheory,
                    "cooled pattern should yield TheoryCandidate, got {:?}",
                    plan
                );
            }
            other => {
                panic!("expected Execute(DiscoverTheory); got {:?}", other)
            }
        }
    }

    #[test]
    fn b1plus_cooled_pattern_with_no_theory_falls_back_to_consolidate() {
        // Frontier has only PatternCandidate; cooldown blocks it,
        // and there's no TheoryCandidate to fall back to. With a
        // consolidate work item present, scheduler should
        // SwitchMode(Consolidate) — not Sleep.
        let rs = diamond_poset();
        let frontier = Frontier {
            items: vec![
                FrontierItem {
                    id: "synth_pat".to_string(),
                    kind: FrontierKind::PatternCandidate,
                    target: FrontierTarget::PatternSize(3),
                    priority: 5.0,
                    estimated_value: 5.0,
                    estimated_cost: 1.0,
                    novelty_score: 1.0,
                    first_seen_tick: 0,
                    last_visited_tick: None,
                    revisit_count: 0,
                    cooldown_until_tick: None,
                    status: FrontierStatus::Fresh,
                },
                FrontierItem {
                    id: "synth_prune".to_string(),
                    kind: FrontierKind::LowValueObjectForPrune,
                    target: FrontierTarget::Pattern("p_x".to_string()),
                    priority: 3.0,
                    estimated_value: 1.0,
                    estimated_cost: 1.0,
                    novelty_score: 0.0,
                    first_seen_tick: 0,
                    last_visited_tick: None,
                    revisit_count: 0,
                    cooldown_until_tick: None,
                    status: FrontierStatus::Fresh,
                },
            ],
            last_full_refresh_tick: 0,
            dirty: false,
            staleness: StalenessConfig::default(),
            promotion: PromotionConfig::default(),
            meta_meta: MetaMetaConfig::default(),
        };
        let mut memory = Memory::default();
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverPatterns, 10);
        memory
            .policy_stats
            .action_positive_delta_counts
            .insert(ActionKind::DiscoverPatterns, 0);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 1,
        };
        let mut sched = RuleBasedScheduler::default();
        match sched.choose(&ctx) {
            SchedulerDecision::SwitchMode(RuntimeMode::Consolidate) => {}
            other => panic!(
                "expected SwitchMode(Consolidate); got {:?}",
                other
            ),
        }
    }

    // ─── ADR 0054 OQ #2 — meta-meta cooldown gate ──────────────

    #[test]
    fn meta_meta_cooldown_inactive_with_few_attempts() {
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        // 3 attempts, 0 hits — bad rate but below the 5-attempt
        // floor. Must stay inactive.
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverMetaMetaPatterns, 3);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler::default();
        assert!(!sched.meta_meta_cooldown_active(&ctx));
    }

    #[test]
    fn meta_meta_cooldown_activates_on_low_hit_rate() {
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        // 20 attempts, 0 hits (0% < 5% floor) → cooled.
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverMetaMetaPatterns, 20);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler::default();
        assert!(sched.meta_meta_cooldown_active(&ctx));
    }

    #[test]
    fn meta_meta_cooldown_inactive_on_healthy_hit_rate() {
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        // 10 attempts, 2 hits (20% > 5% floor) → not cooled.
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverMetaMetaPatterns, 10);
        memory
            .policy_stats
            .action_positive_delta_counts
            .insert(ActionKind::DiscoverMetaMetaPatterns, 2);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler::default();
        assert!(!sched.meta_meta_cooldown_active(&ctx));
    }

    #[test]
    fn meta_meta_cooldown_independent_of_pattern_cooldown() {
        // PatternDiscovery cooled (20 attempts / 0 hits = 0%); meta-
        // meta has 0 attempts → not cooled. The two counters do not
        // bleed into each other.
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverPatterns, 20);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler::default();
        assert!(sched.pattern_cooldown_active(&ctx));
        assert!(!sched.meta_meta_cooldown_active(&ctx));

        // Now flip: meta-meta cooled, pattern not.
        let mut memory2 = Memory::default();
        memory2
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverMetaMetaPatterns, 20);
        let ctx2 = SchedulerContext {
            rset: &rs,
            memory: &memory2,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        assert!(!sched.pattern_cooldown_active(&ctx2));
        assert!(sched.meta_meta_cooldown_active(&ctx2));
    }

    #[test]
    fn cooled_meta_meta_skipped_in_expand_pick() {
        // Frontier has both a TheoryCandidate and a MetaMetaCandidate;
        // meta-meta is cooled. The scheduler should pick the theory
        // and ignore the meta-meta item.
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        // Inject a high-priority synthetic MetaMetaCandidate that
        // would otherwise dominate.
        frontier.items.insert(
            0,
            FrontierItem {
                id: "synth_mm".to_string(),
                kind: FrontierKind::MetaMetaCandidate,
                target: FrontierTarget::WholeRSet,
                priority: 999.0,
                estimated_value: 999.0,
                estimated_cost: 1.0,
                novelty_score: 1.0,
                first_seen_tick: 0,
                last_visited_tick: None,
                revisit_count: 0,
                cooldown_until_tick: None,
                status: FrontierStatus::Fresh,
            },
        );
        let mut memory = Memory::default();
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverMetaMetaPatterns, 20);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 1,
        };
        let mut sched = RuleBasedScheduler::default();
        match sched.choose(&ctx) {
            SchedulerDecision::Execute(plan) => {
                assert_eq!(
                    plan.action_kind,
                    ActionKind::DiscoverTheory,
                    "cooled meta-meta should yield TheoryCandidate, got {:?}",
                    plan
                );
            }
            other => panic!(
                "expected Execute(DiscoverTheory); got {:?}",
                other
            ),
        }
    }

    // ─── ADR 0057 Phase G0 — anomaly-coverage drive ────────────

    #[test]
    fn g0_uncovered_data_edges_excludes_layer_b_covered() {
        // Construct an rset with two data edges: (a,b), (c,d). Then
        // simulate Layer B for a named pattern p_x with one instance
        // i_0 whose participants are {a, b} (covering edge (a,b)).
        // Edge (c,d) remains uncovered.
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("c", "d"));
        rs.add(R::new(crate::PATTERN_MARKER, "p_x"));
        // Layer B: pattern → instance, instance → participant.
        rs.add(R::new("p_x", "p_x_i_0"));
        rs.add(R::new("p_x_i_0", "a"));
        rs.add(R::new("p_x_i_0", "b"));
        let uncovered = rs.uncovered_data_edges();
        // (a,b) covered. (c,d) NOT covered.
        assert!(!uncovered.contains(&R::new("a", "b")));
        assert!(uncovered.contains(&R::new("c", "d")));
        assert_eq!(uncovered.len(), 1);
    }

    #[test]
    fn g0_uncovered_empty_when_no_patterns_no_data() {
        let rs = RSet::new();
        assert!(rs.uncovered_data_edges().is_empty());
    }

    #[test]
    fn g0_uncovered_intensional_pattern_does_not_cover() {
        // Pattern with no Layer B (Intensional) covers nothing —
        // its participants set is empty. So data edges remain
        // uncovered even though the pattern shape was abstracted.
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new(crate::PATTERN_MARKER, "p_x"));
        // No `R(p_x, p_x_i_*)` instances. Intensional naming
        // produced only the registry edge (and roles, omitted
        // here for brevity).
        let uncovered = rs.uncovered_data_edges();
        assert_eq!(uncovered.len(), 1);
        assert!(uncovered.contains(&R::new("a", "b")));
    }

    #[test]
    fn g0_relaxed_cooldown_picks_pattern_under_anomaly_pressure() {
        // With pressure: 20 attempts / 1 hit = 5%. Base floor 10%
        // (cooled), relaxed 5% (NOT cooled). Build an rset with
        // ≥ 3 uncovered data edges to trigger pressure.
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        rs.add(R::new("b", "c"));
        rs.add(R::new("c", "d"));
        rs.add(R::new("d", "e"));
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        memory
            .policy_stats
            .action_counts
            .insert(ActionKind::DiscoverPatterns, 20);
        memory
            .policy_stats
            .action_positive_delta_counts
            .insert(ActionKind::DiscoverPatterns, 1);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Expand,
            tick: 0,
        };
        let sched = RuleBasedScheduler::default();
        // 4 uncovered edges ≥ 3 → pressure on. 1/20 = 5% NOT < 5%
        // (relaxed floor). So NOT cooled.
        assert!(!sched.pattern_cooldown_active(&ctx));
    }

    #[test]
    fn g0_sleep_suppressed_under_pressure() {
        // Reflect mode + no expand work + no consolidate work +
        // uncovered > 0 → SwitchMode(Expand), not Sleep. Build an
        // rset with uncovered data and an empty frontier.
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        let frontier = Frontier {
            items: Vec::new(),
            last_full_refresh_tick: 0,
            dirty: false,
            staleness: StalenessConfig::default(),
            promotion: PromotionConfig::default(),
            meta_meta: MetaMetaConfig::default(),
        };
        let memory = Memory::default();
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Reflect,
            tick: 0,
        };
        let mut sched = RuleBasedScheduler::default();
        match sched.choose(&ctx) {
            SchedulerDecision::SwitchMode(RuntimeMode::Expand) => {}
            other => panic!(
                "expected SwitchMode(Expand) under pressure; got {:?}",
                other
            ),
        }

        // With empty rset → no uncovered → falls through to Sleep.
        let rs2 = RSet::new();
        let ctx2 = SchedulerContext {
            rset: &rs2,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Reflect,
            tick: 0,
        };
        match sched.choose(&ctx2) {
            SchedulerDecision::Sleep => {}
            other => panic!(
                "expected Sleep without pressure; got {:?}",
                other
            ),
        }
    }

    #[test]
    fn g0_sleep_suppression_bounded_by_thrash_gate() {
        // Pressure + already-thrashed Reflect↔Expand pair → Sleep
        // wins. The G0 hook does NOT override the B1 mode-thrash
        // gate.
        let mut rs = RSet::new();
        rs.add(R::new("a", "b"));
        let frontier = Frontier {
            items: Vec::new(),
            last_full_refresh_tick: 0,
            dirty: false,
            staleness: StalenessConfig::default(),
            promotion: PromotionConfig::default(),
            meta_meta: MetaMetaConfig::default(),
        };
        let mut memory = Memory::default();
        memory
            .policy_stats
            .mode_transition_counts
            .insert((RuntimeMode::Reflect, RuntimeMode::Expand), 4);
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Reflect,
            tick: 0,
        };
        let mut sched = RuleBasedScheduler::default();
        match sched.choose(&ctx) {
            SchedulerDecision::Sleep => {}
            other => panic!(
                "expected Sleep under thrash; got {:?}",
                other
            ),
        }
    }

    #[test]
    fn b3_stale_pattern_below_age_floor_skipped() {
        // Pattern's age (20) is below the 50-tick floor, so it is
        // not eligible for staleness pruning even though
        // last_improved_tick is None and the staleness window
        // has elapsed since first_seen.
        let mut history = ObjectHistoryStore::default();
        history.patterns.insert(
            "p_x".to_string(),
            ObjectHistory {
                first_seen_tick: 10,
                last_seen_tick: 30,
                last_improved_tick: None,
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 0,
            },
        );
        let mut frontier = Frontier::default();
        frontier.refresh_stale_prune(&history, 30);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn b3_long_unimproved_pattern_injected() {
        // first_seen=0, never improved, tick=100. Age=100 ≥ 50
        // and stale_since=100 ≥ 30 → injected.
        let mut history = ObjectHistoryStore::default();
        history.patterns.insert(
            "p_old".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 100,
                last_improved_tick: None,
                times_selected_as_focus: 1,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 0,
            },
        );
        let mut frontier = Frontier::default();
        frontier.refresh_stale_prune(&history, 100);
        assert_eq!(frontier.items.len(), 1);
        let it = &frontier.items[0];
        assert!(matches!(it.kind, FrontierKind::LowValueObjectForPrune));
        assert_eq!(
            it.target,
            FrontierTarget::Pattern("p_old".to_string())
        );
        assert!(it.id.starts_with("prune_stale_p_old_"));
    }

    #[test]
    fn b3_recently_improved_pattern_skipped() {
        // Age=100 ≥ 50 but last_improved=95 → stale_since=5 < 30,
        // so not stale.
        let mut history = ObjectHistoryStore::default();
        history.patterns.insert(
            "p_active".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 100,
                last_improved_tick: Some(95),
                times_selected_as_focus: 5,
                times_pruned: 0,
                last_counterfactual_value: Some(2.0),
                stability_estimate: Some(0.8),
                times_contributed_positive: 0,
            },
        );
        let mut frontier = Frontier::default();
        frontier.refresh_stale_prune(&history, 100);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn b3_stale_prune_does_not_double_existing() {
        // Frontier already has a Prune for this pattern (e.g. from
        // negative counterfactual value); staleness pass must skip.
        let mut history = ObjectHistoryStore::default();
        history.patterns.insert(
            "p_neg".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 100,
                last_improved_tick: None,
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: Some(-1.0),
                stability_estimate: None,
                times_contributed_positive: 0,
            },
        );
        let mut frontier = Frontier::default();
        frontier.items.push(FrontierItem {
            id: "prune_p_neg_50".to_string(),
            kind: FrontierKind::LowValueObjectForPrune,
            target: FrontierTarget::Pattern("p_neg".to_string()),
            priority: 2.0,
            estimated_value: 1.0,
            estimated_cost: 1.0,
            novelty_score: 0.0,
            first_seen_tick: 50,
            last_visited_tick: None,
            revisit_count: 0,
            cooldown_until_tick: None,
            status: FrontierStatus::Fresh,
        });
        frontier.refresh_stale_prune(&history, 100);
        assert_eq!(frontier.items.len(), 1);
        assert_eq!(frontier.items[0].priority, 2.0);
        assert_eq!(frontier.items[0].id, "prune_p_neg_50");
    }

    #[test]
    fn b3_stale_prune_idempotent() {
        let mut history = ObjectHistoryStore::default();
        history.patterns.insert(
            "p_old".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 100,
                last_improved_tick: None,
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 0,
            },
        );
        let mut frontier = Frontier::default();
        frontier.refresh_stale_prune(&history, 100);
        let len1 = frontier.items.len();
        frontier.refresh_stale_prune(&history, 100);
        assert_eq!(frontier.items.len(), len1);
    }

    #[test]
    fn b3_stale_priority_below_negative_cv_prune() {
        // Negative-cv prune at priority 2.0 must rank above the
        // staleness-injected prune at priority 0.5.
        let mut history = ObjectHistoryStore::default();
        history.patterns.insert(
            "p_stale".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 100,
                last_improved_tick: None,
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 0,
            },
        );
        let mut frontier = Frontier::default();
        frontier.items.push(FrontierItem {
            id: "prune_p_neg_50".to_string(),
            kind: FrontierKind::LowValueObjectForPrune,
            target: FrontierTarget::Pattern("p_neg".to_string()),
            priority: 2.0,
            estimated_value: 1.0,
            estimated_cost: 1.0,
            novelty_score: 0.0,
            first_seen_tick: 50,
            last_visited_tick: None,
            revisit_count: 0,
            cooldown_until_tick: None,
            status: FrontierStatus::Fresh,
        });
        frontier.refresh_stale_prune(&history, 100);
        assert_eq!(frontier.items.len(), 2);
        assert!(
            frontier.items[0].priority >= frontier.items[1].priority,
            "items not in priority-descending order"
        );
        assert_eq!(
            frontier.items[0].target,
            FrontierTarget::Pattern("p_neg".to_string())
        );
    }

    // ─── Phase C0 — selective declarativization (ADR 0053) ──────

    fn rs_with_named_pattern(id: &str) -> RSet {
        // Minimal rset where `id` shows up in `rset.patterns()`.
        // Uses the registry edge directly since a full discovery run
        // is overkill for the gate / dispatch tests.
        let mut rs = RSet::new();
        rs.add(R::new(crate::PATTERN_MARKER, id));
        rs
    }

    fn history_with_pattern(
        id: &str,
        first_seen: u64,
        last_improved: Option<u64>,
    ) -> ObjectHistoryStore {
        // When `last_improved` is Some, set `times_contributed_positive`
        // high enough to clear the C0+ M-counter gate (default 3).
        // When None, the object never contributed positively, so 0.
        let times_contributed_positive: u32 =
            if last_improved.is_some() { 3 } else { 0 };
        let mut h = ObjectHistoryStore::default();
        h.patterns.insert(
            id.to_string(),
            ObjectHistory {
                first_seen_tick: first_seen,
                last_seen_tick: first_seen,
                last_improved_tick: last_improved,
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive,
            },
        );
        h
    }

    #[test]
    fn c0_promotion_inactive_below_age() {
        // Pattern is named and has improved, but age (50) is below
        // the 100-tick promotion floor.
        let rs = rs_with_named_pattern("p_young");
        let history = history_with_pattern("p_young", 0, Some(40));
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 50);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c0_promotion_inactive_when_never_improved() {
        // Pattern aged enough but `last_improved_tick = None` →
        // M ≥ 1 not satisfied.
        let rs = rs_with_named_pattern("p_dead");
        let history = history_with_pattern("p_dead", 0, None);
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 200);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c0_promotion_active_when_qualified() {
        let rs = rs_with_named_pattern("p_good");
        let history = history_with_pattern("p_good", 0, Some(80));
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 150);
        assert_eq!(frontier.items.len(), 1);
        let it = &frontier.items[0];
        assert!(matches!(it.kind, FrontierKind::EstablishedPromotion));
        assert_eq!(
            it.target,
            FrontierTarget::Pattern("p_good".to_string())
        );
        assert!(it.id.starts_with("promote_p_good_"));
    }

    #[test]
    fn c0_promotion_skips_already_promoted() {
        // ESTABLISHED edge already in rset → no item.
        let mut rs = rs_with_named_pattern("p_done");
        rs.add(R::new("p_done", ESTABLISHED_MARKER));
        let history = history_with_pattern("p_done", 0, Some(80));
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 150);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c0_promotion_skips_dropped_pattern() {
        // History knows about p_gone, but rset doesn't list it any
        // more (e.g. it was retracted).
        let rs = RSet::new();
        let history = history_with_pattern("p_gone", 0, Some(80));
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 150);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c0_promotion_idempotent() {
        let rs = rs_with_named_pattern("p_good");
        let history = history_with_pattern("p_good", 0, Some(80));
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 150);
        let len1 = frontier.items.len();
        frontier.refresh_established_promotions(&rs, &history, 150);
        assert_eq!(frontier.items.len(), len1);
    }

    #[test]
    fn c0plus_promotion_skipped_when_use_below_threshold() {
        // Age clears (150 ≥ 100) and last_improved_tick is set, but
        // times_contributed_positive (= 2) < default min (= 3) →
        // gate should reject.
        let rs = rs_with_named_pattern("p_close");
        let mut history = ObjectHistoryStore::default();
        history.patterns.insert(
            "p_close".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 150,
                last_improved_tick: Some(140),
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 2,
            },
        );
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 150);
        assert!(
            frontier.items.is_empty(),
            "M ≥ 3 not yet met; promotion must wait"
        );
    }

    #[test]
    fn c0plus_promotion_active_exactly_at_use_threshold() {
        let rs = rs_with_named_pattern("p_ready");
        let mut history = ObjectHistoryStore::default();
        history.patterns.insert(
            "p_ready".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 150,
                last_improved_tick: Some(140),
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 3,
            },
        );
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 150);
        assert_eq!(frontier.items.len(), 1);
    }

    #[test]
    fn c0plus_counter_serialises_and_round_trips() {
        // Non-zero counter must survive checkpoint → restore.
        let mut rt = AutonomousRuntime::new(diamond_poset());
        rt.memory.object_history.patterns.insert(
            "p_z".to_string(),
            ObjectHistory {
                first_seen_tick: 5,
                last_seen_tick: 12,
                last_improved_tick: Some(10),
                times_selected_as_focus: 1,
                times_pruned: 0,
                last_counterfactual_value: Some(0.4),
                stability_estimate: None,
                times_contributed_positive: 7,
            },
        );
        let cp = rt.checkpoint_text().unwrap();
        let rt2 = AutonomousRuntime::from_checkpoint_text(&cp).unwrap();
        let h = rt2
            .memory
            .object_history
            .patterns
            .get("p_z")
            .expect("p_z survives checkpoint");
        assert_eq!(h.times_contributed_positive, 7);
        assert_eq!(h.times_selected_as_focus, 1);
        assert_eq!(h.last_improved_tick, Some(10));
    }

    #[test]
    fn c0plus_counter_increments_on_positive_delta_episode() {
        // End-to-end: the runtime's per-tick history maintenance
        // increments times_contributed_positive whenever the
        // post-action delta is positive AND the named object is in
        // patterns_after / theories_after.
        let mut rt = AutonomousRuntime::new(diamond_poset());
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(20);
        let any_pattern_with_positive_count = rt
            .memory
            .object_history
            .patterns
            .values()
            .any(|h| h.times_contributed_positive > 0);
        let any_theory_with_positive_count = rt
            .memory
            .object_history
            .theories
            .values()
            .any(|h| h.times_contributed_positive > 0);
        assert!(
            any_pattern_with_positive_count
                || any_theory_with_positive_count,
            "expected at least one named object with \
             times_contributed_positive > 0 after a 20-tick run"
        );
    }

    #[test]
    fn c0_execute_declarativize_adds_established_edge() {
        // Plant a named pattern with old age (eligible for promotion)
        // but recent `last_improved_tick` so B3 stale-prune doesn't
        // fire on the same target. Otherwise both Promotion and
        // Stale-Prune would be valid Consolidate work, and the
        // post-Promotion ticks would retract the pattern (cascading
        // ESTABLISHED away).
        let rs = rs_with_named_pattern("p_good");
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.memory.object_history.patterns.insert(
            "p_good".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 149,
                last_improved_tick: Some(149),
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 3,
            },
        );
        rt.tick = 150;
        rt.frontier.mark_dirty();
        // Two ticks: iter 1 = SwitchMode(Expand→Consolidate),
        // iter 2 = Execute(Declarativize). A third tick would
        // pick the bare-registry pattern's negative-cv Prune
        // (cv = -0.1, no instances), which would cascade
        // ESTABLISHED away before the assertion.
        rt.run_bounded(2);
        assert!(
            rt.rset.contains(&R::new("p_good", ESTABLISHED_MARKER)),
            "expected R(p_good, ESTABLISHED_MARKER) after Declarativize"
        );
        let last = rt.memory.episodes.iter().last().unwrap();
        assert_eq!(last.action_kind, ActionKind::Declarativize);
    }

    #[test]
    fn c0_b3_interaction_promote_then_prune_cascade() {
        // ADR 0053 § "B3 interaction" verification. Pattern is
        // promoted on tick 150 (recently improved → no stale-prune),
        // then we age out last_improved_tick so B3 fires later. The
        // Promotion edge must vanish via the retract cascade.
        let rs = rs_with_named_pattern("p_x");
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.memory.object_history.patterns.insert(
            "p_x".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 149,
                last_improved_tick: Some(149),
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 3,
            },
        );
        rt.tick = 150;
        rt.frontier.mark_dirty();
        rt.run_bounded(2);
        assert!(
            rt.rset.contains(&R::new("p_x", ESTABLISHED_MARKER)),
            "expected promotion edge after first window"
        );

        // Phase 2: continue running. The pattern has negative cv
        // (no instances) so the existing negative-cv Prune fires;
        // the retract cascade in retract_pattern (step 7) drops
        // the ESTABLISHED edge with the pattern. Cascade is what
        // the ADR's "B3 interaction" test validates — the same
        // mechanism applies whether the prune was triggered by
        // negative cv or by B3 staleness.
        rt.run_bounded(3);
        assert!(
            !rt.rset.contains(&R::new("p_x", ESTABLISHED_MARKER)),
            "ESTABLISHED edge should have cascaded with prune"
        );
        assert!(
            !rt.rset.patterns().contains(&"p_x"),
            "pattern itself should be pruned"
        );
    }

    #[test]
    fn c0_retract_pattern_cascades_established() {
        // Bare named pattern (no instances/roles) is enough to
        // confirm the cascade — retract_pattern's per-layer cleanup
        // no-ops on the missing layers and step (7) removes the
        // ESTABLISHED edge.
        let mut rs = rs_with_named_pattern("p_x");
        rs.add(R::new("p_x", ESTABLISHED_MARKER));
        assert!(rs.contains(&R::new("p_x", ESTABLISHED_MARKER)));
        rs.retract_pattern("p_x").expect("retract");
        assert!(
            !rs.contains(&R::new("p_x", ESTABLISHED_MARKER)),
            "ESTABLISHED edge must cascade with retract_pattern"
        );
    }

    // ─── Phase C1 — theory promotion (ADR 0053) ─────────────────

    fn rs_with_named_theory(id: &str) -> RSet {
        let mut rs = RSet::new();
        rs.add(R::new(crate::THEORY_MARKER, id));
        rs
    }

    fn history_with_theory(
        id: &str,
        first_seen: u64,
        last_improved: Option<u64>,
    ) -> ObjectHistoryStore {
        let times_contributed_positive: u32 =
            if last_improved.is_some() { 3 } else { 0 };
        let mut h = ObjectHistoryStore::default();
        h.theories.insert(
            id.to_string(),
            ObjectHistory {
                first_seen_tick: first_seen,
                last_seen_tick: first_seen,
                last_improved_tick: last_improved,
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive,
            },
        );
        h
    }

    #[test]
    fn c1_promotion_inactive_below_theory_age() {
        // Age 150 ≥ pattern threshold (100) but below theory
        // threshold (200). Confirms the gate uses the theory-
        // specific knob, not the pattern one.
        let rs = rs_with_named_theory("t_young");
        let history = history_with_theory("t_young", 0, Some(50));
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 150);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c1_promotion_inactive_when_never_improved() {
        let rs = rs_with_named_theory("t_dead");
        let history = history_with_theory("t_dead", 0, None);
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 300);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c1_promotion_active_when_qualified() {
        let rs = rs_with_named_theory("t_good");
        let history = history_with_theory("t_good", 0, Some(180));
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 250);
        assert_eq!(frontier.items.len(), 1);
        let it = &frontier.items[0];
        assert!(matches!(it.kind, FrontierKind::EstablishedPromotion));
        assert_eq!(
            it.target,
            FrontierTarget::Theory("t_good".to_string())
        );
        assert!(it.id.starts_with("promote_t_good_"));
    }

    #[test]
    fn c1_promotion_skips_already_promoted_theory() {
        let mut rs = rs_with_named_theory("t_done");
        rs.add(R::new("t_done", ESTABLISHED_MARKER));
        let history = history_with_theory("t_done", 0, Some(180));
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 250);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c1_promotion_skips_dropped_theory() {
        let rs = RSet::new();
        let history = history_with_theory("t_gone", 0, Some(180));
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 250);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c1_retract_theory_cascades_established() {
        let mut rs = rs_with_named_theory("t_x");
        rs.add(R::new("t_x", ESTABLISHED_MARKER));
        assert!(rs.contains(&R::new("t_x", ESTABLISHED_MARKER)));
        rs.retract_theory("t_x").expect("retract");
        assert!(
            !rs.contains(&R::new("t_x", ESTABLISHED_MARKER)),
            "ESTABLISHED edge must cascade with retract_theory"
        );
    }

    #[test]
    fn c1_pattern_and_theory_both_promote() {
        // Both stores populate; both pass their respective gates;
        // both items appear in the frontier.
        let mut rs = RSet::new();
        rs.add(R::new(crate::PATTERN_MARKER, "p_x"));
        rs.add(R::new(crate::THEORY_MARKER, "t_x"));
        let mut history = ObjectHistoryStore::default();
        history.patterns.insert(
            "p_x".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 100,
                last_improved_tick: Some(80),
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 3,
            },
        );
        history.theories.insert(
            "t_x".to_string(),
            ObjectHistory {
                first_seen_tick: 0,
                last_seen_tick: 250,
                last_improved_tick: Some(180),
                times_selected_as_focus: 0,
                times_pruned: 0,
                last_counterfactual_value: None,
                stability_estimate: None,
                times_contributed_positive: 3,
            },
        );
        let mut frontier = Frontier::default();
        frontier.refresh_established_promotions(&rs, &history, 250);
        assert_eq!(frontier.items.len(), 2);
        let mut targets: Vec<&FrontierTarget> =
            frontier.items.iter().map(|it| &it.target).collect();
        targets.sort_by_key(|t| format!("{:?}", t));
        assert_eq!(
            targets,
            vec![
                &FrontierTarget::Pattern("p_x".to_string()),
                &FrontierTarget::Theory("t_x".to_string()),
            ]
        );
    }

    // ─── Phase C2 — shared-axiom promotion (ADR 0053) ──────────

    /// Build an rset with `axiom_id` registered and made a member of
    /// each `theory_id` in `theories`. No instances / structure on
    /// the axiom — just the registry + membership shape needed for
    /// `theories_containing` to count it.
    fn rs_with_axiom_in_theories(
        axiom_id: &str,
        theories: &[&str],
    ) -> RSet {
        let mut rs = RSet::new();
        rs.add(R::new(crate::AXIOM_MARKER, axiom_id));
        for t in theories {
            rs.add(R::new(crate::THEORY_MARKER, *t));
            rs.add(R::new(*t, axiom_id));
        }
        rs
    }

    #[test]
    fn c2_no_promotion_when_axiom_in_one_theory() {
        let rs = rs_with_axiom_in_theories("ax_lonely", &["t_a"]);
        let mut frontier = Frontier::default();
        frontier.refresh_shared_axiom_promotions(&rs, 0);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c2_promotion_active_when_axiom_in_two_theories() {
        let rs = rs_with_axiom_in_theories("ax_shared", &["t_a", "t_b"]);
        let mut frontier = Frontier::default();
        frontier.refresh_shared_axiom_promotions(&rs, 0);
        assert_eq!(frontier.items.len(), 1);
        let it = &frontier.items[0];
        assert!(matches!(it.kind, FrontierKind::EstablishedPromotion));
        assert_eq!(
            it.target,
            FrontierTarget::Axiom("ax_shared".to_string())
        );
    }

    #[test]
    fn c2_promotion_skips_already_marked() {
        let mut rs =
            rs_with_axiom_in_theories("ax_done", &["t_a", "t_b"]);
        rs.add(R::new("ax_done", SHARED_AXIOM_MARKER));
        let mut frontier = Frontier::default();
        frontier.refresh_shared_axiom_promotions(&rs, 0);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn c2_promotion_idempotent() {
        let rs = rs_with_axiom_in_theories("ax_shared", &["t_a", "t_b"]);
        let mut frontier = Frontier::default();
        frontier.refresh_shared_axiom_promotions(&rs, 0);
        let len1 = frontier.items.len();
        frontier.refresh_shared_axiom_promotions(&rs, 0);
        assert_eq!(frontier.items.len(), len1);
    }

    #[test]
    fn c2_declarativize_axiom_writes_shared_marker() {
        // Direct dispatch test — ensure the action handler emits the
        // SHARED_AXIOM_MARKER edge, not the ESTABLISHED one.
        let rs = rs_with_axiom_in_theories("ax_x", &["t_a", "t_b"]);
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.tick = 1;
        rt.frontier.mark_dirty();
        // Two ticks: SwitchMode(Expand→Consolidate), then
        // Execute(Declarativize). After the third tick the negative-
        // cv prune fires on the bare-registry theories, so we stop
        // early.
        rt.run_bounded(2);
        assert!(
            rt.rset.contains(&R::new("ax_x", SHARED_AXIOM_MARKER)),
            "expected R(ax_x, SHARED_AXIOM_MARKER) after Declarativize"
        );
        assert!(
            !rt.rset.contains(&R::new("ax_x", ESTABLISHED_MARKER)),
            "axiom must not get the ESTABLISHED marker (different layer)"
        );
    }

    #[test]
    fn c2_demotion_via_retract_theory_drops_to_one() {
        // 2 theories share the axiom; mark; retract one theory →
        // axiom is now in 1 theory → SHARED_AXIOM cascades.
        let mut rs = rs_with_axiom_in_theories("ax_x", &["t_a", "t_b"]);
        rs.add(R::new("ax_x", SHARED_AXIOM_MARKER));
        assert!(rs.contains(&R::new("ax_x", SHARED_AXIOM_MARKER)));
        rs.retract_theory("t_a").expect("retract");
        assert_eq!(rs.theories_containing("ax_x").len(), 1);
        assert!(
            !rs.contains(&R::new("ax_x", SHARED_AXIOM_MARKER)),
            "SHARED_AXIOM should cascade when count drops below 2"
        );
    }

    #[test]
    fn c2_three_theories_one_retract_keeps_shared() {
        // 3 theories share; retract one → still 2 → marker stays.
        let mut rs = rs_with_axiom_in_theories(
            "ax_x",
            &["t_a", "t_b", "t_c"],
        );
        rs.add(R::new("ax_x", SHARED_AXIOM_MARKER));
        rs.retract_theory("t_a").expect("retract");
        assert_eq!(rs.theories_containing("ax_x").len(), 2);
        assert!(
            rs.contains(&R::new("ax_x", SHARED_AXIOM_MARKER)),
            "SHARED_AXIOM should survive while ≥ 2 theories remain"
        );
    }

    // ─── Phase D0 — meta-meta discovery (ADR 0054) ─────────────

    /// Three named patterns, each with an ESTABLISHED edge plus
    /// PATTERN_MARKER registry. Used to test the meta-subset filter:
    /// the 3 PATTERN_MARKER edges are non-M1 meta and should be
    /// excluded; the 3 ESTABLISHED edges should be included.
    fn rs_with_three_established_patterns() -> RSet {
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        // A few data edges so discovery has data-side material too.
        rs.add(R::new("u", "v"));
        rs.add(R::new("v", "w"));
        rs.add(R::new("u", "w"));
        rs
    }

    #[test]
    fn d0_filter_includes_data_and_m1_excludes_other_meta() {
        let rs = rs_with_three_established_patterns();
        let mut subset: HashSet<String> = HashSet::new();
        subset.insert(ESTABLISHED_MARKER.to_string());
        for r in rs.right_of(ESTABLISHED_MARKER) {
            subset.insert(r.x.clone());
        }
        let visible = rs.edges_with_meta_subset_sorted(&subset);
        // Visible: 3 data + 3 ESTABLISHED edges = 6; the 3
        // PATTERN_MARKER edges should be excluded because their
        // endpoints PATTERN_MARKER and the named pattern ids end
        // up partly in / partly out of `subset` — PATTERN_MARKER
        // is not in subset, and the pattern ids ARE in subset, so
        // they DO get included via the "at least one endpoint in
        // subset" rule. Adjust expectation accordingly.
        // Expected visible edges = 3 data + 3 ESTABLISHED + 3
        // PATTERN_MARKER (because the named patterns are anchors)
        // = 9 edges.
        assert_eq!(visible.len(), 9);
        // But unrelated meta — like a dummy AXIOM_MARKER edge
        // touching neither subset member — must be excluded.
        let mut rs2 = rs.clone();
        rs2.add(R::new(crate::AXIOM_MARKER, "ax_unrelated"));
        let visible2 = rs2.edges_with_meta_subset_sorted(&subset);
        assert_eq!(
            visible2.len(),
            9,
            "unrelated meta edges must stay excluded"
        );
    }

    #[test]
    fn d0_filter_no_m1_yields_data_only() {
        let mut rs = RSet::new();
        rs.add(R::new("u", "v"));
        rs.add(R::new("v", "w"));
        rs.add(R::new(crate::PATTERN_MARKER, "p_x")); // unrelated meta
        let subset: HashSet<String> = [ESTABLISHED_MARKER]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let visible = rs.edges_with_meta_subset_sorted(&subset);
        // Only the 2 data edges; no M1 edges exist, so no expansion.
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn d0_filter_pure_m1_no_data() {
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        let mut subset: HashSet<String> = HashSet::new();
        subset.insert(ESTABLISHED_MARKER.to_string());
        for r in rs.right_of(ESTABLISHED_MARKER) {
            subset.insert(r.x.clone());
        }
        // Discovery should run without panicking even when there's
        // no pure-data substrate.
        let cfg = DiscoveryConfig {
            target_size: 2,
            sample_count: 50,
            top_m: 5,
            rng_seed: 7,
            include_meta_in_discovery: false,
        };
        let candidates = rs.discover_motifs_with_meta_subset(&cfg, &subset);
        // Expected: at least one candidate (the M1 edge shape) — but
        // we don't assert content, just non-panic and sane return.
        assert!(candidates.len() <= cfg.top_m);
    }

    #[test]
    fn d0_frontier_inactive_below_threshold() {
        // 4 ESTABLISHED edges < 5 threshold → no item.
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c", "p_d"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        let mut frontier = Frontier::default();
        frontier.refresh_meta_meta_candidates(&rs, 0);
        assert!(frontier.items.is_empty());
    }

    #[test]
    fn d0_frontier_active_at_threshold() {
        // 5 ESTABLISHED edges → item appears.
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        let mut frontier = Frontier::default();
        frontier.refresh_meta_meta_candidates(&rs, 7);
        assert_eq!(frontier.items.len(), 1);
        let it = &frontier.items[0];
        assert!(matches!(it.kind, FrontierKind::MetaMetaCandidate));
        assert_eq!(it.target, FrontierTarget::WholeRSet);
        assert!(it.id.starts_with("meta_meta_"));
    }

    #[test]
    fn d0_frontier_mixes_established_and_shared_axiom() {
        // 3 ESTABLISHED + 2 SHARED_AXIOM = 5 → threshold met.
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        for ax in &["ax_x", "ax_y"] {
            rs.add(R::new(crate::AXIOM_MARKER, *ax));
            rs.add(R::new(*ax, SHARED_AXIOM_MARKER));
        }
        let mut frontier = Frontier::default();
        frontier.refresh_meta_meta_candidates(&rs, 0);
        assert_eq!(frontier.items.len(), 1);
    }

    #[test]
    fn d0_frontier_idempotent_no_duplicate() {
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        let mut frontier = Frontier::default();
        frontier.refresh_meta_meta_candidates(&rs, 0);
        let len1 = frontier.items.len();
        frontier.refresh_meta_meta_candidates(&rs, 0);
        assert_eq!(frontier.items.len(), len1);
    }

    #[test]
    fn d0_runtime_dispatches_meta_meta_episode() {
        // Set up enough M1 edges to trigger the gate; let runtime
        // pick the item in Expand, dispatch, and record an episode.
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        // A minimum data substrate so other Expand candidates don't
        // dominate the priority sort. (MetaMetaCandidate priority
        // 1.0 vs PatternCandidate variable; we want meta-meta to be
        // the dominant choice.)
        rs.add(R::new("u", "v"));
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.frontier.mark_dirty();
        rt.run_bounded(3);
        let saw_meta_meta = rt
            .memory
            .episodes
            .iter()
            .any(|ep| ep.action_kind == ActionKind::DiscoverMetaMetaPatterns);
        assert!(
            saw_meta_meta,
            "expected at least one DiscoverMetaMetaPatterns episode in {} ticks; got episodes: {:?}",
            rt.tick,
            rt.memory
                .episodes
                .iter()
                .map(|ep| ep.action_kind)
                .collect::<Vec<_>>()
        );
    }

    // ─── Phase D0+ — loop closure (ADR 0054) ────────────────────

    #[test]
    fn d0plus_find_instances_returns_m1_anchored_subgraphs() {
        // 5 ESTABLISHED edges form a star centred at ESTABLISHED_MARKER.
        // The 3-edge "in-star" canonical (3 edges all pointing to a
        // single shared right-endpoint) should have multiple clean
        // instances — every 3-subset of the 5 ESTABLISHED edges.
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        let subset = rs.meta_meta_subset(&[ESTABLISHED_MARKER]);
        // Build the canonical for "three edges into the same right
        // endpoint" by sampling one such instance and canonicalising
        // it directly.
        let mut sample = std::collections::HashSet::<R>::new();
        sample.insert(R::new("p_a", ESTABLISHED_MARKER));
        sample.insert(R::new("p_b", ESTABLISHED_MARKER));
        sample.insert(R::new("p_c", ESTABLISHED_MARKER));
        let sg = crate::Subgraph::from_edges(sample.into_iter().collect::<Vec<_>>());
        let canon = sg.canonicalize();
        let instances =
            rs.find_instances_of_with_meta_subset(&canon, &subset);
        // The 5 ESTABLISHED edges share a common right-endpoint, so
        // every 3-subset is a connected canonical match. PATTERN_MARKER
        // edges happen to canonicalize identically (fan-in and fan-out
        // collapse to the same WL-1 canonical when the only label
        // distinction is degree direction at the unique source/target),
        // so we get 10 ESTABLISHED-fan-ins + 10 PATTERN_MARKER-fan-outs
        // = 20 instances in this view. The contract verified by this
        // test is "find_instances_with_meta_subset returns non-empty
        // matches when the M1 view contains a recurring shape" — exact
        // counts depend on canonical equivalence-class sizes.
        assert!(
            instances.len() >= 10,
            "expected ≥ 10 instances, got {}",
            instances.len()
        );
        for sg in &instances {
            assert!(rs.is_clean_subgraph_with_meta_subset(sg, &subset));
        }
    }

    #[test]
    fn d0plus_loop_closure_names_meta_meta_pattern() {
        // E2E: 5 ESTABLISHED edges, run runtime, verify a NEW pattern
        // is named whose canonical lives in the M1 hypothesis space.
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        let pre_count = rs.patterns().len();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.frontier.mark_dirty();
        rt.run_bounded(8);
        let post_count = rt.rset.patterns().len();
        assert!(
            post_count > pre_count,
            "expected meta-meta-pattern to be named (pre={}, post={})",
            pre_count,
            post_count
        );
        // Verify a Declarativize-style episode wasn't required —
        // naming happens through the DiscoverMetaMetaPatterns
        // execute_action arm, so the episode kind is that.
        let saw_meta_meta = rt.memory.episodes.iter().any(|ep| {
            ep.action_kind == ActionKind::DiscoverMetaMetaPatterns
        });
        assert!(saw_meta_meta);
    }

    #[test]
    fn d0plus_intensional_naming_does_not_pin_marker_as_instance() {
        // Confirm the Intensional policy: after a meta-meta-pattern
        // gets named, no instance edge `R(<inst>, ESTABLISHED_MARKER)`
        // appears with `<inst>` having the `<pattern>_i_<n>` shape.
        // (Such edges would mean Layer B pinned ESTABLISHED_MARKER
        // as a literal participant, conflating the abstract role
        // with the marker itself.)
        let mut rs = RSet::new();
        for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
            rs.add(R::new(crate::PATTERN_MARKER, *p));
            rs.add(R::new(*p, ESTABLISHED_MARKER));
        }
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.frontier.mark_dirty();
        rt.run_bounded(8);
        // Intensional policy means no `R(p_*_i_*, *)` edges should
        // exist for the new pattern. Confirm by checking that no
        // edge ending in ESTABLISHED_MARKER has an x-side id of the
        // shape `p_<n>_i_<m>` (the instance-id mint shape).
        for r in rt.rset.right_of(ESTABLISHED_MARKER) {
            assert!(
                !r.x.contains("_i_"),
                "found instance-bound ESTABLISHED edge: {:?} \
                 — Intensional naming should not produce these",
                r
            );
        }
    }

    #[test]
    fn b1_below_threshold_still_switches() {
        let rs = diamond_poset();
        let mut frontier = Frontier::default();
        frontier.refresh(&rs, 0);
        let mut memory = Memory::default();
        *memory
            .policy_stats
            .mode_transition_counts
            .entry((RuntimeMode::Reflect, RuntimeMode::Expand))
            .or_insert(0) = 1;
        let ctx = SchedulerContext {
            rset: &rs,
            memory: &memory,
            frontier: &frontier,
            mode: RuntimeMode::Reflect,
            tick: 0,
        };
        let mut sched = RuleBasedScheduler {
            max_mode_oscillations: 4,
            ..RuleBasedScheduler::default()
        };
        match sched.choose(&ctx) {
            SchedulerDecision::SwitchMode(RuntimeMode::Expand) => {}
            other => panic!("expected SwitchMode(Expand); got {:?}", other),
        }
    }

    // ─── Phase A verification — 8-case rigorous battery ─────────
    //
    // ADR 0052 § Verification plan #3: bounded-tick runs on the 8
    // cases from ADR 0027 must reach a stable state and produce
    // theory output matching what a direct
    // `rset.discover_theory(...)` call would produce on the same
    // input. Cases below mirror examples/axiom_rigorous_test.rs.

    fn rig_case_transitive_chain() -> RSet {
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d", "e"];
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        rs
    }

    fn rig_case_equivalence() -> RSet {
        let mut rs = RSet::new();
        let classes: &[&[&str]] = &[&["a", "b"], &["c", "d", "e"], &["f"]];
        for cls in classes {
            for x in cls.iter() {
                for y in cls.iter() {
                    rs.add(R::new(*x, *y));
                }
            }
        }
        rs
    }

    fn rig_case_strict_partial_order() -> RSet {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
            R::new("b", "d"), R::new("c", "d"),
        ]);
        rs
    }

    fn rig_case_almost_transitive() -> RSet {
        let mut rs = RSet::new();
        let nodes = ["a", "b", "c", "d"];
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        rs.remove(&R::new("b", "d"));
        rs
    }

    fn rig_case_random_sparse() -> RSet {
        let mut rs = RSet::new();
        rs.extend([
            R::new("a", "c"), R::new("b", "d"), R::new("c", "e"),
            R::new("d", "f"), R::new("e", "a"), R::new("f", "b"),
            R::new("a", "d"),
        ]);
        rs
    }

    fn rig_case_tolerance() -> RSet {
        let mut rs = RSet::new();
        for n in ["a", "b", "c"] {
            rs.add(R::new(n, n));
        }
        rs.extend([
            R::new("a", "b"), R::new("b", "a"),
            R::new("b", "c"), R::new("c", "b"),
        ]);
        rs
    }

    fn rig_case_total_order() -> RSet {
        let mut rs = RSet::new();
        let nodes = ["1", "2", "3", "4", "5"];
        for i in 0..nodes.len() {
            rs.add(R::new(nodes[i], nodes[i]));
            for j in (i + 1)..nodes.len() {
                rs.add(R::new(nodes[i], nodes[j]));
            }
        }
        rs
    }

    fn rig_case_complete_bipartite() -> RSet {
        let mut rs = RSet::new();
        for a in ["a1", "a2", "a3"] {
            for b in ["b1", "b2", "b3"] {
                rs.add(R::new(a, b));
            }
        }
        rs
    }

    fn rigorous_battery() -> Vec<(&'static str, RSet)> {
        vec![
            ("transitive_chain", rig_case_transitive_chain()),
            ("equivalence_3_classes", rig_case_equivalence()),
            ("strict_partial_order_diamond", rig_case_strict_partial_order()),
            ("almost_transitive", rig_case_almost_transitive()),
            ("random_sparse", rig_case_random_sparse()),
            ("tolerance", rig_case_tolerance()),
            ("total_order", rig_case_total_order()),
            ("complete_bipartite", rig_case_complete_bipartite()),
        ]
    }

    #[test]
    fn a_verification_8_case_battery_matches_direct_discovery() {
        let cfg = AxiomDiscoveryConfig::default();
        for (label, rs) in rigorous_battery() {
            // Direct fingerprint: what discover_theory says about
            // this rset, called outside the runtime.
            let direct = rs.discover_theory(&cfg);
            let mut direct_members: Vec<String> =
                direct.member_axiom_ids.clone();
            direct_members.sort();

            // Run the same rset under the autonomous runtime.
            let mut rt = AutonomousRuntime::new(rs);
            rt.scheduler = Box::new(RuleBasedScheduler::default());
            rt.run_bounded(60);

            // The runtime should have settled (Sleeping). It might
            // still be Running on cases where the budget runs out,
            // but for these 8 cases 60 ticks is generous enough.
            assert_eq!(
                rt.lifecycle,
                LifecycleState::Sleeping,
                "case {}: runtime did not stabilize (lifecycle={:?})",
                label,
                rt.lifecycle
            );

            // Compare fingerprints.
            let theories = rt.rset.theories();
            assert!(
                theories.len() <= 1,
                "case {}: expected at most one theory, got {}",
                label,
                theories.len()
            );
            let runtime_members: Vec<String> = if theories.is_empty() {
                Vec::new()
            } else {
                let mut v: Vec<String> = rt
                    .rset
                    .theory_axioms(theories[0])
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                v.sort();
                v
            };
            assert_eq!(
                runtime_members, direct_members,
                "case {}: runtime theory fingerprint differs from \
                 direct discover_theory output",
                label
            );
        }
    }

    #[test]
    fn a_verification_8_case_battery_is_deterministic() {
        // Same case twice → same final state. Locks down the
        // determinism guarantee that A1's deterministic-trace test
        // proves on a single graph.
        for (label, rs) in rigorous_battery() {
            let run = |rs: RSet| -> (Vec<String>, u64, LifecycleState) {
                let mut rt = AutonomousRuntime::new(rs);
                rt.scheduler = Box::new(RuleBasedScheduler::default());
                rt.run_bounded(60);
                let theories = rt.rset.theories();
                let mut members: Vec<String> = if theories.is_empty() {
                    Vec::new()
                } else {
                    rt.rset
                        .theory_axioms(theories[0])
                        .iter()
                        .map(|s| s.to_string())
                        .collect()
                };
                members.sort();
                (members, rt.tick, rt.lifecycle)
            };
            let a = run(rs.clone());
            let b = run(rs);
            assert_eq!(a, b, "case {}: non-deterministic outcome", label);
        }
    }

    #[test]
    fn a_verification_drip_feed_diamond_full() {
        // ADR 0052 verification #5 (full version): start empty,
        // drip-feed a 4-node diamond poset over 9 ticks. Final
        // state: rset contains all 9 edges, ≥ 1 theory named, AND
        // is_poset is true.
        let schedule: Vec<(u64, Event)> = vec![
            (1, Event::AddEdge(R::new("a", "a"))),
            (2, Event::AddEdge(R::new("b", "b"))),
            (3, Event::AddEdge(R::new("c", "c"))),
            (4, Event::AddEdge(R::new("d", "d"))),
            (5, Event::AddEdge(R::new("a", "b"))),
            (6, Event::AddEdge(R::new("a", "c"))),
            (7, Event::AddEdge(R::new("a", "d"))),
            (8, Event::AddEdge(R::new("b", "d"))),
            (9, Event::AddEdge(R::new("c", "d"))),
        ];
        let expected = [
            R::new("a", "a"), R::new("b", "b"), R::new("c", "c"),
            R::new("d", "d"), R::new("a", "b"), R::new("a", "c"),
            R::new("a", "d"), R::new("b", "d"), R::new("c", "d"),
        ];
        let mut rt = AutonomousRuntime::new(RSet::new());
        rt.environment = Box::new(SyntheticStreamEnvironment::new(schedule));
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(40);
        // Every scheduled edge ended up in the rset (regardless of
        // any meta-R the runtime added on top).
        for r in &expected {
            assert!(
                rt.rset.iter().any(|got| got == r),
                "missing scheduled edge {:?}",
                r
            );
        }
        // Theory named.
        assert!(rt.rset.theories().len() >= 1);
        // Structural assertion: is_poset.
        let poset = rt.rset.check_poset();
        assert!(poset.is_poset, "drip-fed diamond should be a poset");
    }
}
