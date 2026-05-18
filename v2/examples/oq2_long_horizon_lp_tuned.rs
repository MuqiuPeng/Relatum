//! ADR 0080 LP-threshold-tuning verification (2026-05-11).
//!
//! Pre-tuning: this configuration (3000-tick OQ#2 with auto-mint)
//! hung at ~5-min monitor intervals because LP_WINDOW=30 +
//! LP_THRESHOLD=0.05 kept drive-driven dispatches firing for ~5min
//! after canonical-set saturation (~tick 250).
//!
//! Post-tuning (LP_WINDOW=10, LP_DRIVE_THRESHOLD=0.10): expected
//! to complete in well under 5 minutes because gates close after
//! ~10 zero-mint dispatches instead of ~30.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq2::build_oq2_stream,
    RSet,
};

const HORIZON_TICKS: u64 = 6000;
const SNAPSHOT_INTERVAL: u64 = 500;

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" OQ#2 long-horizon — ADR 0080 LP-threshold-tuned");
    println!(" HORIZON={} ticks  (pre-tuning hung at this size)", HORIZON_TICKS);
    println!("════════════════════════════════════════════════════════");
    println!();

    let t_start = std::time::Instant::now();
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_oq2_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    let mut tick: u64 = 0;
    while tick < HORIZON_TICKS {
        let next_target = (tick + SNAPSHOT_INTERVAL).min(HORIZON_TICKS);
        let step = next_target - tick;
        let t_step = std::time::Instant::now();
        rt.run_bounded(step);
        tick = next_target;

        let dp_count = rt.memory.policy_stats.action_counts
            .get(&ActionKind::DiscoverPatterns).copied().unwrap_or(0);
        let dp_pos = rt.memory.policy_stats.action_positive_delta_counts
            .get(&ActionKind::DiscoverPatterns).copied().unwrap_or(0);
        let prune_count = rt.memory.policy_stats.action_counts
            .get(&ActionKind::PruneLowValueObjects).copied().unwrap_or(0);
        println!(" tick={:>5} | rset={:>4} ax={:>2} ths={:>2} pats={:>2} eps={:>4} | DP={:>3}/{}({:>2}%) prune={:>2} | step {:.1}s",
                 tick,
                 rt.rset.iter().count(),
                 rt.rset.axioms().len(),
                 rt.rset.theories().len(),
                 rt.rset.patterns().len(),
                 rt.memory.episodes.len(),
                 dp_count, dp_pos,
                 if dp_count > 0 { dp_pos * 100 / dp_count } else { 0 },
                 prune_count,
                 t_step.elapsed().as_secs_f64());
        use std::io::Write;
        std::io::stdout().flush().ok();
    }

    let total_secs = t_start.elapsed().as_secs_f64();
    println!();
    println!(" Total wall-clock: {:.1}s ({:.1} min)", total_secs, total_secs / 60.0);
    println!(" Final state: rset={} ax={} ths={} pats={} eps={}",
             rt.rset.iter().count(),
             rt.rset.axioms().len(),
             rt.rset.theories().len(),
             rt.rset.patterns().len(),
             rt.memory.episodes.len());

    println!();
    println!("--- end ---");
}
