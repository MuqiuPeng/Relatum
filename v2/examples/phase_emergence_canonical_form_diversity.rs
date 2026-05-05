//! Phase Emergence — Cross-substrate canonical-form comparison.
//!
//! ADR 0075 piece 3. The kernel audit verified that
//! `autonomous_pass` mints patterns on every canonical substrate.
//! It also flagged that comparing pattern *ids* (p_0..p_N) across
//! substrates is meaningless — those are per-RSet counters that
//! collide by accident.
//!
//! This slice replaces the id-based comparison with a
//! canonical-form comparison. For each minted pattern, take its
//! `pattern_structure` (the registered canonical form, which is
//! a stable subgraph fingerprint per ADR 0009/0029) and compare
//! across substrates. Two substrates "share" a pattern iff they
//! mint patterns with the same canonical form.
//!
//! Predictions (ranked by remaining plausibility after the
//! audit):
//! - **Strong substrate-distinctness**: every substrate produces
//!   ≥1 canonical form unique to it. Falsifies the
//!   "RSet collapse" diagnosis.
//! - **Partial overlap**: OQ#1 / long5k / narrow_a share many
//!   forms (they had isomorphic axiom-path RSets); OQ#2 produces
//!   distinct forms. Refines "RSet collapse" to: axiom path
//!   collapses, pattern path doesn't.
//! - **Total overlap**: every form appears on every substrate.
//!   "RSet collapse" generalises beyond axiom path.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment, Event,
    },
    test_substrates::{
        long5k::build_5k_stream, narrow_a::build_narrow_a_stream,
        oq1::build_long_stream, oq2::build_oq2_stream,
    },
    AutonomousConfig, AutonomousOutcome, CanonicalForm, DiscoveryConfig,
    NamingPolicy, RSet, RefinementConfig,
};
use std::collections::{BTreeMap, HashMap, HashSet};

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

/// Compress a canonical form (Vec<(u64, u64)>) into a stable short
/// hex tag for display. The full form is the actual identity; the
/// tag is just for table layout.
fn canonical_tag(canon: &CanonicalForm) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    canon.hash(&mut h);
    format!("can_{:012x}", h.finish())
}

#[derive(Debug, Clone)]
struct PatternFingerprint {
    pattern_id: String,
    canonical: CanonicalForm,
    canonical_tag: String,
    instance_count: usize,
    participant_count: usize,
    edge_count: usize,
}

#[derive(Debug, Default)]
struct SubstrateAnalysis {
    label: String,
    ticks: u64,
    fingerprints: Vec<PatternFingerprint>,
}

