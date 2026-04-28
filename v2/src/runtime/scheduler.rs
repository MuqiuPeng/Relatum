//! Scheduler trait + read-only context view + the simplest stub
//! scheduler. ADR 0052 / A1.

use super::action::{ActionKind, ActionPlan, FrontierTarget, SchedulerDecision};
use super::lifecycle::RuntimeMode;
use super::{Frontier, Memory};
use crate::RSet;

/// Read-only view handed to `Scheduler::choose`. ADR 0052 / A1.
pub struct SchedulerContext<'a> {
    pub rset: &'a RSet,
    pub memory: &'a Memory,
    pub frontier: &'a Frontier,
    pub mode: RuntimeMode,
    pub tick: u64,
    /// ADR 0063 / Phase H2.0 step 3b — normalized drive signal
    /// (combined / Σ active_weights), weight-invariant. Pre-3b
    /// callers / tests can leave this 0.0; the EP anti-stagnation
    /// gate consults it as one half of an AND with `zero_streak`.
    pub normalized_drive_signal: f64,
}

impl<'a> SchedulerContext<'a> {
    /// Construct without an explicit `normalized_drive_signal`.
    /// Defaults to 0.0 — i.e., "drive signal says stagnate". Used
    /// by tests + pre-3b code paths that don't yet compute the
    /// signal. ADR 0063 / Phase H2.0 step 3b.
    pub fn new(
        rset: &'a RSet,
        memory: &'a Memory,
        frontier: &'a Frontier,
        mode: RuntimeMode,
        tick: u64,
    ) -> Self {
        Self {
            rset,
            memory,
            frontier,
            mode,
            tick,
            normalized_drive_signal: 0.0,
        }
    }
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
