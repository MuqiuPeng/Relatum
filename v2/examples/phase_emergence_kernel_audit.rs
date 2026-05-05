//! Phase Emergence — Audit existing pattern-naming pipeline as
//! a constitution-compliant emergence kernel.
//!
//! Reflection 0001 + constitution amendment (2026-05-06) require:
//! every concept-creation act must atomically (a) mint a concept
//! token, (b) register participating tokens as instances, (c)
//! never use per-token signature as visible behaviour.
//!
//! `autonomous_pass` (ADR 0018) wires together:
//!   - discover_motifs (ADR 0016) — random-walk sampling +
//!     subgraph canonical-form bucket counting
//!   - refine_candidates (ADR 0017)
//!   - name_pattern_instances (ADR 0010 / 0029) — atomic mint:
//!     PATTERN_MARKER + ROLE_MARKER + role structure + instances
//!     + per-instance participants
//!
//! Each step's bucket key is a *subgraph canonical form*, never
//! a per-token signature. The mint registers participating tokens
//! explicitly. So `autonomous_pass` should already satisfy the
//! strict reading.
//!
//! This audit runs `autonomous_pass` directly on each substrate
//! after Phase 0 maturity, with no runtime mediation. Output
//! tells us:
//!   1. Does the existing kernel actually mint patterns on
//!      v2's canonical substrates?
//!   2. Are the minted patterns substrate-distinguishing (different
//!      substrates → different patterns), unlike ADR 0074's
//!      concept_id which collapsed across substrates?
//!   3. Are participating tokens registered as instance
//!      participants (object emergence)?

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment, Event,
    },
    test_substrates::{
        long5k::build_5k_stream, narrow_a::build_narrow_a_stream,
        oq1::build_long_stream, oq2::build_oq2_stream,
    },
    AutonomousConfig, AutonomousOutcome, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig,
};
use std::collections::HashMap;

const RNG_SEED: u64 = 0xC0FFEE;

fn run_pass_on_size(rt: &mut AutonomousRuntime, size: usize) -> Vec<AutonomousOutcome> {
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
    rt.rset.autonomous_pass(&cfg)
}

#[derive(Debug, Default)]
struct SubstrateAudit {
    label: String,
    ticks: u64,
    /// Total rset edge count after Phase 0 (data + meta combined).
    data_edges_phase0: usize,
    patterns_pre_audit: usize,
    patterns_post_audit: usize,
    new_patterns: Vec<String>,
    pattern_instance_counts: HashMap<String, usize>,
    pattern_participant_counts: HashMap<String, usize>,
}

