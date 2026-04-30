//! Multi-substrate consolidation diagnostic.
//!
//! Runs the ADR 0070+0071+0072 pipeline on three substrates of
//! decreasing structural friendliness:
//!
//!   1. OQ#1   — 4-regime canonical (per `phase_consolidation_oq1_diagnostic`)
//!   2. long5k — same regime types as OQ#1, 5000-tick budget; per
//!      C.2 produces identical 6 shape families. Validates that the
//!      consolidation triad transfers across the same regime family.
//!   3. OQ#2   — tournament + lattice + star; per C.2.1 the runtime
//!      discovers 0 template axioms (transitivity violations).
//!      Tests graceful-degradation: pipeline must not crash; should
//!      produce empty/Indeterminate output rather than misleading
//!      recommendations.
//!
//! Purpose: the OQ#1 diagnostic showed the triad works on its
//! native substrate. This slice answers "does it work on OTHER
//! substrates?" — both ones structurally similar (long5k) and
//! structurally hostile (OQ#2).

use relatum_v2::{
    runtime::{
        AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    test_substrates::{
        long5k::build_5k_stream, oq1::build_long_stream as oq1_stream,
        oq2::build_oq2_stream,
    },
    FamilyQualityClass, MergeRationale, RSet, RecommendedIntervention,
    TheoryDemoteReason, TheoryQualityClass, TheoryQualityReport,
};
use std::collections::{HashMap, HashSet};

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

fn rec_short(r: &RecommendedIntervention) -> String {
    match r {
        RecommendedIntervention::None => "None".to_string(),
        RecommendedIntervention::ShadowMonitor { reason } => {
            format!("ShadowMonitor({})", reason)
        }
        RecommendedIntervention::FamilyDemote { family_id, family_class } => {
            let cls = match family_class {
                FamilyQualityClass::Signal => "Signal",
                FamilyQualityClass::Noise => "Noise",
                FamilyQualityClass::Uniform => "Uniform",
                FamilyQualityClass::Mixed => "Mixed",
            };
            format!("FamilyDemote({}, {})", family_id, cls)
        }
        RecommendedIntervention::AxiomRepair { axiom_ids } => {
            format!("AxiomRepair({})", axiom_ids.len())
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

fn fmt_opt(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{:.4}", x),
        None => "—".to_string(),
    }
}

#[derive(Debug, Clone)]
struct SubstrateSummary {
    name: String,
    ticks: u64,
    axioms: usize,
    theories: usize,
    l2_families: usize,
    substrates_built: usize,
    qualifying_axioms: usize,
    reports: Vec<TheoryQualityReport>,
    recommendations: Vec<(String, RecommendedIntervention)>,
}

/// Run the full pipeline on one substrate. Returns a self-describing
/// summary so the caller can tabulate across substrates.
fn diagnose(
    name: &str,
    ticks: u64,
    stream: Vec<(u64, Event)>,
) -> SubstrateSummary {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: {}  ({} ticks)", name, ticks);
    println!("════════════════════════════════════════════════════════");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    let axioms: HashSet<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();
    let l2_families = rt.rset.axiom_shape_families().len();

    println!(
        "  axioms={}  theories={} ({:?})  L2 families={}",
        axioms.len(),
        theories.len(),
        theories,
        l2_families,
    );

    // Build substrates per theory
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
    println!("  generated {} per-theory substrates", substrates.len());

    // primary_rates
    let mut primary_rates: HashMap<String, f64> = HashMap::new();
    for ax in &axioms {
        if let Some(r) =
            rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS)
        {
            primary_rates.insert(ax.clone(), r);
        }
    }
    println!(
        "  {} axioms have ≥{} predictions",
        primary_rates.len(),
        MIN_AXIOM_PREDICTIONS,
    );

    // Reports
    let reports = rt.rset.theory_quality_report_all(&substrates, &primary_rates);

    if reports.is_empty() {
        println!();
        println!("  → no theories registered; no reports to compute.");
        println!("  → recommendation pipeline degenerates to empty.");
        return SubstrateSummary {
            name: name.to_string(),
            ticks,
            axioms: axioms.len(),
            theories: theories.len(),
            l2_families,
            substrates_built: substrates.len(),
            qualifying_axioms: primary_rates.len(),
            reports: vec![],
            recommendations: vec![],
        };
    }

    println!();
    println!(
        "  {:<8} {:>4} {:>10} {:>10} {:>7} {:>7} {:>10}",
        "theory", "axs", "p_mean", "c_mean", "noise#", "sig#", "summary",
    );
    for r in &reports {
        println!(
            "  {:<8} {:>4} {:>10} {:>10} {:>7} {:>7} {:>10}",
            r.theory_id,
            r.axiom_count,
            fmt_opt(r.primary_rate_mean),
            fmt_opt(r.cross_precision_mean),
            r.noise_family_axiom_count,
            r.signal_family_axiom_count,
            class_label(r.summary_class),
        );
    }

    // Recommendations
    println!();
    println!("  Recommendations:");
    let mut recs: Vec<(String, RecommendedIntervention)> = Vec::new();
    for r in &reports {
        let others: Vec<_> = reports
            .iter()
            .filter(|o| o.theory_id != r.theory_id)
            .cloned()
            .collect();
        let rec = RSet::recommend_intervention(r, &others);
        println!("    {:<8} {}", r.theory_id, rec_short(&rec));
        recs.push((r.theory_id.clone(), rec));
    }

    SubstrateSummary {
        name: name.to_string(),
        ticks,
        axioms: axioms.len(),
        theories: theories.len(),
        l2_families,
        substrates_built: substrates.len(),
        qualifying_axioms: primary_rates.len(),
        reports,
        recommendations: recs,
    }
}

fn main() {
    println!("=== Multi-substrate consolidation diagnostic (ADR 0070+0071+0072) ===");

    let oq1 = diagnose("OQ#1", 1000, oq1_stream());
    let long5k = diagnose("long5k", 1500, build_5k_stream());
    let oq2 = diagnose("OQ#2", 4500, build_oq2_stream());

    // ── Cross-substrate comparison ─────────────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Cross-substrate comparison");
    println!("════════════════════════════════════════════════════════");
    println!(
        "  {:<10} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
        "substrate", "ticks", "axs", "thrs", "L2", "subs", "qual",
    );
    for s in [&oq1, &long5k, &oq2] {
        println!(
            "  {:<10} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
            s.name,
            s.ticks,
            s.axioms,
            s.theories,
            s.l2_families,
            s.substrates_built,
            s.qualifying_axioms,
        );
    }

    // ── Recommendation tally per substrate ─────────────────────
    println!();
    println!("  Recommendation distribution:");
    for s in [&oq1, &long5k, &oq2] {
        let (mut none, mut shadow, mut fdemote, mut repair, mut tdemote, mut superset, mut merge, mut manual) =
            (0, 0, 0, 0, 0, 0, 0, 0);
        for (_, r) in &s.recommendations {
            match r {
                RecommendedIntervention::None => none += 1,
                RecommendedIntervention::ShadowMonitor { .. } => shadow += 1,
                RecommendedIntervention::FamilyDemote { .. } => fdemote += 1,
                RecommendedIntervention::AxiomRepair { .. } => repair += 1,
                RecommendedIntervention::TheoryDemote { .. } => tdemote += 1,
                RecommendedIntervention::DemoteSuperset { .. } => superset += 1,
                RecommendedIntervention::Merge { .. } => merge += 1,
                RecommendedIntervention::Manual { .. } => manual += 1,
            }
        }
        println!(
            "    {:<10} None={} Shadow={} FamilyDem={} Repair={} TheoryDem={} Super={} Merge={} Manual={}",
            s.name, none, shadow, fdemote, repair, tdemote, superset, merge, manual,
        );
    }

    // ── Cross-substrate sanity verdict ─────────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Sanity verdict per substrate");
    println!("════════════════════════════════════════════════════════");

    fn check_oq1_long5k(s: &SubstrateSummary) -> (usize, usize) {
        // Same expectations as OQ#1: t_2 / t_3 should be None;
        // t_0 should target noise.
        let mut passed = 0;
        let mut total = 0;
        for (label, t, want_noise_target) in [
            ("t_2 → None", "t_2", false),
            ("t_3 → None", "t_3", false),
            ("t_0 → noise-targeting", "t_0", true),
        ] {
            total += 1;
            let rec = s.recommendations.iter().find(|(n, _)| n == t);
            match (rec, want_noise_target) {
                (Some((_, RecommendedIntervention::None)), false) => passed += 1,
                (Some((_, r)), true)
                    if matches!(
                        r,
                        RecommendedIntervention::FamilyDemote { .. }
                            | RecommendedIntervention::TheoryDemote { .. }
                            | RecommendedIntervention::DemoteSuperset { .. }
                            | RecommendedIntervention::AxiomRepair { .. }
                    ) =>
                {
                    passed += 1
                }
                _ => {}
            }
            let _ = label;
        }
        (passed, total)
    }

    // OQ#1
    let (oq1_p, oq1_t) = check_oq1_long5k(&oq1);
    println!(
        "  OQ#1   sanity: {}/{} (expected: t_0 noise-target; t_2/t_3 None)",
        oq1_p, oq1_t,
    );

    // long5k — same expected pattern as OQ#1
    let (l5_p, l5_t) = check_oq1_long5k(&long5k);
    println!(
        "  long5k sanity: {}/{} (expected: same as OQ#1 — same regime types per C.2)",
        l5_p, l5_t,
    );

    // OQ#2 — substrate is structurally hostile per C.2.1.
    // Sanity check: the pipeline must NOT crash; if it produced
    // any reports, recommendations should NOT be aggressive
    // (no FamilyDemote on a 0-family substrate, etc.).
    println!();
    println!(
        "  OQ#2 graceful-degradation:",
    );
    println!(
        "    - axioms={}  theories={}  L2 families={}",
        oq2.axioms, oq2.theories, oq2.l2_families,
    );
    let no_aggressive = oq2.recommendations.iter().all(|(_, r)| {
        !matches!(
            r,
            RecommendedIntervention::FamilyDemote { .. }
                | RecommendedIntervention::TheoryDemote { .. }
        )
    });
    if oq2.theories == 0 {
        println!("    → empty pipeline (no theories); no recommendations to make ✓");
    } else if no_aggressive {
        println!(
            "    → pipeline produced {} recommendation(s); none aggressive (no FamilyDemote/TheoryDemote) ✓",
            oq2.recommendations.len(),
        );
    } else {
        println!("    → ✗ aggressive recommendation on a structurally-hostile substrate");
    }

    // ── Final verdict ──────────────────────────────────────────
    let oq1_ok = oq1_p == oq1_t;
    let l5_ok = l5_p == l5_t;
    let oq2_ok = oq2.theories == 0 || no_aggressive;
    println!();
    println!("════════════════════════════════════════════════════════");
    if oq1_ok && l5_ok && oq2_ok {
        println!(
            "  → STRONGLY POSITIVE: triad works across all 3 substrates."
        );
        println!("    OQ#1 + long5k: same-regime-type generalization.");
        println!("    OQ#2: graceful degradation on structurally-hostile data.");
    } else {
        println!(
            "  → MIXED: OQ#1={}, long5k={}, OQ#2={}",
            if oq1_ok { "✓" } else { "✗" },
            if l5_ok { "✓" } else { "✗" },
            if oq2_ok { "✓" } else { "✗" },
        );
    }
    println!();
    println!("--- end ---");
}
