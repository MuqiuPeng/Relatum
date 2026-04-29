//! Phase F.2.1 — Quality-aware merge candidate selector.
//!
//! F.2 surfaced family-signature complementarity as a merge selection
//! signal, but its top pick (t_0, t_2) on OQ#1 was flagged with a
//! caveat: t_0 is noisy, merging would dilute t_2's quality.
//!
//! F.2.1 combines two filters:
//!   1. Both theories must clear a cross-precision quality floor
//!      (column-mean cross-precision ≥ FLOOR, excluding self-diagonal)
//!   2. Among eligible pairs, pick highest signature complementarity
//!
//! Expected on OQ#1: t_0 fails the quality floor (cross-prec ≈ 0.32),
//! so (t_0, *) pairs are rejected. Among eligible pairs, the new best
//! is a NEW pick distinct from Alpha-5 / F.2 / F.3 — F.2.1 occupies
//! a unique slot in the merge-selector family.

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
const QUALITY_FLOOR: f64 = 0.50;

fn predict_all_axioms(substrate: &RSet, axiom_ids: &[String]) -> HashSet<R> {
    let mut total: HashSet<R> = HashSet::new();
    for ax in axiom_ids {
        total.extend(substrate.forward_apply_axiom(ax));
    }
    total
}

fn precision(predicted: &HashSet<R>, actual: &HashSet<R>) -> Option<f64> {
    if predicted.is_empty() { return None; }
    let inter = predicted.iter().filter(|r| actual.contains(*r)).count();
    Some(inter as f64 / predicted.len() as f64)
}

fn theory_family_signature(rset: &RSet, theory_id: &str) -> HashSet<String> {
    let members: HashSet<String> = rset
        .theory_axioms(theory_id)
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut sig: HashSet<String> = HashSet::new();
    for sf in rset.axiom_shape_families() {
        let in_sf: HashSet<String> = rset
            .shape_family_members(sf)
            .into_iter()
            .map(str::to_owned)
            .collect();
        if !in_sf.is_disjoint(&members) {
            sig.insert(sf.to_string());
        }
    }
    sig
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() { return 1.0; }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 { return 0.0; }
    inter / union
}

fn main() {
    println!("=== Phase F.2.1 — Quality-aware merge selector ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    rt.rset.discover_axiom_shape_families(2);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();

    let axioms_by_theory: Vec<Vec<String>> = theories.iter()
        .map(|t| rt.rset.theory_axioms(t).into_iter().map(str::to_owned).collect())
        .collect();
    let mut all_axiom_ids: HashSet<String> = HashSet::new();
    for axs in &axioms_by_theory {
        for ax in axs { all_axiom_ids.insert(ax.clone()); }
    }

    // Build substrates
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut gen = match rt.rset.generate_substrate_from_theory(t, NUM_GEN_IDS, SEED_DENSITY, seed) {
            Ok(g) => g, Err(_) => continue,
        };
        for ax in &all_axiom_ids { gen.register_axiom_with_intension(ax); }
        substrates.push(gen);
    }

    // Cross-precision matrix
    let mut matrix: Vec<Vec<Option<f64>>> = vec![vec![None; theories.len()]; substrates.len()];
    for (i, sub_i) in substrates.iter().enumerate() {
        let actual_i: HashSet<R> = sub_i.iter().cloned().collect();
        for j in 0..theories.len() {
            let predicted_j = predict_all_axioms(sub_i, &axioms_by_theory[j]);
            matrix[i][j] = precision(&predicted_j, &actual_i);
        }
    }

    // Per-theory cross-precision column mean (excluding self-diagonal)
    println!();
    println!("=== Cross-precision quality (excluding self) ===");
    let mut quality: Vec<f64> = Vec::new();
    for j in 0..theories.len() {
        let mut sum = 0.0;
        let mut count = 0;
        for k in 0..substrates.len() {
            if k == j { continue; }
            if let Some(v) = matrix[k][j] {
                sum += v; count += 1;
            }
        }
        let q = if count > 0 { sum / count as f64 } else { 0.0 };
        quality.push(q);
        println!("  {}: {:.4} {}",
            theories[j], q,
            if q >= QUALITY_FLOOR { "PASS" } else { "FAIL (below floor)" });
    }
    println!();
    println!("(quality floor = {})", QUALITY_FLOOR);

    // Family signatures
    let signatures: Vec<HashSet<String>> = theories.iter()
        .map(|t| theory_family_signature(&rt.rset, t))
        .collect();

    // Pairwise complementarity (1 - Jaccard) + quality gate
    println!();
    println!("=== Pairwise candidates ===");
    println!("{:>10} {:>10} {:>15} {:>10} {:>10}",
             "a", "b", "complement", "a_qual", "b_qual");

    let mut eligible: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..theories.len() {
        for j in (i+1)..theories.len() {
            let comp = 1.0 - jaccard(&signatures[i], &signatures[j]);
            let qa = quality[i];
            let qb = quality[j];
            let pass = qa >= QUALITY_FLOOR && qb >= QUALITY_FLOOR;
            println!("{:>10} {:>10} {:>15.4} {:>10.4} {:>10.4} {}",
                theories[i], theories[j], comp, qa, qb,
                if pass { "ELIGIBLE" } else { "rejected" });
            if pass {
                eligible.push((i, j, comp));
            }
        }
    }

    // Pick best eligible by complementarity
    println!();
    println!("=== Verdict ===");
    if eligible.is_empty() {
        println!("  NULL — no pair clears the quality floor on both sides");
    } else {
        let best = eligible.iter()
            .max_by(|a, b| a.2.partial_cmp(&b.2).unwrap()).unwrap();
        println!(
            "  POSITIVE — best quality-aware merge candidate: ({}, {}) at complementarity {:.4}",
            theories[best.0], theories[best.1], best.2,
        );
        println!();
        println!("  Comparison with prior selectors:");
        println!("    Alpha-3++++ Jaccard:    (t_0, t_1)   ← subset+noise (wrong)");
        println!("    Alpha-5 smart:          (t_2, t_3)");
        println!("    F.2 raw complementarity: (t_0, t_2)   ← caveat: t_0 noisy");
        println!("    F.3 cross-precision:    (t_2, t_3)");
        println!("    F.2.1 (this):           ({}, {})   ← quality + complementarity composite",
            theories[best.0], theories[best.1]);
        if (theories[best.0] != "t_2" || theories[best.1] != "t_3") &&
           (theories[best.0] != "t_0" || theories[best.1] != "t_2") {
            println!();
            println!("  → distinct from prior selectors; F.2.1 occupies a unique slot in the merge family");
        }
    }
    println!();
    println!("--- end ---");
}
