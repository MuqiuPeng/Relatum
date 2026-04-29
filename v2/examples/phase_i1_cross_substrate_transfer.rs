//! Phase I.1 — Cross-substrate theory transfer.
//!
//! C.2 showed independently-trained runtimes on OQ#1 and long5k converge
//! to the same shape families. I.1 asks the stronger question: does a
//! TRAINED state TRANSFER without re-derivation?
//!
//! Method:
//!   1. Train rt_oq1 on OQ#1 (1000 ticks) → axioms_OQ1, substrates_OQ1
//!   2. Train rt_l5k on long5k (1500 ticks) → axioms_L5K, substrates_L5K
//!   3. For each axiom from rt_oq1: compute cross-precision on long5k's substrates
//!      (i.e., apply axioms learned from OQ#1 to long5k-imagined data)
//!   4. Compare to within-substrate cross-precision baseline
//!
//! Expected outcomes:
//!   - HIGH transfer (within-substrate ≈ cross-substrate cross-precision):
//!     trained state is portable. Strong claim.
//!   - LOW transfer (cross-substrate ≪ within-substrate): training is
//!     substrate-specific. Theories are tuned to their training substrate.
//!   - PARTIAL transfer: signal axioms transfer (universal), noise axioms
//!     don't.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::{long5k::build_5k_stream, oq1::build_long_stream as oq1_stream},
    R, RSet,
};
use std::collections::HashSet;

const TICKS_OQ1: u64 = 1000;
const TICKS_L5K: u64 = 1500;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;

fn build_substrates(rt: &AutonomousRuntime, theories: &[String]) -> Vec<RSet> {
    let all_axiom_ids: HashSet<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();
    let mut subs: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut g = match rt.rset.generate_substrate_from_theory(
            t, NUM_GEN_IDS, SEED_DENSITY, seed,
        ) {
            Ok(g) => g, Err(_) => continue,
        };
        for ax in &all_axiom_ids { g.register_axiom_with_intension(ax); }
        subs.push(g);
    }
    subs
}

fn axiom_cross_precision_on_subs(
    axiom_id: &str,
    substrates: &[RSet],
) -> Option<f64> {
    let mut sum = 0.0;
    let mut count = 0;
    for sub in substrates {
        let predicted: HashSet<R> = sub.forward_apply_axiom(axiom_id);
        if predicted.is_empty() { continue; }
        let actual: HashSet<R> = sub.iter().cloned().collect();
        let inter = predicted.iter().filter(|r| actual.contains(*r)).count();
        sum += inter as f64 / predicted.len() as f64;
        count += 1;
    }
    if count == 0 { None } else { Some(sum / count as f64) }
}

fn run_substrate(name: &str, ticks: u64, stream: Vec<(u64, relatum_v2::runtime::Event)>) -> AutonomousRuntime {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);
    println!("[{}] axioms={} theories={}",
        name, rt.rset.axioms().len(), rt.rset.theories().len());
    rt
}

