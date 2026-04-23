//! Subgraph matching demo (ADR 0015).
//!
//! Replays ADR 0014's attach_only scenario with the new
//! subgraph-matching-backed attach pass. The cases that fragmentation
//! missed — asymmetric 2-chain, internal 2-chain segments of the
//! original data — now attach correctly.
//!
//! Used to produce `logs/2026-04-23_subgraph_matching.log`.

use relatum_v2::{CanonicalForm, NamingDecision, NamingPolicy, RSet, SkipReason, R};

fn main() {
    let mut rs = build_mixed_graph();

    println!("=== Phase 1 — discovery pass (default policy) ===");
    let disc = rs.run_naming_pass(&NamingPolicy::default());
    print_decisions(&disc);
    println!("patterns registered: {}", rs.patterns().len());
    for pid in sorted(rs.patterns()) {
        println!("  {}  instances = {}", pid, rs.instances_of(pid).len());
    }
    println!("RSet size: {}", rs.len());
    println!();

    println!("=== Phase 2 — attach-only pass on the ORIGINAL data ===");
    println!("(ADR 0015: subgraph matching should find 2-chain instances");
    println!(" that compound-class discovery fragmented.)");
    let attach_policy = NamingPolicy {
        min_edges: 2,
        min_instances: 1,
        skip_meta_subgraphs: true,
        attach_only: true,
    };
    let attach1 = rs.run_naming_pass(&attach_policy);
    print_decisions(&attach1);
    for pid in sorted(rs.patterns()) {
        let insts = rs.instances_of(pid);
        println!("  {}  instances = {}", pid, insts.len());
        for inst in &insts {
            let participants: Vec<&str> = {
                let mut v: Vec<&str> = rs.participants_of(inst).into_iter().collect();
                v.sort();
                v
            };
            println!("    {}  participants: {{{}}}", inst, participants.join(", "));
        }
    }
    println!("RSet size: {}", rs.len());
    println!();

    println!("=== Phase 3 — new data arrives ===");
    let new_edges = [
        R::new("m1", "m2"),
        R::new("m2", "m3"),
        R::new("m3", "m1"),
        R::new("u", "v"),
        R::new("v", "w"),
        R::new("q1", "q2"),
        R::new("q1", "q3"),
        R::new("q3", "q4"),
    ];
    for r in &new_edges {
        rs.add(r.clone());
    }
    println!("added {} edges; RSet size: {}", new_edges.len(), rs.len());
    println!();

    println!("=== Phase 4 — attach-only pass with new data ===");
    println!("(ADR 0015: fresh 3-cycle AND fresh 2-chain should both attach.)");
    let attach2 = rs.run_naming_pass(&attach_policy);
    print_decisions(&attach2);
    for pid in sorted(rs.patterns()) {
        let insts = rs.instances_of(pid);
        println!("  {}  instances = {}", pid, insts.len());
    }
    println!("RSet size: {}", rs.len());
    println!();

    println!("=== Phase 5 — second attach pass is idempotent ===");
    let before = rs.len();
    let patterns_before = rs.patterns().len();
    let total_inst_before: usize = rs
        .patterns()
        .iter()
        .map(|p| rs.instances_of(p).len())
        .sum();
    rs.run_naming_pass(&attach_policy);
    let total_inst_after: usize = rs
        .patterns()
        .iter()
        .map(|p| rs.instances_of(p).len())
        .sum();
    println!(
        "RSet size: {} -> {}  (delta {})",
        before,
        rs.len(),
        rs.len() as i64 - before as i64
    );
    println!(
        "pattern count: {} -> {}",
        patterns_before,
        rs.patterns().len()
    );
    println!(
        "instance total: {} -> {}",
        total_inst_before, total_inst_after
    );
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

fn sorted<'a>(mut v: Vec<&'a str>) -> Vec<&'a str> {
    v.sort();
    v
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
        };
        println!("  canonical {:?}  ->  {}", canon, verdict);
    }
}
