//! Phase D.5 — Engineered (or naturally occurring) disagreement substrate.
//!
//! D.3.1 found that primary-rate and cross-precision rank theories the
//! same way on OQ#1 and on the engineered narrow_a substrate. Conjecture:
//! signals correlate strongly because they measure the same property
//! from different angles.
//!
//! D.5 takes a different approach: rather than engineer a stream from
//! scratch (D.3.1's path), look at PER-AXIOM disagreement on OQ#1.
//! Beta-1 already identified shape_premise_p0-0_p1-2 as a noise family
//! (low cross-precision, ~0.49). What's the primary rate of those
//! axioms? If primary rate is high, the noise family naturally
//! exhibits primary/cross disagreement.
//!
//! Method:
//!   1. Run OQ#1 to convergence (1000 ticks)
//!   2. For each axiom, compute (primary_rate, cross_precision)
//!   3. Plot the scatter; compute correlation
//!   4. Identify per-axiom disagreements
//!   5. If disagreements exist, the pair (primary, cross) IS arbitrating
//!      between two genuine signals at the axiom layer — even when they
//!      converge at the THEORY layer (D.3.1 / Alpha-3+++)

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    R, RSet,
};
use std::collections::HashSet;

const TICKS_PHASE_0: u64 = 1000;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
const DISAGREEMENT_THRESHOLD: f64 = 0.30; // |primary - cross| ≥ this

fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
    assert_eq!(xs.len(), ys.len());
    let n = xs.len() as f64;
    let mean_x: f64 = xs.iter().sum::<f64>() / n;
    let mean_y: f64 = ys.iter().sum::<f64>() / n;
    let mut num = 0.0;
    let mut den_x = 0.0;
    let mut den_y = 0.0;
    for i in 0..xs.len() {
        let dx = xs[i] - mean_x;
        let dy = ys[i] - mean_y;
        num += dx * dy;
        den_x += dx * dx;
        den_y += dy * dy;
    }
    if den_x == 0.0 || den_y == 0.0 { return 0.0; }
    num / (den_x.sqrt() * den_y.sqrt())
}

fn main() {
    println!("=== Phase D.5 — Per-axiom primary-rate vs cross-precision disagreement ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    rt.rset.discover_axiom_shape_families(2);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();

    let all_axiom_ids: HashSet<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();

    // Build substrates
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut gen = match rt.rset.generate_substrate_from_theory(
            t, NUM_GEN_IDS, SEED_DENSITY, seed,
        ) {
            Ok(g) => g, Err(_) => continue,
        };
        for ax in &all_axiom_ids { gen.register_axiom_with_intension(ax); }
        substrates.push(gen);
    }

    // Per-axiom (primary, cross) pairs
    let mut axiom_data: Vec<(String, f64, f64)> = Vec::new();
    let axioms: Vec<String> = rt.rset.axioms().into_iter().map(str::to_owned).collect();
    for ax in &axioms {
        let primary = match rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS) {
            Some(r) => r, None => continue,
        };
        let cross = match rt.rset.axiom_cross_precision(ax, &substrates) {
            Some(r) => r, None => continue,
        };
        axiom_data.push((ax.clone(), primary, cross));
    }

    // Print
    println!();
    println!("=== Per-axiom data ===");
    println!("{:<35} {:>10} {:>10} {:>10} {:>10}", "axiom", "primary", "cross", "diff", "family");
    let families: Vec<String> = rt.rset.axiom_shape_families().into_iter().map(str::to_owned).collect();
    let fam_membership: Vec<(String, HashSet<String>)> = families.iter()
        .map(|f| (f.clone(), rt.rset.shape_family_members(f)
            .into_iter().map(str::to_owned).collect()))
        .collect();

    let mut disagreements: Vec<(String, f64, f64)> = Vec::new();
    for (ax, primary, cross) in &axiom_data {
        let diff = (primary - cross).abs();
        let fam: Vec<&str> = fam_membership.iter()
            .filter(|(_, m)| m.contains(ax)).map(|(f, _)| f.as_str()).collect();
        let fam_str = if fam.is_empty() { "—".to_string() } else { fam.join(", ") };
        let star = if diff >= DISAGREEMENT_THRESHOLD { " *" } else { "  " };
        println!(
            "{:<35} {:>10.4} {:>10.4} {:>10.4} {:<} {}",
            ax, primary, cross, diff, fam_str, star,
        );
        if diff >= DISAGREEMENT_THRESHOLD {
            disagreements.push((ax.clone(), *primary, *cross));
        }
    }

    // Correlation
    let xs: Vec<f64> = axiom_data.iter().map(|(_, p, _)| *p).collect();
    let ys: Vec<f64> = axiom_data.iter().map(|(_, _, c)| *c).collect();
    let r = pearson(&xs, &ys);

    println!();
    println!("=== Correlation ===");
    println!("  Pearson r(primary, cross) over {} axioms: {:.4}", axiom_data.len(), r);

    println!();
    println!("=== Disagreements (|primary - cross| ≥ {}) ===", DISAGREEMENT_THRESHOLD);
    if disagreements.is_empty() {
        println!("  (none)");
    } else {
        for (ax, p, c) in &disagreements {
            println!("  {}: primary={:.4}, cross={:.4}", ax, p, c);
        }
    }

    println!();
    println!("=== Verdict ===");
    if !disagreements.is_empty() {
        println!(
            "  POSITIVE — {} axioms exhibit primary/cross disagreement on OQ#1.",
            disagreements.len(),
        );
        println!("  D.3.1's NULL was at the THEORY layer; D.5 finds disagreement at the AXIOM layer.");
        println!("  Interpretation: primary rate and cross-precision arbitrate per-axiom even when");
        println!("  theory-level rankings agree. Composite signal is meaningful at fine granularity.");
    } else if r > 0.95 {
        println!(
            "  NULL with high correlation r={:.4} — primary and cross are nearly identical.",
            r,
        );
    } else {
        println!(
            "  PARTIAL — correlation r={:.4} but no per-axiom disagreement crosses {} threshold.",
            r, DISAGREEMENT_THRESHOLD,
        );
    }
    println!();
    println!("--- end ---");
}
