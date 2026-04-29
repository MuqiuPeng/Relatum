//! Phase Alpha-3++ — multi-round iterative theory demotion
//! (ADR 0066 follow-up after Alpha-3+).
//!
//! Phase Alpha-3+ showed that a single tournament-driven theory
//! demotion improves measured aggregate quality (+12% mean,
//! +29% min hit rate) without perturbing the rest of the
//! runtime. Phase Alpha-3++ asks the natural follow-up: if we
//! repeat the demote-rerun cycle, does the system converge to
//! a stable theory set, or does each round find a new "worst"
//! to demote?
//!
//! Empirical questions:
//! - Does each round demote a different theory, or do later
//!   rounds find no further demote candidate (fixed point)?
//! - Does the runtime re-discover demoted theories during the
//!   intervening run?
//! - Do mean / min hit rates improve monotonically across
//!   rounds?
//!
//! Run structure:
//!   Phase 0: discover (1000 ticks)
//!   For each iteration i in 0..N:
//!     Tournament + retract bottom theory (if below threshold)
//!     Run another 1000 ticks
//!   Final tournament
//!
//! Captured to `logs/<date>_phase_alpha_theory_demote_loop_n.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    }, RSet,
};
use std::collections::HashSet;

const ITERATIONS: usize = 3;
const TICKS_PER_ROUND: u64 = 1000;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
/// Demote threshold: only retract a theory whose aggregated
/// hit rate is below this. Set conservatively to catch only
/// clearly-noisy theories. Below this we consider the system
/// "converged" — no further demotion candidate.
const DEMOTE_THRESHOLD: f64 = 0.50;
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
    let mean = if qual.is_empty() { 0.0 } else { qual.iter().sum::<f64>() / qual.len() as f64 };
    let min = qual.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    (qual.len(), mean, if min == f64::INFINITY { 0.0 } else { min })
}

fn main() {
    println!(
        "=== ADR 0066 Phase Alpha-3++ — multi-round iterative theory demotion ({} iterations × {} ticks) ===",
        ITERATIONS, TICKS_PER_ROUND,
    );
    println!("DEMOTE_THRESHOLD = {}", DEMOTE_THRESHOLD);

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    println!();
    println!("--- Phase 0: initial discovery ({} ticks) ---", TICKS_PER_ROUND);
    rt.run_bounded(TICKS_PER_ROUND);
    println!(
        "tick={} episodes={} theories={} axioms={}",
        rt.tick,
        rt.memory.episodes.len(),
        rt.rset.theories().len(),
        rt.rset.axioms().len(),
    );
    let init_ranks = rank_theories(&rt);
    print_tournament("Phase 0 (initial)", &init_ranks);

    let mut history: Vec<(usize, f64, f64, usize, Option<String>)> = Vec::new(); // (iter, mean, min, theory_count, demoted_id)
    let (q0, m0, n0) = aggregate_stats(&init_ranks);
    history.push((0, m0, n0, q0, None));

    let mut demoted_in_history: HashSet<String> = HashSet::new();

    for iter in 1..=ITERATIONS {
        let ranks = rank_theories(&rt);
        let target = ranks
            .iter()
            .filter(|r| r.aggregated_hit_rate.is_some())
            .last()
            .map(|r| (r.id.clone(), r.aggregated_hit_rate.unwrap()));
        let demoted = match target {
            Some((id, rate)) if rate < DEMOTE_THRESHOLD => {
                let removed = rt.rset.retract_theory(&id).expect("retract_theory");
                println!();
                println!(
                    "--- Iteration {}: retract '{}' (rate {:.4}) — {} edges removed ---",
                    iter, id, rate, removed,
                );
                demoted_in_history.insert(id.clone());
                Some(id)
            }
            Some((id, rate)) => {
                println!();
                println!(
                    "--- Iteration {}: lowest theory '{}' has rate {:.4} ≥ threshold {} — converged, no further demote ---",
                    iter, id, rate, DEMOTE_THRESHOLD,
                );
                None
            }
            None => {
                println!();
                println!("--- Iteration {}: no qualifying theory; stopping ---", iter);
                None
            }
        };

        if demoted.is_none() {
            // Even at convergence, run one more block to confirm stability.
            rt.run_bounded(TICKS_PER_ROUND);
            let post = rank_theories(&rt);
            let (q, mean, min) = aggregate_stats(&post);
            history.push((iter, mean, min, q, None));
            print_tournament(&format!("After iter {} (no demote, ran {} ticks)", iter, TICKS_PER_ROUND), &post);
            break;
        }

        rt.run_bounded(TICKS_PER_ROUND);
        let post = rank_theories(&rt);
        let (q, mean, min) = aggregate_stats(&post);
        history.push((iter, mean, min, q, demoted.clone()));
        print_tournament(&format!("After iter {} ({} ticks)", iter, TICKS_PER_ROUND), &post);

        // Observe re-discovery: if the demoted theory id reappears
        // in the new theory set, that's interesting.
        if let Some(d) = &demoted {
            let resurfaced = rt.rset.theories().iter().any(|t| t == d);
            if resurfaced {
                println!("  (note: theory '{}' resurfaced in new theory set!)", d);
            } else {
                println!("  (theory '{}' did NOT resurface)", d);
            }
        }
    }

    // Summary table
    println!();
    println!("=== Iteration history ===");
    println!(
        "{:>5} {:>10} {:>10} {:>5} {:>15} {:>10}",
        "iter", "mean", "min", "qual", "demoted", "Δ_mean",
    );
    let mut prev_mean: Option<f64> = None;
    for (iter, mean, min, qual, demoted) in &history {
        let delta = match prev_mean {
            Some(p) => format!("{:+.4}", mean - p),
            None => "—".to_string(),
        };
        let dm = match demoted {
            Some(s) => s.clone(),
            None => "—".to_string(),
        };
        println!(
            "{:>5} {:>10.4} {:>10.4} {:>5} {:>15} {:>10}",
            iter, mean, min, qual, dm, delta,
        );
        prev_mean = Some(*mean);
    }

    // Verdict
    println!();
    println!("=== Verdict ===");
    let h0 = history.first().expect("history non-empty");
    let hN = history.last().expect("history non-empty");
    let mean_change = hN.1 - h0.1;
    let min_change = hN.2 - h0.2;
    println!(
        "  initial: mean={:.4} min={:.4} qualifying={}",
        h0.1, h0.2, h0.3,
    );
    println!(
        "  final:   mean={:.4} min={:.4} qualifying={}",
        hN.1, hN.2, hN.3,
    );
    println!(
        "  Δ mean: {:+.4}; Δ min: {:+.4}",
        mean_change, min_change,
    );

    let total_demoted = history.iter().filter(|(_, _, _, _, d)| d.is_some()).count();
    println!("  total theories demoted: {}", total_demoted);
    println!("  unique demote ids: {:?}", demoted_in_history);
    if total_demoted < ITERATIONS {
        println!("  → system converged (no further demote candidate before max iterations)");
    } else {
        println!("  → all {} iterations performed demote; may not have converged", ITERATIONS);
    }

    println!();
    println!("--- end ---");
}
