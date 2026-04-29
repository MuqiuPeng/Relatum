//! Phase F.3 — Cross-precision-driven theory merge candidate.
//!
//! Two theories are "functionally equivalent" when their column means
//! across substrates are nearly identical. F.3 surfaces such pairs as
//! merge candidates — they consolidate without information loss.
//!
//! Method:
//!  1. Compute cross-precision matrix
//!  2. For each pair (t_i, t_j), compare COLUMN profiles (not means)
//!     — i.e., for each substrate k, |precision[k][i] - precision[k][j]|
//!  3. Profiles within ε across all substrates → propose merge
//!
//! On OQ#1 expected: t_2 and t_3 might have similar profiles
//! (both above-threshold). t_0 and t_1 differ substantially.

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
const PROFILE_DIFF_THRESHOLD: f64 = 0.15;

fn predict_all_axioms(substrate: &RSet, axiom_ids: &[String]) -> HashSet<R> {
    let mut total: HashSet<R> = HashSet::new();
    for ax in axiom_ids {
        total.extend(substrate.forward_apply_axiom(ax));
    }
    total
}

fn precision(predicted: &HashSet<R>, actual: &HashSet<R>) -> Option<f64> {
    if predicted.is_empty() {
        return None;
    }
    let inter = predicted.iter().filter(|r| actual.contains(*r)).count();
    Some(inter as f64 / predicted.len() as f64)
}

fn main() {
    println!("=== Phase F.3 — Cross-precision-driven merge candidate ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    if theories.len() < 2 {
        println!("→ INAPPLICABLE — too few theories");
        return;
    }

    let axioms_by_theory: Vec<Vec<String>> = theories
        .iter()
        .map(|t| {
            rt.rset
                .theory_axioms(t)
                .into_iter()
                .map(str::to_owned)
                .collect()
        })
        .collect();
    let mut all_axiom_ids: HashSet<String> = HashSet::new();
    for axs in &axioms_by_theory {
        for ax in axs {
            all_axiom_ids.insert(ax.clone());
        }
    }

    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut gen = match rt
            .rset
            .generate_substrate_from_theory(t, NUM_GEN_IDS, SEED_DENSITY, seed)
        {
            Ok(g) => g,
            Err(_) => continue,
        };
        for ax in &all_axiom_ids {
            gen.register_axiom_with_intension(ax);
        }
        substrates.push(gen);
    }

    // Build full matrix.
    let mut matrix: Vec<Vec<Option<f64>>> = vec![vec![None; theories.len()]; substrates.len()];
    for (i, sub_i) in substrates.iter().enumerate() {
        let actual_i: HashSet<R> = sub_i.iter().cloned().collect();
        for j in 0..theories.len() {
            let predicted_j = predict_all_axioms(sub_i, &axioms_by_theory[j]);
            matrix[i][j] = precision(&predicted_j, &actual_i);
        }
    }

    println!();
    println!("=== Cross-precision matrix ===");
    print!("{:>10}", "");
    for t in &theories { print!("{:>10}", t); }
    println!();
    for (i, sub) in theories.iter().enumerate() {
        print!("{:>10}", sub);
        for j in 0..theories.len() {
            match matrix[i][j] {
                Some(v) => print!("{:>10.4}", v),
                None => print!("{:>10}", "—"),
            }
        }
        println!();
    }

    // Compare pairwise column profiles (excluding diagonal cells).
    println!();
    println!("=== Pairwise column-profile difference ===");
    println!("{:>10} {:>10} {:>15}", "a", "b", "max_diff");
    let mut equiv_pairs: Vec<(String, String, f64)> = Vec::new();
    for i in 0..theories.len() {
        for j in (i + 1)..theories.len() {
            let mut max_diff: f64 = 0.0;
            let mut have_data = false;
            for k in 0..substrates.len() {
                if k == i || k == j { continue; }
                if let (Some(a), Some(b)) = (matrix[k][i], matrix[k][j]) {
                    let d = (a - b).abs();
                    if d > max_diff { max_diff = d; }
                    have_data = true;
                }
            }
            if !have_data { continue; }
            println!(
                "{:>10} {:>10} {:>15.4}",
                theories[i], theories[j], max_diff,
            );
            if max_diff <= PROFILE_DIFF_THRESHOLD {
                equiv_pairs.push((theories[i].clone(), theories[j].clone(), max_diff));
            }
        }
    }

    println!();
    println!("=== Verdict ===");
    if equiv_pairs.is_empty() {
        println!(
            "  → NULL — no pair has column-profile max diff ≤ {} (no functionally-equivalent theories under this metric)",
            PROFILE_DIFF_THRESHOLD,
        );
    } else {
        println!(
            "  → POSITIVE — found {} candidate merge pair(s) by cross-precision profile equivalence:",
            equiv_pairs.len(),
        );
        for (a, b, d) in &equiv_pairs {
            println!("    ({}, {}) max_diff={:.4}", a, b, d);
        }
    }
    println!();
    println!("--- end ---");
}
