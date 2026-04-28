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
//!
//! Module layout (post-2026-04-28 refactor; ADR 0067):
//! - `action` — ActionKind, FrontierTarget, ActionPlan, SchedulerDecision
//! - `lifecycle` — LifecycleState, RuntimeMode, BudgetState
//! - `environment` — Event, Environment trait + NoOp/SyntheticStream
//! - `drive` — Drive trait + 3 baseline impls + DriveMix
//! - `scheduler` — Scheduler trait + SchedulerContext + StubScheduler
//! - `scheduler_rule` — RuleBasedScheduler
//! - `scheduler_meta` — MetaScheduler (Phase H0)
//! - `scheduler_ucb` — UcbCompositeScheduler (Phase Alpha-1, negative)
//! - `memory` — Episode, transitions, history, prediction state, Memory
//! - `frontier` — FrontierKind/Status/Item, configs, Frontier
//! - `autonomous` — AutonomousRuntime + main tick loop
//! - `persistence` — A3 serialization helpers + parse_checkpoint

mod action;
mod autonomous;
mod drive;
mod environment;
mod frontier;
mod lifecycle;
mod memory;
mod persistence;
mod scheduler;
mod scheduler_meta;
mod scheduler_rule;
mod scheduler_ucb;

pub use action::{ActionKind, ActionPlan, FrontierTarget, SchedulerDecision};
pub use autonomous::AutonomousRuntime;
pub(crate) use autonomous::theory_pair_has_relation;
pub(crate) use persistence::{action_kind_to_str, parse_action_kind};
pub use drive::{
    CompressionDrive, Drive, DriveABState, DriveMix, ModeThrashPenalty,
    PredictionErrorDrive,
};
pub use environment::{
    should_wake, Environment, Event, NoOpEnvironment, SyntheticStreamEnvironment,
};
pub use frontier::{
    Frontier, FrontierItem, FrontierKind, FrontierStatus, MetaMetaConfig,
    PromotionConfig, StalenessConfig,
};
pub use lifecycle::{BudgetState, LifecycleState, RuntimeMode};
pub use memory::{
    Episode, LifecycleTransition, Memory, ModeTransition, ObjectHistory,
    ObjectHistoryStore, PolicyStats, PredictionState, SequenceStats,
};
pub use scheduler::{Scheduler, SchedulerContext, StubScheduler};
pub use scheduler_meta::{MetaABState, MetaScheduler};
pub use scheduler_rule::RuleBasedScheduler;
pub use scheduler_ucb::UcbCompositeScheduler;

#[cfg(test)]
mod tests;
