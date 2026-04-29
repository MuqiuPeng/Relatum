//! UCB1 composite-selection scheduler. Wraps an inner `Scheduler`,
//! intercepts `Execute(ExecuteComposite)` decisions and replaces them
//! with a UCB1-selected composite over eligible CompositeCandidate
//! frontier items. ADR 0065 / Phase Alpha-1 (negative result).

use super::action::{ActionKind, ActionPlan, FrontierTarget, SchedulerDecision};
use super::scheduler::{Scheduler, SchedulerContext};
use super::{FrontierItem, FrontierKind, Memory};

// ─── ADR 0065 / Phase Alpha-1 — UCB1 composite selection ──────────
//
// Wraps an inner `Scheduler`. Intercepts `Execute(ExecuteComposite)`
// decisions and replaces the inner scheduler's choice with a UCB1-
// selected composite over all eligible `CompositeCandidate` items
// in the frontier. Non-composite decisions delegate to the inner
// scheduler unchanged.
//
// UCB1 selection: argmax over candidates of
//   mean_reward(c) + exploration_const * sqrt(ln(N) / visits(c))
//
// where mean_reward and visits come from `memory.episodes` —
// counting `ExecuteComposite` episodes whose target matches each
// candidate's seq_id. This makes "visits" a true bandit visit count
// (active dispatch), distinct from `SequenceStats.pair_post_ep_count`
// (passive observation).
//
// AlphaGo-flavor: this is the **selection** rule of MCTS, applied
// to v2's composite layer. Tree expansion / rollouts / value
// network are intentionally absent — see ADR 0065 § cost asymmetry.

/// Wrap any `Scheduler` with UCB1 selection over composite
/// candidates. ADR 0065 / Phase Alpha-1.
pub struct UcbCompositeScheduler {
    pub inner: Box<dyn Scheduler>,
    pub exploration_const: f64,
}

impl UcbCompositeScheduler {
    pub fn new(inner: Box<dyn Scheduler>) -> Self {
        Self {
            inner,
            exploration_const: std::f64::consts::SQRT_2,
        }
    }

    pub fn with_exploration_const(
        inner: Box<dyn Scheduler>,
        c: f64,
    ) -> Self {
        Self {
            inner,
            exploration_const: c,
        }
    }

    /// Per-composite visit count + mean reward from episode
    /// history. Iterates `memory.episodes` and counts
    /// `ExecuteComposite` whose target matches `seq_id`.
    pub(crate) fn composite_stats(memory: &Memory, seq_id: &str) -> (u64, f64) {
        let mut visits: u64 = 0;
        let mut reward_sum: f64 = 0.0;
        for ep in &memory.episodes {
            if ep.action_kind != ActionKind::ExecuteComposite {
                continue;
            }
            if let FrontierTarget::ActionSequence(s) = &ep.target {
                if s == seq_id {
                    visits += 1;
                    reward_sum += ep.delta;
                }
            }
        }
        let mean = if visits == 0 {
            0.0
        } else {
            reward_sum / visits as f64
        };
        (visits, mean)
    }

    /// UCB1 score. Unvisited candidates return `f64::INFINITY` so
    /// they're always picked first (cold-start invariant).
    pub(crate) fn ucb1_score(
        &self,
        mean: f64,
        visits: u64,
        total_visits: u64,
    ) -> f64 {
        if visits == 0 {
            return f64::INFINITY;
        }
        if total_visits == 0 {
            return mean;
        }
        let exploration = self.exploration_const
            * ((total_visits as f64).ln() / visits as f64).sqrt();
        mean + exploration
    }

    /// Pick the composite candidate with highest UCB1 score, if
    /// any composites are eligible. Returns `None` when no
    /// composite items present in frontier.
    fn ucb_select_composite<'a>(
        &self,
        ctx: &'a SchedulerContext<'_>,
    ) -> Option<&'a FrontierItem> {
        let composites: Vec<&FrontierItem> = ctx
            .frontier
            .items
            .iter()
            .filter(|it| {
                matches!(it.kind, FrontierKind::CompositeCandidate)
            })
            .collect();
        if composites.is_empty() {
            return None;
        }
        let stats: Vec<(&FrontierItem, u64, f64)> = composites
            .iter()
            .filter_map(|it| match &it.target {
                FrontierTarget::ActionSequence(s) => {
                    let (visits, mean) =
                        Self::composite_stats(ctx.memory, s);
                    Some((*it, visits, mean))
                }
                _ => None,
            })
            .collect();
        let total: u64 = stats.iter().map(|(_, v, _)| v).sum();
        stats
            .into_iter()
            .max_by(|a, b| {
                let sa = self.ucb1_score(a.2, a.1, total);
                let sb = self.ucb1_score(b.2, b.1, total);
                sa.partial_cmp(&sb)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(it, _, _)| it)
    }
}

impl Scheduler for UcbCompositeScheduler {
    fn choose(&mut self, ctx: &SchedulerContext<'_>) -> SchedulerDecision {
        let base = self.inner.choose(ctx);
        // Only intercept ExecuteComposite decisions.
        let plan = match &base {
            SchedulerDecision::Execute(p)
                if p.action_kind == ActionKind::ExecuteComposite =>
            {
                p.clone()
            }
            _ => return base,
        };
        let _ = plan; // keep base's plan as fallback (used below)
        if let Some(item) = self.ucb_select_composite(ctx) {
            return SchedulerDecision::Execute(ActionPlan {
                action_kind: ActionKind::ExecuteComposite,
                target: item.target.clone(),
            });
        }
        base
    }
}