fn analyze(label: &str, stream: Vec<(u64, Event)>, ticks: u64) -> SubstrateAnalysis {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    // Run autonomous_pass for sizes 2-5 to mint a stable set.
    for size in [2usize, 3, 4, 5] {
        let _ = run_pass_on_size(&mut rt, size);
    }

    let mut fingerprints: Vec<PatternFingerprint> = Vec::new();
    for p in rt.rset.patterns() {
        let canon = match rt.rset.pattern_structure(p) {
            Some(c) => c,
            None => continue,
        };
        let instance_ids: Vec<String> =
            rt.rset.instances_of(p).iter().map(|s| s.to_string()).collect();
        let mut participants: HashSet<String> = HashSet::new();
        for inst in &instance_ids {
            for r in rt.rset.left_of(inst) {
                participants.insert(r.y.to_string());
            }
        }
        let edge_count = canon.len();
        fingerprints.push(PatternFingerprint {
            pattern_id: p.to_string(),
            canonical_tag: canonical_tag(&canon),
            canonical: canon,
            instance_count: instance_ids.len(),
            participant_count: participants.len(),
            edge_count,
        });
    }

    SubstrateAnalysis {
        label: label.to_string(),
        ticks,
        fingerprints,
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Cross-substrate canonical-form comparison (ADR 0075 #3)");
    println!("════════════════════════════════════════════════════════");
    println!(" Replaces the kernel audit's pattern-id comparison with");
    println!(" the canonical-form comparison: do different substrates");
    println!(" actually mint structurally-distinct emergent patterns?");

    let analyses = vec![
        analyze("OQ#1", build_long_stream(), 1000),
        analyze("long5k", build_5k_stream(), 1500),
        analyze("narrow_a", build_narrow_a_stream(), 500),
        analyze("OQ#2", build_oq2_stream(), 4500),
    ];

    // Per-substrate summary.
    println!();
    println!(" Per-substrate canonical-form inventory:");
    for a in &analyses {
        println!();
        println!("   {} ({} ticks): {} patterns minted", a.label, a.ticks, a.fingerprints.len());
        for fp in &a.fingerprints {
            println!(
                "     {} → {} (size={}, instances={}, participants={})",
                fp.pattern_id, fp.canonical_tag,
                fp.edge_count, fp.instance_count, fp.participant_count,
            );
        }
    }

    // Cross-substrate matrix: which canonical_tag appears on which substrate.
    let mut by_tag: BTreeMap<String, HashMap<String, &PatternFingerprint>> = BTreeMap::new();
    for a in &analyses {
        for fp in &a.fingerprints {
            by_tag
                .entry(fp.canonical_tag.clone())
                .or_default()
                .insert(a.label.clone(), fp);
        }
    }

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Canonical-form × substrate matrix");
    println!("════════════════════════════════════════════════════════");
    print!(" {:<20} {:>5}", "canonical", "size");
    for a in &analyses {
        print!(" {:>10}", a.label);
    }
    println!();
    for (tag, by_substrate) in &by_tag {
        let any_fp = by_substrate.values().next().copied().unwrap();
        print!(" {:<20} {:>5}", tag, any_fp.edge_count);
        for a in &analyses {
            match by_substrate.get(&a.label) {
                Some(fp) => print!(" {:>10}", fp.instance_count),
                None => print!(" {:>10}", "—"),
            }
        }
        println!();
    }

    // Verdict.
    let total_distinct = by_tag.len();
    let universal: Vec<&String> = by_tag
        .iter()
        .filter(|(_, m)| m.len() == analyses.len())
        .map(|(t, _)| t)
        .collect();
    let mut substrate_specific: HashMap<String, usize> = HashMap::new();
    for a in &analyses {
        substrate_specific.insert(a.label.clone(), 0);
    }
    for (_, m) in &by_tag {
        if m.len() == 1 {
            let label = m.keys().next().unwrap().clone();
            *substrate_specific.get_mut(&label).unwrap() += 1;
        }
    }

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Diversity verdict");
    println!("════════════════════════════════════════════════════════");
    println!(" Total distinct canonical forms across substrates: {}", total_distinct);
    println!(" Universal (mint on every substrate): {}", universal.len());
    for tag in &universal {
        println!("   {}", tag);
    }
    println!(" Substrate-specific (mint on exactly one substrate):");
    for (label, count) in &substrate_specific {
        println!("   {}: {}", label, count);
    }

    let any_substrate_specific: usize = substrate_specific.values().sum();
    println!();
    if total_distinct > 0 && any_substrate_specific == 0 && universal.len() == total_distinct {
        println!(" → TOTAL OVERLAP — every minted canonical form appears on");
        println!("   every substrate. The pattern-path output is also");
        println!("   substrate-isomorphic, generalising the axiom-path");
        println!("   collapse to all of v2's emergent abstractions.");
    } else if any_substrate_specific > 0 {
        println!(" → SUBSTRATE-DISTINCT — at least {} canonical forms appear", any_substrate_specific);
        println!("   on exactly one substrate. The pattern path produces");
        println!("   structural diversity that the axiom path collapses,");
        println!("   confirming the kernel-audit reversal: emergent");
        println!("   patterns ARE substrate-specific even when axioms");
        println!("   are not.");
    } else {
        println!(" → MIXED — overlap exists but no canonical is universal");
        println!("   across all 4 substrates. Each substrate contributes");
        println!("   some shared, some specific structural patterns.");
    }

    // Pairwise overlap report — useful for understanding which
    // substrates cluster.
    println!();
    println!(" Pairwise overlap (canonical-form Jaccard):");
    let labels: Vec<String> = analyses.iter().map(|a| a.label.clone()).collect();
    print!(" {:<14}", "");
    for l in &labels {
        print!(" {:>10}", l);
    }
    println!();
    for a in &analyses {
        print!(" {:<14}", a.label);
        let a_set: HashSet<&str> = a.fingerprints.iter()
            .map(|f| f.canonical_tag.as_str()).collect();
        for b in &analyses {
            let b_set: HashSet<&str> = b.fingerprints.iter()
                .map(|f| f.canonical_tag.as_str()).collect();
            let inter = a_set.intersection(&b_set).count();
            let union = a_set.union(&b_set).count();
            let j = if union == 0 { 0.0 } else { inter as f64 / union as f64 };
            print!(" {:>10.2}", j);
        }
        println!();
    }

    println!();
    println!("--- end ---");
}
