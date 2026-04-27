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
        AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet,
};
use std::time::Instant;

const HORIZON: u64 = 2000;

fn build_long_stream() -> Vec<(u64, Event)> {
    let mut schedule = Vec::new();
    let regime_a_phases: [&[&str]; 5] = [
        &["a1", "a2", "a3", "a4"],
        &["a5", "a6", "a7", "a8"],
        &["a9", "a10", "a11", "a12"],
        &["a13", "a14", "a15", "a16"],
        &["a17", "a18", "a19", "a20"],
    ];
    for (i, ns) in regime_a_phases.iter().enumerate() {
        let off = 1 + (i as u64) * 100;
        for k in 0..4 {
            schedule.push((off + k as u64, Event::AddEdge(R::new(ns[k], ns[k]))));
        }
        schedule.push((off + 7, Event::AddEdge(R::new(ns[0], ns[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(ns[0], ns[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(ns[0], ns[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(ns[1], ns[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(ns[2], ns[3]))));
    }
    let regime_b_phases: [(&[&str], &[&str]); 5] = [
        (&["bL1", "bL2"], &["bR1", "bR2", "bR3"]),
        (&["bL3", "bL4"], &["bR4", "bR5", "bR6"]),
        (&["bL5", "bL6"], &["bR7", "bR8", "bR9"]),
        (&["bL7", "bL8"], &["bR10", "bR11", "bR12"]),
        (&["bL9", "bL10"], &["bR13", "bR14", "bR15"]),
    ];
    for (i, (lefts, rights)) in regime_b_phases.iter().enumerate() {
        let off = 501 + (i as u64) * 100;
        let mut t = 0u64;
        for l in lefts.iter() {
            for r in rights.iter() {
                schedule.push((off + t, Event::AddEdge(R::new(*l, *r))));
                t += 2;
            }
        }
    }
    let regime_c_phases: [&[&[&str]]; 5] = [
        &[&["c_a1", "c_a2"], &["c_b1", "c_b2", "c_b3"]],
        &[&["c_a3", "c_a4"], &["c_b4", "c_b5", "c_b6"]],
        &[&["c_a5", "c_a6"], &["c_b7", "c_b8", "c_b9"]],
        &[&["c_a7", "c_a8"], &["c_b10", "c_b11", "c_b12"]],
        &[&["c_a9", "c_a10"], &["c_b13", "c_b14", "c_b15"]],
    ];
    for (i, classes) in regime_c_phases.iter().enumerate() {
        let off = 1001 + (i as u64) * 100;
        let mut t = 0u64;
        for cls in classes.iter() {
            for x in cls.iter() {
                for y in cls.iter() {
                    schedule.push((off + t, Event::AddEdge(R::new(*x, *y))));
                    t += 1;
                }
            }
        }
    }
    let regime_d_phases: [&[&str]; 5] = [
        &["d1", "d2", "d3", "d4"],
        &["d5", "d6", "d7", "d8"],
        &["d9", "d10", "d11", "d12"],
        &["d13", "d14", "d15", "d16"],
        &["d17", "d18", "d19", "d20"],
    ];
    for (i, ns) in regime_d_phases.iter().enumerate() {
        let off = 1501 + (i as u64) * 100;
        for k in 0..4 {
            schedule.push((off + k as u64, Event::AddEdge(R::new(ns[k], ns[k]))));
        }
        schedule.push((off + 5, Event::AddEdge(R::new(PATTERN_MARKER, ns[0]))));
        schedule.push((off + 6, Event::AddEdge(R::new(ns[0], ESTABLISHED_MARKER))));
        schedule.push((off + 7, Event::AddEdge(R::new(ns[0], ns[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(ns[0], ns[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(ns[0], ns[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(ns[1], ns[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(ns[2], ns[3]))));
    }
    schedule
}

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
