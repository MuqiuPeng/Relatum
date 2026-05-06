//! ADR 0077 — Pattern quality framework audit.
//!
//! Runs OQ#1 / OQ#2 to maturity, manually invokes
//! `autonomous_pass` to mint a fresh pattern population, then
//! audits every pattern with the new `pattern_quality_report` +
//! `recommend_pattern_intervention` API.
//!
//! Expected reading after Phase Emergence's prior findings:
//! - OQ#1-clade patterns at sizes 2-5 (kernel audit's 7 patterns)
//!   should mostly be Signal or Mixed
//! - OQ#2-only patterns (Phase 0075 piece 3 finding) should
//!   include the 84-instance 3-cycle as a clear Signal
//! - With imagined-substrate cross validation, some patterns
//!   may flip to Anomalous if they don't appear elsewhere

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment, Event,
    },
    test_substrates::{oq1::build_long_stream, oq2::build_oq2_stream},
    AutonomousConfig, DiscoveryConfig, NamingPolicy, PatternQualityClass,
    RSet, RecommendedPatternIntervention, RefinementConfig,
    SamplingMatchConfig,
};

const RNG_SEED: u64 = 0xC0FFEE;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const SUBSTRATE_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const SAMPLING_BUDGET: usize = 200;

fn run_pass_on_size(rt: &mut AutonomousRuntime, size: usize) {
    let cfg = AutonomousConfig {
        discovery: DiscoveryConfig {
            target_size: size,
            sample_count: 400,
            top_m: 20,
            rng_seed: RNG_SEED.wrapping_add(size as u64 * 0x9E37),
            include_meta_in_discovery: false,
        },
        refinement: RefinementConfig {
            max_tries: 200,
            rng_seed: RNG_SEED.wrapping_add(size as u64 * 0xDEAD),
        },
        naming: NamingPolicy::default(),
        instance_sampling: None,
    };
    let _ = rt.rset.autonomous_pass(&cfg);
}

fn class_label(c: PatternQualityClass) -> &'static str {
    match c {
        PatternQualityClass::Signal => "Signal",
        PatternQualityClass::Mixed => "Mixed",
        PatternQualityClass::Redundant => "Redundant",
        PatternQualityClass::Anomalous => "Anomalous",
        PatternQualityClass::Indeterminate => "Indeterminate",
    }
}

fn rec_label(r: &RecommendedPatternIntervention) -> String {
    match r {
        RecommendedPatternIntervention::None => "None".to_string(),
        RecommendedPatternIntervention::ShadowMonitor { reason } => {
            format!("ShadowMonitor: {}", reason)
        }
        RecommendedPatternIntervention::PatternRetract { reason } => {
            format!("PatternRetract: {}", reason)
        }
        RecommendedPatternIntervention::PatternMergeWith { partner, reason } => {
            format!("PatternMergeWith({}): {}", partner, reason)
        }
        RecommendedPatternIntervention::Manual { reason } => {
            format!("Manual: {}", reason)
        }
    }
}

fn audit(label: &str, stream: Vec<(u64, Event)>, ticks: u64) {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: {} ({} ticks)", label, ticks);
    println!("════════════════════════════════════════════════════════");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    // Manual mint to populate pattern set (sizes 2-5).
    for size in [2usize, 3, 4, 5] {
        run_pass_on_size(&mut rt, size);
    }

    let pattern_count = rt.rset.patterns().len();
    if pattern_count == 0 {
        println!(" No patterns minted. Skipping.");
        return;
    }
    println!(" {} patterns minted.", pattern_count);

    // Build cross-substrate validation set: generate one
    // imagined substrate per registered theory.
    let theories: Vec<String> = rt
        .rset
        .theories()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = SUBSTRATE_SEED_BASE
            .wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        if let Ok(g) = rt.rset.generate_substrate_from_theory(
            t, NUM_GEN_IDS, SEED_DENSITY, seed,
        ) {
            substrates.push(g);
        }
    }
    println!(
        " Cross-substrate validation: {} substrates from {} theories (sampling budget {})",
        substrates.len(), theories.len(), SAMPLING_BUDGET,
    );

    // ADR 0077 originally used exhaustive find_instances_of which
    // hung on size-4/5 canonicals over 100-node substrates. Now
    // pass a SamplingMatchConfig so cross-substrate matching uses
    // sample_instances_of (ADR 0024) — bounded cost, approximate
    // counts. The Anomalous classification path is now reachable.
    let sampling_cfg = SamplingMatchConfig {
        sample_count: SAMPLING_BUDGET,
        rng_seed: RNG_SEED,
    };
    let reports = rt.rset.pattern_quality_report_all(
        &substrates,
        Some(&sampling_cfg),
    );

    println!();
    println!(" === Pattern quality reports ===");
    println!(
        " {:<6} {:>4} {:>4} {:>4} {:>5} {:>5} {:>5} {:>6} {:<10}",
        "id", "size", "role", "inst", "part", "mdl", "xsub", "ovr",
        "class",
    );
    println!(" {}", "─".repeat(78));
    for r in &reports {
        let xsub = r
            .cross_substrate_match_count
            .map(|n| format!("{}", n))
            .unwrap_or_else(|| "—".to_string());
        println!(
            " {:<6} {:>4} {:>4} {:>4} {:>5} {:>5} {:>5} {:>6.2} {:<10}",
            r.pattern_id,
            r.canonical_size,
            r.role_count,
            r.instance_count,
            r.distinct_participants,
            r.mdl_gain,
            xsub,
            r.overlap_score,
            class_label(r.summary_class),
        );
    }

    println!();
    println!(" === Recommendations ===");
    for r in &reports {
        let rec = RSet::recommend_pattern_intervention(r, &reports);
        println!("   {} → {}", r.pattern_id, rec_label(&rec));
    }

    // Aggregate distribution.
    let mut by_class: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();
    for r in &reports {
        *by_class.entry(class_label(r.summary_class)).or_insert(0) += 1;
    }
    println!();
    println!(" === Class distribution ===");
    let mut classes: Vec<_> = by_class.into_iter().collect();
    classes.sort_by(|a, b| b.1.cmp(&a.1));
    for (k, c) in classes {
        println!("   {:<14} {}", k, c);
    }

    // Highlight extremes.
    let max_mdl = reports.iter().map(|r| r.mdl_gain).max().unwrap_or(0);
    let max_inst = reports.iter().map(|r| r.instance_count).max().unwrap_or(0);
    println!();
    println!(" === Extremes ===");
    if let Some(top_mdl) = reports.iter().find(|r| r.mdl_gain == max_mdl) {
        println!(
            "   highest MDL gain: {} ({}, instances={}, class={})",
            top_mdl.pattern_id, max_mdl, top_mdl.instance_count,
            class_label(top_mdl.summary_class),
        );
    }
    if let Some(top_inst) = reports.iter().find(|r| r.instance_count == max_inst) {
        println!(
            "   highest instance count: {} ({}, class={})",
            top_inst.pattern_id, max_inst, class_label(top_inst.summary_class),
        );
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" ADR 0077 — Pattern quality + intervention audit");
    println!("════════════════════════════════════════════════════════");
    println!(" Builds the pattern population (autonomous_pass sizes 2-5)");
    println!(" then runs pattern_quality_report + recommend_pattern_");
    println!(" intervention on each. Cross-substrate validation uses");
    println!(" imagined substrates generated from each theory.");

    audit("OQ#1", build_long_stream(), 1000);
    audit("OQ#2", build_oq2_stream(), 4500);

    println!();
    println!("--- end ---");
}
