//! Phase F.1.1 — Per-axiom cross-precision in family discovery.
//!
//! F.1 shipped `axiom_cross_precision(ax, substrates)` — per-axiom
//! quality scalar. Family discovery (Beta-1) groups axioms by
//! structural shape only — ignoring quality.
//!
//! F.1.1 fuses them: for each shape family, summarize the quality
//! of its members. Adds a quality dimension on top of the structural
//! one. Useful downstream (B.2 family-level demote can choose
//! threshold by mean; A merge selector might prefer families with
//! low variance, etc.).
//!
//! Method:
//!   1. Run runtime to build axioms + theories
//!   2. Discover shape families
//!   3. Generate per-theory substrates (DreamCoder-style)
//!   4. For each family, compute per-member axiom_cross_precision
//!   5. Aggregate: mean, std, min, max
//!
//! Expected on OQ#1:
//!   - shape_premise_p0-0_p1-2 (4 noise axioms): low mean, near-zero variance
//!   - shape_premise_p0-1 (signal axioms): high mean
//!   - shape_premise_p0-1_p1-2 (signal axioms): high mean
//!   - shape_conclusion_*: mixed (per B.3)

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

fn stats(xs: &[f64]) -> (f64, f64, f64, f64) {
    if xs.is_empty() { return (0.0, 0.0, 0.0, 0.0); }
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    let variance = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64;
    let std = variance.sqrt();
    let min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (mean, std, min, max)
}

fn main() {
    println!("=== Phase F.1.1 — Per-axiom cross-precision in family discovery ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    rt.rset.discover_axiom_shape_families(2); // mints premise + conclusion families per B.3

    // Build substrates from each theory
    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();

    let all_axiom_ids: HashSet<String> = rt.rset.axioms().into_iter().map(str::to_owned).collect();

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
    println!();
    println!("substrates: {} (one per theory)", substrates.len());

    let families: Vec<String> = rt.rset.axiom_shape_families().into_iter().map(str::to_owned).collect();
    println!("families discovered: {}", families.len());

    // Per-family quality summary
    println!();
    println!("=== Family quality summary ===");
    println!(
        "{:<35} {:>6} {:>10} {:>10} {:>10} {:>10}",
        "family", "n_mem", "mean", "std", "min", "max",
    );
    let mut family_rows: Vec<(String, usize, f64, f64, f64, f64)> = Vec::new();
    for fam in &families {
        let members: Vec<String> = rt.rset.shape_family_members(fam)
            .into_iter().map(str::to_owned).collect();
        let xprecs: Vec<f64> = members.iter()
            .filter_map(|m| rt.rset.axiom_cross_precision(m, &substrates))
            .collect();
        let (mean, std, min, max) = stats(&xprecs);
        family_rows.push((fam.clone(), xprecs.len(), mean, std, min, max));
        println!(
            "{:<35} {:>6} {:>10.4} {:>10.4} {:>10.4} {:>10.4}",
            fam, xprecs.len(), mean, std, min, max,
        );
    }

    // Classify families by quality dimension
    println!();
    println!("=== Classification ===");
    println!("(thresholds: mean ≥ 0.80 = high; std < 0.05 = uniform)");
    let mut signal_families: Vec<&str> = Vec::new();
    let mut noise_families: Vec<&str> = Vec::new();
    let mut uniform_families: Vec<&str> = Vec::new();
    for (fam, _n, mean, std, _min, _max) in &family_rows {
        if *mean >= 0.80 {
            signal_families.push(fam);
            println!("  [signal]  {}: mean={:.4}", fam, mean);
        } else if *mean < 0.50 {
            noise_families.push(fam);
            println!("  [noise]   {}: mean={:.4}", fam, mean);
        }
        if *std < 0.05 {
            uniform_families.push(fam);
            println!("  [uniform] {}: std={:.4}", fam, std);
        }
    }

    println!();
    println!("=== Verdict ===");
    println!(
        "  {} signal famil{}, {} noise famil{}, {} uniform famil{}",
        signal_families.len(),
        if signal_families.len() == 1 { "y" } else { "ies" },
        noise_families.len(),
        if noise_families.len() == 1 { "y" } else { "ies" },
        uniform_families.len(),
        if uniform_families.len() == 1 { "y" } else { "ies" },
    );
    println!();
    if !noise_families.is_empty() && !signal_families.is_empty() {
        println!("  POSITIVE — quality dimension separates noise from signal families.");
        println!(
            "  Beta-1's structural classification + F.1.1 quality = full 2D classification of axiom families.",
        );
    } else {
        println!("  PARTIAL — quality dimension does not cleanly separate on this run.");
    }
    println!();
    println!("--- end ---");
}
