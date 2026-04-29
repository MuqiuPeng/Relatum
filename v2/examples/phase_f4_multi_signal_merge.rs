//! Phase F.4 — Multi-signal composite merge picker.
//!
//! Three independent merge selectors built up over Rounds 1-2:
//!   - Alpha-5 smart: non-subset member-set Jaccard
//!   - F.3:           cross-precision column-profile equivalence
//!   - F.2.1:         quality-aware family-signature complementarity
//!
//! Each picks a top candidate from a different angle. F.4 asks:
//! which pair gets agreement across signals?
//!
//! Method: rank pairs by each signal (top-1, top-2). Score each pair
//! by Borda-style aggregation:
//!   +2 for top-1, +1 for top-2, 0 otherwise (per signal)
//! Highest aggregate = highest-confidence merge pair.
//!
//! Expected on OQ#1: (t_2, t_3) wins Alpha-5 + F.3 = 4 points;
//! (t_1, t_2) wins F.2.1 = 2 points; aggregate confidence
//! concentrates on (t_2, t_3).

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
    for ax in axiom_ids { total.extend(substrate.forward_apply_axiom(ax)); }
    total
}

fn precision(predicted: &HashSet<R>, actual: &HashSet<R>) -> Option<f64> {
    if predicted.is_empty() { return None; }
    let inter = predicted.iter().filter(|r| actual.contains(*r)).count();
    Some(inter as f64 / predicted.len() as f64)
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() { return 1.0; }
    let inter = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 { 0.0 } else { inter / union }
}

fn is_subset_or_superset(a: &HashSet<String>, b: &HashSet<String>) -> bool {
    a.is_subset(b) || b.is_subset(a)
}

fn theory_family_signature(rset: &RSet, theory_id: &str) -> HashSet<String> {
    let members: HashSet<String> =
        rset.theory_axioms(theory_id).into_iter().map(str::to_owned).collect();
    let mut sig: HashSet<String> = HashSet::new();
    for sf in rset.axiom_shape_families() {
        let in_sf: HashSet<String> =
            rset.shape_family_members(sf).into_iter().map(str::to_owned).collect();
        if !in_sf.is_disjoint(&members) {
            sig.insert(sf.to_string());
        }
    }
    sig
}

