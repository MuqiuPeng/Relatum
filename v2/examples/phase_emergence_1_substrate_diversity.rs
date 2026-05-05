//! Phase Emergence-1 — Substrate-diversity falsifiability probe.
//!
//! ADR 0074's first ship validated concept identity portability
//! across OQ#1 + long5k. But Phase 0072-A showed those substrates
//! converge to isomorphic RSets at matching ticks — so identity
//! transfer between them is structurally "free".
//!
//! The real falsifiability test: does the concept mint identically
//! on substrates with **structurally distinct discovered theories**?
//! - narrow_a: only diamond posets, 5 phases × 100 ticks
//! - OQ#2: tournament + lattice + star, 3 regimes × 5 phases
//!
//! Predictions:
//! - **Strong universality**: same concept id on all 4 substrates
//!   → the (shape_conclusion_c0-2, shape_premise_p0-1_p1-2) pair
//!   is a structural feature of *any* sufficiently mature signal
//!   theory.
//! - **OQ#1-clade universality**: id portable on OQ#1 + long5k
//!   only; narrow_a / OQ#2 produce different concept ids (or
//!   none) → concept is structural but per-clade. Still genuine
//!   creation; the universality scope is narrower.
//! - **Substrate-specific**: each substrate produces its own set
//!   of concept ids with no overlap → concepts are real but
//!   episode-scoped. Phase Emergence-1's portability claim
//!   weakens to "deterministic given the mined RSet".
//! - **Null**: narrow_a / OQ#2 don't produce any concepts at any
//!   ticks → either too sparse for the propose pipeline, or
//!   concept-mining requires more theory diversity than these
//!   substrates produce. Need richer substrates or relaxed
//!   `min_theories` config.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment, Event,
    },
    test_substrates::{
        long5k::build_5k_stream, narrow_a::build_narrow_a_stream,
        oq1::build_long_stream, oq2::build_oq2_stream,
    },
    ConceptCandidate, ConceptMiningConfig, RSet,
};
use std::collections::{HashMap, HashSet};

const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const MIN_AXIOM_PREDICTIONS: u64 = 5;

#[derive(Debug, Clone)]
struct ProbeOutcome {
    label: String,
    ticks: u64,
    theory_count: usize,
    axiom_count: usize,
    family_count: usize,
    proposed: usize,
    validated: usize,
    registered: Vec<ConceptCandidate>,
}

