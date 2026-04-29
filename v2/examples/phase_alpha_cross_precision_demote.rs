//! Phase Alpha-8 — Cross-precision-driven demote (ADR 0066 follow-up
//! to Alpha-7).
//!
//! Alpha-7 showed cross-precision (DreamCoder-style imagined
//! substrate cross-validation) is a theory-quality signal
//! INDEPENDENT of primary-stream hit rate. This slice asks the next
//! question: can we *make decisions* using only cross-precision,
//! without consulting the primary stream's hit-rate counters?
//!
//! Method:
//! 1. Phase 0: 1000 ticks on OQ#1 to discover theories.
//! 2. Dream phase: generate substrates per theory + register all
//!    theories' axioms in every substrate.
//! 3. Compute cross-precision matrix (column means).
//! 4. Demote the theory with the lowest column mean — i.e., the
//!    theory whose axioms make the most false predictions across
//!    imagined substrates.
//! 5. Run another 1000 ticks; report tournament.
//!
//! Comparison: Alpha-3+ demoted t_0 by primary-stream hit rate
//! (0.3757). If Alpha-8 also demotes t_0, the post-state should be
//! byte-identical (same demote target → same downstream effects on
//! the deterministic stream).
//!
//! Falsifiable hypotheses:
//! - POSITIVE: Alpha-8 picks t_0 → matches Alpha-3+ verdict via
//!   independent signal source.
//! - DIVERGENT: Alpha-8 picks a different theory → cross-precision
//!   and primary-stream hit rate disagree.
//!
//! Captured to `logs/<date>_phase_alpha_cross_precision_demote.log`.

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
const MIN_AXIOM_PREDICTIONS: u64 = 5;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
use relatum_v2::test_substrates::oq1::build_long_stream;

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
            i + 1,
            r.id,
            r.axiom_count,
            r.qualifying_axioms,
            hr_str,
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

