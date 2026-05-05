//! Phase Emergence — Render minted patterns as readable shapes.
//!
//! ADR 0075 piece (b). Builds on the kernel audit + canonical-form
//! diversity slices. Each substrate's autonomous_pass mints
//! patterns whose canonical forms have been compared as opaque
//! hashes; this slice renders them as readable text shapes via
//! `format_pattern_shape`.
//!
//! Special focus: the 5 OQ#2-only canonical forms identified by
//! the diversity slice. What do they actually look like?

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment, Event,
    },
    test_substrates::{
        long5k::build_5k_stream, narrow_a::build_narrow_a_stream,
        oq1::build_long_stream, oq2::build_oq2_stream,
    },
    AutonomousConfig, CanonicalForm, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig,
};
use std::collections::{BTreeMap, HashMap, HashSet};

const RNG_SEED: u64 = 0xC0FFEE;

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

fn canonical_tag(canon: &CanonicalForm) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut h = DefaultHasher::new();
    canon.hash(&mut h);
    format!("can_{:012x}", h.finish())
}

struct SubstratePatterns {
    label: String,
    /// pattern_id → (canonical_tag, instance_count, shape_text)
    patterns: Vec<(String, String, usize, String)>,
}

fn collect(label: &str, stream: Vec<(u64, Event)>, ticks: u64) -> SubstratePatterns {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);
    for size in [2usize, 3, 4, 5] {
        run_pass_on_size(&mut rt, size);
    }
    let mut patterns: Vec<(String, String, usize, String)> = Vec::new();
    for p in rt.rset.patterns() {
        let canon = match rt.rset.pattern_structure(p) {
            Some(c) => c,
            None => continue,
        };
        let tag = canonical_tag(&canon);
        let insts = rt.rset.instances_of(p).len();
        let shape = rt.rset.format_pattern_shape(p);
        patterns.push((p.to_string(), tag, insts, shape));
    }
    SubstratePatterns {
        label: label.to_string(),
        patterns,
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Phase Emergence — pattern shape visualization");
    println!("════════════════════════════════════════════════════════");
    println!(" Renders the 12 canonical forms minted across the 4");
    println!(" canonical substrates as readable text shapes. Builds on");
    println!(" the canonical-form diversity slice; replaces opaque");
    println!(" hashes with concrete subgraph shapes.");

    let analyses = vec![
        collect("OQ#1", build_long_stream(), 1000),
        collect("long5k", build_5k_stream(), 1500),
        collect("narrow_a", build_narrow_a_stream(), 500),
        collect("OQ#2", build_oq2_stream(), 4500),
    ];

    // Index by canonical tag for cross-substrate lookup.
    let mut by_tag: BTreeMap<String, (Vec<String>, String, usize)> = BTreeMap::new();
    // tag → (substrates_seen, canonical_shape_text, max_instance_count)
    for a in &analyses {
        for (_pid, tag, insts, shape) in &a.patterns {
            let entry = by_tag.entry(tag.clone()).or_insert((Vec::new(), shape.clone(), 0));
            entry.0.push(a.label.clone());
            if *insts > entry.2 {
                entry.2 = *insts;
            }
        }
    }

    // Per-substrate full shape rendering.
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Per-substrate pattern shapes");
    println!("════════════════════════════════════════════════════════");
    for a in &analyses {
        println!();
        println!(" === {} ===", a.label);
        for (pid, tag, insts, shape) in &a.patterns {
            println!();
            println!(" {} [{}, {} instances]", pid, tag, insts);
            // Indent the shape text.
            for line in shape.lines() {
                println!("   {}", line);
            }
        }
    }

    // Cross-substrate canonical inventory grouped by where it appears.
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Canonical inventory by substrate-membership");
    println!("════════════════════════════════════════════════════════");
    let mut universal: Vec<&String> = Vec::new();
    let mut oq1_clade_only: Vec<&String> = Vec::new();
    let mut oq2_only: Vec<&String> = Vec::new();
    let mut other: Vec<&String> = Vec::new();
    for (tag, (subs, _shape, _max)) in &by_tag {
        let s_set: HashSet<&str> = subs.iter().map(String::as_str).collect();
        let has_oq2 = s_set.contains("OQ#2");
        let oq1_clade: HashSet<&str> = ["OQ#1", "long5k", "narrow_a"].into_iter().collect();
        let in_clade = s_set.iter().any(|s| oq1_clade.contains(s));
        if s_set.len() == analyses.len() {
            universal.push(tag);
        } else if has_oq2 && !in_clade {
            oq2_only.push(tag);
        } else if !has_oq2 && in_clade {
            oq1_clade_only.push(tag);
        } else {
            other.push(tag);
        }
    }

    let print_group = |title: &str, group: &[&String]| {
        println!();
        println!(" {} ({} canonical{}):",
                 title,
                 group.len(),
                 if group.len() == 1 { "" } else { "s" });
        for tag in group {
            let (subs, shape, max_insts) = &by_tag[*tag];
            let first_line = shape.lines().next().unwrap_or("(empty)");
            println!(
                "   {} [{}; max instances={}]",
                tag, subs.join(", "), max_insts,
            );
            // Strip the redundant "p_X" prefix from format_pattern_shape's
            // first line. Actually keep it — we want the shape descriptor.
            // Print remaining lines for the structural breakdown.
            println!("     {}", first_line);
            for line in shape.lines().skip(1) {
                println!("     {}", line);
            }
        }
    };
    print_group("Universal (mint on every substrate)", &universal);
    print_group("OQ#1-clade only (OQ#1 / long5k / narrow_a)", &oq1_clade_only);
    print_group("OQ#2-only", &oq2_only);
    if !other.is_empty() {
        print_group("Mixed-membership (other)", &other);
    }

    // Summary.
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Summary");
    println!("════════════════════════════════════════════════════════");
    println!(" {} universal canonical(s)", universal.len());
    println!(" {} OQ#1-clade-only canonical(s)", oq1_clade_only.len());
    println!(" {} OQ#2-only canonical(s)", oq2_only.len());
    println!(" {} mixed-membership canonical(s)", other.len());
    println!(" {} total distinct canonical forms", by_tag.len());

    // ── Suppress 'unused' on the variable; compute total insts.
    let total: usize = by_tag.values().map(|(_, _, m)| *m).sum::<usize>();
    let _ = HashMap::<String, usize>::from([("placeholder".to_string(), total)]);

    println!();
    println!("--- end ---");
}
