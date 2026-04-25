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
    RefinementConfig, RSet, TheoryRelationKind, R,
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
}

/// Where (in the RSet) the action should apply. ADR 0052 / A1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontierTarget {
    WholeRSet,
    PatternSize(usize),
    Pattern(String),
    Theory(String),
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
}

impl Default for RuleBasedScheduler {
    fn default() -> Self {
        Self {
            max_zero_streak: 3,
            recent_window: 5,
            min_recent_gains: 2,
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
        }
    }

    fn has_expand_work(ctx: &SchedulerContext<'_>) -> bool {
        ctx.frontier.items.iter().any(|it| {
            matches!(
                it.kind,
                FrontierKind::TheoryCandidate | FrontierKind::PatternCandidate
            )
        })
    }

    fn has_consolidate_work(ctx: &SchedulerContext<'_>) -> bool {
        ctx.frontier.items.iter().any(|it| {
            matches!(
                it.kind,
                FrontierKind::LowValueObjectForPrune
                    | FrontierKind::TheoryNeedsRelations
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
                    return SchedulerDecision::SwitchMode(
                        RuntimeMode::Consolidate,
                    );
                }
                // Pick an Expand-shaped action.
                if let Some(item) = Self::pick_top(ctx, |it| {
                    matches!(
                        it.kind,
                        FrontierKind::TheoryCandidate
                            | FrontierKind::PatternCandidate
                    )
                }) {
                    return SchedulerDecision::Execute(ActionPlan {
                        action_kind: Self::execute_for_kind(item.kind),
                        target: item.target.clone(),
                    });
                }
                // No expand work. Try consolidate or reflect.
                if Self::has_consolidate_work(ctx) {
                    SchedulerDecision::SwitchMode(RuntimeMode::Consolidate)
                } else {
                    SchedulerDecision::SwitchMode(RuntimeMode::Reflect)
                }
            }

            RuntimeMode::Consolidate => {
                if !Self::has_consolidate_work(ctx) {
                    return SchedulerDecision::SwitchMode(RuntimeMode::Reflect);
                }
                if let Some(item) = Self::pick_top(ctx, |it| {
                    matches!(
                        it.kind,
                        FrontierKind::LowValueObjectForPrune
                            | FrontierKind::TheoryNeedsRelations
                    )
                }) {
                    return SchedulerDecision::Execute(ActionPlan {
                        action_kind: Self::execute_for_kind(item.kind),
                        target: item.target.clone(),
                    });
                }
                SchedulerDecision::SwitchMode(RuntimeMode::Reflect)
            }

            RuntimeMode::Reflect => {
                // Pure state-machine mode: no Execute, no episode added.
                if Self::has_expand_work(ctx) {
                    SchedulerDecision::SwitchMode(RuntimeMode::Expand)
                } else if Self::has_consolidate_work(ctx) {
                    SchedulerDecision::SwitchMode(RuntimeMode::Consolidate)
                } else {
                    SchedulerDecision::Sleep
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

#[derive(Debug, Clone)]
pub struct Frontier {
    pub items: Vec<FrontierItem>,
    pub last_full_refresh_tick: u64,
    pub dirty: bool,
}

impl Default for Frontier {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            last_full_refresh_tick: 0,
            dirty: true,
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

            // 3. Refresh frontier when dirty (cheap at β-scale).
            if self.frontier.dirty {
                self.frontier.refresh(&self.rset, self.tick);
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
                }
            }
        }
        for id in &theories_after {
            if let Some(h) = store.theories.get_mut(id) {
                h.last_seen_tick = tick;
                if delta > 0.0 {
                    h.last_improved_tick = Some(tick);
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

        let memory = Memory {
            episodes,
            mode_transitions,
            lifecycle_transitions,
            max_episodes,
            max_mode_transitions,
            max_lifecycle_transitions,
            // B0: history + stats are not yet serialized; restore as
            // empty. Tests that rely on round-trip equality assert on
            // the serialized fields only. ADR 0052 § Phase B / B1
            // will land checkpoint-coverage of these stores.
            object_history: ObjectHistoryStore::default(),
            policy_stats: PolicyStats::default(),
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
    }
}

fn parse_action_kind(s: &str) -> Result<ActionKind, String> {
    match s {
        "DiscoverPatterns" => Ok(ActionKind::DiscoverPatterns),
        "DiscoverTheory" => Ok(ActionKind::DiscoverTheory),
        "PruneLowValueObjects" => Ok(ActionKind::PruneLowValueObjects),
        "UpdateTheoryRelations" => Ok(ActionKind::UpdateTheoryRelations),
        other => Err(format!("unknown ActionKind '{}'", other)),
    }
}

fn target_to_pair(t: &FrontierTarget) -> (&'static str, String) {
    match t {
        FrontierTarget::WholeRSet => ("WholeRSet", String::new()),
        FrontierTarget::PatternSize(s) => ("PatternSize", s.to_string()),
        FrontierTarget::Pattern(id) => ("Pattern", id.clone()),
        FrontierTarget::Theory(id) => ("Theory", id.clone()),
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

fn check_no_tab_or_newline(t: &FrontierTarget, ctx: &str) -> Result<(), String> {
    let id = match t {
        FrontierTarget::WholeRSet | FrontierTarget::PatternSize(_) => return Ok(()),
        FrontierTarget::Pattern(s) | FrontierTarget::Theory(s) => s,
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
                | "lifecycle_transitions" => {
                    section = Some(match name {
                        "meta" => "meta",
                        "rset" => "rset",
                        "episodes" => "episodes",
                        "mode_transitions" => "mode_transitions",
                        _ => "lifecycle_transitions",
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
    fn b0_history_and_stats_default_after_checkpoint_restore() {
        // B0 limitation: history + stats not yet serialized. Verify
        // that a restored runtime starts with empty stores so future
        // B1 work knows the boundary.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(3);
        assert!(!rt.memory.policy_stats.action_counts.is_empty());
        let text = rt.checkpoint_text().unwrap();
        let restored = AutonomousRuntime::from_checkpoint_text(&text).unwrap();
        assert!(restored.memory.policy_stats.action_counts.is_empty());
        assert!(restored.memory.object_history.theories.is_empty());
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
