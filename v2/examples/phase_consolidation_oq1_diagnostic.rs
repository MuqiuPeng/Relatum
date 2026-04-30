//! Consolidation triad end-to-end diagnostic on OQ#1.
//!
//! Runs the full ADR 0070 + 0071 + 0072 pipeline on a freshly
//! converged OQ#1 runtime and prints, for every theory:
//!  - The unified quality report (ADR 0071)
//!  - The recommended intervention (ADR 0072)
//!
//! Purpose: validate that the consolidated APIs produce sensible
//! recommendations on REAL data, not just the synthetic-input
//! unit tests. Specifically check:
//!  - t_0 (noisy, contains the variance-zero noise family) →
//!    expect FamilyDemote(shape_premise_p0-0_p1-2)
//!  - t_2 / t_3 (universal predictors, cross-prec 1.0) → expect None
//!  - t_1 (mid-quality, cross-prec ~0.68) → expect Manual / Merge
//!
//! No new mechanism — this is a demonstration that the layer/
//! observation/policy triad works end-to-end on the canonical
//! substrate.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    FamilyQualityClass, MergeRationale, RSet, RecommendedIntervention,
    TheoryDemoteReason, TheoryQualityClass,
};
use std::collections::{HashMap, HashSet};

const TICKS_PHASE_0: u64 = 1000;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const MIN_AXIOM_PREDICTIONS: u64 = 5;

fn class_label(c: TheoryQualityClass) -> &'static str {
    match c {
        TheoryQualityClass::Signal => "Signal",
        TheoryQualityClass::Mixed => "Mixed",
        TheoryQualityClass::Noise => "Noise",
        TheoryQualityClass::Indeterminate => "Indet.",
    }
}

fn fclass_label(c: Option<FamilyQualityClass>) -> &'static str {
    match c {
        Some(FamilyQualityClass::Signal) => "Signal",
        Some(FamilyQualityClass::Noise) => "Noise",
        Some(FamilyQualityClass::Uniform) => "Uniform",
        Some(FamilyQualityClass::Mixed) => "Mixed",
        None => "—",
    }
}

fn rec_label(r: &RecommendedIntervention) -> String {
    match r {
        RecommendedIntervention::None => "None".to_string(),
        RecommendedIntervention::ShadowMonitor { reason } => {
            format!("ShadowMonitor({})", reason)
        }
        RecommendedIntervention::FamilyDemote { family_id, family_class } => {
            format!("FamilyDemote({}, {:?})", family_id, family_class)
        }
        RecommendedIntervention::AxiomRepair { axiom_ids } => {
            format!("AxiomRepair({:?})", axiom_ids)
        }
        RecommendedIntervention::TheoryDemote { reason } => match reason {
            TheoryDemoteReason::BothDimensionsLow => {
                "TheoryDemote(BothDimensionsLow)".to_string()
            }
            TheoryDemoteReason::NoiseDominated => {
                "TheoryDemote(NoiseDominated)".to_string()
            }
        },
        RecommendedIntervention::DemoteSuperset { cleaner_subset_theory } => {
            format!("DemoteSuperset(of {})", cleaner_subset_theory)
        }
        RecommendedIntervention::Merge { partner_theory, rationale } => {
            let r = match rationale {
                MergeRationale::Equivalent => "Equivalent",
                MergeRationale::Complementary => "Complementary",
                MergeRationale::HighQualityBoth => "HighQualityBoth",
            };
            format!("Merge(with {}, {})", partner_theory, r)
        }
        RecommendedIntervention::Manual { reason } => {
            format!("Manual({})", reason)
        }
    }
}

fn fmt_opt_f64(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{:.4}", x),
        None => "—".to_string(),
    }
}

