//! Phase Emergence-1 — Shape co-occurrence concept mining.
//!
//! ADR 0074. First implementation step of the concept-emergence
//! pivot (ADR 0073). Demonstrates the full propose-validate-
//! register loop on a real substrate, then re-runs on long5k to
//! show cross-substrate identity portability.
//!
//! Pipeline per substrate:
//!   1. Run runtime to Phase 0 maturity
//!   2. Build TheoryQualityReports (ADR 0071)
//!   3. propose_concept_candidates (ADR 0074 step 1)
//!   4. validate_concept on each candidate (step 2)
//!   5. register_concept for passing candidates (step 3)
//!   6. Print all registered concepts + status
//!
//! Cross-substrate test: do the same constituent-shape ids
//! appear on both substrates? Per Phase 0072-A's finding, OQ#1
//! and long5k @ matching ticks converge to isomorphic RSets, so
//! identical concept ids should mint on both.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment, Event,
    },
    test_substrates::{long5k::build_5k_stream, oq1::build_long_stream},
    ConceptCandidate, ConceptMiningConfig, ConceptStatus, RSet,
    TheoryQualityClass,
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

fn status_label(s: ConceptStatus) -> &'static str {
    match s {
        ConceptStatus::Live => "Live",
        ConceptStatus::Stale => "Stale",
        ConceptStatus::Validated => "Validated",
        ConceptStatus::Falsified => "Falsified",
    }
}

