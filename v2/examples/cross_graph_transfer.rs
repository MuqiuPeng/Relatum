//! Cross-graph pattern transfer demo (ADR 0023).
//!
//! Learn patterns on graph A, export the library, apply to graph B.
//! B gets the same structural types named, with B's own identifiers.

use relatum_v2::{
    AutonomousConfig, AutonomousOutcome, DiscoveryConfig, NamingPolicy, RSet, RefinementConfig, R,
};

fn main() {
    // Graph A: the canonical mixed graph.
    let mut graph_a = build_mixed_graph_a();
    let config = AutonomousConfig {
        discovery: DiscoveryConfig {
            target_size: 3,
            sample_count: 200,
            top_m: 10,
            rng_seed: 2024,
            include_meta_in_discovery: false,
        },
        refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
        naming: NamingPolicy::default(),
        instance_sampling: None,
    };

    println!("=== Graph A — learn patterns ===");
    graph_a.autonomous_pass(&config);
    println!("patterns in A: {}", graph_a.patterns().len());
    let library = graph_a.canonical_library();
    println!("library size: {}", library.len());
    for c in &library {
        println!("  canonical {:?}", c);
    }
    println!();

    // Graph B: independent data, no patterns named yet, but shares
    // structural motifs with A (some 3-chains, a 3-cycle, a 3-star).
    let mut graph_b = build_mixed_graph_b();
    println!("=== Graph B — apply library ===");
    println!("B size before: {} edges, patterns: {}", graph_b.len(), graph_b.patterns().len());
    let outcomes = graph_b.attach_canonicals(&library, &NamingPolicy::default());
    for o in &outcomes {
        match o {
            AutonomousOutcome::NewPattern { pattern_id, instance_count, canonical } => {
                println!(
                    "  NewPattern({})  {} instances  canonical={:?}",
                    pattern_id, instance_count, canonical
                );
            }
            AutonomousOutcome::Existing { pattern_id, canonical } => {
                println!("  Existing({})  canonical={:?}", pattern_id, canonical);
            }
            AutonomousOutcome::Skipped { canonical, reason } => {
                println!("  Skipped  reason={:?}  canonical={:?}", reason, canonical);
            }
        }
    }
    println!();
    println!("B size after: {} edges, patterns: {}", graph_b.len(), graph_b.patterns().len());
    let mut patterns: Vec<&str> = graph_b.patterns();
    patterns.sort();
    for p in patterns {
        println!("  {}  instances={}", p, graph_b.instances_of(p).len());
    }
}

fn build_mixed_graph_a() -> RSet {
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

fn build_mixed_graph_b() -> RSet {
    // Independent data: two chains and a cycle (same structural motifs
    // as A, completely different identifiers).
    let mut rs = RSet::new();
    // A 4-node chain:
    rs.extend([R::new("x1", "x2"), R::new("x2", "x3"), R::new("x3", "x4")]);
    // Another 4-node chain:
    rs.extend([R::new("y1", "y2"), R::new("y2", "y3"), R::new("y3", "y4")]);
    // A 3-cycle:
    rs.extend([R::new("z1", "z2"), R::new("z2", "z3"), R::new("z3", "z1")]);
    rs
}
