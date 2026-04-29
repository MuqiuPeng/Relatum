//! Phase Alpha-3+ — iterative theory demotion (ADR 0066 follow-up).
//!
//! Phase Alpha-3 (`phase_alpha_theory_tournament.rs`) showed
//! theories DIFFERENTIATE strongly (spread 0.6095) on the OQ #1
//! substrate, with one theory ("t_0", broad-and-noisy) at hit
//! rate 0.39 and one ("t_2", narrow-and-precise) at 0.99. This
//! follow-up actually *demotes* the worst theory mid-run and
//! observes:
//!
//! - Do its load-bearing axioms (the 99.92% axiom that's also
//!   in t_2) survive the demotion?
//! - Do its bad axioms (0.04-0.05 hit rate) get cleaned up?
//! - Does the runtime continue productively after demotion?
//! - Does the post-demotion tournament show better aggregate
//!   theory quality?
//!
//! Run structure:
//! 1. Run runtime for 1000 ticks (Phase 1 — discovery).
//! 2. Tournament — print ranking.
//! 3. Identify worst theory; retract it via
//!    `RSet::retract_theory`.
//! 4. Run runtime for another 1000 ticks (Phase 2 — post-demote).
//! 5. Tournament — print ranking again. Compare.
//!
//! Captured to `logs/<date>_phase_alpha_theory_demote_loop.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    }, RSet,
};
use std::collections::HashSet;

