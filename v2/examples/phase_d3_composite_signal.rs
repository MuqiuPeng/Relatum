//! Phase D.3 — Composite scheduler signal: blend primary-rate +
//! cross-precision-mean.
//!
//! Alpha-3+ uses primary-stream hit rate per axiom for tournament.
//! Alpha-8 uses cross-precision column mean (DreamCoder dream phase)
//! for an INDEPENDENT decision signal, with Alpha-9 showing it
//! crosses the 0.50 demote threshold 250 ticks earlier on OQ#1.
//!
//! D.3 asks: does a weighted blend produce a more decisive ranking
//! than either alone, especially at small T where primary-rate is
//! still maturing?
//!
//! composite_score(theory) = α * primary_rate + (1-α) * cross_prec_mean
//!
//! For each T ∈ {100, 200, 350, 500, 1000}, compute primary,
//! cross-precision, and composite (α=0.5) rankings. Report which
//! signal is most decisive (lowest score below threshold) at each T.
//!
//! Captured to `logs/<date>_phase_d3_composite_signal.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    R, RSet,
};
use std::collections::HashSet;

const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const COMPOSITE_ALPHA: f64 = 0.5; // 0.5 = equal blend

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

#[derive(Clone)]
struct CompositeRow {
    theory: String,
    primary_rate: Option<f64>,
    cross_mean: Option<f64>,
    composite: Option<f64>,
}

fn compute_signals(rt: &AutonomousRuntime) -> Vec<CompositeRow> {
    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    if theories.is_empty() {
        return Vec::new();
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
    // Generate substrates (same as Alpha-7).
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
    // Cross-precision column means (off-diagonal).
    let mut col_sum = vec![0.0; theories.len()];
    let mut col_count = vec![0usize; theories.len()];
    for (i, sub_i) in substrates.iter().enumerate() {
        let actual_i: HashSet<R> = sub_i.iter().cloned().collect();
        for j in 0..theories.len() {
            if i == j {
                continue;
            }
            let predicted_j = predict_all_axioms(sub_i, &axioms_by_theory[j]);
            if let Some(p) = precision(&predicted_j, &actual_i) {
                col_sum[j] += p;
                col_count[j] += 1;
            }
        }
    }
    // Primary-rate per theory.
    let mut rows: Vec<CompositeRow> = Vec::new();
    for j in 0..theories.len() {
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
        let primary_rate = if q == 0 { None } else { Some(sum / q as f64) };
        let cross_mean = if col_count[j] == 0 {
            None
        } else {
            Some(col_sum[j] / col_count[j] as f64)
        };
        let composite = match (primary_rate, cross_mean) {
            (Some(p), Some(c)) => Some(COMPOSITE_ALPHA * p + (1.0 - COMPOSITE_ALPHA) * c),
            _ => None,
        };
        rows.push(CompositeRow {
            theory: theories[j].clone(),
            primary_rate,
            cross_mean,
            composite,
        });
    }
    rows
}

fn pick_bottom_by<F: Fn(&CompositeRow) -> Option<f64>>(
    rows: &[CompositeRow],
    f: F,
) -> Option<(String, f64)> {
    let mut best: Option<(String, f64)> = None;
    for row in rows {
        if let Some(v) = f(row) {
            match &best {
                None => best = Some((row.theory.clone(), v)),
                Some((_, w)) if v < *w => best = Some((row.theory.clone(), v)),
                _ => {}
            }
        }
    }
    best
}

fn run_at_t(t: u64) -> Vec<CompositeRow> {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(t);
    compute_signals(&rt)
}

fn main() {
    println!(
        "=== Phase D.3 — Composite scheduler signal (α={}) ===",
        COMPOSITE_ALPHA,
    );
    println!(
        "Composite = {} × primary_rate + {} × cross_prec_mean",
        COMPOSITE_ALPHA, 1.0 - COMPOSITE_ALPHA,
    );
    let ts = [100u64, 200, 350, 500, 1000];
    println!();
    println!(
        "{:>5} {:>12} {:>12} {:>12} {:>12}",
        "T", "primary", "cross", "composite", "agree?"
    );
    let mut composite_first = None;
    let mut primary_first = None;
    let mut cross_first = None;
    for &t in &ts {
        let rows = run_at_t(t);
        let p_bottom = pick_bottom_by(&rows, |r| r.primary_rate);
        let c_bottom = pick_bottom_by(&rows, |r| r.cross_mean);
        let comp_bottom = pick_bottom_by(&rows, |r| r.composite);
        let p_id = p_bottom.as_ref().map(|x| x.0.clone()).unwrap_or_else(|| "—".into());
        let c_id = c_bottom.as_ref().map(|x| x.0.clone()).unwrap_or_else(|| "—".into());
        let comp_id = comp_bottom.as_ref().map(|x| x.0.clone()).unwrap_or_else(|| "—".into());
        let agree = p_id == c_id && c_id == comp_id;
        let p_v = p_bottom.as_ref().map(|x| format!("{}={:.4}", x.0, x.1)).unwrap_or_default();
        let c_v = c_bottom.as_ref().map(|x| format!("{}={:.4}", x.0, x.1)).unwrap_or_default();
        let comp_v = comp_bottom.as_ref().map(|x| format!("{}={:.4}", x.0, x.1)).unwrap_or_default();
        println!(
            "{:>5} {:>12} {:>12} {:>12} {:>12}",
            t, p_v, c_v, comp_v,
            if agree { "yes" } else { "no" },
        );
        // First T where bottom score < 0.50 threshold.
        if primary_first.is_none() && p_bottom.as_ref().map_or(false, |x| x.1 < 0.50) {
            primary_first = Some(t);
        }
        if cross_first.is_none() && c_bottom.as_ref().map_or(false, |x| x.1 < 0.50) {
            cross_first = Some(t);
        }
        if composite_first.is_none() && comp_bottom.as_ref().map_or(false, |x| x.1 < 0.50) {
            composite_first = Some(t);
        }
    }
    println!();
    println!("=== First T to cross 0.50 demote threshold ===");
    println!(
        "  primary  : T={}",
        primary_first.map(|x| x.to_string()).unwrap_or_else(|| "never".into()),
    );
    println!(
        "  cross    : T={}",
        cross_first.map(|x| x.to_string()).unwrap_or_else(|| "never".into()),
    );
    println!(
        "  composite: T={}",
        composite_first.map(|x| x.to_string()).unwrap_or_else(|| "never".into()),
    );
    println!();
    println!("=== Verdict ===");
    match (primary_first, cross_first, composite_first) {
        (Some(p), Some(c), Some(comp)) => {
            if comp <= c.min(p) {
                println!(
                    "  → POSITIVE — composite crosses threshold at T={} ≤ min(primary={}, cross={})",
                    comp, p, c,
                );
            } else if comp == c {
                println!("  → TIE-WITH-CROSS");
            } else {
                println!("  → INTERMEDIATE — composite at T={} is between primary and cross", comp);
            }
        }
        _ => println!("  → INSUFFICIENT-DATA"),
    }
    println!();
    println!("--- end ---");
}
