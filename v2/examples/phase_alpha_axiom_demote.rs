//! Phase Alpha-4 — per-axiom tournament + orphan retraction
//! (ADR 0066 follow-up).
//!
//! Phase Alpha-3+ showed that demoting the worst *theory* leaves
//! that theory's exclusive axioms as **orphans** in rset (no
//! longer referenced by any theory) — but they still consume
//! `forward_apply_axiom` cycles each tick. This experiment goes
//! one level finer: after the theory-level demotion, also
//! retract orphan axioms whose hit rate is below a threshold.
//!
//! Because `RSet::retract_axiom` fails when an axiom is still
//! referenced by any theory, this experiment only retracts
//! orphan axioms — exactly the residue that Phase Alpha-3+
//! left behind.
//!
//! Run structure:
//!   Phase 1: discover (1000 ticks)
//!   Tournament + retract worst theory (Alpha-3+ step)
//!   Per-axiom tournament + retract orphans below threshold (Alpha-4)
//!   Phase 2: continue (1000 more ticks)
//!   Final tournament + comparison
//!
//! Captured to `logs/<date>_phase_alpha_axiom_demote.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    }, RSet,
};
use std::collections::HashSet;
use std::time::Instant;

const PHASE_1_TICKS: u64 = 1000;
const PHASE_2_TICKS: u64 = 1000;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
/// Orphan-axiom retraction threshold. Empirically calibrated:
/// at 1000-tick Phase 1, t_0's orphan axioms have hit rates
/// 0.10-0.12 (less converged than at 2000 ticks where they're
/// 0.04-0.05). Threshold 0.15 catches them at this horizon.
/// At longer Phase 1, lower thresholds become viable.
const AXIOM_RETENTION_THRESHOLD: f64 = 0.15;
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

/// Per-axiom report: hit rate + whether axiom is orphan
/// (no theory references) + theory memberships.
#[derive(Clone)]
struct AxiomReport {
    id: String,
    hit_rate: Option<f64>,
    is_orphan: bool,
    theory_count: usize,
}

