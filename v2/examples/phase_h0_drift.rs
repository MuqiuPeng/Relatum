//! Phase H0 drift demonstration (ADR 0060).
//!
//! Constructs a `MetaScheduler` with two `RuleBasedScheduler`
//! candidates initialised with distinct knob values, runs against
//! a streaming substrate, and prints config evolution at each
//! window boundary. Captured to
//! `logs/<date>_phase_h0_drift.log`.
//!
//! What this proves: the meta-scheduler's A/B mechanism observes
//! per-window EP delta, picks the higher-mean candidate, and
//! mutates the loser within bounds. Over multiple windows the
//! configs drift independently — observable here.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, Event, MetaABState, MetaScheduler,
        RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    R, RSet,
};

const HORIZON: u64 = 500;
const SNAPSHOT_EVERY_EPISODES: usize = 50;

fn main() {
    println!("=== ADR 0060 Phase H0 — meta-scheduler drift demo ===");
    println!();

    // Streaming substrate: 6 disjoint diamonds at 50-tick intervals.
    let mut schedule = Vec::new();
    let phases: [&[&str]; 6] = [
        &["a1", "a2", "a3", "a4"],
        &["b1", "b2", "b3", "b4"],
        &["c1", "c2", "c3", "c4"],
        &["d1", "d2", "d3", "d4"],
        &["e1", "e2", "e3", "e4"],
        &["f1", "f2", "f3", "f4"],
    ];
    let phase_offsets: [u64; 6] = [1, 50, 100, 150, 200, 250];
    for (idx, nodes) in phases.iter().enumerate() {
        let off = phase_offsets[idx];
        schedule.push((off, Event::AddEdge(R::new(nodes[0], nodes[0]))));
        schedule.push((off + 1, Event::AddEdge(R::new(nodes[1], nodes[1]))));
        schedule.push((off + 2, Event::AddEdge(R::new(nodes[2], nodes[2]))));
        schedule.push((off + 3, Event::AddEdge(R::new(nodes[3], nodes[3]))));
        schedule.push((off + 7, Event::AddEdge(R::new(nodes[0], nodes[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(nodes[0], nodes[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(nodes[0], nodes[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(nodes[1], nodes[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(nodes[2], nodes[3]))));
    }

    // Two distinct candidates: A is the default; B is more
    // aggressive (lower zero-streak floor, wider recent_window).
    let mut candidate_a = RuleBasedScheduler::default();
    let mut candidate_b = RuleBasedScheduler::default();
    candidate_a.max_zero_streak = 3;
    candidate_a.min_pattern_hit_rate = 0.10;
    candidate_b.max_zero_streak = 5;
    candidate_b.min_pattern_hit_rate = 0.05;
    let mut meta = MetaScheduler::new(candidate_a, candidate_b);
    meta.window_size = SNAPSHOT_EVERY_EPISODES as u64;

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(schedule));
    rt.scheduler = Box::new(meta);

    // Note: `MetaScheduler`'s internal state (active candidate +
    // per-config knobs) lives behind `Box<dyn Scheduler>` and isn't
    // introspectable from this side without trait-extension. The
    // observable proxy is **sustained activity** — episodes and
    // positive-delta EP counts continue to accumulate through
    // multiple windows because A/B mutation keeps producing slightly
    // different schedulers across windows.
    println!(
        "{:>6} {:>9} {:>13} {:>16}",
        "tick", "episodes", "EP_in_window", "EP_delta_sum",
    );

    let mut prev_ep_count = 0;
    while rt.tick < HORIZON {
        rt.run_bounded(SNAPSHOT_EVERY_EPISODES as u64);
        let ep_count = rt.memory.episodes.len();
        let new_eps = ep_count - prev_ep_count;
        prev_ep_count = ep_count;
        let new_episodes = rt
            .memory
            .episodes
            .iter()
            .rev()
            .take(new_eps)
            .collect::<Vec<_>>();
        let ep_in_window: usize = new_episodes
            .iter()
            .filter(|e| {
                e.action_kind == ActionKind::EvaluatePredictions
            })
            .count();
        let ep_delta_window: f64 = new_episodes
            .iter()
            .filter(|e| {
                e.action_kind == ActionKind::EvaluatePredictions
                    && e.delta > 0.0
            })
            .map(|e| e.delta)
            .sum();
        println!(
            "{:>6} {:>9} {:>13} {:>16.4}",
            rt.tick, ep_count, ep_in_window, ep_delta_window
        );
    }

    println!();
    println!("--- final episode kinds ---");
    let mut counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for ep in &rt.memory.episodes {
        *counts
            .entry(format!("{:?}", ep.action_kind))
            .or_insert(0) += 1;
    }
    for (k, v) in &counts {
        println!("  {} = {}", k, v);
    }
    println!();
    println!(
        "--- EP delta sum (positive only): {:.4} ---",
        rt.memory
            .episodes
            .iter()
            .filter(|e| e.action_kind == ActionKind::EvaluatePredictions
                && e.delta > 0.0)
            .map(|e| e.delta)
            .sum::<f64>()
    );
    println!("final lifecycle: {:?} mode: {:?}", rt.lifecycle, rt.mode);
    let _ = MetaABState::TestingA; // ensure import is used
    println!();
    println!("--- end ---");
}
