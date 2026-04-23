//! γ naming-pass demo (ADR 0012).
//!
//! Runs `run_naming_pass` on the canonical ADR 0007 mixed graph under
//! the default policy, shows decisions, demonstrates idempotence via
//! the participant-set dedup on a second call, and shows how tuning
//! the policy changes outcomes.
//!
//! Used to produce `logs/2026-04-23_gamma_naming_pass.log`.

use relatum_v2::{CanonicalForm, NamingDecision, NamingPolicy, RSet, SkipReason, R};

fn main() {
    // ====================================================
    // Pass 1 — default policy on a fresh RSet
    // ====================================================
    let mut rs = build_mixed_graph();
    let edges_before = rs.len();
    let default_policy = NamingPolicy::default();

    println!("=== Pass 1 — default policy on fresh RSet ===");
    println!(
        "policy: min_edges={} min_instances={} skip_meta={}",
        default_policy.min_edges,
        default_policy.min_instances,
        default_policy.skip_meta_subgraphs
    );
    println!("RSet size before: {}", edges_before);
    let decisions = rs.run_naming_pass(&default_policy);
    print_decisions(&decisions);
    println!("RSet size after:  {}", rs.len());
    println!("Patterns registered: {}", rs.patterns().len());
    println!();

    // ====================================================
    // Pass 2 — same policy on the enlarged RSet (idempotence)
    // ====================================================
    println!("=== Pass 2 — same policy on the enlarged RSet ===");
    println!("(should dedup: no new patterns, no new instances)");
    let before = rs.len();
    let patterns_before = rs.patterns().len();
    let decisions2 = rs.run_naming_pass(&default_policy);
    print_decisions(&decisions2);
    println!("RSet size delta: {} -> {}", before, rs.len());
    println!(
        "Patterns delta:  {} -> {}",
        patterns_before,
        rs.patterns().len()
    );
    println!();

    // ====================================================
    // Pass 3 — min_instances=2 on a fresh RSet
    // ====================================================
    let mut rs = build_mixed_graph();
    let tighter = NamingPolicy { min_edges: 2, min_instances: 2, skip_meta_subgraphs: true, attach_only: false };
    println!("=== Pass 3 — tighter policy on fresh RSet ===");
    println!(
        "policy: min_edges={} min_instances={} skip_meta={}",
        tighter.min_edges, tighter.min_instances, tighter.skip_meta_subgraphs
    );
    let decisions3 = rs.run_naming_pass(&tighter);
    print_decisions(&decisions3);
    println!("Patterns registered: {}", rs.patterns().len());
    println!();

    // ====================================================
    // Pass 4 — permissive policy (min_edges=1) on a fresh RSet
    // ====================================================
    let mut rs = build_mixed_graph();
    let permissive = NamingPolicy { min_edges: 1, min_instances: 1, skip_meta_subgraphs: true, attach_only: false };
    println!("=== Pass 4 — permissive policy (min_edges=1) on fresh RSet ===");
    println!(
        "policy: min_edges={} min_instances={} skip_meta={}",
        permissive.min_edges, permissive.min_instances, permissive.skip_meta_subgraphs
    );
    let decisions4 = rs.run_naming_pass(&permissive);
    print_decisions(&decisions4);
    println!("Patterns registered: {}", rs.patterns().len());
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
                format!("Skipped(BelowMinEdges edges={} min={})", edges, min)
            }
            NamingDecision::Skipped(SkipReason::BelowMinInstances { instances, min }) => {
                format!(
                    "Skipped(BelowMinInstances instances={} min={})",
                    instances, min
                )
            }
            NamingDecision::Skipped(SkipReason::AlreadyKnown) => "Skipped(AlreadyKnown)".to_string(),
            NamingDecision::Skipped(SkipReason::NoMatchingPattern) => "Skipped(NoMatchingPattern)".to_string(),
        };
        println!("  canonical {:?}  ->  {}", canon, verdict);
    }
}
