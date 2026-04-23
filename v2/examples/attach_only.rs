//! Attach-only mode demo (ADR 0014).
//!
//! Two-phase workflow on the canonical mixed graph:
//!   1. Discovery pass — default policy names 3 patterns.
//!   2. New data arrives (another 3-cycle, a 2-chain, and a novel
//!      "T-fork" subgraph the registry has never seen).
//!   3. Attach-only pass — patterns are frozen; only instances can
//!      be added. The T-fork is reported as NoMatchingPattern.
//!
//! Used to produce `logs/2026-04-23_attach_only.log`.

use relatum_v2::{CanonicalForm, NamingDecision, NamingPolicy, RSet, SkipReason, R};

fn main() {
    let mut rs = build_mixed_graph();

    println!("=== Phase 1 — discovery pass (default policy) ===");
    let disc_decisions = rs.run_naming_pass(&NamingPolicy::default());
    print_decisions(&disc_decisions);
    println!("patterns registered: {}", rs.patterns().len());
    println!("RSet size: {}", rs.len());
    println!();

    println!("=== Phase 2 — new data arrives ===");
    let new_edges = [
        // Fresh 3-cycle on new identifiers
        R::new("m1", "m2"),
        R::new("m2", "m3"),
        R::new("m3", "m1"),
        // Fresh 2-chain
        R::new("u", "v"),
        R::new("v", "w"),
        // Novel "T-fork": one node has two outgoing edges of different
        // downstream structure — not matching any registered pattern.
        R::new("q1", "q2"),
        R::new("q1", "q3"),
        R::new("q3", "q4"),
    ];
    for r in &new_edges {
        rs.add(r.clone());
        println!("  added R({}, {})", r.x, r.y);
    }
    println!("RSet size: {}", rs.len());
    println!();

    println!("=== Phase 3 — attach-only pass ===");
    let attach_policy = NamingPolicy {
        min_edges: 2,
        min_instances: 1,
        skip_meta_subgraphs: true,
        attach_only: true,
    };
    let attach_decisions = rs.run_naming_pass(&attach_policy);
    print_decisions(&attach_decisions);
    println!();

    println!("patterns registered: {}  (must equal phase-1 count)", rs.patterns().len());
    let total_instances: usize = rs
        .patterns()
        .iter()
        .map(|p| rs.instances_of(p).len())
        .sum();
    println!("total named instances: {}", total_instances);

    // Drill into the pattern that absorbed the new cycle/chain
    println!();
    println!("=== Per-pattern instance counts ===");
    let mut patterns: Vec<&str> = rs.patterns();
    patterns.sort();
    for p in patterns {
        println!("  {}  instances = {}", p, rs.instances_of(p).len());
    }
}

fn build_mixed_graph() -> RSet {
    let mut rs = RSet::new();
    rs.extend([
        R::new("c1", "c2"), R::new("c2", "c3"),
        R::new("c3", "c4"), R::new("c4", "c5"),
    ]);
    rs.extend([
        R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
    ]);
    rs.extend([
        R::new("s", "sa"), R::new("s", "sb"), R::new("s", "sc"),
    ]);
    rs.extend([
        R::new("t1", "t2"), R::new("t1", "t3"), R::new("t2", "t4"),
    ]);
    rs.add(R::new("ie1", "ie2"));
    rs
}

fn print_decisions(decisions: &[(CanonicalForm, NamingDecision)]) {
    for (canon, decision) in decisions {
        let verdict = match decision {
            NamingDecision::Named(pid) => format!("Named({})", pid),
            NamingDecision::Skipped(SkipReason::BelowMinEdges { edges, min }) => {
                format!("Skipped(BelowMinEdges {}<{})", edges, min)
            }
            NamingDecision::Skipped(SkipReason::BelowMinInstances { instances, min }) => {
                format!("Skipped(BelowMinInstances {}<{})", instances, min)
            }
            NamingDecision::Skipped(SkipReason::AlreadyKnown) => "Skipped(AlreadyKnown)".to_string(),
            NamingDecision::Skipped(SkipReason::NoMatchingPattern) => {
                "Skipped(NoMatchingPattern)".to_string()
            }
        };
        println!("  canonical {:?}  ->  {}", canon, verdict);
    }
}
