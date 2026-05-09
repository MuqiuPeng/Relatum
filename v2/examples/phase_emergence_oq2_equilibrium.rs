//! Focused long-horizon observation on OQ#2, post-ADR 0079.
//!
//! OQ#2 is the only canonical substrate where ADR 0079's
//! drive→scheduler integration changes behaviour (OQ#1-clade's
//! drive is silent at maturity). This example zooms in on OQ#2
//! and runs 15000 ticks (~3x past stream end at tick 4209) to
//! observe the long-tail mint-and-trim equilibrium.
//!
//! Snapshots track per-class delta to surface oscillation if
//! any: pattern adds / pattern retracts / drive activity.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, LifecycleState, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    test_substrates::oq2::build_oq2_stream,
    RSet,
};

const HORIZON_TICKS: u64 = 800;
const SNAPSHOT_INTERVAL: u64 = 50;

#[derive(Debug, Clone)]
struct Snapshot {
    tick: u64,
    axioms: usize,
    theories: usize,
    patterns: usize,
    pattern_instances: usize,
    episodes: usize,
    dp_count: u64,
    dp_pos: u64,
    prune_count: u64,
    drive_unexplained: usize,
    drive_buckets: usize,
    wake_count: usize,
}

fn snapshot(rt: &AutonomousRuntime, tick: u64) -> Snapshot {
    let drive = rt.rset.unexplained_drive_signal();
    let stats = &rt.memory.policy_stats;
    let total_pat_inst: usize = rt
        .rset
        .patterns()
        .iter()
        .map(|p| rt.rset.instances_of(p).len())
        .sum();
    let wake_count = rt
        .memory
        .lifecycle_transitions
        .iter()
        .filter(|t| {
            matches!(t.from, LifecycleState::Sleeping)
                && matches!(t.to, LifecycleState::Running)
        })
        .count();
    Snapshot {
        tick,
        axioms: rt.rset.axioms().len(),
        theories: rt.rset.theories().len(),
        patterns: rt.rset.patterns().len(),
        pattern_instances: total_pat_inst,
        episodes: rt.memory.episodes.len(),
        dp_count: stats.action_counts
            .get(&ActionKind::DiscoverPatterns).copied().unwrap_or(0),
        dp_pos: stats.action_positive_delta_counts
            .get(&ActionKind::DiscoverPatterns).copied().unwrap_or(0),
        prune_count: stats.action_counts
            .get(&ActionKind::PruneLowValueObjects).copied().unwrap_or(0),
        drive_unexplained: drive.unexplained_count,
        drive_buckets: drive.distinct_canonicals,
        wake_count,
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" OQ#2 long-horizon equilibrium observation (post ADR 0079)");
    println!("════════════════════════════════════════════════════════");
    println!(" Horizon: {} ticks | snapshots every {}",
             HORIZON_TICKS, SNAPSHOT_INTERVAL);

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(
        SyntheticStreamEnvironment::new(build_oq2_stream())
    );
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    let mut snaps: Vec<Snapshot> = Vec::new();
    let mut current_tick: u64 = 0;
    while current_tick < HORIZON_TICKS {
        let next_target = (current_tick + SNAPSHOT_INTERVAL).min(HORIZON_TICKS);
        let step = next_target - current_tick;
        rt.run_bounded(step);
        current_tick = next_target;
        snaps.push(snapshot(&rt, current_tick));
    }

    println!();
    println!(
        " {:>6} {:>4} {:>4} {:>4} {:>9} {:>5} {:>5} {:>4} {:>5} {:>9} {:>4} {:>4}",
        "tick", "axs", "ths", "pat", "pat_ins", "eps",
        "DP", "DPp", "prune", "drv_unex", "buc", "wake",
    );
    let mut prev_eps = 0usize;
    let mut prev_pat = 0usize;
    let mut prev_pat_inst = 0usize;
    for s in &snaps {
        let eps_d = s.episodes as i64 - prev_eps as i64;
        let pat_d = s.patterns as i64 - prev_pat as i64;
        let inst_d = s.pattern_instances as i64 - prev_pat_inst as i64;
        prev_eps = s.episodes;
        prev_pat = s.patterns;
        prev_pat_inst = s.pattern_instances;
        let pat_str = format!("{}{}", s.patterns,
            match pat_d.signum() { 1 => "↑", -1 => "↓", _ => " " });
        let inst_str = format!("{}({:+})", s.pattern_instances, inst_d);
        let eps_str = format!("{}({:+})", s.episodes, eps_d);
        println!(
            " {:>6} {:>4} {:>4} {:>5} {:>9} {:>9} {:>5} {:>4} {:>5} {:>9} {:>4} {:>4}",
            s.tick,
            s.axioms,
            s.theories,
            pat_str,
            inst_str,
            eps_str,
            s.dp_count,
            s.dp_pos,
            s.prune_count,
            s.drive_unexplained,
            s.drive_buckets,
            s.wake_count,
        );
    }

    let final_s = snaps.last().unwrap();
    let mid_s = &snaps[snaps.len() / 2];
    println!();
    println!(" Phases of activity:");
    println!("   first half (ticks 0-{}): episodes +{}, pat_instances +{}",
             mid_s.tick, mid_s.episodes,
             mid_s.pattern_instances);
    println!(
        "   second half (ticks {}-{}): episodes +{}, pat_instances +{}",
        mid_s.tick, final_s.tick,
        final_s.episodes - mid_s.episodes,
        final_s.pattern_instances as i64 - mid_s.pattern_instances as i64,
    );

    // Detect oscillation: pattern_count direction changes.
    let mut oscillations = 0;
    let mut last_dir = 0i32;
    for w in snaps.windows(2) {
        let d = w[1].patterns as i32 - w[0].patterns as i32;
        if d != 0 && d.signum() != last_dir {
            oscillations += 1;
            last_dir = d.signum();
        }
    }
    println!("   pattern_count direction changes: {}", oscillations);
    println!("   peak patterns: {}",
             snaps.iter().map(|s| s.patterns).max().unwrap_or(0));
    println!("   final patterns: {}", final_s.patterns);

    println!();
    println!("--- end ---");
}
