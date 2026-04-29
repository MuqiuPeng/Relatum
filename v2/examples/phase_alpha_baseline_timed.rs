//! Baseline timed run for Phase Alpha perf-regression diagnosis.
//!
//! Runs the same OQ #1 substrate for 2000 ticks WITHOUT any
//! intervention (no theory demote, no axiom retract). Times
//! per-100-tick chunks. Provides the control against which
//! `phase_alpha_axiom_demote_timed.log`'s Phase 2 timing can
//! be compared, to isolate retract-attributable overhead from
//! inherent forward_apply scaling.
//!
//! Captured to `logs/<date>_phase_alpha_baseline_timed.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    }, RSet,
};
use std::time::Instant;

const HORIZON: u64 = 2000;
use relatum_v2::test_substrates::oq1::build_long_stream;

fn main() {
    println!(
        "=== Phase Alpha — baseline timed run (HORIZON={}, 100-tick chunks, no intervention) ===",
        HORIZON
    );

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment =
        Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    let chunk = 100u64;
    let chunks = HORIZON / chunk;
    let total_start = Instant::now();
    for i in 0..chunks {
        let chunk_start = Instant::now();
        rt.run_bounded(chunk);
        let chunk_elapsed = chunk_start.elapsed();
        println!(
            "  chunk {:>2}/{}: tick={:>4} episodes={:>3} axioms={:>2} theories={} elapsed={:>6.2}s ({:>5.1}ms/tick)",
            i + 1,
            chunks,
            rt.tick,
            rt.memory.episodes.len(),
            rt.rset.axioms().len(),
            rt.rset.theories().len(),
            chunk_elapsed.as_secs_f64(),
            chunk_elapsed.as_secs_f64() * 1000.0 / chunk as f64,
        );
    }
    let total = total_start.elapsed();
    println!();
    println!(
        "Total: tick={} episodes={} axioms={} theories={} elapsed={:.2}s ({:.1}ms/tick avg)",
        rt.tick,
        rt.memory.episodes.len(),
        rt.rset.axioms().len(),
        rt.rset.theories().len(),
        total.as_secs_f64(),
        total.as_secs_f64() * 1000.0 / HORIZON as f64,
    );
    println!();
    println!("--- end ---");
}
