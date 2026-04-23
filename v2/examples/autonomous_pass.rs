//! Autonomous abstraction demo (ADR 0018).
//!
//! One invocation of `autonomous_pass` on the canonical mixed graph.
//! No user-supplied canonical or instance list — the system samples,
//! refines, and names patterns on its own. A second pass demonstrates
//! idempotence (every candidate resolves to `Existing`, nothing new
//! created).
//!
//! Used to produce `logs/2026-04-23_autonomous_pass.log`.

use relatum_v2::{
    AutonomousConfig, AutonomousOutcome, AutonomousSkip, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig, R, SkipReason,
};

fn main() {
    let mut rs = build_mixed_graph();
    let edges_before = rs.len();
    let config = default_config();

    println!("=== Before autonomous_pass ===");
    println!("RSet size: {} edges", edges_before);
    println!("patterns registered: {}", rs.patterns().len());
    println!();

    println!("=== Pass 1 — autonomous discovery on fresh RSet ===");
    let first = rs.autonomous_pass(&config);
    summarize(&first);
    println!();
    println!("RSet size: {} edges (delta {})", rs.len(), rs.len() as i64 - edges_before as i64);
    println!("patterns registered: {}", rs.patterns().len());
    for pid in sorted_patterns(&rs) {
        println!("  {}  {} instance(s)", pid, rs.instances_of(pid).len());
    }
    println!();

    println!("=== Pass 2 — idempotence ===");
    let before = rs.len();
    let patterns_before = rs.patterns().len();
    let second = rs.autonomous_pass(&config);
    summarize(&second);
    println!();
    println!(
        "RSet size: {} → {}  (delta {})",
        before,
        rs.len(),
        rs.len() as i64 - before as i64
    );
    println!(
        "patterns: {} → {}",
        patterns_before,
        rs.patterns().len()
    );
}

fn default_config() -> AutonomousConfig {
    AutonomousConfig {
        discovery: DiscoveryConfig {
            target_size: 3,
            sample_count: 200,
            top_m: 10,
            rng_seed: 2024,
        },
        refinement: RefinementConfig {
            max_tries: 200,
            rng_seed: 999,
        },
        naming: NamingPolicy::default(),
    }
}

fn sorted_patterns(rs: &RSet) -> Vec<&str> {
    let mut v: Vec<&str> = rs.patterns();
    v.sort();
    v
}

fn summarize(outcomes: &[AutonomousOutcome]) {
    println!("{} outcomes:", outcomes.len());
    for o in outcomes {
        match o {
            AutonomousOutcome::NewPattern { pattern_id, instance_count, canonical } => {
                println!(
                    "  NewPattern({})  {} instances  canonical={:?}",
                    pattern_id, instance_count, canonical
                );
            }
            AutonomousOutcome::Existing { pattern_id, canonical } => {
                println!(
                    "  Existing({})    canonical={:?}",
                    pattern_id, canonical
                );
            }
            AutonomousOutcome::Skipped { canonical, reason } => {
                let r = match reason {
                    AutonomousSkip::NoCleanInstance => "NoCleanInstance".to_string(),
                    AutonomousSkip::PolicyFiltered(SkipReason::BelowMinEdges { edges, min }) => {
                        format!("Policy(BelowMinEdges {}<{})", edges, min)
                    }
                    AutonomousSkip::PolicyFiltered(SkipReason::BelowMinInstances { instances, min }) => {
                        format!("Policy(BelowMinInstances {}<{})", instances, min)
                    }
                    AutonomousSkip::PolicyFiltered(SkipReason::AlreadyKnown) => {
                        "Policy(AlreadyKnown)".to_string()
                    }
                };
                println!("  Skipped        {}  canonical={:?}", r, canonical);
            }
        }
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
