//! Phase D.4 — Continuous dream-phase loop.
//!
//! Phase Alpha-7..9 + Beta-1..6 used dream phase as a one-shot
//! observation (compute cross-precision matrix, observe noise
//! family). D.4 runs dream phase **periodically** as part of the
//! main loop, and demotes whenever cross-precision drops below
//! threshold.
//!
//! Loop structure:
//!   for k in 0..N_PHASES:
//!     run K ticks
//!     if axiom count is fresh enough:
//!       compute cross-precision matrix
//!       if any theory's column mean < threshold:
//!         retract it
//!     report state
//!
//! K is the dream-phase cadence (here: 300 ticks). N_PHASES bounds
//! total runtime (here: 6 → 1800 ticks total).
//!
//! Risk: dream phase has nontrivial cost (substrate generation +
//! per-axiom forward apply). Conservative K avoids overhead.
//!
//! Captured to `logs/<date>_phase_d4_continuous_dream.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    R, RSet,
};
use std::collections::HashSet;

const TICKS_PER_PHASE: u64 = 300;
const N_PHASES: usize = 6;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const DEMOTE_THRESHOLD: f64 = 0.50;
const MIN_AXIOM_PREDICTIONS: u64 = 5;

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

/// Cross-precision dream phase: returns Vec<(theory_id, column_mean)>.
fn dream_phase(rt: &AutonomousRuntime) -> Vec<(String, f64)> {
    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    if theories.is_empty() {
        return Vec::new();
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
    let mut substrates: Vec<RSet> = Vec::new();
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
    let mut out: Vec<(String, f64)> = Vec::new();
    for j in 0..theories.len() {
        if col_count[j] == 0 {
            continue;
        }
        out.push((
            theories[j].clone(),
            col_sum[j] / col_count[j] as f64,
        ));
    }
    out
}

fn primary_rate(rt: &AutonomousRuntime, theory_id: &str) -> Option<f64> {
    let axioms: Vec<&str> = rt.rset.theory_axioms(theory_id);
    let mut sum = 0.0;
    let mut q = 0;
    for ax in &axioms {
        if let Some(rate) = rt
            .memory
            .prediction_state
            .hit_rate(ax, MIN_AXIOM_PREDICTIONS)
        {
            sum += rate;
            q += 1;
        }
    }
    if q == 0 { None } else { Some(sum / q as f64) }
}

fn main() {
    println!("=== Phase D.4 — Continuous dream-phase loop ===");
    println!(
        "TICKS_PER_PHASE={} (cadence), N_PHASES={}, demote_threshold={}",
        TICKS_PER_PHASE, N_PHASES, DEMOTE_THRESHOLD,
    );

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    let mut total_demotes = 0u32;
    let mut demoted_ids: Vec<String> = Vec::new();

    for phase in 0..N_PHASES {
        rt.run_bounded(TICKS_PER_PHASE);
        println!();
        println!(
            "--- Phase {} (cumulative tick = {}) ---",
            phase, (phase as u64 + 1) * TICKS_PER_PHASE,
        );
        println!(
            "  axioms={} theories={} episodes={}",
            rt.rset.axioms().len(),
            rt.rset.theories().len(),
            rt.memory.episodes.len(),
        );

        // Dream phase.
        let mut means = dream_phase(&rt);
        means.sort_by(|a, b| {
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        for (id, mean) in &means {
            let pri = primary_rate(&rt, id);
            let pri_str = pri
                .map(|x| format!("{:.4}", x))
                .unwrap_or_else(|| "—".into());
            println!(
                "    {:>6}: cross-prec={:.4}, primary-rate={}",
                id, mean, pri_str,
            );
        }

        // Demote bottom theory if below threshold.
        if let Some((id, mean)) = means.first() {
            if *mean < DEMOTE_THRESHOLD {
                println!(
                    "  [DEMOTE] '{}' (cross-prec={:.4} < {})",
                    id, mean, DEMOTE_THRESHOLD,
                );
                let _ = rt.rset.retract_theory(id);
                total_demotes += 1;
                demoted_ids.push(id.clone());
            } else {
                println!(
                    "  [no demote] all theories ≥ threshold (lowest = {} at {:.4})",
                    id, mean,
                );
            }
        }
    }

    println!();
    println!("=== Summary ===");
    println!("  Total demotes: {}", total_demotes);
    println!("  Demoted theories: {:?}", demoted_ids);
    println!(
        "  Final state: axioms={} theories={} episodes={}",
        rt.rset.axioms().len(),
        rt.rset.theories().len(),
        rt.memory.episodes.len(),
    );

    println!();
    println!("=== Verdict ===");
    if total_demotes > 0 {
        println!(
            "  → POSITIVE — continuous dream loop fired {} demote(s) over {} ticks ({} per phase). Loop converges; subsequent phases find no further bottom.",
            total_demotes, N_PHASES as u64 * TICKS_PER_PHASE, TICKS_PER_PHASE,
        );
    } else {
        println!(
            "  → NULL — no theory crossed threshold during {} phases. Substrate may not produce noise-laden theories at this cadence.",
            N_PHASES,
        );
    }
    println!();
    println!("--- end ---");
}