fn rank_axioms(rt: &AutonomousRuntime) -> Vec<AxiomReport> {
    let mut reports = Vec::new();
    for ax in rt.rset.axioms() {
        let hr = rt
            .memory
            .prediction_state
            .hit_rate(ax, MIN_AXIOM_PREDICTIONS);
        let theory_refs = rt.rset.theories_containing(ax);
        reports.push(AxiomReport {
            id: ax.to_string(),
            hit_rate: hr,
            is_orphan: theory_refs.is_empty(),
            theory_count: theory_refs.len(),
        });
    }
    reports.sort_by(|a, b| match (a.hit_rate, b.hit_rate) {
        (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    reports
}

fn print_axiom_table(label: &str, reports: &[AxiomReport]) {
    println!();
    println!("=== Axiom ranking: {} ===", label);
    println!(
        "{:>40} {:>10} {:>8} {:>8}",
        "axiom_id", "hit_rate", "orphan", "theories",
    );
    for r in reports {
        let hr_str = match r.hit_rate {
            Some(x) => format!("{:.4}", x),
            None => "—".to_string(),
        };
        println!(
            "{:>40} {:>10} {:>8} {:>8}",
            r.id,
            hr_str,
            if r.is_orphan { "yes" } else { "no" },
            r.theory_count,
        );
    }
}

fn main() {
    println!(
        "=== ADR 0066 Phase Alpha-4 — per-axiom tournament + orphan retract (PHASE_1={}, PHASE_2={}, threshold={}) ===",
        PHASE_1_TICKS, PHASE_2_TICKS, AXIOM_RETENTION_THRESHOLD
    );

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment =
        Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    // ── Phase 1 ──────────────────────────────────────────────
    println!();
    println!("--- Phase 1: discovery (1000 ticks) ---");
    let t0 = Instant::now();
    rt.run_bounded(PHASE_1_TICKS);
    let phase1_elapsed = t0.elapsed();
    println!(
        "tick={} episodes={} theories={} axioms={} elapsed={:.2}s ({:.1}ms/tick)",
        rt.tick,
        rt.memory.episodes.len(),
        rt.rset.theories().len(),
        rt.rset.axioms().len(),
        phase1_elapsed.as_secs_f64(),
        phase1_elapsed.as_secs_f64() * 1000.0 / PHASE_1_TICKS as f64,
    );

    let phase1_theory_ranks = rank_theories(&rt);
    print_tournament("end of Phase 1 (theories)", &phase1_theory_ranks);

    // ── Step A: theory-level demotion (Alpha-3+) ────────────
    println!();
    println!("--- Step A: theory-level demotion (Alpha-3+) ---");
    let demote_target = phase1_theory_ranks
        .iter()
        .filter(|r| r.aggregated_hit_rate.is_some())
        .last()
        .map(|r| r.id.clone());
    if let Some(target) = demote_target {
        let removed = rt
            .rset
            .retract_theory(&target)
            .expect("retract_theory");
        println!(
            "Retracted theory: {} ({} edges removed)",
            target, removed
        );
    } else {
        println!("(no eligible theory to retract)");
    }

    // ── Step B: per-axiom orphan tournament (Alpha-4) ────────
    let phase1_axiom_ranks = rank_axioms(&rt);
    print_axiom_table(
        "post-theory-demote, all axioms",
        &phase1_axiom_ranks,
    );

    println!();
    println!(
        "--- Step B: per-axiom orphan retract (threshold {} for hit rate) ---",
        AXIOM_RETENTION_THRESHOLD
    );
    let mut retracted: Vec<String> = Vec::new();
    let mut skipped_referenced: Vec<String> = Vec::new();
    let mut skipped_high_rate: Vec<String> = Vec::new();
    let mut skipped_no_data: Vec<String> = Vec::new();
    for r in &phase1_axiom_ranks {
        match r.hit_rate {
            Some(rate) => {
                if rate >= AXIOM_RETENTION_THRESHOLD {
                    skipped_high_rate.push(r.id.clone());
                    continue;
                }
            }
            None => {
                skipped_no_data.push(r.id.clone());
                continue;
            }
        }
        if !r.is_orphan {
            skipped_referenced.push(r.id.clone());
            continue;
        }
        match rt.rset.retract_axiom(&r.id) {
            Ok(edges) => {
                retracted.push(r.id.clone());
                println!(
                    "  retracted: {} (rate={:.4}, edges removed={})",
                    r.id,
                    r.hit_rate.unwrap_or(0.0),
                    edges,
                );
            }
            Err(e) => {
                println!(
                    "  retract failed for {}: {:?}",
                    r.id, e
                );
                skipped_referenced.push(r.id.clone());
            }
        }
    }
    println!();
    println!(
        "  retracted: {} | skipped (theory-referenced): {} | skipped (rate ≥ {}): {} | skipped (no data): {}",
        retracted.len(),
        skipped_referenced.len(),
        AXIOM_RETENTION_THRESHOLD,
        skipped_high_rate.len(),
        skipped_no_data.len(),
    );

    // ── Phase 2 ──────────────────────────────────────────────
    println!();
    println!("--- Phase 2: post-cleanup ({} ticks; per-100-tick timed) ---", PHASE_2_TICKS);
    let phase2_start = Instant::now();
    let chunk = 100u64;
    let chunks = PHASE_2_TICKS / chunk;
    for i in 0..chunks {
        let chunk_start = Instant::now();
        rt.run_bounded(chunk);
        let chunk_elapsed = chunk_start.elapsed();
        println!(
            "  Phase 2 chunk {}/{}: tick={} episodes={} elapsed={:.2}s ({:.1}ms/tick)",
            i + 1,
            chunks,
            rt.tick,
            rt.memory.episodes.len(),
            chunk_elapsed.as_secs_f64(),
            chunk_elapsed.as_secs_f64() * 1000.0 / chunk as f64,
        );
    }
    let phase2_elapsed = phase2_start.elapsed();
    println!(
        "tick={} episodes={} theories={} axioms={} total elapsed={:.2}s ({:.1}ms/tick avg)",
        rt.tick,
        rt.memory.episodes.len(),
        rt.rset.theories().len(),
        rt.rset.axioms().len(),
        phase2_elapsed.as_secs_f64(),
        phase2_elapsed.as_secs_f64() * 1000.0 / PHASE_2_TICKS as f64,
    );

    let phase2_theory_ranks = rank_theories(&rt);
    print_tournament("end of Phase 2 (theories)", &phase2_theory_ranks);

    let phase2_axiom_ranks = rank_axioms(&rt);
    print_axiom_table("end of Phase 2, all axioms", &phase2_axiom_ranks);

    // ── Comparison summary ───────────────────────────────────
    println!();
    println!("=== Phase 1 vs Phase 2 comparison ===");
    let p1_theory_count = phase1_theory_ranks.len();
    let p2_theory_count = phase2_theory_ranks.len();
    let p1_axiom_count = phase1_axiom_ranks.len();
    let p2_axiom_count = phase2_axiom_ranks.len();
    let p1_t_qual: Vec<f64> = phase1_theory_ranks
        .iter()
        .filter_map(|r| r.aggregated_hit_rate)
        .collect();
    let p2_t_qual: Vec<f64> = phase2_theory_ranks
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
        p1_theory_count,
        p2_theory_count,
        p2_theory_count as i64 - p1_theory_count as i64,
    );
    println!(
        "axioms:             {} → {} (Δ {})",
        p1_axiom_count,
        p2_axiom_count,
        p2_axiom_count as i64 - p1_axiom_count as i64,
    );
    println!(
        "theory mean rate:   {:.4} → {:.4} (Δ {:+.4})",
        mean(&p1_t_qual),
        mean(&p2_t_qual),
        mean(&p2_t_qual) - mean(&p1_t_qual),
    );
    println!(
        "theory min  rate:   {:.4} → {:.4} (Δ {:+.4})",
        min(&p1_t_qual),
        min(&p2_t_qual),
        min(&p2_t_qual) - min(&p1_t_qual),
    );
    println!(
        "  (axioms retracted at intervention: {})",
        retracted.len()
    );

    // Verify retracted axioms didn't come back.
    let retracted_set: HashSet<String> = retracted.iter().cloned().collect();
    let phase2_axiom_ids: HashSet<String> = phase2_axiom_ranks
        .iter()
        .map(|r| r.id.clone())
        .collect();
    let resurrected: Vec<String> = retracted_set
        .intersection(&phase2_axiom_ids)
        .cloned()
        .collect();
    if resurrected.is_empty() {
        println!("  retracted axioms NOT resurrected during Phase 2 ✓");
    } else {
        println!(
            "  ⚠ retracted axioms re-appeared: {:?}",
            resurrected
        );
    }

    println!();
    println!("--- end ---");
}
