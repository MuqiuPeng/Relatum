//! Phase Alpha-3 — internal theory self-play tournament
//! (ADR 0066).
//!
//! Run a runtime on the OQ #1 substrate; at the end, rank all
//! discovered theories by their aggregated per-axiom prediction
//! accuracy (post-hoc). Print the tournament: which theories
//! would survive an "ESTABLISHED-by-comparison" gate, which
//! would be demoted.
//!
//! This is the smallest tractable prototype of cognitive-game-
//! framing's self-play candidate (a). Pure observational — no
//! runtime changes, no demotion. The empirical question:
//! does v2 actually produce theories with materially different
//! predictive accuracy?
//!
//! Captured to `logs/<date>_phase_alpha_theory_tournament.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    }, RSet,
};

const HORIZON: u64 = 2000;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
use relatum_v2::test_substrates::oq1::build_long_stream;

#[derive(Clone)]
struct TheoryReport {
    id: String,
    axiom_count: usize,
    qualifying_axioms: usize,
    aggregated_hit_rate: Option<f64>,
    per_axiom_hit_rates: Vec<(String, Option<f64>)>,
}

fn rank_theories(rt: &AutonomousRuntime) -> Vec<TheoryReport> {
    let theories: Vec<&str> = rt.rset.theories();
    let mut reports = Vec::new();
    for t in theories {
        let axioms: Vec<&str> = rt.rset.theory_axioms(t);
        let mut per_axiom: Vec<(String, Option<f64>)> = Vec::new();
        let mut sum: f64 = 0.0;
        let mut qualifying: usize = 0;
        for ax in &axioms {
            let hr = rt
                .memory
                .prediction_state
                .hit_rate(ax, MIN_AXIOM_PREDICTIONS);
            per_axiom.push((ax.to_string(), hr));
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
            per_axiom_hit_rates: per_axiom,
        });
    }
    // Sort by aggregated hit rate descending. Theories with no
    // qualifying axioms (None) sink to the bottom.
    reports.sort_by(|a, b| {
        match (a.aggregated_hit_rate, b.aggregated_hit_rate) {
            (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    reports
}

fn main() {
    println!(
        "=== ADR 0066 Phase Alpha-3 — theory self-play tournament (HORIZON={}) ===",
        HORIZON
    );

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment =
        Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(HORIZON);

    println!();
    println!(
        "Final runtime state: tick={} episodes={} theories={} axioms={}",
        rt.tick,
        rt.memory.episodes.len(),
        rt.rset.theories().len(),
        rt.rset.axioms().len(),
    );

    let reports = rank_theories(&rt);

    println!();
    println!("=== Tournament results (ranked by aggregated hit rate) ===");
    if reports.is_empty() {
        println!("  (no theories discovered — tournament has no participants)");
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

    // Per-theory axiom breakdown for the top-1 and bottom-1 (if
    // they differ) — gives a sense of where accuracy lives.
    if reports.len() >= 2 {
        println!();
        println!("=== Per-axiom breakdown: top-ranked theory ===");
        print_axiom_breakdown(&reports[0]);
        println!();
        println!("=== Per-axiom breakdown: bottom-ranked theory ===");
        print_axiom_breakdown(&reports[reports.len() - 1]);
    }

    // Diagnostic summary.
    println!();
    println!("=== Tournament diagnostic ===");
    let with_agg: Vec<f64> = reports
        .iter()
        .filter_map(|r| r.aggregated_hit_rate)
        .collect();
    if with_agg.is_empty() {
        println!("  Verdict: NO theory has enough qualifying axioms");
        println!("  → tournament has no signal; candidate (a) silent");
    } else if with_agg.len() == 1 {
        println!(
            "  Verdict: ONLY 1 theory has qualifying axioms (hit_rate = {:.4})",
            with_agg[0]
        );
        println!("  → tournament has no opponents; need richer substrate");
    } else {
        let max = with_agg.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min = with_agg.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let mean: f64 = with_agg.iter().sum::<f64>() / with_agg.len() as f64;
        let spread = max - min;
        println!(
            "  Theories with signal: {} / {}",
            with_agg.len(),
            reports.len()
        );
        println!("  Hit-rate range: [{:.4}, {:.4}] (spread {:.4})", min, max, spread);
        println!("  Hit-rate mean:  {:.4}", mean);
        if spread > 0.20 {
            println!("  Verdict: theories DIFFERENTIATE (spread > 0.20) — tournament has selection signal");
        } else if spread > 0.05 {
            println!("  Verdict: theories MILDLY DIFFERENTIATE (0.05 < spread <= 0.20) — partial signal");
        } else {
            println!("  Verdict: theories DO NOT DIFFERENTIATE (spread <= 0.05) — uniform tournament");
        }
    }

    println!();
    println!("--- end ---");
}

fn print_axiom_breakdown(r: &TheoryReport) {
    println!("  theory_id: {}", r.id);
    println!(
        "{:>40} {:>15}",
        "axiom_id", "hit_rate",
    );
    for (ax, hr) in &r.per_axiom_hit_rates {
        let hr_str = match hr {
            Some(x) => format!("{:.4}", x),
            None => "—".to_string(),
        };
        println!("{:>40} {:>15}", ax, hr_str);
    }
}
