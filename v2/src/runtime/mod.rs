//! Autonomous runtime layer for v2. ADR 0052 Phase A0.
//!
//! This module is the "spin loop" scaffolding: the minimum control
//! layer that wraps existing v2 mechanisms (discovery, naming,
//! scoring) into a tick-driven runtime. A0 supports a single mode
//! (Expand), a stub scheduler (always `DiscoverTheory`), and a
//! NoOpEnvironment. Frontier, mode switching, sleep/wake, and
//! counterfactual-driven pruning land in later phases (A1–A3).

use crate::{AxiomDiscoveryConfig, RSet, R};
use std::collections::VecDeque;

// ─── lifecycle + mode ──────────────────────────────────────────────

/// Macro state of the runtime. ADR 0052.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    /// Initial boot; memory/frontier being populated.
    Booting,
    /// Actively running; scheduler choosing actions.
    Running,
    /// Idle but resumable — no events, nothing to do right now.
    Sleeping,
    /// Terminated. Not resumable.
    Stopped,
}

/// Micro state within Running — what kind of work the runtime is
/// doing. Phase A0 only uses `Expand`. ADR 0052.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    Expand,
    Consolidate,
    Reflect,
}

// ─── budget ────────────────────────────────────────────────────────

/// Two-dimensional budget (step counts only; no wall-clock).
/// ADR 0052.
#[derive(Debug, Clone, Copy)]
pub struct BudgetState {
    /// Optional ceiling on total ticks for a `run_bounded` call.
    /// `None` means unbounded — caller uses `lifecycle == Stopped`.
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

// ─── action + decision ─────────────────────────────────────────────

/// Coarse action taxonomy. Phase A0 only executes `DiscoverTheory`;
/// the other two are defined for type-level stability across phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    DiscoverPatterns,
    DiscoverTheory,
    PruneLowValueObjects,
}

/// An action the runtime will execute if the scheduler returns it.
#[derive(Debug, Clone)]
pub struct ActionPlan {
    pub action_kind: ActionKind,
}

/// What the scheduler wants to happen next.
#[derive(Debug, Clone)]
pub enum SchedulerDecision {
    Execute(ActionPlan),
    SwitchMode(RuntimeMode),
    Sleep,
    Stop,
}

/// Scheduler trait — pluggable policy. Phase A0 ships `StubScheduler`.
pub trait Scheduler {
    fn choose(
        &mut self,
        rset: &RSet,
        memory: &Memory,
        mode: RuntimeMode,
        tick: u64,
    ) -> SchedulerDecision;
}

/// Simplest possible scheduler: every call, ask for
/// `DiscoverTheory`. Used by Phase A0 tests to verify the main
/// loop itself is correct; real policy arrives in Phase A1.
pub struct StubScheduler;

impl Scheduler for StubScheduler {
    fn choose(
        &mut self,
        _rset: &RSet,
        _memory: &Memory,
        _mode: RuntimeMode,
        _tick: u64,
    ) -> SchedulerDecision {
        SchedulerDecision::Execute(ActionPlan {
            action_kind: ActionKind::DiscoverTheory,
        })
    }
}

// ─── environment ───────────────────────────────────────────────────

/// External stimulus the runtime may react to.
#[derive(Debug, Clone)]
pub enum Event {
    AddEdge(R),
    RemoveEdge(R),
    Tick,
}

/// Source of events. Phase A0 provides `NoOpEnvironment` only.
pub trait Environment {
    fn poll(&mut self) -> Vec<Event>;
}

/// An environment that never produces events. Used by tests that
/// want to observe runtime behavior on a fixed RSet.
pub struct NoOpEnvironment;

impl Environment for NoOpEnvironment {
    fn poll(&mut self) -> Vec<Event> {
        Vec::new()
    }
}

// ─── memory (M0) ───────────────────────────────────────────────────

/// One action-and-its-outcome record.
#[derive(Debug, Clone)]
pub struct Episode {
    pub id: u64,
    pub tick: u64,
    pub mode: RuntimeMode,
    pub action_kind: ActionKind,
    pub score_before: f64,
    pub score_after: f64,
    pub delta: f64,
}

/// Operational (M0) memory. Phase A0 only has an episodic ring
/// buffer; object histories and policy stats arrive in Phase B.
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

// ─── runtime ───────────────────────────────────────────────────────

/// Long-running control layer wrapping an `RSet`. Phase A0: ticks,
/// polls the environment, asks the scheduler, executes one action
/// per tick, records an episode. No frontier, no mode switch, no
/// sleep/wake logic yet. ADR 0052.
pub struct AutonomousRuntime {
    pub rset: RSet,
    pub lifecycle: LifecycleState,
    pub mode: RuntimeMode,
    pub memory: Memory,
    pub scheduler: Box<dyn Scheduler>,
    pub environment: Box<dyn Environment>,

    pub tick: u64,
    pub episode_counter: u64,
    pub steps_since_last_gain: u64,
    pub budget: BudgetState,
    pub current_score: f64,
}

