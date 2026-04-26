//! Phase H2.0 OQ #1 experiment — hand-tuned vs equal-weighted
//! DriveMix initialization (ADR 0063 § Open questions, #1).
//!
//! Two runs over the same multi-regime substrate
//! (HORIZON=2000), differing only in DriveMix initial weights:
//!
//! - **hand-tuned** baseline: compression 0.5 / prediction_error
//!   0.4 / mode_thrash 0.1 (`DriveMix::baseline()`).
//! - **equal-weighted**: 0.333 / 0.333 / 0.333.
//!
//! Step 2's shadow-only property says episode behaviour should
//! be byte-identical regardless of DriveMix weights — this
//! experiment verifies that empirically AND captures the
//! combined-signal trajectory under each init so step 3b's
//! threshold calibration has data to work with.
//!
//! Captured to `logs/<date>_phase_h2_0_oq1_experiment.log`.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, DriveABState, DriveMix, Event,
        RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet,
};
use std::collections::HashMap;

const HORIZON: u64 = 2000;
const SNAPSHOT_EVERY: u64 = 200;

#[derive(Clone)]
struct OQ1Snapshot {
    tick: u64,
    episodes: usize,
    ep_attempts: u64,
    composite_attempts: u64,
    pairs_named: usize,
    triples_named: usize,
    drive_state: String,
    w_compression: f64,
    w_prediction_error: f64,
    w_mode_thrash: f64,
    combined_signal: f64,
}

fn snapshot(rt: &AutonomousRuntime) -> OQ1Snapshot {
    OQ1Snapshot {
        tick: rt.tick,
        episodes: rt.memory.episodes.len(),
        ep_attempts: rt
            .memory
            .policy_stats
            .action_counts
            .get(&ActionKind::EvaluatePredictions)
            .copied()
            .unwrap_or(0),
        composite_attempts: rt
            .memory
            .policy_stats
            .action_counts
            .get(&ActionKind::ExecuteComposite)
            .copied()
            .unwrap_or(0),
        pairs_named: rt.rset.action_sequence_pairs().len(),
        triples_named: rt.rset.action_sequence_triples().len(),
        drive_state: match rt.drive_mix.state {
            DriveABState::TestingA => "A".to_string(),
            DriveABState::TestingB => "B".to_string(),
        },
        w_compression: rt
            .drive_mix
            .active_weights()
            .get("compression")
            .copied()
            .unwrap_or(0.0),
        w_prediction_error: rt
            .drive_mix
            .active_weights()
            .get("prediction_error")
            .copied()
            .unwrap_or(0.0),
        w_mode_thrash: rt
            .drive_mix
            .active_weights()
            .get("mode_thrash")
            .copied()
            .unwrap_or(0.0),
        combined_signal: rt.combined_drive_signal(),
    }
}

fn print_header(label: &str) {
    println!();
    println!("=== run: {} ===", label);
    println!(
        "{:>5} {:>6} {:>6} {:>5} {:>6} {:>4} {:>3} {:>5} {:>5} {:>5} {:>7}",
        "tick",
        "epis",
        "ep",
        "comp",
        "pairs",
        "tri",
        "ab",
        "wC",
        "wPE",
        "wMT",
        "sig",
    );
}