fn run_substrate(
    label: &str,
    stream: Vec<(u64, Event)>,
    ticks: u64,
) -> Vec<ConceptCandidate> {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: {}  (ticks: {})", label, ticks);
    println!("════════════════════════════════════════════════════════");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    let theories: Vec<String> = {
        let mut v: Vec<String> =
            rt.rset.theories().into_iter().map(str::to_owned).collect();
        v.sort();
        v
    };
    let axioms: HashSet<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();
    let families = rt.rset.axiom_shape_families().len();
    println!(
        " Phase 0: {} theories ({:?}); {} axioms; {} L2 families",
        theories.len(), theories, axioms.len(), families,
    );

    // Generate per-theory substrates for cross-precision.
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut g = match rt.rset.generate_substrate_from_theory(
            t, NUM_GEN_IDS, SEED_DENSITY, seed,
        ) {
            Ok(g) => g, Err(_) => continue,
        };
        for ax in &axioms {
            g.register_axiom_with_intension(ax);
        }
        substrates.push(g);
    }

    let mut primary_rates: HashMap<String, f64> = HashMap::new();
    for ax in &axioms {
        if let Some(r) =
            rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS)
        {
            primary_rates.insert(ax.clone(), r);
        }
    }
    let reports = rt.rset.theory_quality_report_all(&substrates, &primary_rates);

    println!();
    println!(" Theory quality summary:");
    println!("   {:<6} {:>8} {:>10} {:>10}",
             "theory", "class", "p_mean", "c_mean");
    for r in &reports {
        let p = r.primary_rate_mean.map(|x| format!("{:.4}", x)).unwrap_or("—".to_string());
        let c = r.cross_precision_mean.map(|x| format!("{:.4}", x)).unwrap_or("—".to_string());
        println!("   {:<6} {:>8} {:>10} {:>10}",
                 r.theory_id, class_label(r.summary_class), p, c);
    }

    // ── Step 1: propose ────────────────────────────────────────
    let cfg = ConceptMiningConfig::default();
    let candidates = rt.rset.propose_concept_candidates(&cfg, &reports);
    println!();
    println!(" Step 1 — Proposed candidates: {}", candidates.len());
    for (i, c) in candidates.iter().enumerate() {
        println!("   [{}] id={}", i, c.id);
        println!("       constituents: {:?}", c.constituent_shapes);
        println!("       attested in:  {:?}", c.theories_attested);
    }

    // ── Step 2: validate ───────────────────────────────────────
    println!();
    println!(" Step 2 — Validating candidates (floor = {:.2})...",
             cfg.validation_floor);
    let mut validated: Vec<ConceptCandidate> = Vec::new();
    for mut c in candidates {
        match rt.rset.validate_concept(&c, &substrates, cfg.validation_floor) {
            Some(mean) => {
                println!("   ✓ {} cross_precision_mean={:.4} → PASS", c.id, mean);
                c.aggregate_cross_precision = Some(mean);
                validated.push(c);
            }
            None => {
                println!("   ✗ {} → FAIL", c.id);
            }
        }
    }

    // ── Step 2.5: prefer maximal subsumed concept ──────────────
    // If both {A, B} and {A, B, C} validated, register only the
    // maximal one. This mirrors ADR 0072's DemoteSuperset
    // philosophy: don't double-register sub-concepts of validated
    // maximal concepts.
    validated.sort_by(|a, b| {
        b.constituent_shapes
            .len()
            .cmp(&a.constituent_shapes.len())
    });
    let mut to_register: Vec<ConceptCandidate> = Vec::new();
    for c in validated {
        let cset: HashSet<&str> =
            c.constituent_shapes.iter().map(String::as_str).collect();
        let subsumed = to_register.iter().any(|r| {
            let rset: HashSet<&str> =
                r.constituent_shapes.iter().map(String::as_str).collect();
            cset.is_subset(&rset) && cset.len() < rset.len()
        });
        if !subsumed {
            to_register.push(c);
        }
    }

    // ── Step 3: register ───────────────────────────────────────
    println!();
    println!(" Step 3 — Registering {} concept(s)...", to_register.len());
    let mut registered: Vec<ConceptCandidate> = Vec::new();
    for c in &to_register {
        match rt.rset.register_concept(c) {
            Ok(id) => {
                let xprec = rt.rset.concept_cross_precision_at_mint(&id)
                    .map(|x| format!("{:.4}", x))
                    .unwrap_or("—".to_string());
                println!("   ✓ registered: {}", id);
                println!("     xprec_at_mint: {}", xprec);
                println!("     status: {}",
                         status_label(rt.rset.concept_status(&id)));
                registered.push(c.clone());
            }
            Err(e) => {
                println!("   ✗ register failed: {:?}", e);
            }
        }
    }

    // ── Final concept inventory ────────────────────────────────
    println!();
    println!(" Concept inventory: {} live", rt.rset.concepts().len());
    for id in rt.rset.concepts() {
        println!("   {}", id);
        println!("     constituents: {:?}",
                 rt.rset.concept_constituent_shapes(id));
        println!("     attested:     {:?}",
                 rt.rset.concept_attested_theories(id));
        println!("     status:       {}",
                 status_label(rt.rset.concept_status(id)));
    }

    registered
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Phase Emergence-1 — Shape co-occurrence concept mining");
    println!("════════════════════════════════════════════════════════");
    println!(" ADR 0074 — propose / validate / register loop on");
    println!(" two structurally-isomorphic substrates (OQ#1 and long5k)");
    println!(" to demonstrate cross-substrate concept identity.");

    // Substrate 1: OQ#1 @ 1000 ticks (canonical Phase 0 maturity)
    let oq1_concepts = run_substrate("OQ#1", build_long_stream(), 1000);

    // Substrate 2: long5k @ 1500 ticks (matching maturity per Phase C.2)
    let long5k_concepts = run_substrate("long5k", build_5k_stream(), 1500);

    // ── Cross-substrate identity check ─────────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Cross-substrate identity check");
    println!("════════════════════════════════════════════════════════");
    let oq1_ids: HashSet<&str> =
        oq1_concepts.iter().map(|c| c.id.as_str()).collect();
    let long5k_ids: HashSet<&str> =
        long5k_concepts.iter().map(|c| c.id.as_str()).collect();
    let shared: Vec<&&str> = oq1_ids.intersection(&long5k_ids).collect();
    let oq1_only: Vec<&&str> = oq1_ids.difference(&long5k_ids).collect();
    let long5k_only: Vec<&&str> = long5k_ids.difference(&oq1_ids).collect();

    println!(" OQ#1 registered:   {}", oq1_ids.len());
    println!(" long5k registered: {}", long5k_ids.len());
    println!(" shared concept ids: {}", shared.len());
    for id in &shared {
        println!("   ✓ {}", id);
    }
    if !oq1_only.is_empty() {
        println!(" OQ#1-only:");
        for id in &oq1_only {
            println!("   - {}", id);
        }
    }
    if !long5k_only.is_empty() {
        println!(" long5k-only:");
        for id in &long5k_only {
            println!("   - {}", id);
        }
    }

    println!();
    if !shared.is_empty() {
        println!(" → POSITIVE — at least one concept identity is portable");
        println!("   across OQ#1 and long5k. Per the Phase 0072-A finding");
        println!("   that the two substrates converge to isomorphic RSets,");
        println!("   this is the expected outcome and validates the");
        println!("   propose-validate-register loop end-to-end.");
    } else if !oq1_concepts.is_empty() || !long5k_concepts.is_empty() {
        println!(" → MIXED — concepts mint on at least one substrate but");
        println!("   identities don't transfer. Investigate shape-id");
        println!("   determinism or mining config mismatch.");
    } else {
        println!(" → NULL — no concepts mint on either substrate. Either");
        println!("   the propose-validate gate is too strict for these");
        println!("   substrates' Phase 0 maturity, or the shape families");
        println!("   discovered don't co-occur as expected.");
    }

    println!();
    println!("--- end ---");
}
