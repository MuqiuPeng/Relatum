//! Capability demonstration — what v2 can do, end-to-end on
//! one substrate (OQ#2), showing every layer of the cognitive
//! substrate stack as of 2026-05-07.
//!
//! Sections:
//!   1. Stream → axiom discovery (runtime, ADR 0027 / 0030)
//!   2. Theory quality (ADR 0071)
//!   3. Theory intervention (ADR 0072)
//!   4. Pattern emergence (ADR 0010 / 0018)
//!   5. Pattern shape rendering (ADR 0075 b)
//!   6. Pattern quality (ADR 0077, with cross-substrate sampling)
//!   7. Pattern intervention (ADR 0077)
//!   8. Micro-agent ecosystem (ADR 0076)
//!   9. Drive signal (ADR 0078)
//!
//! Picks OQ#2 as the substrate — it's the most "challenging"
//! one in v2's repertoire (sparse axioms, substrate-distinct
//! patterns, high drive).

use relatum_v2::{
    runtime::{
        agent_classes, agent_outcome_distribution, agent_target_overlap,
        ActionKind, AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    test_substrates::oq2::build_oq2_stream,
    AutonomousConfig, DiscoveryConfig, NamingPolicy,
    PatternQualityClass, RSet, RecommendedIntervention,
    RecommendedPatternIntervention, RefinementConfig, SamplingMatchConfig,
    TheoryQualityClass,
};
use std::collections::HashMap;

const RNG_SEED: u64 = 0xC0FFEE;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const SUBSTRATE_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const SAMPLING_BUDGET: usize = 200;
const MIN_AXIOM_PREDICTIONS: u64 = 5;

fn section(n: usize, title: &str) {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!("  {}. {}", n, title);
    println!("════════════════════════════════════════════════════════");
}

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

fn theory_class_label(c: TheoryQualityClass) -> &'static str {
    match c {
        TheoryQualityClass::Signal => "Signal",
        TheoryQualityClass::Mixed => "Mixed",
        TheoryQualityClass::Noise => "Noise",
        TheoryQualityClass::Indeterminate => "Indet.",
    }
}

fn pattern_class_label(c: PatternQualityClass) -> &'static str {
    match c {
        PatternQualityClass::Signal => "Signal",
        PatternQualityClass::Mixed => "Mixed",
        PatternQualityClass::Redundant => "Redundant",
        PatternQualityClass::Anomalous => "Anomalous",
        PatternQualityClass::Indeterminate => "Indet.",
    }
}

fn theory_rec_label(r: &RecommendedIntervention) -> String {
    match r {
        RecommendedIntervention::None => "None".to_string(),
        RecommendedIntervention::ShadowMonitor { reason } => {
            format!("ShadowMonitor: {}", reason)
        }
        RecommendedIntervention::FamilyDemote { family_id, .. } => {
            format!("FamilyDemote({})", family_id)
        }
        RecommendedIntervention::AxiomRepair { axiom_ids } => {
            format!("AxiomRepair({:?})", axiom_ids)
        }
        RecommendedIntervention::TheoryDemote { reason } => {
            format!("TheoryDemote: {:?}", reason)
        }
        RecommendedIntervention::DemoteSuperset { cleaner_subset_theory } => {
            format!("DemoteSuperset(of {})", cleaner_subset_theory)
        }
        RecommendedIntervention::Merge { partner_theory, rationale } => {
            format!("Merge(with {}, {:?})", partner_theory, rationale)
        }
        RecommendedIntervention::Manual { reason } => {
            format!("Manual: {}", reason)
        }
    }
}