impl AutonomousRuntime {
    /// Construct a runtime wrapping `rset` with stub scheduler and
    /// NoOp environment. Callers who want custom policy or input
    /// streams swap `scheduler` / `environment` before `run`.
    pub fn new(rset: RSet) -> Self {
        let current_score = rset.abstraction_score();
        Self {
            rset,
            lifecycle: LifecycleState::Running,
            mode: RuntimeMode::Expand,
            memory: Memory::default(),
            scheduler: Box::new(StubScheduler),
            environment: Box::new(NoOpEnvironment),
            tick: 0,
            episode_counter: 0,
            steps_since_last_gain: 0,
            budget: BudgetState::new(1),
            current_score,
        }
    }

    /// Run for at most `max_ticks` ticks. Stops early if the
    /// scheduler returns `Stop` or `Sleep`. Phase A0.
    pub fn run_bounded(&mut self, max_ticks: u64) {
        let start_tick = self.tick;
        while self.tick - start_tick < max_ticks
            && self.lifecycle != LifecycleState::Stopped
            && self.lifecycle != LifecycleState::Sleeping
        {
            self.tick += 1;
            self.budget.reset_per_tick();

            // 1. Ingest events (NoOp env in A0 yields nothing).
            let events = self.environment.poll();
            self.apply_events(events);

            // 2. Scheduler chooses.
            let decision = self.scheduler.choose(
                &self.rset,
                &self.memory,
                self.mode,
                self.tick,
            );

            // 3. Dispatch.
            match decision {
                SchedulerDecision::Execute(plan) => {
                    self.execute_and_record(plan);
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
                    // name_theory reuses id on member-set match, so
                    // calling it every tick is safe — idempotent after
                    // the first successful pass.
                    let _ = self.rset.name_theory(&ids);
                }
            }
            ActionKind::DiscoverPatterns => {
                // Phase A1 hooks this up to autonomous_pass.
            }
            ActionKind::PruneLowValueObjects => {
                // Phase A2 hooks this up to rank_by_counterfactual +
                // retract_*.
            }
        }
    }
}

// ─── tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EdgeTemplate, AxiomTemplate, AX_ANTISYMMETRY, AX_REFLEXIVITY};

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

    // ADR 0052 Phase A0.

    #[test]
    fn a0_runtime_runs_bounded_ticks() {
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(10);
        // Each tick produces one episode (stub scheduler always executes).
        assert_eq!(rt.memory.len(), 10);
        assert_eq!(rt.tick, 10);
        assert_eq!(rt.lifecycle, LifecycleState::Running);
    }

    #[test]
    fn a0_runtime_discovers_theory_on_diamond() {
        // First tick should name the poset theory. Subsequent ticks
        // are idempotent (name_theory reuses the id).
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(5);
        // The RSet should now contain exactly one named theory.
        assert_eq!(rt.rset.theories().len(), 1);
        // That theory includes transitivity, reflexivity, antisymmetry.
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
        assert!(members.contains(&crate::axiom_template_id(&transitivity).as_str()));
    }

    #[test]
    fn a0_score_monotone_non_decreasing() {
        // StubScheduler + DiscoverTheory is idempotent after first
        // successful pass, so score should never go down.
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        let start_score = rt.current_score;
        rt.run_bounded(10);
        assert!(rt.current_score >= start_score);
    }

    #[test]
    fn a0_first_episode_has_positive_delta_on_structured_input() {
        // On a poset, the first DiscoverTheory should name the theory
        // and lift the score. Subsequent deltas may be zero (idempotent).
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.run_bounded(5);
        let first = rt.memory.episodes.front().unwrap();
        assert!(first.delta > 0.0,
            "first episode should lift score, got delta={}", first.delta);
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
                score_before: 0.0,
                score_after: 0.0,
                delta: 0.0,
            });
        }
        assert_eq!(mem.len(), 3);
        // Oldest three (ids 0-6) dropped; newest three (ids 7-9) kept.
        let kept: Vec<u64> = mem.episodes.iter().map(|e| e.id).collect();
        assert_eq!(kept, vec![7, 8, 9]);
    }

    #[test]
    fn a0_run_bounded_is_additive() {
        // Calling run_bounded twice should be equivalent to one call
        // with the sum (for noop-environment, stub-scheduler setup).
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
                _rset: &RSet,
                _memory: &Memory,
                _mode: RuntimeMode,
                _tick: u64,
            ) -> SchedulerDecision {
                if self.called {
                    SchedulerDecision::Stop
                } else {
                    self.called = true;
                    SchedulerDecision::Execute(ActionPlan {
                        action_kind: ActionKind::DiscoverTheory,
                    })
                }
            }
        }
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(StopAfterOne { called: false });
        rt.run_bounded(100);
        assert_eq!(rt.lifecycle, LifecycleState::Stopped);
        // One Execute episode, then Stop (no second episode).
        assert_eq!(rt.memory.len(), 1);
    }

    #[test]
    fn a0_sleep_decision_halts_loop() {
        struct SleepImmediately;
        impl Scheduler for SleepImmediately {
            fn choose(
                &mut self,
                _rset: &RSet,
                _memory: &Memory,
                _mode: RuntimeMode,
                _tick: u64,
            ) -> SchedulerDecision {
                SchedulerDecision::Sleep
            }
        }
        let rs = diamond_poset();
        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(SleepImmediately);
        rt.run_bounded(100);
        assert_eq!(rt.lifecycle, LifecycleState::Sleeping);
        // No episodes recorded — scheduler never returned Execute.
        assert!(rt.memory.is_empty());
    }
}
