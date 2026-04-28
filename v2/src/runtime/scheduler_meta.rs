//! Meta-scheduler: A/B controller wrapping two `RuleBasedScheduler`
//! candidates and mutating the loser by `EvaluatePredictions` mean
//! delta. ADR 0060 / Phase H0.

use super::action::{ActionKind, SchedulerDecision};
use super::scheduler::{Scheduler, SchedulerContext};
use super::scheduler_rule::RuleBasedScheduler;

// ─── ADR 0060 Phase H0 — meta-scheduler (A/B parameter tuning) ──

/// A/B controller wrapping two `RuleBasedScheduler` candidates.
/// Each tested over a window of `window_size` total memory episodes;
/// at end of B's window, mean `EvaluatePredictions` delta picks a
/// winner; loser's config is mutated within bounds; cycle restarts.
/// ADR 0060.
///
/// State is intentionally NOT persisted across checkpoint —
/// the A/B progress restarts on each new `run_bounded` invocation.
/// Caller can construct a fresh `MetaScheduler` with the prior
/// winner's config to continue tuning across restarts.
pub struct MetaScheduler {
    pub candidate_a: RuleBasedScheduler,
    pub candidate_b: RuleBasedScheduler,
    pub state: MetaABState,
    pub window_size: u64,
    pub stage_start_episode_count: u64,
    pub last_completed_a_mean: Option<f64>,
    pub rng_state: u64,
}

/// Which candidate is currently active in the A/B cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaABState {
    TestingA,
    TestingB,
}

impl MetaScheduler {
    pub fn new(
        candidate_a: RuleBasedScheduler,
        candidate_b: RuleBasedScheduler,
    ) -> Self {
        Self {
            candidate_a,
            candidate_b,
            state: MetaABState::TestingA,
            window_size: 50,
            stage_start_episode_count: 0,
            last_completed_a_mean: None,
            rng_state: 0xa55_a55a_5_a55a_5a55,
        }
    }

    /// Compute the mean `EvaluatePredictions` delta over episodes
    /// `[start, end)` of `ctx.memory.episodes`. Returns 0.0 when
    /// no EP episodes occur in the window.
    fn ep_mean_in_range(
        ctx: &SchedulerContext<'_>,
        start: usize,
        end: usize,
    ) -> f64 {
        if end <= start {
            return 0.0;
        }
        let mut sum: f64 = 0.0;
        let mut count: u64 = 0;
        for ep in ctx
            .memory
            .episodes
            .iter()
            .skip(start)
            .take(end - start)
        {
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

    /// Pick a tunable knob and scale by ×0.8 or ×1.25, clamped to
    /// declared per-knob bounds. ADR 0060 § H0 mutation.
    pub(crate) fn mutate(sched: &mut RuleBasedScheduler, rng_state: &mut u64) {
        // Inline xorshift-ish step (independent of `discover_motifs`'s
        // PRNG). Deterministic across platforms.
        let step = |s: &mut u64| {
            *s = s.wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *s >> 32
        };
        let knob_idx = (step(rng_state) % 6) as usize;
        let direction_up = step(rng_state) & 1 == 1;
        let factor: f64 = if direction_up { 1.25 } else { 0.8 };
        match knob_idx {
            0 => {
                let v = sched.min_pattern_hit_rate * factor;
                sched.min_pattern_hit_rate = v.clamp(0.01, 0.5);
            }
            1 => {
                let v = (sched.min_pattern_attempts_before_cooldown as f64
                    * factor) as u64;
                sched.min_pattern_attempts_before_cooldown =
                    v.clamp(1, 50);
            }
            2 => {
                let v = (sched.max_zero_streak as f64 * factor) as usize;
                sched.max_zero_streak = v.clamp(1, 20);
            }
            3 => {
                let v = (sched.recent_window as f64 * factor) as usize;
                sched.recent_window = v.clamp(1, 20);
            }
            4 => {
                let v =
                    (sched.min_recent_gains as f64 * factor) as usize;
                sched.min_recent_gains = v.clamp(1, 10);
            }
            5 => {
                let v = (sched.max_mode_oscillations as f64 * factor) as u64;
                sched.max_mode_oscillations = v.clamp(1, 20);
            }
            _ => unreachable!(),
        }
    }

    /// Inspect the elapsed window and decide whether to advance the
    /// A/B state machine. Called from `choose` before delegating.
    pub(crate) fn maybe_advance(&mut self, ctx: &SchedulerContext<'_>) {
        let now = ctx.memory.episodes.len() as u64;
        let elapsed = now.saturating_sub(self.stage_start_episode_count);
        if elapsed < self.window_size {
            return;
        }
        let stage_start = self.stage_start_episode_count as usize;
        let mean = Self::ep_mean_in_range(ctx, stage_start, now as usize);
        match self.state {
            MetaABState::TestingA => {
                self.last_completed_a_mean = Some(mean);
                self.state = MetaABState::TestingB;
            }
            MetaABState::TestingB => {
                let a_mean = self
                    .last_completed_a_mean
                    .take()
                    .unwrap_or(0.0);
                let b_mean = mean;
                if a_mean >= b_mean {
                    Self::mutate(&mut self.candidate_b, &mut self.rng_state);
                } else {
                    Self::mutate(&mut self.candidate_a, &mut self.rng_state);
                }
                self.state = MetaABState::TestingA;
            }
        }
        self.stage_start_episode_count = now;
    }
}

impl Scheduler for MetaScheduler {
    fn choose(&mut self, ctx: &SchedulerContext<'_>) -> SchedulerDecision {
        self.maybe_advance(ctx);
        match self.state {
            MetaABState::TestingA => self.candidate_a.choose(ctx),
            MetaABState::TestingB => self.candidate_b.choose(ctx),
        }
    }
}