fn main() {
    println!("=== Phase I.1 — Cross-substrate theory transfer ===");

    // ---- Train both runtimes ----
    println!();
    println!("--- Training ---");
    let rt_oq1 = run_substrate("OQ#1", TICKS_OQ1, oq1_stream());
    let rt_l5k = run_substrate("long5k", TICKS_L5K, build_5k_stream());

    // CRITICAL: rather than re-using `generate_substrate_from_theory`
    // (which produces IDENTICAL substrates for both runtimes when they
    // converge to the same theories — making the "transfer" test
    // degenerate), we use each runtime's TRAINED RSET DIRECTLY as the
    // evaluation substrate. This is a genuine cross-substrate test:
    // axioms from RT_A get applied to RT_B's actual data graph.

    // To compare apples-to-apples, register every axiom in BOTH rsets
    // (so forward_apply has the intension on both sides).
    let oq1_axioms: HashSet<String> = rt_oq1.rset.axioms().into_iter().map(str::to_owned).collect();
    let l5k_axioms: HashSet<String> = rt_l5k.rset.axioms().into_iter().map(str::to_owned).collect();

    let mut rset_oq1 = rt_oq1.rset.clone();
    let mut rset_l5k = rt_l5k.rset.clone();
    for ax in &l5k_axioms {
        rset_oq1.register_axiom_with_intension(ax);
    }
    for ax in &oq1_axioms {
        rset_l5k.register_axiom_with_intension(ax);
    }

    fn axiom_precision_on_rset(axiom_id: &str, rset: &RSet) -> Option<f64> {
        let predicted: HashSet<R> = rset.forward_apply_axiom(axiom_id);
        if predicted.is_empty() { return None; }
        let actual: HashSet<R> = rset.iter().cloned().collect();
        let inter = predicted.iter().filter(|r| actual.contains(*r)).count();
        Some(inter as f64 / predicted.len() as f64)
    }

    // ---- Per-axiom precision on each RSet ----
    println!();
    println!("=== OQ#1 axioms: precision on OQ#1 rset (within) vs long5k rset (transfer) ===");
    println!("{:<35} {:>12} {:>12} {:>12}", "axiom", "within", "across", "delta");
    let mut within_oq: Vec<f64> = Vec::new();
    let mut across_oq: Vec<f64> = Vec::new();
    for ax in &oq1_axioms {
        let w = axiom_precision_on_rset(ax, &rset_oq1);
        let a = axiom_precision_on_rset(ax, &rset_l5k);
        if let (Some(w), Some(a)) = (w, a) {
            println!("{:<35} {:>12.4} {:>12.4} {:>12.4}", ax, w, a, a - w);
            within_oq.push(w);
            across_oq.push(a);
        }
    }
    let common = within_oq.len();
    let mean_within_oq = if common > 0 {
        within_oq.iter().sum::<f64>() / common as f64 } else { 0.0 };
    let mean_across_oq = if common > 0 {
        across_oq.iter().sum::<f64>() / common as f64 } else { 0.0 };

    println!();
    println!("=== long5k axioms: precision on long5k rset (within) vs OQ#1 rset (transfer) ===");
    println!("{:<35} {:>12} {:>12} {:>12}", "axiom", "within", "across", "delta");
    let mut within_l5: Vec<f64> = Vec::new();
    let mut across_l5: Vec<f64> = Vec::new();
    for ax in &l5k_axioms {
        let w = axiom_precision_on_rset(ax, &rset_l5k);
        let a = axiom_precision_on_rset(ax, &rset_oq1);
        if let (Some(w), Some(a)) = (w, a) {
            println!("{:<35} {:>12.4} {:>12.4} {:>12.4}", ax, w, a, a - w);
            within_l5.push(w);
            across_l5.push(a);
        }
    }
    let common2 = within_l5.len();
    let mean_within_l5 = if common2 > 0 {
        within_l5.iter().sum::<f64>() / common2 as f64 } else { 0.0 };
    let mean_across_l5 = if common2 > 0 {
        across_l5.iter().sum::<f64>() / common2 as f64 } else { 0.0 };

    // ---- Aggregates ----
    println!();
    println!("=== Aggregate transfer ===");
    println!("  OQ#1 axioms on OQ#1 rset (within):   mean = {:.4}", mean_within_oq);
    println!("  OQ#1 axioms on long5k rset (across): mean = {:.4}", mean_across_oq);
    println!("    ratio = {:.4}",
             if mean_within_oq > 0.0 { mean_across_oq / mean_within_oq } else { 0.0 });
    println!();
    println!("  long5k axioms on long5k rset (within):   mean = {:.4}", mean_within_l5);
    println!("  long5k axioms on OQ#1 rset (across):     mean = {:.4}", mean_across_l5);
    println!("    ratio = {:.4}",
             if mean_within_l5 > 0.0 { mean_across_l5 / mean_within_l5 } else { 0.0 });

    println!();
    println!("=== Verdict ===");
    let ratio_oq = if mean_within_oq > 0.0 { mean_across_oq / mean_within_oq } else { 0.0 };
    let ratio_l5 = if mean_within_l5 > 0.0 { mean_across_l5 / mean_within_l5 } else { 0.0 };
    let avg_ratio = (ratio_oq + ratio_l5) / 2.0;
    if avg_ratio >= 0.90 {
        println!(
            "  STRONG TRANSFER (avg ratio {:.4}) — axioms transfer cleanly to the other rset",
            avg_ratio,
        );
    } else if avg_ratio >= 0.70 {
        println!(
            "  PARTIAL TRANSFER (avg ratio {:.4}) — some quality lost cross-rset",
            avg_ratio,
        );
    } else {
        println!(
            "  WEAK TRANSFER (avg ratio {:.4}) — axioms are tuned to their training rset",
            avg_ratio,
        );
    }
    println!();
    println!("--- end ---");
}
