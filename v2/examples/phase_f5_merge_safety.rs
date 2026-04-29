//! Phase F.5 — Empirical merge safety test.
//!
//! F.4 picked `(t_2, t_3)` at 66.7% confidence. The picker is a PROPOSAL —
//! does executing the merge actually preserve quality?
//!
//! Method:
//!   1. Train rt on OQ#1
//!   2. Pre-merge: compute t_2, t_3, and union-axiom cross-precision
//!   3. merge_theories(t_2, t_3) → t_merged
//!   4. Post-merge: compute t_merged cross-precision
//!   5. Verify: t_merged ≥ max(t_2_pre, t_3_pre) − ε  (no degradation)
//!
//! Expected per F.3 finding (t_2 and t_3 have identical column profiles):
//!   - t_merged cross-precision ≈ both pre-merge values
//!   - No quality regression
//!   - Lossless consolidation

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

fn theory_cross_precision(rset: &RSet, theory_id: &str, theories: &[String]) -> f64 {
    let theory_axioms: Vec<String> = rset.theory_axioms(theory_id)
        .into_iter().map(str::to_owned).collect();
    let all_axiom_ids: HashSet<String> = rset.axioms().into_iter().map(str::to_owned).collect();
    let mut sum = 0.0;
    let mut count = 0;
    for (i, t) in theories.iter().enumerate() {
        if t == theory_id { continue; } // exclude self-substrate
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut sub = match rset.generate_substrate_from_theory(t, NUM_GEN_IDS, SEED_DENSITY, seed) {
            Ok(g) => g, Err(_) => continue,
        };
        for ax in &all_axiom_ids { sub.register_axiom_with_intension(ax); }
        let actual: HashSet<R> = sub.iter().cloned().collect();
        let predicted = predict_all_axioms(&sub, &theory_axioms);
        if let Some(p) = precision(&predicted, &actual) {
            sum += p;
            count += 1;
        }
    }
    if count > 0 { sum / count as f64 } else { 0.0 }
}

fn main() {
    println!("=== Phase F.5 — Empirical merge safety test ===");
    println!();

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    println!("training: {} theories ({:?})", theories.len(), theories);

    // ---- Pre-merge baselines ----
    let t2 = "t_2";
    let t3 = "t_3";
    let t2_axioms: HashSet<String> = rt.rset.theory_axioms(t2)
        .into_iter().map(str::to_owned).collect();
    let t3_axioms: HashSet<String> = rt.rset.theory_axioms(t3)
        .into_iter().map(str::to_owned).collect();
    let union_axioms: HashSet<String> = t2_axioms.union(&t3_axioms).cloned().collect();

    println!();
    println!("=== Pre-merge ===");
    println!("  t_2 axioms: {} ({:?})", t2_axioms.len(), t2_axioms);
    println!("  t_3 axioms: {} ({:?})", t3_axioms.len(), t3_axioms);
    println!("  union:      {} axioms", union_axioms.len());

    let t2_xprec = theory_cross_precision(&rt.rset, t2, &theories);
    let t3_xprec = theory_cross_precision(&rt.rset, t3, &theories);
    println!("  t_2 cross-precision (excl self): {:.4}", t2_xprec);
    println!("  t_3 cross-precision (excl self): {:.4}", t3_xprec);

    // ---- Execute merge ----
    println!();
    println!("=== Executing merge_theories(t_2, t_3) ===");
    let merged_id = match rt.rset.merge_theories(t2, t3) {
        Ok(id) => id,
        Err(e) => {
            println!("  MERGE FAILED: {:?}", e);
            return;
        }
    };
    println!("  merged → {}", merged_id);

    // Snapshot post-merge state
    let mut theories_post: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories_post.sort();
    println!("  theories after merge: {:?}", theories_post);

    let merged_axioms: HashSet<String> = rt.rset.theory_axioms(&merged_id)
        .into_iter().map(str::to_owned).collect();
    println!("  merged axioms: {} (expected {})", merged_axioms.len(), union_axioms.len());
    assert_eq!(merged_axioms, union_axioms, "merge lost or added axioms");

    // ---- Post-merge cross-precision ----
    println!();
    println!("=== Post-merge ===");
    let merged_xprec = theory_cross_precision(&rt.rset, &merged_id, &theories_post);
    println!("  {} cross-precision (excl self): {:.4}", merged_id, merged_xprec);

    // ---- Safety verdict ----
    println!();
    println!("=== Safety verdict ===");
    let max_pre = t2_xprec.max(t3_xprec);
    let min_pre = t2_xprec.min(t3_xprec);
    let delta_max = merged_xprec - max_pre;
    let delta_min = merged_xprec - min_pre;
    println!("  max(t_2_pre, t_3_pre) = {:.4}", max_pre);
    println!("  min(t_2_pre, t_3_pre) = {:.4}", min_pre);
    println!("  merged                = {:.4}", merged_xprec);
    println!("  delta vs max: {:+.4}", delta_max);
    println!("  delta vs min: {:+.4}", delta_min);
    println!();
    let safe = delta_max >= -0.05; // tolerate small numerical drift
    if safe {
        println!("  POSITIVE — merge preserves quality (no degradation > 5%)");
        println!("  F.4's 66.7% confidence pick is empirically VALID.");
    } else {
        println!("  NEGATIVE — merge degraded quality. F.4's confidence claim FAILED.");
    }
    println!();
    println!("--- end ---");
}
