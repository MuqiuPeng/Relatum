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
        AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet,
};

const HORIZON: u64 = 2000;
const MIN_AXIOM_PREDICTIONS: u64 = 5;

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