fn print_row(s: &OQ1Snapshot) {
    println!(
        "{:>5} {:>6} {:>6} {:>5} {:>6} {:>4} {:>3} {:>5.2} {:>5.2} {:>5.2} {:>7.3}",
        s.tick,
        s.episodes,
        s.ep_attempts,
        s.composite_attempts,
        s.pairs_named,
        s.triples_named,
        s.drive_state,
        s.w_compression,
        s.w_prediction_error,
        s.w_mode_thrash,
        s.combined_signal,
    );
}

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
            schedule
                .push((off + k as u64, Event::AddEdge(R::new(ns[k], ns[k]))));
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
            schedule
                .push((off + k as u64, Event::AddEdge(R::new(ns[k], ns[k]))));
        }
        schedule.push((off + 5, Event::AddEdge(R::new(PATTERN_MARKER, ns[0]))));
        schedule
            .push((off + 6, Event::AddEdge(R::new(ns[0], ESTABLISHED_MARKER))));
        schedule.push((off + 7, Event::AddEdge(R::new(ns[0], ns[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(ns[0], ns[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(ns[0], ns[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(ns[1], ns[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(ns[2], ns[3]))));
    }

    schedule
}

fn build_runtime(drive_mix: DriveMix) -> AutonomousRuntime {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment =
        Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.drive_mix = drive_mix;
    rt
}

struct RunResult {
    label: &'static str,
    snapshots: Vec<OQ1Snapshot>,
    final_drive_mix_a: HashMap<String, f64>,
    final_drive_mix_b: HashMap<String, f64>,
}

fn run_with(
    label: &'static str,
    initial_dm: DriveMix,
) -> RunResult {
    let mut rt = build_runtime(initial_dm);
    print_header(label);
    let mut snapshots = vec![snapshot(&rt)];
    print_row(snapshots.last().unwrap());
    while rt.tick < HORIZON {
        rt.run_bounded(SNAPSHOT_EVERY);
        snapshots.push(snapshot(&rt));
        print_row(snapshots.last().unwrap());
    }
    RunResult {
        label,
        snapshots,
        final_drive_mix_a: rt.drive_mix.candidate_a.clone(),
        final_drive_mix_b: rt.drive_mix.candidate_b.clone(),
    }
}

fn equal_weighted_mix() -> DriveMix {
    let mut weights: HashMap<String, f64> = HashMap::new();
    weights.insert("compression".to_string(), 0.333);
    weights.insert("prediction_error".to_string(), 0.333);
    weights.insert("mode_thrash".to_string(), 0.333);
    DriveMix::with_weights(weights)
}

fn main() {
    println!(
        "=== ADR 0063 OQ #1 — hand-tuned vs equal-weighted DriveMix init (HORIZON={}) ===",
        HORIZON
    );

    let hand_tuned = run_with("hand_tuned", DriveMix::baseline());
    let equal = run_with("equal_weighted", equal_weighted_mix());

    println!();
    println!("=== final candidate weights ===");
    let print_weights = |label: &str, m: &HashMap<String, f64>| {
        println!("  {}:", label);
        let mut keys: Vec<&String> = m.keys().collect();
        keys.sort();
        for k in keys {
            println!("    {}: {:.4}", k, m.get(k).copied().unwrap_or(0.0));
        }
    };
    println!("hand_tuned:");
    print_weights("candidate_a", &hand_tuned.final_drive_mix_a);
    print_weights("candidate_b", &hand_tuned.final_drive_mix_b);
    println!("equal_weighted:");
    print_weights("candidate_a", &equal.final_drive_mix_a);
    print_weights("candidate_b", &equal.final_drive_mix_b);

    println!();
    println!("=== shadow-only verification ===");
    let final_h = hand_tuned.snapshots.last().unwrap();
    let final_e = equal.snapshots.last().unwrap();
    let same_episodes = final_h.episodes == final_e.episodes;
    let same_ep = final_h.ep_attempts == final_e.ep_attempts;
    let same_comp = final_h.composite_attempts == final_e.composite_attempts;
    let same_pairs = final_h.pairs_named == final_e.pairs_named;
    let same_tri = final_h.triples_named == final_e.triples_named;
    println!(
        "episodes match: {} (hand={}, equal={})",
        same_episodes, final_h.episodes, final_e.episodes
    );
    println!(
        "ep_attempts match: {} (hand={}, equal={})",
        same_ep, final_h.ep_attempts, final_e.ep_attempts
    );
    println!(
        "composite match: {} (hand={}, equal={})",
        same_comp, final_h.composite_attempts, final_e.composite_attempts
    );
    println!(
        "pairs match: {} (hand={}, equal={})",
        same_pairs, final_h.pairs_named, final_e.pairs_named
    );
    println!(
        "triples match: {} (hand={}, equal={})",
        same_tri, final_h.triples_named, final_e.triples_named
    );
    let all_match = same_episodes && same_ep && same_comp && same_pairs && same_tri;
    println!();
    if all_match {
        println!(
            "VERDICT: shadow-only property HOLDS — DriveMix weights\n          do not yet affect runtime behaviour."
        );
    } else {
        println!(
            "VERDICT: shadow-only property VIOLATED — investigate."
        );
    }

    println!();
    println!("=== combined_drive_signal trajectory comparison ===");
    println!(
        "{:>5} {:>10} {:>10} {:>10}",
        "tick", "hand", "equal", "delta"
    );
    for i in 0..hand_tuned.snapshots.len() {
        let h = &hand_tuned.snapshots[i];
        let e = &equal.snapshots[i];
        println!(
            "{:>5} {:>10.4} {:>10.4} {:>10.4}",
            h.tick,
            h.combined_signal,
            e.combined_signal,
            e.combined_signal - h.combined_signal,
        );
    }

    println!();
    println!("--- end ---");
}
