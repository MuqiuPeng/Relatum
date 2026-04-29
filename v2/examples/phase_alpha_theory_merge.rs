//! Phase Alpha-3++++ — theory deduplication / merge as a third
//! intervention (ADR 0066 follow-up to Alpha-3+/3++/3+++).
//!
//! Phase Alpha-3+++ surfaced the surprise finding: on OQ#1, t_0 and
//! t_1 are *functionally equivalent* on their qualifying axiom set
//! (both reach 0.6664 hit rate after intervention). Demote works
//! not because t_0 is bad, but because its good core is redundantly
//! captured by t_1.
//!
//! This slice tries the natural third intervention: instead of
//! demote-or-repair, *merge* the redundant pair into one theory
//! whose member set is the union. Concept-lattice / FCA-style
//! consolidation.
//!
//! Three paths from byte-identical Phase 0 (deterministic stream):
//!
//!   Path A (demote, Alpha-3+ baseline):
//!     run 1000 → retract_theory(bottom) → run 1000
//!
//!   Path B (repair, Alpha-3+++ baseline):
//!     run 1000 → retract_theory_member(bottom, noise_axs) → run 1000
//!
//!   Path C (merge, treatment):
//!     run 1000 → merge_theories(top_jaccard_pair) → run 1000
//!
//! Success criterion (positive):
//!   - Path C's merged theory hit rate ≥ DEMOTE_THRESHOLD
//!   - Path C mean ≥ Path A mean (or qualifying ≥ Path A's)
//!   - Path C total qualifying axioms ≥ Path A total
//!
//! Inconclusive: no theory pair has Jaccard ≥ MERGE_JACCARD_FLOOR;
//! merge has nothing to bite on.
//!
//! Negative: merging mixes good axioms with noise (because Jaccard
//! considers all members not just qualifying ones), and the merged
//! theory's hit rate drops below threshold.
//!
//! Captured to `logs/<date>_phase_alpha_theory_merge.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    }, RSet,
};
use std::collections::HashSet;

const TICKS_PER_PHASE: u64 = 1000;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
const DEMOTE_THRESHOLD: f64 = 0.50;
const REPAIR_AXIOM_THRESHOLD: f64 = 0.20;
/// Minimum Jaccard similarity (computed on full member sets) to
/// consider a pair as a merge candidate. 0.50 means the pair must
/// share at least half of their union.
const MERGE_JACCARD_FLOOR: f64 = 0.30;
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

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 { 0.0 } else { inter as f64 / union as f64 }
}

fn pick_merge_candidates(rt: &AutonomousRuntime) -> Option<(String, String, f64)> {
    let theories: Vec<String> = rt.rset.theories().into_iter().map(str::to_owned).collect();
    let n = theories.len();
    if n < 2 { return None; }
    let mut member_sets: Vec<HashSet<String>> = Vec::with_capacity(n);
    for t in &theories {
        let s: HashSet<String> = rt.rset.theory_axioms(t).into_iter().map(str::to_owned).collect();
        member_sets.push(s);
    }
    let mut best: Option<(usize, usize, f64)> = None;
    for i in 0..n {
        for j in (i + 1)..n {
            let j_score = jaccard(&member_sets[i], &member_sets[j]);
            match best {
                None => best = Some((i, j, j_score)),
                Some((_, _, prev)) if j_score > prev => best = Some((i, j, j_score)),
                _ => {}
            }
        }
    }
    best.and_then(|(i, j, s)| {
        if s >= MERGE_JACCARD_FLOOR {
            Some((theories[i].clone(), theories[j].clone(), s))
        } else {
            None
        }
    })
}

fn print_jaccard_matrix(rt: &AutonomousRuntime) {
    let theories: Vec<String> = rt.rset.theories().into_iter().map(str::to_owned).collect();
    let n = theories.len();
    let member_sets: Vec<HashSet<String>> = theories
        .iter()
        .map(|t| {
            rt.rset
                .theory_axioms(t)
                .into_iter()
                .map(str::to_owned)
                .collect()
        })
        .collect();
    println!();
    println!("--- Pairwise Jaccard similarity (full member sets) ---");
    print!("{:>6}", "");
    for t in &theories {
        print!("{:>10}", t);
    }
    println!();
    for i in 0..n {
        print!("{:>6}", theories[i]);
        for j in 0..n {
            if i == j {
                print!("{:>10}", "—");
            } else {
                let s = jaccard(&member_sets[i], &member_sets[j]);
                print!("{:>10.4}", s);
            }
        }
        println!();
    }
}

