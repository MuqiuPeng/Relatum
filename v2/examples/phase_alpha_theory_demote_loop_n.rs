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
        AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet,
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

fn build_long_stream() -> Vec<(u64, Event)> {
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
