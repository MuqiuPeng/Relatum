//! Phase C.2 — Cross-precision validation on long5k.
//!
//! Phase Alpha-7..9 (dream phase, cross-precision-driven demote,
//! varying-T) all ran on OQ#1. C.2 asks: does the
//! cross-precision signal generalize to a different substrate?
//!
//! long5k has 5 regimes (vs OQ#1's 4) and 10 phases per regime
//! (vs 5). It's structurally distinct enough to test
//! generalization.
//!
//! Method: run Phase 0 (1500 ticks for substrate to mature),
//! discover theories + axioms, generate substrates, compute
//! cross-precision matrix + per-axiom mean, identify low-mean
//! axioms.
//!
//! Falsifiable:
//! - POSITIVE: cross-precision identifies at least one theory or
//!   axiom cluster as low-mean, mirroring OQ#1's t_0 behavior
//! - NULL: theories on long5k are uniformly high-precision
//!   (signal doesn't discriminate)
//!
//! Captured to `logs/<date>_phase_c2_long5k_cross_precision.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::long5k::build_5k_stream,
    R, RSet,
};
use std::collections::HashSet;

const TICKS_PHASE_0: u64 = 1500;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;

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
    println!("=== Phase C.2 — Cross-precision validation on long5k ===");
    println!("TICKS_PHASE_0={}, NUM_GEN_IDS={}, SEED_DENSITY={}", TICKS_PHASE_0, NUM_GEN_IDS, SEED_DENSITY);

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_5k_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    let axioms: Vec<String> = rt.rset.axioms().into_iter().map(str::to_owned).collect();

    println!();
    println!("=== Phase 0 state (long5k @ {} ticks) ===", TICKS_PHASE_0);
    println!("  theories: {}", theories.len());
    println!("  axioms  : {}", axioms.len());
    println!("  episodes: {}", rt.memory.episodes.len());
    if theories.is_empty() {
        println!("  → INAPPLICABLE — no theories discovered");
        return;
    }
    println!("  theory list: {:?}", theories);

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
    for (t, axs) in theories.iter().zip(&axioms_by_theory) {
        println!("    {}: {} axioms", t, axs.len());
    }
    let mut all_axiom_ids: HashSet<String> = HashSet::new();
    for axs in &axioms_by_theory {
        for ax in axs {
            all_axiom_ids.insert(ax.clone());
        }
    }

    // Generate substrates per theory.
    let mut substrates: Vec<RSet> = Vec::with_capacity(theories.len());
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
    println!();
    println!("=== Generated {} substrates ===", substrates.len());

    // Cross-precision matrix.
    println!();
    println!("=== Cross-precision matrix (rows: substrate; cols: theory) ===");
    print!("{:>10}", "");
    for t in &theories {
        print!("{:>10}", t);
    }
    println!();
    let mut col_sum = vec![0.0; theories.len()];
    let mut col_count = vec![0usize; theories.len()];
    for (i, sub_i) in substrates.iter().enumerate() {
        let actual_i: HashSet<R> = sub_i.iter().cloned().collect();
        print!("{:>10}", theories[i]);
        for j in 0..theories.len() {
            let predicted_j = predict_all_axioms(sub_i, &axioms_by_theory[j]);
            let p = precision(&predicted_j, &actual_i);
            match p {
                Some(v) => {
                    print!("{:>10.4}", v);
                    if i != j {
                        col_sum[j] += v;
                        col_count[j] += 1;
                    }
                }
                None => print!("{:>10}", "—"),
            }
        }
        println!();
    }

    // Column means + identify lowest.
    println!();
    println!("=== Per-theory column means (off-diagonal) ===");
    println!("{:>10} {:>15}", "theory", "mean");
    let mut bottom: Option<(String, f64)> = None;
    for j in 0..theories.len() {
        if col_count[j] == 0 {
            println!("{:>10} {:>15}", theories[j], "—");
            continue;
        }
        let mean = col_sum[j] / col_count[j] as f64;
        println!("{:>10} {:>15.4}", theories[j], mean);
        match &bottom {
            None => bottom = Some((theories[j].clone(), mean)),
            Some((_, m)) if mean < *m => bottom = Some((theories[j].clone(), mean)),
            _ => {}
        }
    }

    // Discover shape families on long5k axiom set.
    println!();
    println!("=== Beta-1 shape families on long5k ===");
    let mut rt_mut = rt;
    let minted = rt_mut.rset.discover_axiom_shape_families(2);
    println!("  minted {} families", minted.len());
    for shape in &minted {
        let n = rt_mut.rset.shape_family_members(shape).len();
        println!("    {}: {} members", shape, n);
    }

    println!();
    println!("=== Verdict ===");
    if let Some((id, mean)) = bottom {
        if mean < 0.50 {
            println!(
                "  → POSITIVE — cross-precision identifies '{}' as bottom (mean={:.4}) on long5k; signal generalizes from OQ#1.",
                id, mean,
            );
        } else if mean < 0.80 {
            println!(
                "  → MID — bottom theory '{}' (mean={:.4}) is below 0.80 but not yet decisive on long5k; signal weaker but still discriminative.",
                id, mean,
            );
        } else {
            println!(
                "  → NULL — all theories on long5k have mean ≥ 0.80 (bottom = '{}' at {:.4}); signal doesn't surface noise here.",
                id, mean,
            );
        }
    } else {
        println!("  → INAPPLICABLE — no qualifying column means");
    }
    println!();
    println!("--- end ---");
}