fn main() {
    println!(
        "=== ADR 0066 Phase Alpha-3++++ — theory dedup/merge vs demote vs repair ({} ticks per phase) ===",
        TICKS_PER_PHASE,
    );
    println!(
        "DEMOTE_THRESHOLD={}, REPAIR_AXIOM_THRESHOLD={}, MERGE_JACCARD_FLOOR={}",
        DEMOTE_THRESHOLD, REPAIR_AXIOM_THRESHOLD, MERGE_JACCARD_FLOOR,
    );

    // -------- Path A: demote --------
    println!();
    println!("############### Path A: demote (Alpha-3+ baseline) ###############");
    let mut rt_a = fresh_runtime();
    rt_a.run_bounded(TICKS_PER_PHASE);
    let pre_a = rank_theories(&rt_a);
    print_tournament("A:Phase 0", &pre_a);
    print_jaccard_matrix(&rt_a);
    let bottom_a = pick_bottom(&pre_a);
    let (init_qa, init_ma, init_na) = aggregate_stats(&pre_a);
    let demoted_id = match &bottom_a {
        Some((id, rate)) if *rate < DEMOTE_THRESHOLD => {
            let removed = rt_a.rset.retract_theory(id).expect("retract_theory");
            println!();
            println!("--- A: retract '{}' (rate {:.4}) — {} edges removed ---", id, rate, removed);
            Some(id.clone())
        }
        _ => None,
    };
    rt_a.run_bounded(TICKS_PER_PHASE);
    let post_a = rank_theories(&rt_a);
    print_tournament("A: after demote + 1000 ticks", &post_a);
    let (qa, ma, na) = aggregate_stats(&post_a);

    // -------- Path B: repair --------
    println!();
    println!("############### Path B: repair (Alpha-3+++ baseline) ###############");
    let mut rt_b = fresh_runtime();
    rt_b.run_bounded(TICKS_PER_PHASE);
    let pre_b = rank_theories(&rt_b);
    let bottom_b = pick_bottom(&pre_b);
    let repaired_axioms: Vec<String> = match &bottom_b {
        Some((id, rate)) if *rate < DEMOTE_THRESHOLD => {
            let mut to_detach: Vec<String> = Vec::new();
            for (ax, ax_rate, _) in axiom_rates_in_theory(&rt_b, id) {
                if let Some(r) = ax_rate {
                    if r < REPAIR_AXIOM_THRESHOLD {
                        to_detach.push(ax);
                    }
                }
            }
            println!();
            println!(
                "--- B: theory '{}' (rate {:.4}) — detach {} axioms below {} ---",
                id, rate, to_detach.len(), REPAIR_AXIOM_THRESHOLD,
            );
            for ax in &to_detach {
                rt_b.rset.retract_theory_member(id, ax).expect("retract_member");
            }
            to_detach
        }
        _ => Vec::new(),
    };
    rt_b.run_bounded(TICKS_PER_PHASE);
    let post_b = rank_theories(&rt_b);
    print_tournament("B: after repair + 1000 ticks", &post_b);
    let (qb, mb, nb) = aggregate_stats(&post_b);

    // -------- Path C: merge --------
    println!();
    println!("############### Path C: merge (treatment) ###############");
    let mut rt_c = fresh_runtime();
    rt_c.run_bounded(TICKS_PER_PHASE);
    let pre_c = rank_theories(&rt_c);
    print_tournament("C:Phase 0", &pre_c);
    let merge_pair = pick_merge_candidates(&rt_c);
    let merged_info = match &merge_pair {
        Some((a, b, score)) => {
            println!();
            println!(
                "--- C: merge candidates '{}' + '{}' (Jaccard={:.4}) ---",
                a, b, score,
            );
            print_axiom_breakdown("C: candidate A", &rt_c, a);
            print_axiom_breakdown("C: candidate B", &rt_c, b);
            let merged_id = rt_c.rset.merge_theories(a, b).expect("merge_theories");
            println!();
            println!("--- C: merged into '{}' ---", merged_id);
            print_axiom_breakdown("C: merged theory", &rt_c, &merged_id);
            Some((a.clone(), b.clone(), *score, merged_id))
        }
        None => {
            println!();
            println!(
                "--- C: no theory pair has Jaccard >= {} — nothing to merge ---",
                MERGE_JACCARD_FLOOR,
            );
            None
        }
    };
    rt_c.run_bounded(TICKS_PER_PHASE);
    let post_c = rank_theories(&rt_c);
    print_tournament("C: after merge + 1000 ticks", &post_c);
    let (qc, mc, nc) = aggregate_stats(&post_c);

    // -------- Sanity --------
    assert_eq!(
        pre_a.iter().map(|r| (r.id.clone(), r.axiom_count)).collect::<Vec<_>>(),
        pre_b.iter().map(|r| (r.id.clone(), r.axiom_count)).collect::<Vec<_>>(),
        "Phase 0 must match across paths",
    );
    assert_eq!(
        pre_a.iter().map(|r| (r.id.clone(), r.axiom_count)).collect::<Vec<_>>(),
        pre_c.iter().map(|r| (r.id.clone(), r.axiom_count)).collect::<Vec<_>>(),
        "Phase 0 must match across paths",
    );

    // -------- Comparison --------
    println!();
    println!("=== Comparison ===");
    println!("{:>30} {:>10} {:>10} {:>5}", "phase", "mean", "min", "qual");
    println!("{:>30} {:>10.4} {:>10.4} {:>5}", "Phase 0 (all paths)", init_ma, init_na, init_qa);
    println!("{:>30} {:>10.4} {:>10.4} {:>5}", "A: demote", ma, na, qa);
    println!("{:>30} {:>10.4} {:>10.4} {:>5}", "B: repair", mb, nb, qb);
    println!("{:>30} {:>10.4} {:>10.4} {:>5}", "C: merge", mc, nc, qc);

    println!();
    println!("=== Verdict ===");
    let demoted_str = demoted_id.clone().unwrap_or_else(|| "—".to_string());
    println!("  A: demoted          : {}", demoted_str);
    println!("  B: repaired axioms  : {} detached", repaired_axioms.len());
    if let Some((a, b, score, mid)) = &merged_info {
        println!("  C: merged           : '{}' + '{}' (Jaccard={:.4}) → '{}'", a, b, score, mid);
        // Find merged theory's post rate
        let merged_post = post_c.iter().find(|r| &r.id == mid);
        if let Some(r) = merged_post {
            let hr = r.aggregated_hit_rate.map(|x| format!("{:.4}", x)).unwrap_or_else(|| "—".to_string());
            println!("  C: merged post-rate : {} (qualifying={})", hr, r.qualifying_axioms);
        }
    } else {
        println!("  C: merged           : (no candidate)");
    }

    // Verdict classifier
    let merge_target_rate = merged_info
        .as_ref()
        .and_then(|(_, _, _, mid)| post_c.iter().find(|r| &r.id == mid))
        .and_then(|r| r.aggregated_hit_rate);
    let verdict = match (&merged_info, merge_target_rate) {
        (None, _) => "INAPPLICABLE — no pair met Jaccard floor".to_string(),
        (Some(_), None) => "INCONCLUSIVE — merged theory has no qualifying axiom".to_string(),
        (Some(_), Some(rate)) => {
            let rate_ok = rate >= DEMOTE_THRESHOLD;
            let mean_at_least_demote = mc >= ma - 1e-9;
            let qual_at_least_demote = qc >= qa;
            let mean_at_least_repair = mc >= mb - 1e-9;
            if rate_ok && (mean_at_least_demote || qual_at_least_demote) {
                if mean_at_least_repair {
                    "POSITIVE — merge matches/beats both demote and repair".to_string()
                } else {
                    "PARTIAL — merge meets threshold but mean < repair".to_string()
                }
            } else if !rate_ok {
                format!(
                    "NEGATIVE — merged theory rate {:.4} < threshold {} (mixed good and noise?)",
                    rate, DEMOTE_THRESHOLD,
                )
            } else {
                "MIXED — merged passes threshold but underperforms demote on mean+qual".to_string()
            }
        }
    };
    println!("  → {}", verdict);

    println!();
    println!("--- end ---");
}
