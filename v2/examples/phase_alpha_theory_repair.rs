//! Phase Alpha-3+++ — counterexample-guided theory repair vs.
//! whole-theory demotion (ADR 0066 follow-up to Alpha-3+ / Alpha-3++).
//!
//! Phase Alpha-3+ established that demoting the lowest-rated theory
//! improves measured aggregate quality. Phase Alpha-3++ established
//! that the demote loop converges in one iteration on OQ#1.
//!
//! This slice asks the next natural question, borrowing the
//! counterexample-guided specialization paradigm from ILP
//! (FOIL / PROGOL): instead of removing the entire theory, can we
//! repair it by detaching only the failing axioms?
//!
//! Empirical hypothesis:
//!   On OQ#1, t_0 contains 10 axioms whose hit rates range from
//!   0.04 to 1.00. Demote removes all 10 (the 1.0 axiom survives
//!   only because t_2 also references it). Repair would keep t_0
//!   with the good axioms still attributed to it.
//!
//! Two paths from byte-identical Phase 0 state (deterministic stream):
//!
//!   Path A (demote, control)   = Phase Alpha-3+ baseline
//!     run 1000 → retract_theory(bottom) → run 1000 → tournament
//!
//!   Path B (repair, treatment) = new
//!     run 1000 → retract_theory_member(bottom, ax) for each
//!     "counterexample axiom" (hit_rate < REPAIR_THRESHOLD with
//!     ≥ MIN_PREDICTIONS) → run 1000 → tournament
//!
//! Success criterion (positive):
//!   - Repair-path bottom theory's hit rate ≥ DEMOTE_THRESHOLD
//!   - Repair-path mean hit rate ≥ Demote-path mean
//!   - Repair-path qualifying count ≥ Demote-path qualifying count
//!
//! Inconclusive: paths agree (e.g., when t_0's good axioms are
//! already shared by t_2, the qualifying-set ends up identical).
//!
//! Negative: repair-path mean < demote-path mean.
//!
//! Captured to `logs/<date>_phase_alpha_theory_repair.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    }, RSet,
};

const TICKS_PER_PHASE: u64 = 1000;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
const DEMOTE_THRESHOLD: f64 = 0.50;
/// An axiom is a counterexample-class member if its hit rate falls
/// below this threshold (with ≥ MIN_AXIOM_PREDICTIONS evaluations).
/// Set lower than DEMOTE_THRESHOLD so we only detach clearly-noisy
/// axioms, not borderline ones.
const REPAIR_AXIOM_THRESHOLD: f64 = 0.20;
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
            if let Some(rate) = rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS) {
                sum += rate;
                qualifying += 1;
            }
        }
        let aggregated = if qualifying > 0 { Some(sum / qualifying as f64) } else { None };
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
    let mean = if qual.is_empty() { 0.0 } else { qual.iter().sum::<f64>() / qual.len() as f64 };
    let min = qual.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    (qual.len(), mean, if min == f64::INFINITY { 0.0 } else { min })
}

fn fresh_runtime() -> AutonomousRuntime {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

/// Per-axiom rates inside a theory, sorted ascending.
fn axiom_rates_in_theory(
    rt: &AutonomousRuntime,
    theory_id: &str,
) -> Vec<(String, Option<f64>, u64)> {
    let mut rows: Vec<(String, Option<f64>, u64)> = Vec::new();
    for ax in rt.rset.theory_axioms(theory_id) {
        let rate = rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS);
        let total = rt
            .memory
            .prediction_state
            .total_predictions_per_axiom
            .get(ax)
            .copied()
            .unwrap_or(0);
        rows.push((ax.to_string(), rate, total));
    }
    rows.sort_by(|a, b| match (a.1, b.1) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    });
    rows
}

fn print_axiom_breakdown(label: &str, rt: &AutonomousRuntime, theory_id: &str) {
    println!();
    println!("--- Axiom breakdown: {} ({}) ---", label, theory_id);
    println!(
        "{:>40} {:>10} {:>12}",
        "axiom_id", "hit_rate", "predictions",
    );
    for (ax, rate, total) in axiom_rates_in_theory(rt, theory_id) {
        let hr = match rate {
            Some(x) => format!("{:.4}", x),
            None => "—".to_string(),
        };
        println!("{:>40} {:>10} {:>12}", ax, hr, total);
    }
}

