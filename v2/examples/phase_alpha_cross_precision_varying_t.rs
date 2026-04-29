//! Phase Alpha-9 — Cross-precision vs primary-rate convergence speed
//! (ADR 0066 follow-up to Alpha-8).
//!
//! Alpha-8 showed cross-precision and primary-rate make the SAME
//! demote decision at T=1000. This slice asks: at smaller T, which
//! signal converges first to the correct verdict?
//!
//! Hypothesis: primary-rate is noisy at small T because per-axiom
//! prediction counts are small; cross-precision is structural and
//! converges as soon as the theory set stabilizes (typically much
//! earlier than the stream-driven hit-rate counters).
//!
//! Method: for each T ∈ {100, 200, 350, 500, 750, 1000}:
//!   1. Fresh runtime, run T ticks.
//!   2. Primary-stream ranking: which theory is bottom by hit rate,
//!      and how many of its axioms have qualifying predictions?
//!   3. Cross-precision matrix on dream-generated substrates;
//!      report column means; pick lowest as demote candidate.
//!   4. Compare both signals' picks.
//!
//! The "ground truth" reference is t_0 (Alpha-3+/3++/8 verdict).
//!
//! Falsifiable verdicts:
//! - SPEED-WIN: cross-precision picks t_0 at smaller T than
//!   primary-rate does
//! - TIE: both signals agree at every measured T
//! - SPEED-LOSS: cross-precision picks correctly only at large T
//!
//! Captured to `logs/<date>_phase_alpha_cross_precision_varying_t.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet,
};
use std::collections::HashSet;

const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const GROUND_TRUTH_DEMOTE: &str = "t_0"; // Alpha-3+/3++/8 verdict

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
struct PrimaryRanking {
    bottom_id: Option<String>,
    bottom_rate: Option<f64>,
    bottom_qualifying: usize,
    total_qualifying: usize,
    total_theories: usize,
}

