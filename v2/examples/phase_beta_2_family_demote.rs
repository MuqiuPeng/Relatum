//! Phase Beta-2 — Family-level demote intervention (ADR 0068 follow-up).
//!
//! Beta-1 introduced `SHAPE_FAMILY_MARKER` and discovered shape
//! families on OQ#1. The first family `shape_premise_p0-0_p1-2`
//! has 4 members with cross-precision variance ZERO (uniform low
//! 0.4162) — a structurally-coherent noise cluster.
//!
//! Beta-2 closes the loop: when a family's mean cross-precision
//! falls below threshold, retract ALL members wholesale. This is
//! the first runtime intervention driven by **structural
//! abstractions discovered at runtime** (vs theory-level operations
//! like Alpha-3+'s demote, which targeted hand-named theories).
//!
//! Falsifiable:
//! - POSITIVE: family-level demote retracts all 4 noise axioms
//!   from `shape_premise_p0-0_p1-2`; t_0's hit rate rises to
//!   ~0.6664 (matches Alpha-3+++ repair); axiom global
//!   registration also removed (cleaner than repair).
//! - NEGATIVE: family includes axioms with non-uniform behavior;
//!   wholesale demote loses signal.
//!
//! Comparison points:
//! - Alpha-3+ (retract whole t_0): axioms stay registered globally
//! - Alpha-3+++ (detach 4 noise from t_0): axioms stay registered;
//!   theory keeps non-noise members
//! - Beta-2 (this slice): family demote retracts members globally
//!   AND from all containing theories — cleaner deletion
//!
//! Captured to `logs/<date>_phase_beta_2_family_demote.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    }, R, RSet,
};
use std::collections::HashSet;

const TICKS_PHASE_0: u64 = 1000;
const TICKS_AFTER: u64 = 1000;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const SHAPE_MIN_MEMBERS: usize = 2;
/// Family qualifies for demote iff:
///   mean cross-precision < FAMILY_DEMOTE_MEAN_THRESHOLD
///   AND variance < FAMILY_VARIANCE_THRESHOLD
/// The variance gate enforces the "uniform-low" structural signature
/// (Beta-1 noise family had variance=0). Mixed families are excluded.
const FAMILY_DEMOTE_MEAN_THRESHOLD: f64 = 0.65;
const FAMILY_VARIANCE_THRESHOLD: f64 = 0.05;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
use relatum_v2::test_substrates::oq1::build_long_stream;

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

fn axiom_mean_cross_precision(
    ax_id: &str,
    substrates: &[RSet],
) -> Option<f64> {
    let single = vec![ax_id.to_string()];
    let mut sum = 0.0;
    let mut count = 0;
    for sub in substrates {
        let actual: HashSet<R> = sub.iter().cloned().collect();
        let predicted = predict_all_axioms(sub, &single);
        if let Some(p) = precision(&predicted, &actual) {
            sum += p;
            count += 1;
        }
    }
    if count == 0 {
        None
    } else {
        Some(sum / count as f64)
    }
}

#[derive(Clone)]
struct TheoryReport {
    id: String,
    axiom_count: usize,
    qualifying_axioms: usize,
    aggregated_hit_rate: Option<f64>,
}

