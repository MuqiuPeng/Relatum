//! Phase B.8.1 — Lift L5 ceiling via a new L3 discovery kind.
//!
//! B.8 found L5 = 0 because only 1 L4 super-meta exists on OQ#1.
//! Current L3 = "nested families sharing an individual premise edge".
//! B.8.1 proposes a parallel L3 kind: **"L2 families that share a
//! member axiom"** — different relation, additional groupings.
//!
//! Implemented inline (no lib code change) to keep the slice
//! self-contained. If the new L3 kind produces ≥ 2 L4 super-metas
//! (under analogous L4 logic), L5 mints.
//!
//! Method:
//!   1. Run rt → L1 axioms, L2 families
//!   2. Compute existing L3 (premise-edge-shared, via lib API)
//!   3. Compute NEW L3 (member-axiom-shared, inline)
//!   4. Compute L4 over union of L3 sets
//!   5. Audit: L5 prerequisites met?

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    RSet,
};
use std::collections::{BTreeMap, HashSet};

const TICKS_PHASE_0: u64 = 1000;

fn main() {
    println!("=== Phase B.8.1 — New L3 kind to lift L5 ceiling ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    rt.rset.discover_axiom_shape_families(2);
    let l2: Vec<String> = rt.rset.axiom_shape_families().into_iter().map(str::to_owned).collect();
    println!();
    println!("L2 families: {}", l2.len());

    // Existing L3: premise-edge shared
    rt.rset.discover_nested_shape_families(2);
    let l3_existing: Vec<String> = rt.rset.nested_shape_families()
        .into_iter().map(str::to_owned).collect();
    println!("L3 (existing kind, premise-edge-shared): {}", l3_existing.len());
    for n in &l3_existing {
        println!("  {} → members: {:?}", n, rt.rset.nested_shape_family_members(n));
    }

    // NEW L3 kind: family pairs sharing a member axiom
    // ADR 0070 Step 2: lifted from inline code to lib API.
    println!();
    println!("=== ADR 0070 Step 2: discover_nested_shape_families_by_member_overlap ===");

    let new_l3_ids = rt.rset.discover_nested_shape_families_by_member_overlap(2);
    let mut new_l3: Vec<(String, HashSet<String>)> = Vec::new();
    for id in &new_l3_ids {
        let members: HashSet<String> = rt
            .rset
            .nested_shape_family_members(id)
            .into_iter()
            .map(str::to_owned)
            .collect();
        new_l3.push((id.clone(), members));
    }
    new_l3.sort_by(|a, b| a.0.cmp(&b.0));

    println!("L3 (new kind, count): {}", new_l3.len());
    for (id, members) in &new_l3 {
        println!("  {} → families: {:?}", id, members);
    }

    // Combined L3 set (existing + new). For L4 prerequisite, count L2 families
    // that appear in ≥ 2 L3s (across BOTH kinds).
    let mut all_l3: Vec<(String, HashSet<String>)> = Vec::new();
    for n in &l3_existing {
        let members: HashSet<String> = rt.rset.nested_shape_family_members(n)
            .into_iter().map(str::to_owned).collect();
        all_l3.push((n.clone(), members));
    }
    for (id, members) in &new_l3 {
        all_l3.push((id.clone(), members.clone()));
    }
    println!();
    println!("Combined L3 set: {}", all_l3.len());

    // Compute L4 candidates: L2 families appearing in ≥ 2 L3s
    let mut l2_to_l3s: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (l3_id, members) in &all_l3 {
        for m in members {
            l2_to_l3s.entry(m.clone()).or_default().push(l3_id.clone());
        }
    }
    let mut l4_candidates: Vec<(String, Vec<String>)> = Vec::new();
    for (l2_id, l3_list) in &l2_to_l3s {
        if l3_list.len() >= 2 {
            l4_candidates.push((l2_id.clone(), l3_list.clone()));
        }
    }
    println!();
    println!("=== L4 candidates (L2 families in ≥ 2 L3s) ===");
    for (l2_id, l3_list) in &l4_candidates {
        println!("  {} appears in L3s: {:?}", l2_id, l3_list);
    }
    println!("  total L4 candidates: {}", l4_candidates.len());

    // L5 prerequisite: L3 nested families that appear in ≥ 2 L4 super-metas
    let mut l3_to_l4s: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (l4_via, l3_members) in &l4_candidates {
        for l3_id in l3_members {
            l3_to_l4s.entry(l3_id.clone()).or_default().push(format!("super_via_{}", l4_via));
        }
    }
    let mut l5_eligible: Vec<(String, Vec<String>)> = Vec::new();
    for (l3_id, l4_list) in &l3_to_l4s {
        if l4_list.len() >= 2 {
            l5_eligible.push((l3_id.clone(), l4_list.clone()));
        }
    }

    println!();
    println!("=== L5 prerequisite check ===");
    for (l3_id, l4_list) in &l5_eligible {
        println!("  {} appears in L4s: {:?}", l3_id, l4_list);
    }
    println!("  L5-eligible L3s: {}", l5_eligible.len());

    // Verdict
    println!();
    println!("=== Verdict ===");
    if l5_eligible.is_empty() {
        println!("  PARTIAL — adding the new L3 kind produces {} L4 candidates",
                 l4_candidates.len());
        println!("  but still 0 L5 (no L3 appears in ≥ 2 L4s).");
        println!();
        println!("  Diagnosis: L4 super-metas naturally diverge by their distinguishing L2.");
        println!("  Two L4s sharing an L3 means: one L2 family is in two distinct L3 groupings,");
        println!("  AND those L3 groupings overlap on yet another L2. Highly structured requirement.");
    } else {
        println!("  POSITIVE — new L3 kind lifts the ceiling: {} L5 candidates", l5_eligible.len());
    }
    println!();
    println!("--- end ---");
}
