//! Phase B.8 — Layer-5 super-super-meta audit.
//!
//! B.7 (L4) mints 1 super-meta family on OQ#1 (`super_shape_premise_p0-1_p1-2`,
//! containing both nested families `meta_premise_p0-1` and
//! `meta_premise_p1-2`).
//!
//! For Layer 5 to mint, we'd need ≥ 2 L4 super-meta families sharing
//! an L3 nested family. With only 1 super-meta on OQ#1, L5 = 0.
//!
//! This audit:
//!   1. Tabulates instance counts at each layer (L0 → L4)
//!   2. Documents the "structural ceiling" on OQ#1
//!   3. Tests on long5k (different stream, same regime types per C.2)
//!   4. Specifies what substrate property would lift the ceiling
//!
//! Verdict: structural-limit finding (analog to C.2.1's verdict on
//! OQ#2). The current discovery kinds saturate at L4 on this family
//! of substrates; lifting requires more discovery axes.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, Event, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::{long5k::build_5k_stream, oq1::build_long_stream as oq1_stream},
    RSet,
};

fn audit(name: &str, ticks: u64, stream: Vec<(u64, Event)>) {
    println!();
    println!("====== Audit: {} ({} ticks) ======", name, ticks);
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    let l1_count = rt.rset.axioms().len();

    rt.rset.discover_axiom_shape_families(2);
    let l2_count = rt.rset.axiom_shape_families().len();

    rt.rset.discover_nested_shape_families(2);
    let l3_count = rt.rset.nested_shape_families().len();

    rt.rset.discover_super_meta_shape_families(2);
    let l4_count = rt.rset.super_meta_shape_families().len();

    println!("  L1 axioms:                {}", l1_count);
    println!("  L2 shape families:        {}", l2_count);
    println!("  L3 nested families:       {}", l3_count);
    println!("  L4 super-meta-families:   {}", l4_count);

    // Audit L4 contents to see what L5 would need
    let l4_ids: Vec<String> = rt.rset.super_meta_shape_families()
        .into_iter().map(str::to_owned).collect();
    if l4_ids.is_empty() {
        println!("  (no L4 to inspect)");
    } else {
        println!();
        println!("  --- L4 contents ---");
        for sid in &l4_ids {
            let members: Vec<String> = rt.rset.super_meta_shape_family_members(sid)
                .into_iter().map(str::to_owned).collect();
            println!("    {} → {:?}", sid, members);
        }
    }

    // L5 would group L4 super-metas by shared L3 member.
    // Compute: for each L3 nested family, count how many L4s contain it.
    let l3_ids: Vec<String> = rt.rset.nested_shape_families()
        .into_iter().map(str::to_owned).collect();
    println!();
    println!("  --- L5 prerequisite scan (per-L3, # of L4 containers) ---");
    let mut l5_eligible: Vec<(String, usize)> = Vec::new();
    for nid in &l3_ids {
        let mut count = 0;
        for sid in &l4_ids {
            let members: Vec<String> = rt.rset.super_meta_shape_family_members(sid)
                .into_iter().map(str::to_owned).collect();
            if members.contains(nid) { count += 1; }
        }
        println!("    {} appears in {} L4 super-meta(s)", nid, count);
        if count >= 2 { l5_eligible.push((nid.clone(), count)); }
    }
    println!();
    println!("  L5-eligible L3 nested families (appear in ≥ 2 L4s): {}", l5_eligible.len());
    if l5_eligible.is_empty() {
        println!("  → L5 would mint 0 families on this substrate");
    } else {
        println!("  → L5 would mint {} families:", l5_eligible.len());
        for (nid, count) in &l5_eligible {
            println!("    super_super_{} (containing {} L4s)", nid, count);
        }
    }
}

fn main() {
    println!("=== Phase B.8 — Layer-5 super-super-meta audit ===");

    audit("OQ#1", 1000, oq1_stream());
    audit("long5k", 1500, build_5k_stream());

    println!();
    println!("=== Verdict ===");
    println!();
    println!("  STRUCTURAL-LIMIT — the current discovery kinds saturate at L4");
    println!("  on substrates of OQ#1's family. L5 = 0 because only 1 L4 super-meta");
    println!("  exists per substrate; you need ≥ 2 L4s sharing an L3 to mint L5.");
    println!();
    println!("  To lift the ceiling, one of the following is required:");
    println!("    (a) more diverse premise / conclusion shapes → more L2 → more L3 → more L4");
    println!("    (b) additional L2/L3 discovery kinds (e.g., shared-variable-arity,");
    println!("        shared-symmetry, shared-conclusion-orientation)");
    println!("    (c) multi-substrate aggregation: combine multiple substrate runs and");
    println!("        discover families across the union (not yet implemented)");
    println!();
    println!("  This finding parallels C.2.1's verdict (OQ#2 yielded 0 axioms because");
    println!("  the substrate violated transitivity — structural bound, not bug).");
    println!();
    println!("  L4 is the deepest layer the current vocabulary supports. Further");
    println!("  abstraction needs new discovery kinds, not bigger substrates.");
    println!();
    println!("--- end ---");
}
