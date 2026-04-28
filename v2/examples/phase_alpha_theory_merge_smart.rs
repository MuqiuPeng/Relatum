//! Phase Alpha-5 — smart merge candidate selection (ADR 0066 follow-up
//! to Alpha-3++++). Fixes the selection bias of the previous slice.
//!
//! Phase Alpha-3++++ picked the highest-Jaccard pair (t_0, t_1) at
//! 0.60 and got NEGATIVE because t_1 ⊂ t_0 (subset+noise). The
//! "highest Jaccard" heuristic is biased — subset relations tend to
//! produce high Jaccard scores. The right merge target is a pair with
//! *non-trivial unique members on both sides*.
//!
//! On OQ#1, the non-subset overlapping pair is (t_2, t_3) at Jaccard
//! 0.40. Both have unique members:
//!   - t_2 has 3 axioms, t_3 has 4 axioms
//!   - Their intersection is non-trivial but neither is a subset of
//!     the other
//!
//! This slice asks a different question than before:
//!   - demote/repair = "fix the bottom theory"
//!   - merge (smart) = "consolidate two GOOD theories that overlap"
//!
//! These are different interventions targeting different problems.
//! Comparison shows whether union-style merge has *any* positive
//! niche, even when not applied to subset+noise pairs.
//!
//! Captured to `logs/<date>_phase_alpha_theory_merge_smart.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet,
};
use std::collections::HashSet;

const TICKS_PER_PHASE: u64 = 1000;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
const DEMOTE_THRESHOLD: f64 = 0.50;
const REPAIR_AXIOM_THRESHOLD: f64 = 0.20;
/// Jaccard floor — only consider pairs with at least this much
/// overlap. Lower than Alpha-3++++ (0.30) because subset pairs are
/// already excluded; we want to consider any non-trivial overlap.
const MERGE_JACCARD_FLOOR: f64 = 0.20;

