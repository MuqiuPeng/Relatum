//! Phase C.2.1 — OQ#2 non-overlapping regimes validation.
//!
//! OQ#1 has regimes (diamond posets, bipartite, equivalence,
//! markers) that share structural features. C.2 confirmed that
//! cross-precision + shape-family signals generalize to long5k,
//! but long5k uses scaled-up versions of the same regimes.
//!
//! C.2.1 tests on OQ#2: 3 regimes that don't overlap with OQ#1's
//! (tournament with violations, 4-element lattice with self-loops,
//! star network with bidirectional edges). Asks:
//!  1. Does theory discovery find anything?
//!  2. Does shape family discovery surface a noise/structural cluster?
//!  3. Are dream-phase cross-precision signals informative here?
//!
//! Captured to `logs/<date>_phase_c21_oq2_validation.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq2::build_oq2_stream,
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
    println!("=== Phase C.2.1 — OQ#2 non-overlapping-regimes validation ===");
    println!("Stream: oq2 (tournament + lattice + star, 3 regimes × 5 phases)");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_oq2_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    let axioms: Vec<String> = rt.rset.axioms().into_iter().map(str::to_owned).collect();

    println!();
    println!(
        "Phase 0: {} theories, {} axioms, {} episodes",
        theories.len(), axioms.len(), rt.memory.episodes.len(),
    );
    if theories.is_empty() {
        println!("  → INAPPLICABLE — no theories discovered");
        return;
    }
    println!("  theories: {:?}", theories);

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
        println!("    {}: {} axioms = {:?}", t, axs.len(), axs);
    }
    let mut all_axiom_ids: HashSet<String> = HashSet::new();
    for axs in &axioms_by_theory {
        for ax in axs {
            all_axiom_ids.insert(ax.clone());
        }
    }

    // Cross-precision matrix.
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
    let mut col_sum = vec![0.0; theories.len()];
    let mut col_count = vec![0usize; theories.len()];
    for (i, sub_i) in substrates.iter().enumerate() {
        let actual_i: HashSet<R> = sub_i.iter().cloned().collect();
        for j in 0..theories.len() {
            if i == j { continue; }
            let predicted_j = predict_all_axioms(sub_i, &axioms_by_theory[j]);
            if let Some(p) = precision(&predicted_j, &actual_i) {
                col_sum[j] += p;
                col_count[j] += 1;
            }
        }
    }
    println!();
    println!("=== Cross-precision column means ===");
    println!("{:>10} {:>15}", "theory", "mean");
    let mut bottom_id: Option<String> = None;
    let mut bottom_mean: f64 = f64::INFINITY;
    for j in 0..theories.len() {
        if col_count[j] == 0 {
            println!("{:>10} {:>15}", theories[j], "—");
            continue;
        }
        let mean = col_sum[j] / col_count[j] as f64;
        println!("{:>10} {:>15.4}", theories[j], mean);
        if mean < bottom_mean {
            bottom_mean = mean;
            bottom_id = Some(theories[j].clone());
        }
    }

    // Shape families.
    let mut rt2 = rt;
    let minted = rt2.rset.discover_axiom_shape_families(2);
    println!();
    println!("=== Beta-1 shape families on OQ#2 ===");
    println!("  minted {} families:", minted.len());
    for shape in &minted {
        let n = rt2.rset.shape_family_members(shape).len();
        println!("    {}: {} members", shape, n);
    }

    println!();
    println!("=== Verdict ===");
    let theories_ok = theories.len() >= 1;
    let dream_ok = bottom_id.is_some();
    let families_ok = !minted.is_empty();
    let overall = theories_ok && dream_ok && families_ok;
    if overall {
        println!(
            "  → POSITIVE — OQ#2 produces {} theories, {} shape families; cross-precision identifies bottom {} (mean {:.4}). Dream-phase + family discovery generalize to non-overlapping regimes.",
            theories.len(), minted.len(),
            bottom_id.unwrap_or_default(), bottom_mean,
        );
    } else if theories_ok && dream_ok {
        println!(
            "  → PARTIAL — theories + cross-precision work; no shape families (substrate's axioms are too unique to share premises).",
        );
    } else if theories_ok {
        println!("  → MIXED — only theory discovery works.");
    } else {
        println!("  → NULL — substrate didn't expose theory discovery.");
    }
    println!();
    println!("--- end ---");
}