fn rank_theories(rt: &AutonomousRuntime) -> Vec<TheoryReport> {
    let theories: Vec<&str> = rt.rset.theories();
    let mut reports = Vec::new();
    for t in theories {
        let axioms: Vec<&str> = rt.rset.theory_axioms(t);
        let mut sum: f64 = 0.0;
        let mut qualifying: usize = 0;
        for ax in &axioms {
            if let Some(rate) = rt
                .memory
                .prediction_state
                .hit_rate(ax, MIN_AXIOM_PREDICTIONS)
            {
                sum += rate;
                qualifying += 1;
            }
        }
        let aggregated = if qualifying > 0 {
            Some(sum / qualifying as f64)
        } else {
            None
        };
        reports.push(TheoryReport {
            id: t.to_string(),
            axiom_count: axioms.len(),
            qualifying_axioms: qualifying,
            aggregated_hit_rate: aggregated,
        });
    }
    reports.sort_by(|a, b| match (a.aggregated_hit_rate, b.aggregated_hit_rate) {
        (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    reports
}

fn print_tournament(label: &str, reports: &[TheoryReport]) {
    println!();
    println!("=== Tournament: {} ===", label);
    if reports.is_empty() {
        println!("  (no theories)");
        return;
    }
    println!(
        "{:>4} {:>30} {:>5} {:>10} {:>15}",
        "rank", "theory_id", "axs", "qualifying", "agg_hit_rate",
    );
    for (i, r) in reports.iter().enumerate() {
        let hr_str = match r.aggregated_hit_rate {
            Some(x) => format!("{:.4}", x),
            None => "—".to_string(),
        };
        println!(
            "{:>4} {:>30} {:>5} {:>10} {:>15}",
            i + 1, r.id, r.axiom_count, r.qualifying_axioms, hr_str,
        );
    }
}

fn aggregate_stats(reports: &[TheoryReport]) -> (usize, f64, f64) {
    let qual: Vec<f64> = reports.iter().filter_map(|r| r.aggregated_hit_rate).collect();
    let mean = if qual.is_empty() {
        0.0
    } else {
        qual.iter().sum::<f64>() / qual.len() as f64
    };
    let min = qual.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    (
        qual.len(),
        mean,
        if min == f64::INFINITY { 0.0 } else { min },
    )
}

fn main() {
    println!(
        "=== Phase Beta-2 — Family-level demote (mean<{:.2} AND var<{:.4}) ===",
        FAMILY_DEMOTE_MEAN_THRESHOLD, FAMILY_VARIANCE_THRESHOLD,
    );

    // ── Phase 0: discover ───────────────────────────────────────
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let pre = rank_theories(&rt);
    print_tournament("Phase 0 (primary stream)", &pre);
    let (init_q, init_m, init_n) = aggregate_stats(&pre);

    // ── Beta-1: discover families ────────────────────────────────
    // Idempotent call — runtime may have already auto-dispatched
    // DiscoverAxiomShapeFamilies via the scheduler (B.5.1 wiring).
    let _ = rt.rset.discover_axiom_shape_families(SHAPE_MIN_MEMBERS);
    let minted: Vec<String> = rt
        .rset
        .axiom_shape_families()
        .into_iter()
        .map(str::to_owned)
        .collect();
    println!();
    println!("=== Beta-1 step: {} shape families present ===", minted.len());
    for shape in &minted {
        let members = rt.rset.shape_family_members(shape);
        println!("  {}: {} members", shape, members.len());
    }

    if minted.is_empty() {
        println!("  → INAPPLICABLE — no families present");
        return;
    }

    // ── Compute per-family cross-precision ───────────────────────
    let theories_pre: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    let axioms_pre: Vec<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();
    let all_axiom_ids: HashSet<String> = axioms_pre.iter().cloned().collect();

    // Determinism: sort theory ids before generating substrates so
    // seed assignment is stable across runs (theories() returns
    // HashMap-derived order). Beta-1 didn't have this and run-to-run
    // numerics drifted slightly.
    let mut theories_sorted = theories_pre.clone();
    theories_sorted.sort();
    let mut substrates: Vec<RSet> = Vec::with_capacity(theories_sorted.len());
    for (i, t) in theories_sorted.iter().enumerate() {
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

    // For each family, compute mean + variance of cross-precision
    // over its members. Demote if BOTH (mean < threshold) AND
    // (variance < ε) — uniform-low signature.
    println!();
    println!("=== Family cross-precision ===");
    println!(
        "{:>32} {:>5} {:>10} {:>10} {:>10}",
        "family", "n", "mean", "var", "flag",
    );
    let mut families_to_demote: Vec<(String, Vec<String>, f64, f64)> = Vec::new();
    for shape in &minted {
        let members: Vec<String> = rt
            .rset
            .shape_family_members(shape)
            .into_iter()
            .map(str::to_owned)
            .collect();
        let mut values: Vec<f64> = Vec::new();
        for ax in &members {
            if let Some(p) = axiom_mean_cross_precision(ax, &substrates) {
                values.push(p);
            }
        }
        let mean = if values.is_empty() {
            0.0
        } else {
            values.iter().sum::<f64>() / values.len() as f64
        };
        let var = if values.is_empty() {
            0.0
        } else {
            values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                / values.len() as f64
        };
        let qualifies = !values.is_empty()
            && mean < FAMILY_DEMOTE_MEAN_THRESHOLD
            && var < FAMILY_VARIANCE_THRESHOLD;
        let flag = if qualifies { "← DEMOTE" } else { "" };
        println!(
            "{:>32} {:>5} {:>10.4} {:>10.6} {:>10}",
            shape, members.len(), mean, var, flag,
        );
        if qualifies {
            families_to_demote.push((shape.clone(), members, mean, var));
        }
    }

    drop(substrates);

    if families_to_demote.is_empty() {
        println!();
        println!("=== Verdict ===");
        println!(
            "  → INAPPLICABLE — no family with mean<{:.2} AND var<{:.4}",
            FAMILY_DEMOTE_MEAN_THRESHOLD, FAMILY_VARIANCE_THRESHOLD,
        );
        return;
    }

    // ── Beta-2: family-level demote (via ADR 0070 Step 2 lib API)
    println!();
    println!("=== Beta-2: family-level demote (via retract_shape_family) ===");
    let mut total_detached_memberships = 0usize;
    let mut total_retracted_axioms = 0usize;
    for (shape, members, mean, var) in &families_to_demote {
        println!();
        println!(
            "  Family {} (mean={:.4} var={:.6}, {} members):",
            shape, mean, var, members.len(),
        );
        match rt.rset.retract_shape_family(shape) {
            Ok(summary) => {
                total_detached_memberships += summary.theory_memberships_detached;
                total_retracted_axioms += summary.axioms_globally_retracted;
                println!(
                    "    layer={:?}, detached {} theory memberships, retracted {} axioms, removed {} structural edges",
                    summary.layer,
                    summary.theory_memberships_detached,
                    summary.axioms_globally_retracted,
                    summary.structural_edges_removed,
                );
            }
            Err(e) => {
                println!("    ✗ retract_shape_family failed: {:?}", e);
            }
        }
    }
    println!();
    println!(
        "  Detached {} (theory, axiom) memberships; globally retracted {} axioms.",
        total_detached_memberships, total_retracted_axioms,
    );

    // ── Run continuation ────────────────────────────────────────
    rt.run_bounded(TICKS_AFTER);
    let post = rank_theories(&rt);
    print_tournament("After family-level demote + 1000 ticks", &post);
    let (q, m, n) = aggregate_stats(&post);

    // ── Comparison ──────────────────────────────────────────────
    println!();
    println!("=== Comparison vs prior baselines ===");
    println!(
        "  Phase 0:                          mean={:.4} min={:.4} qual={}",
        init_m, init_n, init_q,
    );
    println!(
        "  Alpha-3+ (whole t_0 demote):      mean=0.8401 min=0.6664 qual=3 (reference)",
    );
    println!(
        "  Alpha-3+++ (4-axiom repair):      mean=0.7967 min=0.6664 qual=4 (reference)",
    );
    println!(
        "  Beta-2 (family demote):           mean={:.4} min={:.4} qual={}",
        m, n, q,
    );
    println!(
        "  Beta-2 axioms remaining:          {} (was {} pre-Beta-2)",
        rt.rset.axioms().len(), axioms_pre.len(),
    );

    // Verdict
    println!();
    println!("=== Verdict ===");
    let matches_repair = (m - 0.7967).abs() < 1e-3 && (n - 0.6664).abs() < 1e-3 && q == 4;
    let matches_demote = (m - 0.8401).abs() < 1e-3 && (n - 0.6664).abs() < 1e-3 && q == 3;
    let target_above_threshold = post
        .iter()
        .all(|r| match r.aggregated_hit_rate {
            Some(v) => v >= 0.50, // primary-rate demote threshold for "healthy theory"
            None => true,
        });
    let cleaner_than_repair = total_retracted_axioms > 0;
    if target_above_threshold && cleaner_than_repair {
        if matches_repair {
            println!("  → POSITIVE — family demote matches Alpha-3+++ repair on aggregate AND retracts axiom global registrations (cleaner deletion)");
        } else if matches_demote {
            println!("  → POSITIVE — family demote matches Alpha-3+ aggregate AND retracts only the noise axioms (more precise than whole-theory demote)");
        } else {
            println!("  → POSITIVE-NEW — different end state from prior baselines, but all theories above threshold; structurally distinct intervention");
        }
    } else if !target_above_threshold {
        println!("  → MIXED — some theories below threshold post-demote");
    } else {
        println!("  → INCONCLUSIVE");
    }

    println!();
    println!("--- end ---");
}
