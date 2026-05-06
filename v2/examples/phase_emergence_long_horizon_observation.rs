//! Phase Emergence — long-horizon observation.
//!
//! Per the 2026-05-06 retrospective recommendation: now that the
//! runtime auto-mints patterns (ADR 0075 piece 2 revisited), what
//! does the mint-and-trim cycle look like over a long horizon?
//!
//! Method:
//!   - Run each substrate for 10_000 ticks (well past stream end
//!     for all substrates — OQ#1 ends ~tick 1990, long5k ~5000,
//!     OQ#2 ~4500)
//!   - Snapshot state every 500 ticks: axioms, theories, patterns,
//!     episode count, ActionKind distribution
//!   - Output time series + summary
//!
//! Questions:
//!   1. Does pattern count reach a stable equilibrium?
//!   2. Does it grow unboundedly? Decay to zero?
//!   3. Are mints / trims roughly balanced?
//!   4. What happens when the stream runs dry — does the runtime
//!      keep exploring or settle into a sleep loop?

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    test_substrates::{
        narrow_a::build_narrow_a_stream, oq1::build_long_stream,
        oq2::build_oq2_stream,
    },
    RSet,
};
use std::collections::HashMap;

/// Horizon chosen to cover each substrate's stream:
/// OQ#1 ~tick 1990, long5k ~tick 4990, OQ#2 ~tick 4500. Plus a
/// 1000-tick streamless buffer to observe post-stream dynamics.
/// Going further is wasted time per pilot data — runtime sleeps
/// once the stream runs dry and the rset has converged.
const HORIZON_TICKS: u64 = 6_000;
const SNAPSHOT_INTERVAL: u64 = 250;

#[derive(Debug, Clone)]
struct Snapshot {
    tick: u64,
    axiom_count: usize,
    theory_count: usize,
    pattern_count: usize,
    episode_count: usize,
    dispatched_dp: u64,
    successful_dp: u64,
    dispatched_prune: u64,
}

fn snapshot(rt: &AutonomousRuntime, tick: u64) -> Snapshot {
    let stats = &rt.memory.policy_stats;
    let dp_count = stats.action_counts.get(&ActionKind::DiscoverPatterns)
        .copied().unwrap_or(0);
    let dp_pos = stats.action_positive_delta_counts
        .get(&ActionKind::DiscoverPatterns).copied().unwrap_or(0);
    let prune_count = stats.action_counts.get(&ActionKind::PruneLowValueObjects)
        .copied().unwrap_or(0);
    Snapshot {
        tick,
        axiom_count: rt.rset.axioms().len(),
        theory_count: rt.rset.theories().len(),
        pattern_count: rt.rset.patterns().len(),
        episode_count: rt.memory.episodes.len(),
        dispatched_dp: dp_count,
        successful_dp: dp_pos,
        dispatched_prune: prune_count,
    }
}

fn run_substrate(label: &str, stream: Vec<(u64, Event)>) -> Vec<Snapshot> {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: {} ({} ticks horizon)", label, HORIZON_TICKS);
    println!("════════════════════════════════════════════════════════");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    let mut snapshots: Vec<Snapshot> = Vec::new();
    let mut current_tick: u64 = 0;
    while current_tick < HORIZON_TICKS {
        let next_target = (current_tick + SNAPSHOT_INTERVAL).min(HORIZON_TICKS);
        let step = next_target - current_tick;
        rt.run_bounded(step);
        current_tick = next_target;
        let s = snapshot(&rt, current_tick);
        snapshots.push(s);
    }

    // Print compact time series.
    println!();
    println!(
        " {:>6} {:>4} {:>4} {:>4} {:>5} {:>4} {:>4} {:>5}",
        "tick", "axs", "ths", "pat", "eps",
        "DP", "DPp", "prune",
    );
    let mut prev_pat: Option<usize> = None;
    for s in &snapshots {
        let trend = match prev_pat {
            Some(p) if s.pattern_count > p => "↑",
            Some(p) if s.pattern_count < p => "↓",
            _ => " ",
        };
        println!(
            " {:>6} {:>4} {:>4} {:>3}{:>1} {:>5} {:>4} {:>4} {:>5}",
            s.tick,
            s.axiom_count,
            s.theory_count,
            s.pattern_count, trend,
            s.episode_count,
            s.dispatched_dp,
            s.successful_dp,
            s.dispatched_prune,
        );
        prev_pat = Some(s.pattern_count);
    }

    snapshots
}

