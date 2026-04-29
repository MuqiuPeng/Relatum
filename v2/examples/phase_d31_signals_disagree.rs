//! Phase D.3.1 — Signals-disagree substrate (arbitration test).
//!
//! D.3 showed composite signal works mechanically, but on OQ#1
//! both primary-rate and cross-precision always rank t_0 lowest
//! → arbitration value not differentiated.
//!
//! D.3.1 tries a NARROW substrate (regime A only — diamond posets,
//! no bipartite/equivalence/markers). On narrow substrate:
//!   - Primary-rate: every axiom that holds on diamonds gets ~1.0
//!   - Cross-precision: validates against IMAGINED substrates
//!     constructed from each theory; axioms that overpredict
//!     reverse edges still fail there
//!
//! Hypothesis: both signals may now agree more strongly (no noise
//! axioms in this narrow substrate). But the goal is to OBSERVE
//! whether they disagree and how composite resolves.
//!
//! Captured to `logs/<date>_phase_d31_signals_disagree.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::narrow_a::build_narrow_a_stream,
    R, RSet,
};
use std::collections::HashSet;

const TICKS_PHASE_0: u64 = 1000;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const MIN_AXIOM_PREDICTIONS: u64 = 5;

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
    println!("=== Phase D.3.1 — Signals-disagree substrate test ===");
    println!("Stream: narrow_a (regime A only, 5 phases × 100 ticks)");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_narrow_a_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    println!();
    println!("Phase 0 state: {} theories, {} axioms", theories.len(), rt.rset.axioms().len());
    if theories.len() < 2 {
        println!("→ INAPPLICABLE — too few theories to compare signals");
        return;
    }

    // Compute primary-rate per theory.
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

    let primary_rates: Vec<Option<f64>> = theories.iter().enumerate()
        .map(|(j, _t)| {
            let mut sum = 0.0;
            let mut q = 0;
            for ax in &axioms_by_theory[j] {
                if let Some(rate) = rt
                    .memory
                    .prediction_state
                    .hit_rate(ax, MIN_AXIOM_PREDICTIONS)
                {
                    sum += rate;
                    q += 1;
                }
            }
            if q == 0 { None } else { Some(sum / q as f64) }
        })
        .collect();

    // Cross-precision (column means).
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
    let cross_means: Vec<Option<f64>> = (0..theories.len())
        .map(|j| {
            if col_count[j] == 0 { None } else { Some(col_sum[j] / col_count[j] as f64) }
        })
        .collect();

    // Composite (α=0.5).
    let composite: Vec<Option<f64>> = theories.iter().enumerate()
        .map(|(j, _)| {
            match (primary_rates[j], cross_means[j]) {
                (Some(p), Some(c)) => Some(0.5 * p + 0.5 * c),
                _ => None,
            }
        })
        .collect();

    println!();
    println!("{:>6} {:>15} {:>15} {:>15}", "theory", "primary", "cross-prec", "composite");
    for (j, t) in theories.iter().enumerate() {
        let f = |o: Option<f64>| o.map(|x| format!("{:.4}", x)).unwrap_or_else(|| "—".into());
        println!(
            "{:>6} {:>15} {:>15} {:>15}",
            t, f(primary_rates[j]), f(cross_means[j]), f(composite[j]),
        );
    }

    // Find bottom by each signal.
    let pick = |scores: &[Option<f64>]| -> Option<(String, f64)> {
        let mut best: Option<(String, f64)> = None;
        for (j, s) in scores.iter().enumerate() {
            if let Some(v) = s {
                match &best {
                    None => best = Some((theories[j].clone(), *v)),
                    Some((_, w)) if *v < *w => best = Some((theories[j].clone(), *v)),
                    _ => {}
                }
            }
        }
        best
    };
    let p_bot = pick(&primary_rates);
    let c_bot = pick(&cross_means);
    let comp_bot = pick(&composite);

    println!();
    println!("=== Bottoms by signal ===");
    println!("  primary-rate    : {:?}", p_bot);
    println!("  cross-precision : {:?}", c_bot);
    println!("  composite (α=0.5): {:?}", comp_bot);

    println!();
    println!("=== Verdict ===");
    let p_id = p_bot.as_ref().map(|x| x.0.clone());
    let c_id = c_bot.as_ref().map(|x| x.0.clone());
    let comp_id = comp_bot.as_ref().map(|x| x.0.clone());
    if p_id == c_id {
        println!("  → AGREE — both primary and cross pick the same theory; arbitration moot.");
    } else {
        println!("  → DISAGREE — primary picks {:?}, cross picks {:?}", p_id, c_id);
        match comp_id {
            Some(c) if Some(c.clone()) == p_id => println!("    composite sided with primary ({})", c),
            Some(c) if Some(c.clone()) == c_id => println!("    composite sided with cross ({})", c),
            Some(c) => println!("    composite picked a third option: {}", c),
            None => println!("    composite produced no result"),
        }
    }
    println!();
    println!("--- end ---");
}