fn main() {
    println!("=== Phase F.4 — Multi-signal composite merge picker ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);
    rt.rset.discover_axiom_shape_families(2);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    let n = theories.len();

    let member_sets: Vec<HashSet<String>> = theories.iter()
        .map(|t| rt.rset.theory_axioms(t).into_iter().map(str::to_owned).collect())
        .collect();
    let axioms_by_theory: Vec<Vec<String>> = theories.iter()
        .map(|t| rt.rset.theory_axioms(t).into_iter().map(str::to_owned).collect())
        .collect();
    let mut all_axiom_ids: HashSet<String> = HashSet::new();
    for axs in &axioms_by_theory {
        for ax in axs { all_axiom_ids.insert(ax.clone()); }
    }

    // Substrates + cross-prec matrix
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut gen = match rt.rset.generate_substrate_from_theory(t, NUM_GEN_IDS, SEED_DENSITY, seed) {
            Ok(g) => g, Err(_) => continue,
        };
        for ax in &all_axiom_ids { gen.register_axiom_with_intension(ax); }
        substrates.push(gen);
    }
    let mut matrix: Vec<Vec<Option<f64>>> = vec![vec![None; n]; substrates.len()];
    for (i, sub_i) in substrates.iter().enumerate() {
        let actual: HashSet<R> = sub_i.iter().cloned().collect();
        for j in 0..n {
            matrix[i][j] = precision(&predict_all_axioms(sub_i, &axioms_by_theory[j]), &actual);
        }
    }

    // Quality column means (excluding self-substrate)
    let quality: Vec<f64> = (0..n).map(|j| {
        let mut sum = 0.0; let mut count = 0;
        for k in 0..substrates.len() {
            if k == j { continue; }
            if let Some(v) = matrix[k][j] { sum += v; count += 1; }
        }
        if count > 0 { sum / count as f64 } else { 0.0 }
    }).collect();

    // Family signatures
    let signatures: Vec<HashSet<String>> = theories.iter()
        .map(|t| theory_family_signature(&rt.rset, t)).collect();

    // ---- Compute rankings for each selector ----
    // Selector A — Alpha-5: non-subset Jaccard, descending
    let mut alpha5: Vec<((usize, usize), f64)> = Vec::new();
    for i in 0..n {
        for j in (i+1)..n {
            if is_subset_or_superset(&member_sets[i], &member_sets[j]) { continue; }
            alpha5.push(((i, j), jaccard(&member_sets[i], &member_sets[j])));
        }
    }
    alpha5.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Selector B — F.3: cross-precision profile max_diff, ascending (smaller = more equivalent)
    let mut f3: Vec<((usize, usize), f64)> = Vec::new();
    for i in 0..n {
        for j in (i+1)..n {
            let mut max_diff: f64 = 0.0;
            let mut have = false;
            for k in 0..substrates.len() {
                if k == i || k == j { continue; }
                if let (Some(a), Some(b)) = (matrix[k][i], matrix[k][j]) {
                    let d = (a - b).abs();
                    if d > max_diff { max_diff = d; }
                    have = true;
                }
            }
            if have { f3.push(((i, j), max_diff)); }
        }
    }
    f3.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    // Selector C — F.2.1: complementarity gated by quality, descending
    let mut f21: Vec<((usize, usize), f64)> = Vec::new();
    for i in 0..n {
        for j in (i+1)..n {
            if quality[i] < QUALITY_FLOOR || quality[j] < QUALITY_FLOOR { continue; }
            let comp = 1.0 - jaccard(&signatures[i], &signatures[j]);
            f21.push(((i, j), comp));
        }
    }
    f21.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    // Print rankings
    fn show(name: &str, rank: &[((usize, usize), f64)], theories: &[String]) {
        println!();
        println!("--- {} ranking ---", name);
        for (k, ((i, j), s)) in rank.iter().enumerate() {
            println!("  rank {}: ({}, {}) score={:.4}", k+1, theories[*i], theories[*j], s);
        }
    }
    show("Alpha-5 smart (non-subset Jaccard)", &alpha5, &theories);
    show("F.3 (cross-prec profile equivalence)", &f3, &theories);
    show("F.2.1 (quality × complementarity)", &f21, &theories);

    // ---- Borda aggregation ----
    // +2 if pair is rank-1, +1 if rank-2, 0 otherwise (per selector)
    let mut score: std::collections::HashMap<(usize, usize), u32> = std::collections::HashMap::new();
    for sel in [&alpha5, &f3, &f21] {
        if !sel.is_empty() {
            *score.entry(sel[0].0).or_insert(0) += 2;
        }
        if sel.len() > 1 {
            *score.entry(sel[1].0).or_insert(0) += 1;
        }
    }
    let mut leaderboard: Vec<((usize, usize), u32)> = score.into_iter().collect();
    leaderboard.sort_by(|a, b| b.1.cmp(&a.1));

    println!();
    println!("=== Borda leaderboard (top-1: +2, top-2: +1) ===");
    for ((i, j), pts) in &leaderboard {
        println!("  ({}, {}): {} points", theories[*i], theories[*j], pts);
    }

    println!();
    println!("=== Verdict ===");
    if let Some(((i, j), pts)) = leaderboard.first() {
        let max_possible = 6; // 3 selectors × 2 pts each
        let confidence = *pts as f64 / max_possible as f64;
        println!(
            "  F.4 multi-signal pick: ({}, {}) at {} points / {} max ({:.1}% confidence)",
            theories[*i], theories[*j], pts, max_possible, confidence * 100.0,
        );
        println!();
        if confidence >= 0.5 {
            println!("  STRONGLY POSITIVE — multiple independent signals concentrate on the same pair.");
        } else {
            println!("  WEAK SIGNAL — top pair only wins one selector; merges remain uncertain.");
        }
    } else {
        println!("  NULL — no pairs scored");
    }
    println!();
    println!("--- end ---");
}