fn build_long_stream() -> Vec<(u64, Event)> {
    // Identical to OQ#1 4-regime substrate.
    let mut schedule = Vec::new();
    let regime_a_phases: [&[&str]; 5] = [
        &["a1", "a2", "a3", "a4"],
        &["a5", "a6", "a7", "a8"],
        &["a9", "a10", "a11", "a12"],
        &["a13", "a14", "a15", "a16"],
        &["a17", "a18", "a19", "a20"],
    ];
    for (i, ns) in regime_a_phases.iter().enumerate() {
        let off = 1 + (i as u64) * 100;
        for k in 0..4 {
            schedule.push((off + k as u64, Event::AddEdge(R::new(ns[k], ns[k]))));
        }
        schedule.push((off + 7, Event::AddEdge(R::new(ns[0], ns[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(ns[0], ns[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(ns[0], ns[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(ns[1], ns[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(ns[2], ns[3]))));
    }
    let regime_b_phases: [(&[&str], &[&str]); 5] = [
        (&["bL1", "bL2"], &["bR1", "bR2", "bR3"]),
        (&["bL3", "bL4"], &["bR4", "bR5", "bR6"]),
        (&["bL5", "bL6"], &["bR7", "bR8", "bR9"]),
        (&["bL7", "bL8"], &["bR10", "bR11", "bR12"]),
        (&["bL9", "bL10"], &["bR13", "bR14", "bR15"]),
    ];
    for (i, (lefts, rights)) in regime_b_phases.iter().enumerate() {
        let off = 501 + (i as u64) * 100;
        let mut t = 0u64;
        for l in lefts.iter() {
            for r in rights.iter() {
                schedule.push((off + t, Event::AddEdge(R::new(*l, *r))));
                t += 2;
            }
        }
    }
    let regime_c_phases: [&[&[&str]]; 5] = [
        &[&["c_a1", "c_a2"], &["c_b1", "c_b2", "c_b3"]],
        &[&["c_a3", "c_a4"], &["c_b4", "c_b5", "c_b6"]],
        &[&["c_a5", "c_a6"], &["c_b7", "c_b8", "c_b9"]],
        &[&["c_a7", "c_a8"], &["c_b10", "c_b11", "c_b12"]],
        &[&["c_a9", "c_a10"], &["c_b13", "c_b14", "c_b15"]],
    ];
    for (i, classes) in regime_c_phases.iter().enumerate() {
        let off = 1001 + (i as u64) * 100;
        let mut t = 0u64;
        for cls in classes.iter() {
            for x in cls.iter() {
                for y in cls.iter() {
                    schedule.push((off + t, Event::AddEdge(R::new(*x, *y))));
                    t += 1;
                }
            }
        }
    }
    let regime_d_phases: [&[&str]; 5] = [
        &["d1", "d2", "d3", "d4"],
        &["d5", "d6", "d7", "d8"],
        &["d9", "d10", "d11", "d12"],
        &["d13", "d14", "d15", "d16"],
        &["d17", "d18", "d19", "d20"],
    ];
    for (i, ns) in regime_d_phases.iter().enumerate() {
        let off = 1501 + (i as u64) * 100;
        for k in 0..4 {
            schedule.push((off + k as u64, Event::AddEdge(R::new(ns[k], ns[k]))));
        }
        schedule.push((off + 5, Event::AddEdge(R::new(PATTERN_MARKER, ns[0]))));
        schedule.push((off + 6, Event::AddEdge(R::new(ns[0], ESTABLISHED_MARKER))));
        schedule.push((off + 7, Event::AddEdge(R::new(ns[0], ns[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(ns[0], ns[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(ns[0], ns[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(ns[1], ns[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(ns[2], ns[3]))));
    }
    schedule
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

fn is_subset_or_superset(a: &HashSet<String>, b: &HashSet<String>) -> bool {
    a.is_subset(b) || b.is_subset(a)
}

/// Smart merge candidate picker. ADR 0066 Phase Alpha-5.
///
/// Differences from Alpha-3++++ heuristic:
/// - Reject subset pairs (one ⊆ other) explicitly
/// - Among non-subset pairs, pick highest Jaccard ≥ floor
/// - Optionally prefer pairs with both sides above DEMOTE_THRESHOLD
///   (consolidating two "good" theories vs. salvaging a noisy one)
fn pick_merge_candidates_smart(
    rt: &AutonomousRuntime,
) -> (Option<(String, String, f64)>, Vec<(String, String, f64, bool, bool)>) {
    let theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    let n = theories.len();
    let mut diagnostics: Vec<(String, String, f64, bool, bool)> = Vec::new();
    if n < 2 {
        return (None, diagnostics);
    }
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
    let qual_rates: Vec<Option<f64>> = theories
        .iter()
        .map(|t| {
            let axioms: Vec<&str> = rt.rset.theory_axioms(t);
            let mut sum: f64 = 0.0;
            let mut q: usize = 0;
            for ax in &axioms {
                if let Some(r) = rt
                    .memory
                    .prediction_state
                    .hit_rate(ax, MIN_AXIOM_PREDICTIONS)
                {
                    sum += r;
                    q += 1;
                }
            }
            if q == 0 { None } else { Some(sum / q as f64) }
        })
        .collect();

    let mut best: Option<(usize, usize, f64)> = None;
    for i in 0..n {
        for j in (i + 1)..n {
            let j_score = jaccard(&member_sets[i], &member_sets[j]);
            let subset = is_subset_or_superset(&member_sets[i], &member_sets[j]);
            let both_good = qual_rates[i].map(|r| r >= DEMOTE_THRESHOLD).unwrap_or(false)
                && qual_rates[j].map(|r| r >= DEMOTE_THRESHOLD).unwrap_or(false);
            diagnostics.push((
                theories[i].clone(),
                theories[j].clone(),
                j_score,
                subset,
                both_good,
            ));
            if subset {
                continue;
            }
            if j_score < MERGE_JACCARD_FLOOR {
                continue;
            }
            match best {
                None => best = Some((i, j, j_score)),
                Some((_, _, prev)) if j_score > prev => best = Some((i, j, j_score)),
                _ => {}
            }
        }
    }
    let pick = best.map(|(i, j, s)| (theories[i].clone(), theories[j].clone(), s));
    (pick, diagnostics)
}

fn print_pair_diagnostics(
    diag: &[(String, String, f64, bool, bool)],
) {
    println!();
    println!("--- Pair diagnostics ---");
    println!(
        "{:>10} {:>10} {:>8} {:>8} {:>10}",
        "a", "b", "jaccard", "subset?", "both_good?",
    );
    for (a, b, j, sub, both) in diag {
        println!(
            "{:>10} {:>10} {:>8.4} {:>8} {:>10}",
            a, b, j,
            if *sub { "yes" } else { "no" },
            if *both { "yes" } else { "no" },
        );
    }
}

fn main() {
    println!(
        "=== ADR 0066 Phase Alpha-5 — smart-merge vs demote vs repair ({} ticks per phase) ===",
        TICKS_PER_PHASE,
    );
    println!(
        "DEMOTE_THRESHOLD={}, REPAIR_AXIOM_THRESHOLD={}, MERGE_JACCARD_FLOOR={} (subset pairs excluded)",
        DEMOTE_THRESHOLD, REPAIR_AXIOM_THRESHOLD, MERGE_JACCARD_FLOOR,
    );

    // -------- Path A: demote (scoped block, runtime dropped on exit) --------
    println!();
    println!("############### Path A: demote (Alpha-3+ baseline) ###############");
    let (init_qa, init_ma, init_na, demoted_id, qa, ma, na) = {
        let mut rt = fresh_runtime();
        rt.run_bounded(TICKS_PER_PHASE);
        let pre = rank_theories(&rt);
        print_tournament("A:Phase 0", &pre);
        let bottom = pick_bottom(&pre);
        let (init_q, init_m, init_n) = aggregate_stats(&pre);
        let demoted = match &bottom {
            Some((id, rate)) if *rate < DEMOTE_THRESHOLD => {
                let removed = rt.rset.retract_theory(id).expect("retract_theory");
                println!();
                println!(
                    "--- A: retract '{}' (rate {:.4}) — {} edges removed ---",
                    id, rate, removed,
                );
                Some(id.clone())
            }
            _ => None,
        };
        rt.run_bounded(TICKS_PER_PHASE);
        let post = rank_theories(&rt);
        print_tournament("A: after demote + 1000 ticks", &post);
        let (q, m, n) = aggregate_stats(&post);
        (init_q, init_m, init_n, demoted, q, m, n)
        // rt dropped here — memory reclaimed before path B starts
    };

    // -------- Path B: repair (scoped) --------
    println!();
    println!("############### Path B: repair (Alpha-3+++ baseline) ###############");
    let (repaired_count, qb, mb, nb) = {
        let mut rt = fresh_runtime();
        rt.run_bounded(TICKS_PER_PHASE);
        let pre = rank_theories(&rt);
        let bottom = pick_bottom(&pre);
        let count = match &bottom {
            Some((id, rate)) if *rate < DEMOTE_THRESHOLD => {
                let mut to_detach: Vec<String> = Vec::new();
                for (ax, ax_rate, _) in axiom_rates_in_theory(&rt, id) {
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
                    rt.rset.retract_theory_member(id, ax).expect("retract_member");
                }
                to_detach.len()
            }
            _ => 0,
        };
        rt.run_bounded(TICKS_PER_PHASE);
        let post = rank_theories(&rt);
        print_tournament("B: after repair + 1000 ticks", &post);
        let (q, m, n) = aggregate_stats(&post);
        (count, q, m, n)
        // rt dropped here
    };

    // -------- Path C: smart merge (scoped) --------
    println!();
    println!("############### Path C: smart-merge (treatment) ###############");
    let (merged_info, merged_post_rate, merged_post_qual, qc, mc, nc) = {
        let mut rt = fresh_runtime();
        rt.run_bounded(TICKS_PER_PHASE);
        let pre = rank_theories(&rt);
        print_tournament("C:Phase 0", &pre);
        let (merge_pair, diag) = pick_merge_candidates_smart(&rt);
        print_pair_diagnostics(&diag);
        let info: Option<(String, String, f64, String)> = match &merge_pair {
            Some((a, b, score)) => {
                println!();
                println!(
                    "--- C: smart pair '{}' + '{}' (Jaccard={:.4}, NON-subset) ---",
                    a, b, score,
                );
                print_axiom_breakdown("C: candidate A", &rt, a);
                print_axiom_breakdown("C: candidate B", &rt, b);
                let merged_id = rt.rset.merge_theories(a, b).expect("merge_theories");
                println!();
                println!("--- C: merged into '{}' ---", merged_id);
                print_axiom_breakdown("C: merged theory", &rt, &merged_id);
                Some((a.clone(), b.clone(), *score, merged_id))
            }
            None => {
                println!();
                println!(
                    "--- C: no NON-subset pair has Jaccard >= {} — nothing to merge ---",
                    MERGE_JACCARD_FLOOR,
                );
                None
            }
        };
        rt.run_bounded(TICKS_PER_PHASE);
        let post = rank_theories(&rt);
        print_tournament("C: after smart-merge + 1000 ticks", &post);
        let (q, m, n) = aggregate_stats(&post);
        // Lookup merged theory's post-rate before dropping rt.
        let (post_rate, post_qual) = match &info {
            Some((_, _, _, mid)) => {
                let r = post.iter().find(|r| &r.id == mid);
                let rate = r.and_then(|r| r.aggregated_hit_rate);
                let qual = r.map(|r| r.qualifying_axioms).unwrap_or(0);
                (rate, qual)
            }
            None => (None, 0),
        };
        (info, post_rate, post_qual, q, m, n)
        // rt dropped here
    };

    // -------- Comparison --------
    println!();
    println!("=== Comparison ===");
    println!("{:>30} {:>10} {:>10} {:>5}", "phase", "mean", "min", "qual");
    println!(
        "{:>30} {:>10.4} {:>10.4} {:>5}",
        "Phase 0 (all paths)", init_ma, init_na, init_qa,
    );
    println!("{:>30} {:>10.4} {:>10.4} {:>5}", "A: demote", ma, na, qa);
    println!("{:>30} {:>10.4} {:>10.4} {:>5}", "B: repair", mb, nb, qb);
    println!("{:>30} {:>10.4} {:>10.4} {:>5}", "C: smart-merge", mc, nc, qc);

    println!();
    println!("=== Verdict ===");
    let demoted_str = demoted_id.clone().unwrap_or_else(|| "—".to_string());
    println!("  A: demoted          : {}", demoted_str);
    println!("  B: repaired axioms  : {} detached", repaired_count);
    if let Some((a, b, score, mid)) = &merged_info {
        println!(
            "  C: smart-merged     : '{}' + '{}' (Jaccard={:.4}) → '{}'",
            a, b, score, mid,
        );
        let hr = merged_post_rate
            .map(|x| format!("{:.4}", x))
            .unwrap_or_else(|| "—".to_string());
        println!(
            "  C: merged post-rate : {} (qualifying={})",
            hr, merged_post_qual,
        );
    } else {
        println!("  C: smart-merged     : (no candidate, all overlapping pairs were subset)");
    }

    let verdict = match (&merged_info, merged_post_rate) {
        (None, _) => "INAPPLICABLE — no non-subset pair met Jaccard floor".to_string(),
        (Some(_), None) => "INCONCLUSIVE — merged theory has no qualifying axiom".to_string(),
        (Some(_), Some(rate)) => {
            let rate_ok = rate >= DEMOTE_THRESHOLD;
            let mean_at_least_demote = mc >= ma - 1e-9;
            let mean_at_least_repair = mc >= mb - 1e-9;
            let qual_at_least_demote = qc >= qa;
            if rate_ok && (mean_at_least_demote || qual_at_least_demote) {
                if mean_at_least_repair {
                    "POSITIVE — smart-merge matches/beats both demote and repair".to_string()
                } else {
                    "PARTIAL — smart-merge meets threshold but mean < repair".to_string()
                }
            } else if !rate_ok {
                format!(
                    "NEGATIVE — merged theory rate {:.4} < threshold {}",
                    rate, DEMOTE_THRESHOLD,
                )
            } else {
                "MIXED — passes threshold but underperforms demote".to_string()
            }
        }
    };
    println!("  → {}", verdict);

    println!();
    println!("--- end ---");
}
