//! Phase D.6 — Engineered cross-precision-only signal substrate.
//!
//! D.5 found that on OQ#1, primary < cross holds for noise-family axioms
//! (0.11 vs 0.49). But both are below 0.5 — not a STRONG cross-only signal.
//!
//! D.6 looks for the harder pattern: cross ≥ 0.7 AND primary ≤ 0.3.
//! Such an axiom would be DEMOTED by primary alone but RESCUED by
//! cross-precision — a genuine arbitration scenario.
//!
//! Method: engineer a stream that DELIBERATELY OMITS some transitive
//! closures the axioms would predict. The runtime trains on this
//! sparse stream; transitivity-like axioms get LOW primary (predictions
//! not in stream). But substrate generation reproduces the closures
//! (dream phase auto-saturates). Cross-precision should be HIGH.
//!
//! Substrate: 4 small chains with random "missing transitive" gaps.
//! The runtime should still discover a transitivity-like axiom; it
//! just won't be confirmed often by ground.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, Event, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    R, RSet,
};
use std::collections::HashSet;

const TICKS: u64 = 1500;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xD06_C055_AAAA_BEEF;
const MIN_AXIOM_PREDICTIONS: u64 = 1; // lowered for sparse substrate

fn build_sparse_chain_stream() -> Vec<(u64, Event)> {
    let mut s = Vec::new();
    let mut t: u64 = 1;
    // 15 phases of "almost-transitive chains with missing closures"
    // Each chain has 6 nodes; we add direct edges + SOME transitive closures
    // but DELIBERATELY OMIT certain transitive closures.
    for phase in 0..15 {
        let nodes: [String; 6] = std::array::from_fn(|i| format!("p{}_{}", phase, i));
        // Direct chain edges
        for i in 0..5 {
            s.push((t, Event::AddEdge(R::new(&nodes[i][..], &nodes[i+1][..]))));
            t += 1;
        }
        // Add half the transitive closures (not all): vary by phase mod 2
        if phase % 2 == 0 {
            s.push((t, Event::AddEdge(R::new(&nodes[0][..], &nodes[2][..])))); t += 1;
            s.push((t, Event::AddEdge(R::new(&nodes[1][..], &nodes[3][..])))); t += 1;
            // OMIT: R(0,3), R(0,4), R(0,5), R(1,4), R(1,5), R(2,4), R(2,5), R(3,5)
        } else {
            s.push((t, Event::AddEdge(R::new(&nodes[2][..], &nodes[4][..])))); t += 1;
            s.push((t, Event::AddEdge(R::new(&nodes[3][..], &nodes[5][..])))); t += 1;
            // OMIT: most other transitive closures
        }
        // Self-loops for some nodes (not all)
        for (i, n) in nodes.iter().enumerate() {
            if i % 2 == 0 {
                s.push((t, Event::AddEdge(R::new(&n[..], &n[..]))));
                t += 1;
            }
        }
        // Time gap between phases
        t += 50;
    }
    s
}

fn main() {
    println!("=== Phase D.6 — Engineered cross-only signal substrate ===");

    let stream = build_sparse_chain_stream();
    println!();
    println!("stream events: {}", stream.len());

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS);

    println!();
    println!("[trained] axioms={}, theories={}",
        rt.rset.axioms().len(), rt.rset.theories().len());

    // Build per-theory substrates
    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    if theories.is_empty() {
        println!("  → NULL — no theories discovered on this substrate");
        return;
    }

    let all_axiom_ids: HashSet<String> = rt.rset.axioms().into_iter().map(str::to_owned).collect();
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut g = match rt.rset.generate_substrate_from_theory(t, NUM_GEN_IDS, SEED_DENSITY, seed) {
            Ok(g) => g, Err(_) => continue,
        };
        for ax in &all_axiom_ids { g.register_axiom_with_intension(ax); }
        substrates.push(g);
    }

    // Per-axiom (primary, cross)
    let mut axiom_data: Vec<(String, f64, f64)> = Vec::new();
    for ax in &all_axiom_ids {
        let primary = match rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS) {
            Some(r) => r, None => continue,
        };
        let cross = match rt.rset.axiom_cross_precision(ax, &substrates) {
            Some(r) => r, None => continue,
        };
        axiom_data.push((ax.clone(), primary, cross));
    }

    // Look for cross-only signal (cross HIGH, primary LOW)
    println!();
    println!("=== Per-axiom (primary, cross) ===");
    println!("{:<35} {:>10} {:>10} {:>10} {:>15}",
             "axiom", "primary", "cross", "delta", "cross-only?");
    let mut cross_only: Vec<(String, f64, f64)> = Vec::new();
    for (ax, primary, cross) in &axiom_data {
        let delta = cross - primary;
        let is_cross_only = *cross >= 0.7 && *primary <= 0.3;
        let star = if is_cross_only { "★ STRONG CROSS-ONLY" } else { "" };
        println!("{:<35} {:>10.4} {:>10.4} {:>10.4} {}",
                 ax, primary, cross, delta, star);
        if is_cross_only {
            cross_only.push((ax.clone(), *primary, *cross));
        }
    }

    println!();
    println!("=== Verdict ===");
    if !cross_only.is_empty() {
        println!("  POSITIVE — engineered substrate produced {} cross-only axiom(s)", cross_only.len());
        for (ax, p, c) in &cross_only {
            println!("    {}: primary={:.4}, cross={:.4} — primary alone would demote, cross rescues",
                     ax, p, c);
        }
        println!();
        println!("  Composite signal arbitration is genuinely valuable: cross alone keeps these,");
        println!("  primary alone discards them. The composite preserves them at moderate magnitude.");
    } else {
        // Look for the next-best pattern (cross > 0.5 AND primary < 0.5)
        let weaker: Vec<&(String, f64, f64)> = axiom_data.iter()
            .filter(|(_, p, c)| *c > 0.5 && *p < 0.5).collect();
        if !weaker.is_empty() {
            println!("  PARTIAL — no strong cross-only axiom (cross ≥ 0.7 AND primary ≤ 0.3)");
            println!("  but {} axiom(s) show weaker pattern (cross > 0.5 AND primary < 0.5):", weaker.len());
            for (ax, p, c) in &weaker {
                println!("    {}: primary={:.4}, cross={:.4}", ax, p, c);
            }
        } else {
            println!("  NULL — engineered substrate did not produce the cross-only pattern.");
            println!("  primary and cross may be more correlated than D.5 suggested.");
        }
    }
    println!();
    println!("--- end ---");
}
