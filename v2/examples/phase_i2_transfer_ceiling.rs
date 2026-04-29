//! Phase I.2 — Transfer ceiling test.
//!
//! I.1 showed strong transfer between OQ#1 and long5k (same regime types).
//! C.2.1 predicted catastrophic transfer failure to OQ#2 (tournament +
//! lattice + star — fundamentally different regime structure).
//!
//! I.2 actually runs that test:
//!   1. Train rt_oq1 on OQ#1 (1000 ticks) → axioms_OQ1
//!   2. Build raw OQ#2 RSet via stream replay (no training)
//!   3. For each axiom in axioms_OQ1, compute precision on OQ#2 rset
//!   4. Compare to rt_oq1's within-substrate precision
//!
//! Expected: catastrophic precision drop. OQ#2 violates transitivity
//! (per C.2.1's tournament structure). Axioms trained on OQ#1's
//! transitive regimes shouldn't predict OQ#2 well.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, Event, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::{oq1::build_long_stream as oq1_stream, oq2::build_oq2_stream},
    R, RSet,
};
use std::collections::HashSet;

const TICKS_OQ1: u64 = 1000;
const TICKS_OQ2: u64 = 4500;

/// Replay a stream into an RSet via direct edge add (no scheduler/runtime).
fn replay_stream_to_rset(stream: &[(u64, Event)]) -> RSet {
    let mut rset = RSet::new();
    for (_, evt) in stream {
        if let Event::AddEdge(r) = evt {
            rset.add(r.clone());
        }
    }
    rset
}

fn axiom_precision(axiom_id: &str, rset: &RSet) -> Option<f64> {
    let predicted: HashSet<R> = rset.forward_apply_axiom(axiom_id);
    if predicted.is_empty() { return None; }
    let actual: HashSet<R> = rset.iter().cloned().collect();
    let inter = predicted.iter().filter(|r| actual.contains(*r)).count();
    Some(inter as f64 / predicted.len() as f64)
}

fn main() {
    println!("=== Phase I.2 — Transfer ceiling test (OQ#1 axioms → OQ#2 rset) ===");

    // ---- Train on OQ#1 ----
    let mut rt_oq1 = AutonomousRuntime::new(RSet::new());
    rt_oq1.environment = Box::new(SyntheticStreamEnvironment::new(oq1_stream()));
    rt_oq1.scheduler = Box::new(RuleBasedScheduler::default());
    rt_oq1.run_bounded(TICKS_OQ1);

    let oq1_axioms: HashSet<String> =
        rt_oq1.rset.axioms().into_iter().map(str::to_owned).collect();
    println!();
    println!("[OQ#1] trained: {} axioms, {} theories",
             oq1_axioms.len(), rt_oq1.rset.theories().len());

    // ---- Build raw OQ#2 rset ----
    let oq2_stream = build_oq2_stream();
    let oq2_total_events = oq2_stream.len();
    println!();
    println!("[OQ#2] stream events: {} (will replay {} ticks)",
             oq2_total_events, TICKS_OQ2);
    let mut rset_oq2 = replay_stream_to_rset(&oq2_stream);
    println!("[OQ#2] raw rset: {} edges", rset_oq2.len());

    // Register OQ#1 axioms on OQ#2 rset (for forward_apply intension)
    for ax in &oq1_axioms {
        rset_oq2.register_axiom_with_intension(ax);
    }

    // ---- Per-axiom precision: within (OQ#1 rset) vs across (OQ#2 rset) ----
    println!();
    println!("=== Per-axiom transfer ===");
    println!("{:<35} {:>10} {:>10} {:>10}", "axiom", "OQ#1", "OQ#2", "delta");
    let mut within_vals: Vec<f64> = Vec::new();
    let mut across_vals: Vec<f64> = Vec::new();
    let mut catastrophic_drops = 0;
    for ax in &oq1_axioms {
        let w = axiom_precision(ax, &rt_oq1.rset);
        let a = axiom_precision(ax, &rset_oq2);
        match (w, a) {
            (Some(w), Some(a)) => {
                println!("{:<35} {:>10.4} {:>10.4} {:>10.4}", ax, w, a, a - w);
                within_vals.push(w);
                across_vals.push(a);
                if a < 0.5 * w && w > 0.3 { catastrophic_drops += 1; }
            }
            (Some(w), None) => {
                println!("{:<35} {:>10.4} {:>10} {:>10}", ax, w, "(no pred)", "—");
                within_vals.push(w);
                across_vals.push(0.0);
                if w > 0.3 { catastrophic_drops += 1; }
            }
            _ => {
                println!("{:<35} {:>10} {:>10} {:>10}", ax, "—", "—", "—");
            }
        }
    }

    let mean_within = if !within_vals.is_empty() {
        within_vals.iter().sum::<f64>() / within_vals.len() as f64 } else { 0.0 };
    let mean_across = if !across_vals.is_empty() {
        across_vals.iter().sum::<f64>() / across_vals.len() as f64 } else { 0.0 };
    let ratio = if mean_within > 0.0 { mean_across / mean_within } else { 0.0 };

    println!();
    println!("=== Aggregate ===");
    println!("  OQ#1 within precision (mean):  {:.4}", mean_within);
    println!("  OQ#2 across precision (mean):  {:.4}", mean_across);
    println!("  ratio:                         {:.4}", ratio);
    println!("  catastrophic-drop axioms:      {} / {} (precision halved or vanished)",
             catastrophic_drops, oq1_axioms.len());

    println!();
    println!("=== Verdict ===");
    if ratio < 0.5 {
        println!("  CONFIRMED CATASTROPHIC TRANSFER FAILURE — ratio {:.4}", ratio);
        println!("  Axioms trained on OQ#1's transitive regimes do not transfer to OQ#2's");
        println!("  tournament + lattice + star structure. C.2.1's prediction is empirically validated.");
    } else if ratio < 0.8 {
        println!("  PARTIAL TRANSFER — ratio {:.4}, some axioms generalize", ratio);
    } else {
        println!("  UNEXPECTED STRONG TRANSFER — ratio {:.4}", ratio);
    }
    println!();
    println!("--- end ---");
}