fn pattern_rec_label(r: &RecommendedPatternIntervention) -> String {
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

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" v2 capability demonstration on OQ#2");
    println!(" 2026-05-07 — full stack as of ADR 0078");
    println!("════════════════════════════════════════════════════════");

    // Build the runtime + run to maturity.
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_oq2_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(4500);

    // ── 1. Stream → axiom discovery ─────────────────────────────
    section(1, "Discover axioms + theories from the stream (autonomously)");
    println!();
    println!(" After 4500 ticks of OQ#2 (tournament + lattice + star):");
    println!("   {} axioms",   rt.rset.axioms().len());
    println!("   {} theories", rt.rset.theories().len());
    println!("   {} L2 shape families", rt.rset.axiom_shape_families().len());
    println!("   {} patterns (runtime auto-mint)", rt.rset.patterns().len());
    println!("   {} episodes total", rt.memory.episodes.len());
    println!();
    println!(" Theories:");
    for t in rt.rset.theories() {
        let axs: Vec<_> = rt.rset.theory_axioms(t).into_iter().collect();
        println!("   {} = {{{}}}", t, axs.join(", "));
    }

    // Build cross-substrate validation set (for theory + pattern quality).
    let theories: Vec<String> = rt.rset.theories()
        .iter().map(|s| s.to_string()).collect();
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = SUBSTRATE_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        if let Ok(g) = rt.rset.generate_substrate_from_theory(
            t, NUM_GEN_IDS, SEED_DENSITY, seed,
        ) {
            substrates.push(g);
        }
    }
    let axioms_set: std::collections::HashSet<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();
    let mut primary_rates: HashMap<String, f64> = HashMap::new();
    for ax in &axioms_set {
        if let Some(r) = rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS) {
            primary_rates.insert(ax.clone(), r);
        }
    }
    let mut substrates_with_axioms = substrates.clone();
    for sub in substrates_with_axioms.iter_mut() {
        for ax in &axioms_set {
            sub.register_axiom_with_intension(ax);
        }
    }

    // ── 2. Theory quality ───────────────────────────────────────
    section(2, "Assess theory quality (ADR 0071)");
    let theory_reports = rt.rset.theory_quality_report_all(
        &substrates_with_axioms, &primary_rates,
    );
    println!();
    println!(" {:<6} {:>4} {:>10} {:>10} {:>8}",
             "theory", "axs", "p_mean", "c_mean", "class");
    for r in &theory_reports {
        println!(
            " {:<6} {:>4} {:>10} {:>10} {:>8}",
            r.theory_id,
            r.axiom_count,
            r.primary_rate_mean
                .map(|v| format!("{:.3}", v)).unwrap_or_else(|| "—".to_string()),
            r.cross_precision_mean
                .map(|v| format!("{:.3}", v)).unwrap_or_else(|| "—".to_string()),
            theory_class_label(r.summary_class),
        );
    }

    // ── 3. Theory intervention ──────────────────────────────────
    section(3, "Recommend theory intervention (ADR 0072)");
    println!();
    for r in &theory_reports {
        let others: Vec<_> = theory_reports.iter()
            .filter(|o| o.theory_id != r.theory_id).cloned().collect();
        let rec = RSet::recommend_intervention(r, &others);
        println!("   {} → {}", r.theory_id, theory_rec_label(&rec));
    }

    // ── 4. Pattern emergence ────────────────────────────────────
    section(4, "Mint emergent patterns (ADR 0010 / 0018)");
    println!();
    println!(" Manually invoke autonomous_pass for sizes 2-5:");
    for size in [2usize, 3, 4, 5] {
        run_pass_on_size(&mut rt, size);
    }
    println!("   {} patterns now registered", rt.rset.patterns().len());
    println!();
    println!(" Pattern instance + participant counts:");
    println!(" {:<6} {:>5} {:>10} {:>14}",
             "id", "size", "instances", "participants");
    let mut pids: Vec<String> = rt.rset.patterns().iter().map(|s| s.to_string()).collect();
    pids.sort();
    for pid in &pids {
        let canon_size = rt.rset.pattern_structure(pid)
            .map(|c| c.len()).unwrap_or(0);
        let insts: Vec<String> = rt.rset.instances_of(pid).iter()
            .map(|s| s.to_string()).collect();
        let mut participants: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        for inst in &insts {
            for r in rt.rset.left_of(inst) {
                participants.insert(r.y.to_string());
            }
        }
        println!(" {:<6} {:>5} {:>10} {:>14}",
                 pid, canon_size, insts.len(), participants.len());
    }

    // ── 5. Pattern shape rendering ──────────────────────────────
    section(5, "Render minted patterns as readable shapes (ADR 0075 b)");
    println!();
    for pid in &pids {
        let shape = rt.rset.format_pattern_shape(pid);
        // Print first line + edge list.
        for line in shape.lines() {
            println!("   {}", line);
        }
        println!();
    }

    // ── 6. Pattern quality (with cross-substrate sampling) ──────
    section(6, "Assess pattern quality (ADR 0077 + sampling)");
    let sampling_cfg = SamplingMatchConfig {
        sample_count: SAMPLING_BUDGET,
        rng_seed: RNG_SEED,
    };
    let pattern_reports = rt.rset.pattern_quality_report_all(
        &substrates_with_axioms,
        Some(&sampling_cfg),
    );
    println!();
    println!(" {:<6} {:>5} {:>5} {:>5} {:>5} {:>6} {:>10}",
             "id", "size", "inst", "mdl", "xsub", "ovr", "class");
    for r in &pattern_reports {
        let xsub = r.cross_substrate_match_count
            .map(|n| format!("{}", n)).unwrap_or_else(|| "—".to_string());
        println!(
            " {:<6} {:>5} {:>5} {:>5} {:>5} {:>6.2} {:>10}",
            r.pattern_id,
            r.canonical_size,
            r.instance_count,
            r.mdl_gain,
            xsub,
            r.overlap_score,
            pattern_class_label(r.summary_class),
        );
    }

    // ── 7. Pattern intervention ─────────────────────────────────
    section(7, "Recommend pattern intervention (ADR 0077)");
    println!();
    for r in &pattern_reports {
        let rec = RSet::recommend_pattern_intervention(r, &pattern_reports);
        println!("   {} → {}", r.pattern_id, pattern_rec_label(&rec));
    }

    // ── 8. Micro-agent ecosystem ────────────────────────────────
    section(8, "Read the runtime as a multi-agent ecosystem (ADR 0076)");
    let classes = agent_classes(&rt.memory.episodes);
    println!();
    println!(" {} agent classes detected across {} episodes:",
             classes.len(), rt.memory.episodes.len());
    println!();
    println!(" {:<32} {:>6} {:>8} {:>14}",
             "agent class", "eps", "succ%", "first/last tick");
    for c in &classes {
        let class_str = format!("{:?}/{}", c.action_kind, c.target_label);
        let class_short = if class_str.len() > 32 {
            format!("{}…", &class_str[..31])
        } else {
            class_str
        };
        println!(
            " {:<32} {:>6} {:>7.1}% {:>5}-{:<8}",
            class_short,
            c.episode_count,
            c.success_rate * 100.0,
            c.first_tick,
            c.last_tick,
        );
    }

    // Show outcome distribution + target overlap for one notable
    // class — DiscoverPatterns.
    println!();
    println!(" Detail for DiscoverPatterns/PatternSize(2):");
    let dist = agent_outcome_distribution(
        &rt.memory.episodes,
        ActionKind::DiscoverPatterns,
        "PatternSize(2)",
    );
    println!("   episode_count: {}, neg/zer/pos: {}/{}/{}",
             dist.episode_count, dist.negative_count,
             dist.zero_count, dist.positive_count);
    println!("   delta range: [{:.3}, {:.3}], median {:.3}",
             dist.min_delta, dist.max_delta, dist.median_delta);

    let overlap = agent_target_overlap(
        &rt.memory.episodes,
        ActionKind::Declarativize,
    );
    if overlap.total_episodes > 0 {
        println!();
        println!(" Detail for Declarativize/* (target overlap):");
        println!("   {} episodes over {} distinct targets",
                 overlap.total_episodes, overlap.distinct_targets);
        for (target, count) in overlap.target_counts.iter().take(3) {
            println!("     {} × {}", count, target);
        }
    }

    // ── 9. Drive signal ─────────────────────────────────────────
    section(9, "Detect what's not yet learned (ADR 0078)");
    let drive = rt.rset.unexplained_drive_signal();
    println!();
    println!(" Total data edges: {}", drive.total_data_edges);
    println!(" Unexplained edges: {} ({:.1}%)",
             drive.unexplained_count, drive.unexplained_ratio * 100.0);
    println!(" Distinct canonical shapes of unexplained R: {}",
             drive.distinct_canonicals);
    if drive.has_signal() {
        println!();
        println!(" Top buckets (where the system should attend next):");
        println!(" {:<6} {:>10} {:>8} {:>14}",
                 "rank", "components", "edges", "canonical_size");
        for (i, b) in drive.canonical_buckets.iter().take(5).enumerate() {
            println!(
                " #{:<5} {:>10} {:>8} {:>14}",
                i + 1, b.component_count, b.edge_count, b.canonical.len(),
            );
            for r in b.example_edges.iter().take(2) {
                println!("       e.g. R({}, {})", r.x, r.y);
            }
        }
    } else {
        println!();
        println!(" Drive is silent — no unexplained R remains.");
    }

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Capability demonstration complete.");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" v2 stacks 9 distinct capabilities here, all working on");
    println!(" one substrate run. Every capability is a query / mint /");
    println!(" assessment that respects the constitution heavy reading:");
    println!(" no per-token signature classification, all object");
    println!(" emergence atomic with concept registration.");
    println!();
    println!("--- end ---");
}