fn pick_bottom(reports: &[TheoryReport]) -> Option<(String, f64)> {
    reports
        .iter()
        .filter(|r| r.aggregated_hit_rate.is_some())
        .last()
        .map(|r| (r.id.clone(), r.aggregated_hit_rate.unwrap()))
}

fn main() {
    println!(
        "=== ADR 0066 Phase Alpha-3+++ — counterexample-guided theory repair vs demote ({} ticks per phase) ===",
        TICKS_PER_PHASE,
    );
    println!(
        "DEMOTE_THRESHOLD={}, REPAIR_AXIOM_THRESHOLD={}",
        DEMOTE_THRESHOLD, REPAIR_AXIOM_THRESHOLD,
    );

    // -------- Path A: demote control --------
    println!();
    println!("############### Path A: demote (control, = Alpha-3+) ###############");
    let mut rt_a = fresh_runtime();
    rt_a.run_bounded(TICKS_PER_PHASE);
    println!(
        "tick={} episodes={} theories={} axioms={}",
        rt_a.tick,
        rt_a.memory.episodes.len(),
        rt_a.rset.theories().len(),
        rt_a.rset.axioms().len(),
    );
    let pre_a = rank_theories(&rt_a);
    print_tournament("A:Phase 0 (initial)", &pre_a);
    let bottom_a = pick_bottom(&pre_a);
    let (init_qa, init_ma, init_na) = aggregate_stats(&pre_a);

    let demoted_id = match &bottom_a {
        Some((id, rate)) if *rate < DEMOTE_THRESHOLD => {
            print_axiom_breakdown("A:bottom theory before demote", &rt_a, id);
            let removed = rt_a.rset.retract_theory(id).expect("retract_theory");
            println!();
            println!(
                "--- A: retract whole theory '{}' (rate {:.4}) — {} edges removed ---",
                id, rate, removed,
            );
            Some(id.clone())
        }
        _ => {
            println!("--- A: no theory below threshold; nothing to demote ---");
            None
        }
    };
    rt_a.run_bounded(TICKS_PER_PHASE);
    let post_a = rank_theories(&rt_a);
    print_tournament("A: after demote + 1000 ticks", &post_a);
    let (qa, ma, na) = aggregate_stats(&post_a);

    // -------- Path B: repair treatment --------
    println!();
    println!("############### Path B: repair (treatment) ###############");
    let mut rt_b = fresh_runtime();
    rt_b.run_bounded(TICKS_PER_PHASE);
    println!(
        "tick={} episodes={} theories={} axioms={}",
        rt_b.tick,
        rt_b.memory.episodes.len(),
        rt_b.rset.theories().len(),
        rt_b.rset.axioms().len(),
    );
    let pre_b = rank_theories(&rt_b);
    print_tournament("B:Phase 0 (initial)", &pre_b);
    let bottom_b = pick_bottom(&pre_b);

    // Sanity: paths should diverge only after intervention.
    assert_eq!(
        pre_a.iter().map(|r| (r.id.clone(), r.axiom_count)).collect::<Vec<_>>(),
        pre_b.iter().map(|r| (r.id.clone(), r.axiom_count)).collect::<Vec<_>>(),
        "deterministic stream should produce identical Phase 0 theory shape",
    );

    let repaired_axioms: Vec<String> = match &bottom_b {
        Some((id, rate)) if *rate < DEMOTE_THRESHOLD => {
            print_axiom_breakdown("B:bottom theory before repair", &rt_b, id);
            // Counterexample-guided detach: pick axioms below
            // REPAIR_AXIOM_THRESHOLD with sufficient predictions.
            let mut to_detach: Vec<String> = Vec::new();
            for (ax, ax_rate, _total) in axiom_rates_in_theory(&rt_b, id) {
                if let Some(r) = ax_rate {
                    if r < REPAIR_AXIOM_THRESHOLD {
                        to_detach.push(ax);
                    }
                }
            }
            println!();
            println!(
                "--- B: theory '{}' (agg rate {:.4}) — repair will detach {} axioms below {} ---",
                id, rate, to_detach.len(), REPAIR_AXIOM_THRESHOLD,
            );
            for ax in &to_detach {
                let removed = rt_b
                    .rset
                    .retract_theory_member(id, ax)
                    .expect("retract_theory_member");
                println!("    detach '{}' from '{}' ({} edges)", ax, id, removed);
            }
            // Theory should still be registered.
            assert!(
                rt_b.rset.is_theory(id),
                "repair must leave theory registered",
            );
            to_detach
        }
        _ => {
            println!("--- B: no theory below threshold; nothing to repair ---");
            Vec::new()
        }
    };

    rt_b.run_bounded(TICKS_PER_PHASE);
    let post_b = rank_theories(&rt_b);
    print_tournament("B: after repair + 1000 ticks", &post_b);
    let (qb, mb, nb) = aggregate_stats(&post_b);

    // -------- Comparison --------
    println!();
    println!("=== Comparison ===");
    println!(
        "{:>30} {:>10} {:>10} {:>5}",
        "phase", "mean", "min", "qual",
    );
    println!(
        "{:>30} {:>10.4} {:>10.4} {:>5}",
        "Phase 0 (both, identical)", init_ma, init_na, init_qa,
    );
    println!(
        "{:>30} {:>10.4} {:>10.4} {:>5}",
        "Path A: demote (control)", ma, na, qa,
    );
    println!(
        "{:>30} {:>10.4} {:>10.4} {:>5}",
        "Path B: repair (treatment)", mb, nb, qb,
    );

    println!();
    println!("=== Verdict ===");
    let demoted_str = demoted_id.clone().unwrap_or_else(|| "—".to_string());
    println!("  demoted theory (A)        : {}", demoted_str);
    println!("  repaired axioms (B)       : {} detached", repaired_axioms.len());
    if let Some((id, _)) = &bottom_b {
        // After repair, what is t_id's current rate?
        let post_target = post_b.iter().find(|r| &r.id == id);
        match post_target {
            Some(r) => {
                let hr = r
                    .aggregated_hit_rate
                    .map(|x| format!("{:.4}", x))
                    .unwrap_or_else(|| "—".to_string());
                println!(
                    "  B: target theory '{}' post-repair: rate={} qualifying={}",
                    id, hr, r.qualifying_axioms,
                );
            }
            None => {
                println!("  B: target theory '{}' no longer present post-repair", id);
            }
        }
    }
    let mean_delta_b_minus_a = mb - ma;
    let min_delta_b_minus_a = nb - na;
    println!(
        "  Δ mean (B−A): {:+.4}; Δ min (B−A): {:+.4}; Δ qualifying (B−A): {}",
        mean_delta_b_minus_a,
        min_delta_b_minus_a,
        qb as i64 - qa as i64,
    );

    // Verdict classifier
    let verdict = if let Some((id, _)) = &bottom_b {
        let post_target_rate = post_b
            .iter()
            .find(|r| &r.id == id)
            .and_then(|r| r.aggregated_hit_rate);
        let target_above_threshold = post_target_rate
            .map(|x| x >= DEMOTE_THRESHOLD)
            .unwrap_or(false);
        let mean_at_least_a = mb >= ma - 1e-9;
        let qual_at_least_a = qb >= qa;
        if target_above_threshold && mean_at_least_a && qual_at_least_a {
            "POSITIVE — repair theory survives ≥ threshold and matches/beats demote"
        } else if (mb - ma).abs() < 1e-4 && qb == qa {
            "INCONCLUSIVE — repair and demote produce identical aggregate"
        } else if mb < ma {
            "NEGATIVE — repair underperforms demote on mean"
        } else {
            "MIXED — partial wins; see numbers"
        }
    } else {
        "INAPPLICABLE — no theory below threshold to intervene on"
    };
    println!("  → {}", verdict);

    println!();
    println!("--- end ---");
}