fn probe(label: &str, stream: Vec<(u64, Event)>, ticks: u64) -> ProbeOutcome {
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
        " Phase 0: {} theories, {} axioms, {} L2 families",
        theories.len(), axioms.len(), families,
    );
    println!(" theories: {:?}", theories);

    if theories.is_empty() || families < 2 {
        println!(
            " → SKIP: need ≥2 shape families and ≥1 theory; not enough\n   structure at this horizon."
        );
        return ProbeOutcome {
            label: label.to_string(),
            ticks,
            theory_count: theories.len(),
            axiom_count: axioms.len(),
            family_count: families,
            proposed: 0,
            validated: 0,
            registered: Vec::new(),
        };
    }

    // Substrates + reports.
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut g = match rt.rset.generate_substrate_from_theory(
            t, NUM_GEN_IDS, SEED_DENSITY, seed,
        ) {
            Ok(g) => g, Err(_) => continue,
        };
        for ax in &axioms { g.register_axiom_with_intension(ax); }
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

    // Mining.
    let cfg = ConceptMiningConfig::default();
    let proposed = rt.rset.propose_concept_candidates(&cfg, &reports);
    println!(" propose: {} candidate(s)", proposed.len());

    let mut validated_list: Vec<ConceptCandidate> = Vec::new();
    for mut c in proposed.clone() {
        if let Some(mean) = rt.rset.validate_concept(&c, &substrates, cfg.validation_floor) {
            c.aggregate_cross_precision = Some(mean);
            validated_list.push(c);
        }
    }
    println!(" validate: {} pass", validated_list.len());

    // Subsumption (prefer maximal validated concept).
    validated_list.sort_by(|a, b| {
        b.constituent_shapes
            .len()
            .cmp(&a.constituent_shapes.len())
    });
    let mut to_register: Vec<ConceptCandidate> = Vec::new();
    for c in validated_list.clone() {
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

    let mut registered: Vec<ConceptCandidate> = Vec::new();
    for c in &to_register {
        if rt.rset.register_concept(c).is_ok() {
            registered.push(c.clone());
        }
    }
    println!(" register: {} concept(s)", registered.len());
    for c in &registered {
        println!(
            "   ✓ {}: constituents={:?}, attested={:?}, xprec={:.4}",
            c.id,
            c.constituent_shapes,
            c.theories_attested,
            c.aggregate_cross_precision.unwrap_or(0.0),
        );
    }

    ProbeOutcome {
        label: label.to_string(),
        ticks,
        theory_count: theories.len(),
        axiom_count: axioms.len(),
        family_count: families,
        proposed: proposed.len(),
        validated: validated_list.len(),
        registered,
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Phase Emergence-1 — Substrate-diversity falsifiability probe");
    println!("════════════════════════════════════════════════════════");
    println!(" Tests concept identity portability across substrates with");
    println!(" structurally distinct discovered theories. The real falsifiability");
    println!(" test for ADR 0074's universality claim.");

    let mut outcomes: Vec<ProbeOutcome> = Vec::new();

    // OQ#1 + long5k — known isomorphic at chosen ticks (Phase 0072-A baseline).
    outcomes.push(probe("OQ#1", build_long_stream(), 1000));
    outcomes.push(probe("long5k", build_5k_stream(), 1500));

    // narrow_a — only diamond posets, 5 phases × 100 ticks (~500 total events).
    outcomes.push(probe("narrow_a", build_narrow_a_stream(), 500));

    // OQ#2 — tournament + lattice + star. Probe a few maturity levels
    // since 1500-tick reconnaissance was sparse.
    outcomes.push(probe("OQ#2 @ 1500", build_oq2_stream(), 1500));
    outcomes.push(probe("OQ#2 @ 3000", build_oq2_stream(), 3000));
    outcomes.push(probe("OQ#2 @ 4500", build_oq2_stream(), 4500));

    // ── Cross-substrate identity matrix ────────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Cross-substrate identity matrix");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(
        " {:<14} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5}",
        "substrate", "ticks", "ths", "axs", "fam", "prop", "reg",
    );
    for o in &outcomes {
        println!(
            " {:<14} {:>5} {:>5} {:>5} {:>5} {:>5} {:>5}",
            o.label, o.ticks, o.theory_count, o.axiom_count,
            o.family_count, o.proposed, o.registered.len(),
        );
    }

    // Build the union of all registered ids.
    let mut all_ids: HashSet<String> = HashSet::new();
    for o in &outcomes {
        for c in &o.registered {
            all_ids.insert(c.id.clone());
        }
    }
    let mut sorted_ids: Vec<String> = all_ids.into_iter().collect();
    sorted_ids.sort();

    if sorted_ids.is_empty() {
        println!();
        println!(" No concepts registered on ANY substrate. Either thresholds");
        println!(" too strict or substrates too sparse.");
        println!();
        println!("--- end ---");
        return;
    }

    println!();
    println!(" Concept id × substrate (✓ = registered):");
    print!(" {:<35}", "concept id");
    for o in &outcomes {
        print!(" {:<14}", o.label);
    }
    println!();
    for id in &sorted_ids {
        // Truncate id for display.
        let short: String = if id.len() > 32 { format!("{}…", &id[..32]) } else { id.clone() };
        print!(" {:<35}", short);
        for o in &outcomes {
            let hit = o.registered.iter().any(|c| &c.id == id);
            print!(" {:<14}", if hit { "✓" } else { "—" });
        }
        println!();
    }

    // Constituents reveal the substrate's structural fingerprint.
    println!();
    println!(" Constituents per concept id:");
    for id in &sorted_ids {
        let example = outcomes.iter()
            .flat_map(|o| o.registered.iter())
            .find(|c| &c.id == id);
        if let Some(c) = example {
            println!("   {}", id);
            println!("     {:?}", c.constituent_shapes);
        }
    }

    // ── Universality verdict ───────────────────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Universality verdict");
    println!("════════════════════════════════════════════════════════");

    let n_substrates_with_concepts = outcomes.iter()
        .filter(|o| !o.registered.is_empty())
        .count();
    let universal_ids: Vec<&String> = sorted_ids.iter()
        .filter(|id| outcomes.iter()
            .filter(|o| !o.registered.is_empty())
            .all(|o| o.registered.iter().any(|c| &c.id == *id)))
        .collect();

    println!(
        " Substrates that minted ≥1 concept: {}/{}",
        n_substrates_with_concepts, outcomes.len(),
    );
    println!(" Universal concept ids (mint on every minting substrate): {}", universal_ids.len());
    for id in &universal_ids {
        println!("   ✓ {}", id);
    }

    println!();
    if universal_ids.is_empty() {
        if n_substrates_with_concepts >= 2 {
            println!(" → SUBSTRATE-SPECIFIC — concepts mint on multiple substrates");
            println!("   but no id is shared. Concepts are real (deterministic given");
            println!("   their RSet) but episode-scoped. The portability across OQ#1 +");
            println!("   long5k seen in the Phase 0074 ship was due to RSet isomorphism,");
            println!("   NOT structural universality.");
        } else if n_substrates_with_concepts == 1 {
            println!(" → ISOLATED — only one substrate produces concepts. Concept");
            println!("   mining is functional but not yet substrate-agnostic.");
        } else {
            println!(" → NULL — no substrate registers any concept. Mining pipeline");
            println!("   is too strict for current substrate diversity.");
        }
    } else if universal_ids.len() == sorted_ids.len() {
        println!(" → STRONG UNIVERSALITY — every minted concept id is universal across");
        println!("   substrates that produce concepts. The mined patterns are");
        println!("   substrate-structural, not stream-dependent.");
    } else {
        println!(" → MIXED — some concept ids are universal, others substrate-specific.");
        println!("   The universal ids represent genuinely cross-structural patterns;");
        println!("   the others are clade-bound. Both kinds are useful but for different");
        println!("   purposes: universal concepts predict; specific concepts characterize.");
    }

    println!();
    println!("--- end ---");
}