fn main() {
    println!("=== ADR 0070 + 0071 + 0072 end-to-end diagnostic on OQ#1 ===");

    // ── Train on OQ#1 to convergence ────────────────────────────
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    let axioms: HashSet<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();
    let families = rt.rset.axiom_shape_families().len();

    println!();
    println!("Post-convergence state (after {} ticks):", TICKS_PHASE_0);
    println!("  axioms      : {}", axioms.len());
    println!("  theories    : {} ({:?})", theories.len(), theories);
    println!("  L2 families : {}", families);

    // ── Generate per-theory imagined substrates (F.1 setup) ─────
    println!();
    println!("Generating {} per-theory substrates (NUM_GEN_IDS={}, density={})...",
             theories.len(), NUM_GEN_IDS, SEED_DENSITY);
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut g = match rt.rset.generate_substrate_from_theory(
            t, NUM_GEN_IDS, SEED_DENSITY, seed,
        ) {
            Ok(g) => g,
            Err(_) => continue,
        };
        for ax in &axioms {
            g.register_axiom_with_intension(ax);
        }
        substrates.push(g);
    }
    println!("  built {} substrates", substrates.len());

    // ── Build primary_rates from prediction_state ───────────────
    let mut primary_rates: HashMap<String, f64> = HashMap::new();
    for ax in &axioms {
        if let Some(r) =
            rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS)
        {
            primary_rates.insert(ax.clone(), r);
        }
    }
    println!();
    println!("primary_rates: {} axioms have ≥{} predictions",
             primary_rates.len(), MIN_AXIOM_PREDICTIONS);

    // ── ADR 0071: build all reports ─────────────────────────────
    let reports = rt.rset.theory_quality_report_all(&substrates, &primary_rates);

    println!();
    println!("=== ADR 0071 — Theory quality reports ===");
    println!(
        "{:<8} {:>4} {:>10} {:>10} {:>10} {:>10} {:>8} {:>8} {:>10}",
        "theory", "axs", "p_mean", "p_min", "c_mean", "c_min",
        "noise#", "signal#", "summary",
    );
    for r in &reports {
        println!(
            "{:<8} {:>4} {:>10} {:>10} {:>10} {:>10} {:>8} {:>8} {:>10}",
            r.theory_id,
            r.axiom_count,
            fmt_opt_f64(r.primary_rate_mean),
            fmt_opt_f64(r.primary_rate_min),
            fmt_opt_f64(r.cross_precision_mean),
            fmt_opt_f64(r.cross_precision_min),
            r.noise_family_axiom_count,
            r.signal_family_axiom_count,
            class_label(r.summary_class),
        );
    }

    // ── Per-theory deep dive: family memberships + neighborhood ─
    println!();
    println!("=== Per-theory family memberships ===");
    for r in &reports {
        println!();
        println!("{} (axioms: {}):", r.theory_id, r.axiom_count);
        if r.family_memberships.is_empty() {
            println!("  (no family memberships)");
        }
        for m in &r.family_memberships {
            let q = match &m.quality {
                Some(q) => format!(
                    "mean={:.4} std={:.4}",
                    q.mean, q.std,
                ),
                None => "—".to_string(),
            };
            println!(
                "  {} [{}] {}/{} members; class={}; {}",
                m.family_id,
                m.kind.unwrap_or("?"),
                m.members_in_theory,
                m.family_total_members,
                fclass_label(m.class),
                q,
            );
        }
        if let Some(n) = &r.neighborhood {
            let any = !n.equal.is_empty()
                || !n.extends.is_empty()
                || !n.extended_by.is_empty()
                || !n.parallel.is_empty()
                || !n.independent.is_empty();
            if any {
                print!("  neighbors: ");
                if !n.equal.is_empty() {
                    print!("equal={:?} ", n.equal);
                }
                if !n.extends.is_empty() {
                    print!("extends={:?} ", n.extends);
                }
                if !n.extended_by.is_empty() {
                    print!("extended_by={:?} ", n.extended_by);
                }
                if !n.parallel.is_empty() {
                    print!("parallel={:?} ", n.parallel);
                }
                if !n.independent.is_empty() {
                    print!("independent={:?}", n.independent);
                }
                println!();
            }
        }
    }

    // ── ADR 0072: classifier per theory ─────────────────────────
    println!();
    println!("=== ADR 0072 — Recommended interventions ===");
    println!("{:<8} {:<60}", "theory", "recommendation");
    let mut recommendations: Vec<(String, RecommendedIntervention)> = Vec::new();
    for r in &reports {
        let others: Vec<_> = reports
            .iter()
            .filter(|o| o.theory_id != r.theory_id)
            .cloned()
            .collect();
        let rec = RSet::recommend_intervention(r, &others);
        println!("{:<8} {}", r.theory_id, rec_label(&rec));
        recommendations.push((r.theory_id.clone(), rec));
    }

    // ── Sanity verdict ──────────────────────────────────────────
    println!();
    println!("=== Sanity verdict ===");

    // Check 1: t_2 and t_3 should both be None (Signal-class universal predictors).
    let t2_none = recommendations.iter().any(|(t, r)| {
        t == "t_2" && matches!(r, RecommendedIntervention::None)
    });
    let t3_none = recommendations.iter().any(|(t, r)| {
        t == "t_3" && matches!(r, RecommendedIntervention::None)
    });
    println!(
        "  [1] t_2 → None (Signal-class expected): {}",
        if t2_none { "✓" } else { "✗" },
    );
    println!(
        "  [2] t_3 → None (Signal-class expected): {}",
        if t3_none { "✓" } else { "✗" },
    );

    // Check 3: t_0 should target the noise family.
    let t0_targets_noise = recommendations.iter().any(|(t, r)| {
        if t != "t_0" {
            return false;
        }
        match r {
            RecommendedIntervention::FamilyDemote { family_id, .. } => {
                family_id == "shape_premise_p0-0_p1-2"
            }
            RecommendedIntervention::DemoteSuperset { .. } => true,
            RecommendedIntervention::TheoryDemote { .. } => true,
            _ => false,
        }
    });
    println!(
        "  [3] t_0 → noise-targeting intervention: {}",
        if t0_targets_noise { "✓" } else { "✗" },
    );

    // Check 4: classifier produces non-Manual on at least 3 of 4
    // theories (otherwise the heuristics are too conservative).
    let non_manual = recommendations
        .iter()
        .filter(|(_, r)| !matches!(r, RecommendedIntervention::Manual { .. }))
        .count();
    println!(
        "  [4] non-Manual recommendations: {}/{}",
        non_manual,
        recommendations.len(),
    );

    let all_pass = t2_none && t3_none && t0_targets_noise && non_manual >= 3;
    println!();
    if all_pass {
        println!("  → STRONGLY POSITIVE — classifier produces all expected recommendations on OQ#1.");
        println!("    Consolidation triad (0070 + 0071 + 0072) validated end-to-end.");
    } else {
        println!("  → MIXED — some checks failed; see per-check status above.");
    }

    println!();
    println!("--- end ---");
}