fn primary_ranking(rt: &AutonomousRuntime) -> PrimaryRanking {
    let theories: Vec<&str> = rt.rset.theories();
    let mut entries: Vec<(String, Option<f64>, usize)> = Vec::new();
    for t in &theories {
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
        let rate = if qualifying > 0 {
            Some(sum / qualifying as f64)
        } else {
            None
        };
        entries.push((t.to_string(), rate, qualifying));
    }
    // Sort: theories with rates first, ascending (so bottom is last).
    entries.sort_by(|a, b| match (a.1, b.1) {
        (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    let bottom_with_rate = entries.iter().rev().find(|(_, r, _)| r.is_some());
    let total_qualifying = entries.iter().filter(|(_, r, _)| r.is_some()).count();
    PrimaryRanking {
        bottom_id: bottom_with_rate.map(|e| e.0.clone()),
        bottom_rate: bottom_with_rate.and_then(|e| e.1),
        bottom_qualifying: bottom_with_rate.map(|e| e.2).unwrap_or(0),
        total_qualifying,
        total_theories: theories.len(),
    }
}

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

#[derive(Clone)]
struct CrossRanking {
    bottom_id: Option<String>,
    bottom_mean: Option<f64>,
    column_means: Vec<(String, f64)>,
    n_substrates: usize,
}

fn cross_precision_ranking(rt: &AutonomousRuntime) -> CrossRanking {
    let theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    if theories.is_empty() {
        return CrossRanking {
            bottom_id: None,
            bottom_mean: None,
            column_means: Vec::new(),
            n_substrates: 0,
        };
    }
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
    // Generate substrates.
    let mut substrates: Vec<RSet> = Vec::with_capacity(theories.len());
    for (i, t) in theories.iter().enumerate() {
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
    if substrates.is_empty() {
        return CrossRanking {
            bottom_id: None,
            bottom_mean: None,
            column_means: Vec::new(),
            n_substrates: 0,
        };
    }
    // Build matrix; sum off-diagonal precisions per column.
    let mut col_sum = vec![0.0; theories.len()];
    let mut col_count = vec![0usize; theories.len()];
    for (i, sub_i) in substrates.iter().enumerate() {
        let actual_i: HashSet<R> = sub_i.iter().cloned().collect();
        for j in 0..theories.len() {
            if i == j {
                continue;
            }
            let predicted_j = predict_all_axioms(sub_i, &axioms_by_theory[j]);
            if let Some(p) = precision(&predicted_j, &actual_i) {
                col_sum[j] += p;
                col_count[j] += 1;
            }
        }
    }
    let mut column_means: Vec<(String, f64)> = Vec::new();
    let mut bottom: Option<(String, f64)> = None;
    for j in 0..theories.len() {
        if col_count[j] == 0 {
            continue;
        }
        let mean = col_sum[j] / col_count[j] as f64;
        column_means.push((theories[j].clone(), mean));
        let is_lower = match &bottom {
            None => true,
            Some((_, m)) => mean < *m,
        };
        if is_lower {
            bottom = Some((theories[j].clone(), mean));
        }
    }
    column_means.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    let (bid, bm) = match bottom {
        Some((id, m)) => (Some(id), Some(m)),
        None => (None, None),
    };
    CrossRanking {
        bottom_id: bid,
        bottom_mean: bm,
        column_means,
        n_substrates: substrates.len(),
    }
}

fn run_at_t(t: u64) -> (PrimaryRanking, CrossRanking) {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(t);
    let pr = primary_ranking(&rt);
    let cr = cross_precision_ranking(&rt);
    (pr, cr)
    // rt drops here
}

fn opt_to_str(o: &Option<String>) -> String {
    o.clone().unwrap_or_else(|| "—".to_string())
}

fn opt_f64_to_str(o: &Option<f64>) -> String {
    o.map(|x| format!("{:.4}", x))
        .unwrap_or_else(|| "—".to_string())
}

fn main() {
    println!(
        "=== ADR 0066 Phase Alpha-9 — cross-precision vs primary-rate convergence speed ===",
    );
    println!(
        "Sweep over T ∈ {{100, 200, 350, 500, 750, 1000}}; ground-truth target = '{}'",
        GROUND_TRUTH_DEMOTE,
    );
    println!(
        "NUM_GEN_IDS={}, SEED_DENSITY={}, MIN_AXIOM_PREDICTIONS={}",
        NUM_GEN_IDS, SEED_DENSITY, MIN_AXIOM_PREDICTIONS,
    );

    let ts: [u64; 6] = [100, 200, 350, 500, 750, 1000];
    let mut rows: Vec<(u64, PrimaryRanking, CrossRanking)> = Vec::new();

    for &t in &ts {
        println!();
        println!("─────────────────── T = {} ───────────────────", t);
        let (pr, cr) = run_at_t(t);
        println!(
            "  primary    : {} theories, {} qualifying; bottom = {} (rate={}, qualifying axioms={})",
            pr.total_theories,
            pr.total_qualifying,
            opt_to_str(&pr.bottom_id),
            opt_f64_to_str(&pr.bottom_rate),
            pr.bottom_qualifying,
        );
        if cr.n_substrates > 0 {
            print!("  cross-prec : column means ");
            for (i, (id, m)) in cr.column_means.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("{}={:.4}", id, m);
            }
            println!();
            println!(
                "               bottom = {} (mean={})",
                opt_to_str(&cr.bottom_id),
                opt_f64_to_str(&cr.bottom_mean),
            );
        } else {
            println!("  cross-prec : (no theories yet)");
        }
        rows.push((t, pr, cr));
    }

    // ── Summary table ────────────────────────────────────────
    println!();
    println!("=== Summary: who picks the ground-truth target '{}'? ===", GROUND_TRUTH_DEMOTE);
    println!(
        "{:>5} {:>10} {:>14} {:>10} {:>10} {:>14} {:>10} {:>10}",
        "T", "primary?", "primary_rate", "p_qual", "cross?", "cross_mean", "agree?", "n_th"
    );
    let mut primary_first_correct: Option<u64> = None;
    let mut cross_first_correct: Option<u64> = None;
    for (t, pr, cr) in &rows {
        let p_correct = pr.bottom_id.as_deref() == Some(GROUND_TRUTH_DEMOTE);
        let c_correct = cr.bottom_id.as_deref() == Some(GROUND_TRUTH_DEMOTE);
        let agree = match (&pr.bottom_id, &cr.bottom_id) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        };
        if p_correct && primary_first_correct.is_none() {
            primary_first_correct = Some(*t);
        }
        if c_correct && cross_first_correct.is_none() {
            cross_first_correct = Some(*t);
        }
        println!(
            "{:>5} {:>10} {:>14} {:>10} {:>10} {:>14} {:>10} {:>10}",
            t,
            if p_correct { "✓" } else { "✗" },
            opt_f64_to_str(&pr.bottom_rate),
            pr.bottom_qualifying,
            if c_correct { "✓" } else { "✗" },
            opt_f64_to_str(&cr.bottom_mean),
            if agree { "yes" } else { "no" },
            pr.total_theories,
        );
    }

    // ── Verdict ──────────────────────────────────────────────
    println!();
    println!("=== Verdict ===");
    let p_first = primary_first_correct
        .map(|t| t.to_string())
        .unwrap_or_else(|| "never".to_string());
    let c_first = cross_first_correct
        .map(|t| t.to_string())
        .unwrap_or_else(|| "never".to_string());
    println!(
        "  primary-rate first picks '{}' at T = {}",
        GROUND_TRUTH_DEMOTE, p_first,
    );
    println!(
        "  cross-precision first picks '{}' at T = {}",
        GROUND_TRUTH_DEMOTE, c_first,
    );
    match (primary_first_correct, cross_first_correct) {
        (Some(p), Some(c)) if c < p => println!(
            "  → SPEED-WIN — cross-precision converges {} ticks earlier than primary-rate",
            p - c,
        ),
        (Some(p), Some(c)) if c == p => {
            println!("  → TIE — both signals converge at the same T")
        }
        (Some(p), Some(c)) => println!(
            "  → SPEED-LOSS — cross-precision converges {} ticks later than primary-rate",
            c - p,
        ),
        (Some(_), None) => println!(
            "  → CROSS-NEVER — primary-rate converges but cross-precision doesn't (in measured range)",
        ),
        (None, Some(_)) => println!(
            "  → PRIMARY-NEVER — cross-precision converges but primary-rate doesn't",
        ),
        (None, None) => println!(
            "  → NEITHER — neither signal converges in measured range",
        ),
    }

    println!();
    println!("--- end ---");
}
