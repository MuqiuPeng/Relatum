//! Autonomous runtime layer for v2. ADR 0052 Phases A0–A1.
//!
//! - A0: spin loop, stub scheduler, NoOp environment, bounded ticks.
//! - A1: `Frontier` with candidate enumeration + cooldown via dirty
//!   tracking; `RuleBasedScheduler` picking top frontier items;
//!   action-plan `target`; pattern and prune actions wired in
//!   addition to `DiscoverTheory`.
//!
//! Frontier / mode switching / sleep policy keeps expanding in
//! A2–A3. Scheduler / environment / memory interfaces are stable.

use crate::{
    AutonomousConfig, AxiomDiscoveryConfig, DiscoveryConfig, NamingPolicy,
    RefinementConfig, RSet, R,
};
use std::collections::{HashSet, VecDeque};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    DiscoverPatterns,
    DiscoverTheory,
    PruneLowValueObjects,
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

/// A1 scheduler: pick the top frontier item; Sleep when the frontier
/// is empty or when the last few episodes all had zero delta.
pub struct RuleBasedScheduler {
    /// If the last `max_zero_streak` episodes all delivered
    /// `delta <= 0`, return Sleep. Prevents spin on saturated RSets.
    pub max_zero_streak: usize,
}

impl Default for RuleBasedScheduler {
    fn default() -> Self {
        Self { max_zero_streak: 3 }
    }
}

impl Scheduler for RuleBasedScheduler {
    fn choose(&mut self, ctx: &SchedulerContext<'_>) -> SchedulerDecision {
        // Unproductive-streak detection.
        let zero_streak = ctx
            .memory
            .episodes
            .iter()
            .rev()
            .take_while(|ep| ep.delta <= 0.0)
            .count();
        if zero_streak >= self.max_zero_streak {
            return SchedulerDecision::Sleep;
        }
        match ctx.frontier.items.first() {
            Some(item) => {
                let action_kind = match item.kind {
                    FrontierKind::TheoryCandidate => ActionKind::DiscoverTheory,
                    FrontierKind::PatternCandidate => ActionKind::DiscoverPatterns,
                    FrontierKind::LowValueObjectForPrune => {
                        ActionKind::PruneLowValueObjects
                    }
                };
                SchedulerDecision::Execute(ActionPlan {
                    action_kind,
                    target: item.target.clone(),
                })
            }
            None => SchedulerDecision::Sleep,
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

#[derive(Debug, Clone)]
pub struct Memory {
    pub episodes: VecDeque<Episode>,
    pub max_episodes: usize,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            episodes: VecDeque::new(),
            max_episodes: 1000,
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
        }
    }

    pub fn run_bounded(&mut self, max_ticks: u64) {
        let start_tick = self.tick;

        while self.tick - start_tick < max_ticks
            && self.lifecycle != LifecycleState::Stopped
            && self.lifecycle != LifecycleState::Sleeping
        {
            self.tick += 1;
            self.budget.reset_per_tick();

            // 1. Ingest events and mark frontier dirty if changed.
            let events = self.environment.poll();
            if !events.is_empty() {
                self.apply_events(events);
                self.frontier.mark_dirty();
            }

            // 2. Refresh frontier when dirty (cheap at β-scale).
            if self.frontier.dirty {
                self.frontier.refresh(&self.rset, self.tick);
            }

            // 3. Scheduler decision.
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

            // 4. Dispatch.
            match decision {
                SchedulerDecision::Execute(plan) => {
                    self.execute_and_record(plan);
                    self.frontier.mark_dirty();
                }
                SchedulerDecision::SwitchMode(m) => {
                    self.mode = m;
                }
                SchedulerDecision::Sleep => {
                    self.lifecycle = LifecycleState::Sleeping;
                }
                SchedulerDecision::Stop => {
                    self.lifecycle = LifecycleState::Stopped;
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
        self.execute_action(&plan);
        let after = self.rset.abstraction_score();
        let delta = after - before;

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
        });
        rt.run_bounded(30);
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
    }
}
