//! Incremental workflow demo (ADR 0022).
//!
//! Prime the RSet with `autonomous_pass`, add new data, then run
//! `autonomous_and_attach` — the attach phase picks up new instances
//! of the pre-existing pattern.

use relatum_v2::{
    AutonomousConfig, DiscoveryConfig, NamingPolicy, RSet, RefinementConfig, R,
};

fn main() {
    let mut rs = build_mixed_graph();
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
    };

    println!("=== Prime: autonomous_pass ===");
    rs.autonomous_pass(&config);
    print_patterns(&rs);
    println!();

    println!("=== Add new data: another 3-chain on new identifiers ===");
    rs.extend([
        R::new("q1", "q2"),
        R::new("q2", "q3"),
        R::new("q3", "q4"),
    ]);
    println!("RSet size: {}", rs.len());
    println!();

    println!("=== autonomous_and_attach ===");
    let summary = rs.autonomous_and_attach(&config);

    let new_names: usize = summary
        .autonomous
        .iter()
        .filter(|o| matches!(o, relatum_v2::AutonomousOutcome::NewPattern { .. }))
        .count();
    let attach_names: usize = summary
        .attach
        .iter()
        .filter(|(_, d)| matches!(d, relatum_v2::NamingDecision::Named(_)))
        .count();
    println!(
        "autonomous phase: {} new patterns",
        new_names
    );
    println!(
        "attach phase:     {} pattern(s) received new instances",
        attach_names
    );
    println!();

    print_patterns(&rs);
}

fn print_patterns(rs: &RSet) {
    let mut patterns: Vec<&str> = rs.patterns();
    patterns.sort();
    println!("patterns registered: {}", patterns.len());
    for p in patterns {
        println!("  {}  instances={}", p, rs.instances_of(p).len());
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