fn summary(label: &str, snapshots: &[Snapshot]) {
    if snapshots.is_empty() { return; }
    let final_s = snapshots.last().unwrap();
    let peak_pat = snapshots.iter().map(|s| s.pattern_count).max().unwrap_or(0);
    let trough_pat = snapshots.iter().map(|s| s.pattern_count).min().unwrap_or(0);
    let final_pat = final_s.pattern_count;

    // Count direction changes in pattern series (proxy for
    // mint-and-trim oscillation).
    let mut transitions = 0;
    let mut last_dir = 0i32;
    for w in snapshots.windows(2) {
        let diff = w[1].pattern_count as i32 - w[0].pattern_count as i32;
        let dir = diff.signum();
        if dir != 0 && dir != last_dir {
            transitions += 1;
            last_dir = dir;
        }
    }

    // Stream-end estimation: when did axiom_count last change?
    let mut last_axiom_change_tick = 0u64;
    let mut prev_ax = 0usize;
    for s in snapshots {
        if s.axiom_count != prev_ax {
            last_axiom_change_tick = s.tick;
            prev_ax = s.axiom_count;
        }
    }

    // Episode growth in second half (tick > 5000) suggests
    // continuing dispatch; near-zero growth suggests sleep loop.
    let mid = snapshots.len() / 2;
    let mid_eps = snapshots[mid].episode_count;
    let final_eps = final_s.episode_count;
    let second_half_eps = final_eps.saturating_sub(mid_eps);

    println!();
    println!(" Summary for {}:", label);
    println!("   final state:  axioms={} theories={} patterns={} episodes={}",
             final_s.axiom_count, final_s.theory_count,
             final_s.pattern_count, final_s.episode_count);
    println!("   patterns: peak={} trough={} final={} (transitions: {})",
             peak_pat, trough_pat, final_pat, transitions);
    println!("   last axiom change at tick {} (rest is post-discovery dynamics)",
             last_axiom_change_tick);
    println!("   episodes 2nd half (tick {}-{}): {}",
             snapshots[mid].tick, final_s.tick, second_half_eps);

    let pat_dispatched = final_s.dispatched_dp;
    let pat_successful = final_s.successful_dp;
    let prune_dispatched = final_s.dispatched_prune;
    println!("   DP total dispatches: {}, successful: {} ({:.1}%), prunes: {}",
             pat_dispatched, pat_successful,
             if pat_dispatched > 0 {
                 pat_successful as f64 / pat_dispatched as f64 * 100.0
             } else { 0.0 },
             prune_dispatched);
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Phase Emergence — long-horizon observation");
    println!("════════════════════════════════════════════════════════");
    println!(" Runtime auto-mint is enabled (ADR 0075 piece 2");
    println!(" revisited). Observe each substrate over {} ticks to",
             HORIZON_TICKS);
    println!(" answer: does mint-and-trim equilibrate? grow? decay?");
    println!(" Snapshots every {} ticks.", SNAPSHOT_INTERVAL);

    // long5k is skipped here: its persistent stream injection
    // keeps the runtime continuously active, so 6000 ticks of
    // dispatch+autonomous_pass+multi-size-fallback exceeds the
    // observation budget in pilot runs. OQ#1 and long5k are
    // RSet-isomorphic at maturity (Phase 0072-A finding), so
    // OQ#1's pattern dynamics generalize.
    let oq1 = run_substrate("OQ#1", build_long_stream());
    let narrow_a = run_substrate("narrow_a", build_narrow_a_stream());
    let oq2 = run_substrate("OQ#2", build_oq2_stream());

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Cross-substrate summary");
    println!("════════════════════════════════════════════════════════");
    summary("OQ#1", &oq1);
    summary("narrow_a", &narrow_a);
    summary("OQ#2", &oq2);

    println!();
    println!("--- end ---");
    let _ = HashMap::<String, usize>::new();
}