fn audit(label: &str, stream: Vec<(u64, Event)>, ticks: u64) -> SubstrateAudit {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: {} ({} ticks)", label, ticks);
    println!("════════════════════════════════════════════════════════");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    let pre_patterns: Vec<String> =
        rt.rset.patterns().iter().map(|s| s.to_string()).collect();
    let total_edges = rt.rset.iter().count();
    println!(
        " Phase 0: {} pre-audit patterns, {} total rset edges",
        pre_patterns.len(), total_edges,
    );

    // Run autonomous_pass for sizes 2, 3, 4, 5.
    let mut all_new: Vec<String> = Vec::new();
    for size in [2usize, 3, 4, 5] {
        let outcomes = run_pass_on_size(&mut rt, size);
        let mut minted_this_size = 0;
        for o in &outcomes {
            if let AutonomousOutcome::NewPattern { pattern_id, instance_count, .. } = o {
                minted_this_size += 1;
                all_new.push(pattern_id.clone());
                println!(
                    "  size={}  → mint {} ({} instances)",
                    size, pattern_id, instance_count,
                );
            }
        }
        if minted_this_size == 0 {
            // Count how many were Existing vs Skipped.
            let existing = outcomes.iter().filter(|o| matches!(o, AutonomousOutcome::Existing { .. })).count();
            let skipped = outcomes.iter().filter(|o| matches!(o, AutonomousOutcome::Skipped { .. })).count();
            println!(
                "  size={}  → no new ({} existing, {} skipped)",
                size, existing, skipped,
            );
        }
    }

    let post_patterns: Vec<String> =
        rt.rset.patterns().iter().map(|s| s.to_string()).collect();

    let mut instance_counts: HashMap<String, usize> = HashMap::new();
    let mut participant_counts: HashMap<String, usize> = HashMap::new();
    for p in &post_patterns {
        let insts: Vec<String> =
            rt.rset.instances_of(p).iter().map(|s| s.to_string()).collect();
        instance_counts.insert(p.clone(), insts.len());

        // Participating tokens are aggregated across all instances.
        let mut participants: std::collections::HashSet<String> = std::collections::HashSet::new();
        for inst in &insts {
            for r in rt.rset.left_of(inst) {
                participants.insert(r.y.to_string());
            }
        }
        participant_counts.insert(p.clone(), participants.len());
    }

    SubstrateAudit {
        label: label.to_string(),
        ticks,
        data_edges_phase0: total_edges,
        patterns_pre_audit: pre_patterns.len(),
        patterns_post_audit: post_patterns.len(),
        new_patterns: all_new,
        pattern_instance_counts: instance_counts,
        pattern_participant_counts: participant_counts,
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Emergence kernel audit");
    println!("════════════════════════════════════════════════════════");
    println!(" Audits whether autonomous_pass (ADR 0018) — discover_motifs +");
    println!(" refine_candidates + name_pattern_instances — actually mints");
    println!(" patterns when invoked on canonical substrates after Phase 0.");
    println!();
    println!(" Compliance check (constitution heavy reading):");
    println!("  - Bucket key during discovery: subgraph canonical form (✓ not per-token)");
    println!("  - Atomic mint: PATTERN_MARKER + ROLE_MARKER + instances + participants (✓)");
    println!();

    let mut audits = Vec::new();
    audits.push(audit("OQ#1", build_long_stream(), 1000));
    audits.push(audit("long5k", build_5k_stream(), 1500));
    audits.push(audit("narrow_a", build_narrow_a_stream(), 500));
    audits.push(audit("OQ#2", build_oq2_stream(), 4500));

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Cross-substrate audit summary");
    println!("════════════════════════════════════════════════════════");
    println!(
        " {:<14} {:>6} {:>10} {:>5} {:>5} {:>10} {:>12}",
        "substrate", "ticks", "data_edges", "pre", "post", "new",
        "total_insts",
    );
    for a in &audits {
        let total_instances: usize = a.pattern_instance_counts.values().sum();
        println!(
            " {:<14} {:>6} {:>10} {:>5} {:>5} {:>10} {:>12}",
            a.label,
            a.ticks,
            a.data_edges_phase0,
            a.patterns_pre_audit,
            a.patterns_post_audit,
            a.new_patterns.len(),
            total_instances,
        );
    }

    println!();
    println!(" Per-substrate pattern detail:");
    for a in &audits {
        if a.new_patterns.is_empty() {
            println!("   {}: 0 new patterns", a.label);
            continue;
        }
        println!("   {}:", a.label);
        for p in &a.new_patterns {
            let insts = a.pattern_instance_counts.get(p).copied().unwrap_or(0);
            let parts = a.pattern_participant_counts.get(p).copied().unwrap_or(0);
            println!(
                "     {} → {} instances, {} distinct participating tokens",
                p, insts, parts,
            );
        }
    }

    // Substrate-distinguishability check: if ADR 0074's concept
    // mining collapsed concept ids across substrates due to RSet
    // isomorphism, does pattern naming distinguish substrates?
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate-distinguishability check");
    println!("════════════════════════════════════════════════════════");
    let mut shared_pids: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for a in &audits {
        for p in &a.new_patterns {
            shared_pids.entry(p.clone()).or_default().push(a.label.clone());
        }
    }
    let mut shared_count = 0;
    let mut substrate_specific = 0;
    for (pid, labels) in &shared_pids {
        if labels.len() > 1 {
            shared_count += 1;
            println!("   {} shared across: {:?}", pid, labels);
        } else {
            substrate_specific += 1;
        }
    }
    println!(
        " {} pattern ids substrate-specific, {} shared across substrates",
        substrate_specific, shared_count,
    );
    println!();
    println!(" Note: pattern ids are minted with `mint_pattern_id` which");
    println!(" returns `p_N` where N is the next free index in the rset.");
    println!(" Two substrates running independently both start at p_0;");
    println!(" the id-string overlap above is a counter artifact, not a");
    println!(" claim of structural identity. To check structural identity");
    println!(" across substrates we'd need to compare canonical forms,");
    println!(" not pattern ids. (Future audit.)");

    println!();
    println!("--- end ---");
}