const PHASE_1_TICKS: u64 = 1000;
const PHASE_2_TICKS: u64 = 1000;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
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
            let hr = rt
                .memory
                .prediction_state
                .hit_rate(ax, MIN_AXIOM_PREDICTIONS);
            if let Some(rate) = hr {
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

fn collect_axiom_hit_rates(
    rt: &AutonomousRuntime,
) -> Vec<(String, f64)> {
    let mut result = Vec::new();
    for ax in rt.rset.axioms() {
        if let Some(rate) = rt
            .memory
            .prediction_state
            .hit_rate(ax, MIN_AXIOM_PREDICTIONS)
        {
            result.push((ax.to_string(), rate));
        }
    }
    result.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}

fn main() {
    println!(
        "=== ADR 0066 Phase Alpha-3+ — iterative theory demotion (PHASE_1={}, PHASE_2={}) ===",
        PHASE_1_TICKS, PHASE_2_TICKS
    );

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment =
        Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    // ── Phase 1 ──────────────────────────────────────────────
    println!();
    println!("--- Phase 1: discovery (1000 ticks) ---");
    rt.run_bounded(PHASE_1_TICKS);
    println!(
        "tick={} episodes={} theories={} axioms={}",
        rt.tick,
        rt.memory.episodes.len(),
        rt.rset.theories().len(),
        rt.rset.axioms().len(),
    );

    let phase1_ranks = rank_theories(&rt);
    print_tournament("end of Phase 1", &phase1_ranks);

    // Capture pre-demote state.
    let phase1_axiom_hit_rates = collect_axiom_hit_rates(&rt);
    let phase1_axioms_set: HashSet<String> = rt
        .rset
        .axioms()
        .into_iter()
        .map(str::to_owned)
        .collect();

    // ── Demotion intervention ────────────────────────────────
    let demote_target = phase1_ranks
        .iter()
        .filter(|r| r.aggregated_hit_rate.is_some())
        .last()
        .map(|r| r.id.clone());
    let Some(target) = demote_target else {
        println!();
        println!("No theory to demote — aborting.");
        return;
    };
    println!();
    println!("--- Demotion intervention ---");
    println!(
        "Retracting bottom-ranked theory: {} (agg hit rate {:.4})",
        target,
        phase1_ranks
            .iter()
            .find(|r| r.id == target)
            .and_then(|r| r.aggregated_hit_rate)
            .unwrap_or(0.0),
    );
    let target_axioms_pre: HashSet<String> = rt
        .rset
        .theory_axioms(&target)
        .into_iter()
        .map(str::to_owned)
        .collect();
    println!(
        "  theory_axioms before retract: {} → {:?}",
        target_axioms_pre.len(),
        target_axioms_pre,
    );
    let removed = rt
        .rset
        .retract_theory(&target)
        .expect("retract_theory should succeed");
    println!("  meta-R edges removed: {}", removed);

    // Verify axioms survived (per ADR 0030: retract_theory does NOT
    // remove axiom registrations).
    let surviving_axioms: HashSet<String> = rt
        .rset
        .axioms()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let lost_axioms: Vec<String> = target_axioms_pre
        .difference(&surviving_axioms)
        .cloned()
        .collect();
    let kept_axioms: Vec<String> = target_axioms_pre
        .intersection(&surviving_axioms)
        .cloned()
        .collect();
    println!(
        "  axioms surviving demotion: {} / {}",
        kept_axioms.len(),
        target_axioms_pre.len(),
    );
    if !lost_axioms.is_empty() {
        println!("  axioms LOST: {:?}", lost_axioms);
    }

    // ── Phase 2 ──────────────────────────────────────────────
    println!();
    println!("--- Phase 2: post-demote (another 1000 ticks) ---");
    let phase2_target_tick = rt.tick + PHASE_2_TICKS;
    rt.run_bounded(PHASE_2_TICKS);
    println!(
        "tick={} (target={}) episodes={} theories={} axioms={}",
        rt.tick,
        phase2_target_tick,
        rt.memory.episodes.len(),
        rt.rset.theories().len(),
        rt.rset.axioms().len(),
    );

    let phase2_ranks = rank_theories(&rt);
    print_tournament("end of Phase 2", &phase2_ranks);

    // ── Comparison summary ───────────────────────────────────
    println!();
    println!("=== Phase 1 vs Phase 2 ===");
    let p1_count = phase1_ranks.len();
    let p2_count = phase2_ranks.len();
    let p1_qualifying: Vec<f64> = phase1_ranks
        .iter()
        .filter_map(|r| r.aggregated_hit_rate)
        .collect();
    let p2_qualifying: Vec<f64> = phase2_ranks
        .iter()
        .filter_map(|r| r.aggregated_hit_rate)
        .collect();
    let mean = |v: &[f64]| -> f64 {
        if v.is_empty() {
            0.0
        } else {
            v.iter().sum::<f64>() / v.len() as f64
        }
    };
    let min = |v: &[f64]| -> f64 {
        v.iter().fold(f64::INFINITY, |a, &b| a.min(b))
    };
    println!(
        "theories:           {} → {} (Δ {})",
        p1_count,
        p2_count,
        p2_count as i64 - p1_count as i64,
    );
    println!(
        "qualifying:         {} → {}",
        p1_qualifying.len(),
        p2_qualifying.len(),
    );
    println!(
        "mean hit rate:      {:.4} → {:.4} (Δ {:+.4})",
        mean(&p1_qualifying),
        mean(&p2_qualifying),
        mean(&p2_qualifying) - mean(&p1_qualifying),
    );
    println!(
        "min  hit rate:      {:.4} → {:.4} (Δ {:+.4})",
        min(&p1_qualifying),
        min(&p2_qualifying),
        min(&p2_qualifying) - min(&p1_qualifying),
    );

    // Did the demoted theory's load-bearing axiom (the high
    // hit-rate one) survive?
    println!();
    println!("=== Load-bearing axiom survival ===");
    let p1_high: Vec<&(String, f64)> = phase1_axiom_hit_rates
        .iter()
        .filter(|(_, r)| *r > 0.9)
        .collect();
    if p1_high.is_empty() {
        println!("  No axioms had hit rate > 0.9 at end of Phase 1.");
    } else {
        println!(
            "  Axioms with Phase-1 hit rate > 0.9: {}",
            p1_high.len()
        );
        let final_axioms: HashSet<String> = rt
            .rset
            .axioms()
            .into_iter()
            .map(str::to_owned)
            .collect();
        for (ax, rate) in p1_high {
            let surviving = final_axioms.contains(ax);
            let p2_rate = rt
                .memory
                .prediction_state
                .hit_rate(ax, MIN_AXIOM_PREDICTIONS);
            println!(
                "    {} (P1 rate {:.4})  surviving={}  P2 rate={}",
                ax,
                rate,
                surviving,
                p2_rate
                    .map(|r| format!("{:.4}", r))
                    .unwrap_or("—".to_string()),
            );
        }
    }

    let _ = phase1_axioms_set;
    println!();
    println!("--- end ---");
}