/// Forward-apply every axiom in `axiom_ids` on `substrate`, return
/// the union of predicted edges.
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
    println!(
        "=== ADR 0066 Phase Alpha-8 — Cross-precision-driven demote ({} ticks Phase 0 + {} after) ===",
        TICKS_PHASE_0, TICKS_AFTER,
    );
    println!(
        "NUM_GEN_IDS={}, SEED_DENSITY={}",
        NUM_GEN_IDS, SEED_DENSITY,
    );

    // ── Phase 0 (in own scope; release between phases for memory) ────
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    // Snapshot pre-demote tournament (for comparison only — Alpha-8
    // does NOT use this for its decision).
    let pre = rank_theories(&rt);
    print_tournament("Phase 0 (primary-stream view, REFERENCE ONLY)", &pre);
    let (init_q, init_m, init_n) = aggregate_stats(&pre);

    // ── Dream phase: build cross-precision matrix ────────────────────
    let theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
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

    println!();
    println!("=== Dream phase: generate substrate per theory + register all axioms ===");
    let mut substrates: Vec<RSet> = Vec::with_capacity(theories.len());
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut gen = rt
            .rset
            .generate_substrate_from_theory(t, NUM_GEN_IDS, SEED_DENSITY, seed)
            .expect("generate_substrate_from_theory");
        for ax in &all_axiom_ids {
            gen.register_axiom_with_intension(ax);
        }
        println!(
            "  {} → substrate with {} edges, {} identifiers",
            t,
            gen.len(),
            gen.identifiers().len(),
        );
        substrates.push(gen);
    }

    // Compute cross-precision matrix and column means.
    println!();
    println!("=== Cross-precision matrix (rows: substrate; cols: forward-applying theory) ===");
    print!("{:>10}", "");
    for t in &theories {
        print!("{:>10}", t);
    }
    println!();

    let mut col_sum = vec![0.0; theories.len()];
    let mut col_count = vec![0usize; theories.len()];
    let mut col_min = vec![f64::INFINITY; theories.len()];

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
                        if v < col_min[j] {
                            col_min[j] = v;
                        }
                    }
                }
                None => print!("{:>10}", "—"),
            }
        }
        println!();
    }

    // Pick demote target by lowest column mean (excluding diagonal).
    println!();
    println!("=== Per-theory generality (column means, excluding diagonal) ===");
    println!(
        "{:>10} {:>15} {:>15}",
        "theory", "mean_precision", "min_precision",
    );
    let mut worst: Option<(String, f64)> = None;
    for j in 0..theories.len() {
        if col_count[j] == 0 {
            println!("{:>10} {:>15} {:>15}", theories[j], "—", "—");
            continue;
        }
        let mean = col_sum[j] / col_count[j] as f64;
        let min_v = if col_min[j] == f64::INFINITY {
            0.0
        } else {
            col_min[j]
        };
        println!(
            "{:>10} {:>15.4} {:>15.4}",
            theories[j], mean, min_v,
        );
        let is_lower = match &worst {
            None => true,
            Some((_, w)) => mean < *w,
        };
        if is_lower {
            worst = Some((theories[j].clone(), mean));
        }
    }

    let demote_target = match worst {
        Some((id, mean)) => {
            println!();
            println!(
                "--- Cross-precision demote pick: '{}' (column mean={:.4}) ---",
                id, mean,
            );
            Some(id)
        }
        None => {
            println!();
            println!("--- No demote candidate ---");
            None
        }
    };

    // Compare to primary-stream verdict for diagnostic only.
    let primary_bottom = pre
        .iter()
        .filter(|r| r.aggregated_hit_rate.is_some())
        .last()
        .map(|r| (r.id.clone(), r.aggregated_hit_rate.unwrap()));
    if let (Some(target), Some((primary_id, primary_rate))) =
        (&demote_target, &primary_bottom)
    {
        println!();
        println!(
            "  Primary-stream bottom (REFERENCE): '{}' (rate {:.4})",
            primary_id, primary_rate,
        );
        if target == primary_id {
            println!("  ✓ AGREE: cross-precision and primary-stream pick the same theory");
        } else {
            println!(
                "  ✗ DIVERGE: cross-precision picks '{}', primary-stream picks '{}'",
                target, primary_id,
            );
        }
    }

    // ── Apply demote (drop substrates first to free memory) ─────────
    drop(substrates);

    if let Some(id) = &demote_target {
        let removed = rt.rset.retract_theory(id).expect("retract_theory");
        println!();
        println!(
            "--- Retracted '{}' — {} edges removed ---",
            id, removed,
        );
    }

    rt.run_bounded(TICKS_AFTER);
    let post = rank_theories(&rt);
    print_tournament("After cross-precision demote + 1000 ticks", &post);
    let (q_after, m_after, n_after) = aggregate_stats(&post);

    // ── Comparison vs known Alpha-3+ baseline ────────────────────────
    println!();
    println!("=== Comparison vs Alpha-3+ baseline (primary-stream demote) ===");
    println!(
        "  Phase 0:                  mean={:.4} min={:.4} qual={}",
        init_m, init_n, init_q,
    );
    println!(
        "  Alpha-3+ post-demote:     mean=0.8401 min=0.6664 qual=3 (reference)",
    );
    println!(
        "  Alpha-8 post-demote:      mean={:.4} min={:.4} qual={}",
        m_after, n_after, q_after,
    );

    let matches_alpha3 = (m_after - 0.8401).abs() < 1e-3
        && (n_after - 0.6664).abs() < 1e-3
        && q_after == 3;

    println!();
    println!("=== Verdict ===");
    if let (Some(target), Some((primary_id, _))) = (&demote_target, &primary_bottom) {
        if target == primary_id {
            if matches_alpha3 {
                println!("  → POSITIVE — cross-precision picked the same target as primary-stream rate AND the resulting state matches Alpha-3+ baseline byte-identically. Cross-precision is a sufficient demote signal on OQ#1 without consulting primary-stream hit rates.");
            } else {
                println!("  → MIXED — cross-precision picked the same target but resulting state diverges from Alpha-3+ (numerical drift?).");
            }
        } else {
            println!(
                "  → DIVERGENT — cross-precision picked '{}', primary-stream picks '{}'. Investigate the cross-precision metric.",
                target, primary_id,
            );
        }
    } else {
        println!("  → INAPPLICABLE — missing data");
    }

    println!();
    println!("--- end ---");
}
