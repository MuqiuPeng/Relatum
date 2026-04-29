//! Phase H2.0 long-run at HORIZON=5000 (ADR 0063 step 3b α
//! follow-up empirical study).
//!
//! With α load-bearing, DriveMix mutations should now diverge
//! between mixes over time. This run captures fine-grained
//! mutation trajectories at 100-tick resolution to surface:
//!
//! - Do weights converge to a stable mix, or oscillate?
//! - Does the equal-weighted mix drift toward something
//!   resembling the hand-tuned baseline (validation that the
//!   feedback loop is selecting "good" weights), or somewhere
//!   else entirely?
//! - Does α trigger frequency change as weights drift?
//!
//! 5 regimes × 1000 ticks each, mixing all the substrate
//! families used in the 2000-tick OQ #1 experiment plus an
//! added regime E that re-injects axiomatic structure to
//! exercise prediction-error drive.
//!
//! Captured to `logs/<date>_phase_h2_0_long5k.log`.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, DriveABState, DriveMix,
        RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    RSet,
};
use relatum_v2::test_substrates::long5k::build_5k_stream;
use std::collections::HashMap;

const HORIZON: u64 = 5000;
const SNAPSHOT_EVERY: u64 = 100;

#[derive(Clone)]
struct Snap {
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
    normalized_signal: f64,
    cand_a_compression: f64,
    cand_a_pe: f64,
    cand_a_mt: f64,
    cand_b_compression: f64,
    cand_b_pe: f64,
    cand_b_mt: f64,
}

fn snap(rt: &AutonomousRuntime) -> Snap {
    Snap {
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
        normalized_signal: rt.normalized_drive_signal(),
        cand_a_compression: rt
            .drive_mix
            .candidate_a
            .get("compression")
            .copied()
            .unwrap_or(0.0),
        cand_a_pe: rt
            .drive_mix
            .candidate_a
            .get("prediction_error")
            .copied()
            .unwrap_or(0.0),
        cand_a_mt: rt
            .drive_mix
            .candidate_a
            .get("mode_thrash")
            .copied()
            .unwrap_or(0.0),
        cand_b_compression: rt
            .drive_mix
            .candidate_b
            .get("compression")
            .copied()
            .unwrap_or(0.0),
        cand_b_pe: rt
            .drive_mix
            .candidate_b
            .get("prediction_error")
            .copied()
            .unwrap_or(0.0),
        cand_b_mt: rt
            .drive_mix
            .candidate_b
            .get("mode_thrash")
            .copied()
            .unwrap_or(0.0),
    }
}

fn run(label: &str, dm: DriveMix) -> Vec<Snap> {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment =
        Box::new(SyntheticStreamEnvironment::new(build_5k_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.drive_mix = dm;
    println!();
    println!("=== run: {} (HORIZON={}) ===", label, HORIZON);
    println!(
        "{:>5} {:>6} {:>6} {:>5} {:>6} {:>4} {:>3} {:>5} {:>5} {:>5} {:>7} {:>7}   {:>5} {:>5} {:>5}   {:>5} {:>5} {:>5}",
        "tick", "epis", "ep", "comp", "pairs", "tri", "ab",
        "wC", "wPE", "wMT", "comb", "norm",
        "aC", "aPE", "aMT",
        "bC", "bPE", "bMT",
    );
    let mut snaps = vec![snap(&rt)];
    print_snap(&snaps[0]);
    while rt.tick < HORIZON {
        rt.run_bounded(SNAPSHOT_EVERY);
        snaps.push(snap(&rt));
        // Only print every 5th snapshot (every 500 ticks) to keep
        // output manageable; fine-grained data still in vec.
        if (rt.tick / SNAPSHOT_EVERY) % 5 == 0 {
            print_snap(snaps.last().unwrap());
        }
    }
    println!("(final)");
    print_snap(snaps.last().unwrap());
    snaps
}

fn print_snap(s: &Snap) {
    println!(
        "{:>5} {:>6} {:>6} {:>5} {:>6} {:>4} {:>3} {:>5.2} {:>5.2} {:>5.2} {:>7.3} {:>7.3}   {:>5.2} {:>5.2} {:>5.2}   {:>5.2} {:>5.2} {:>5.2}",
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
        s.normalized_signal,
        s.cand_a_compression,
        s.cand_a_pe,
        s.cand_a_mt,
        s.cand_b_compression,
        s.cand_b_pe,
        s.cand_b_mt,
    );
}

fn analyze(label: &str, snaps: &[Snap]) {
    println!();
    println!("=== {} analysis ===", label);
    let final_s = snaps.last().unwrap();
    let initial_s = &snaps[0];
    println!(
        "episodes: {} → {} ({} per 1000 ticks)",
        initial_s.episodes,
        final_s.episodes,
        final_s.episodes as f64 * 1000.0 / final_s.tick as f64,
    );
    println!(
        "EP attempts: {} → {} ({} per 1000 ticks)",
        initial_s.ep_attempts,
        final_s.ep_attempts,
        final_s.ep_attempts as f64 * 1000.0 / final_s.tick as f64,
    );
    println!(
        "composite attempts: {}",
        final_s.composite_attempts
    );
    println!(
        "pairs named (final): {} | triples named (final): {}",
        final_s.pairs_named, final_s.triples_named
    );

    // Mutation count: how many times candidate_a/b weight changed.
    let mut a_changes = 0;
    let mut b_changes = 0;
    for w in snaps.windows(2) {
        let prev = &w[0];
        let curr = &w[1];
        if (prev.cand_a_compression - curr.cand_a_compression).abs() > 1e-9
            || (prev.cand_a_pe - curr.cand_a_pe).abs() > 1e-9
            || (prev.cand_a_mt - curr.cand_a_mt).abs() > 1e-9
        {
            a_changes += 1;
        }
        if (prev.cand_b_compression - curr.cand_b_compression).abs() > 1e-9
            || (prev.cand_b_pe - curr.cand_b_pe).abs() > 1e-9
            || (prev.cand_b_mt - curr.cand_b_mt).abs() > 1e-9
        {
            b_changes += 1;
        }
    }
    println!("mutations: candidate_a {} | candidate_b {}", a_changes, b_changes);

    // α-fire estimate: count snapshots where normalized_signal < -2.0.
    let alpha_eligible_snaps = snaps
        .iter()
        .filter(|s| s.normalized_signal < -2.0)
        .count();
    println!(
        "α-eligible snapshots (sig < -2.0): {}/{}",
        alpha_eligible_snaps,
        snaps.len()
    );

    // Signal range.
    let min_sig = snaps
        .iter()
        .map(|s| s.normalized_signal)
        .fold(f64::INFINITY, f64::min);
    let max_sig = snaps
        .iter()
        .map(|s| s.normalized_signal)
        .fold(f64::NEG_INFINITY, f64::max);
    println!(
        "normalized signal range: {:.3} to {:.3}",
        min_sig, max_sig
    );

    // Final candidate weights summary.
    println!(
        "final candidate_a: c={:.3} pe={:.3} mt={:.3}",
        final_s.cand_a_compression,
        final_s.cand_a_pe,
        final_s.cand_a_mt,
    );
    println!(
        "final candidate_b: c={:.3} pe={:.3} mt={:.3}",
        final_s.cand_b_compression,
        final_s.cand_b_pe,
        final_s.cand_b_mt,
    );
}

fn equal_weighted_mix() -> DriveMix {
    let mut w: HashMap<String, f64> = HashMap::new();
    w.insert("compression".to_string(), 0.333);
    w.insert("prediction_error".to_string(), 0.333);
    w.insert("mode_thrash".to_string(), 0.333);
    DriveMix::with_weights(w)
}

fn main() {
    println!(
        "=== Phase H2.0 step 3b α — long-run HORIZON={} (5 regimes × 1000 ticks) ===",
        HORIZON
    );
    let h_snaps = run("hand_tuned", DriveMix::baseline());
    let e_snaps = run("equal_weighted", equal_weighted_mix());

    analyze("hand_tuned", &h_snaps);
    analyze("equal_weighted", &e_snaps);

    println!();
    println!("--- end ---");
}
