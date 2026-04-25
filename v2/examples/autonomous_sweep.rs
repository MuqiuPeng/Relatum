//! Autonomous sweep demo (ADR 0021).
//!
//! Sweep `autonomous_pass` over target sizes `[2, 3, 4]` on the
//! mixed graph. Later sizes see earlier sizes' named patterns in the
//! registry, so patterns accumulate without duplication.

use relatum_v2::{
    AutonomousConfig, AutonomousOutcome, DiscoveryConfig, NamingPolicy, RSet, RefinementConfig, R,
};

fn main() {
    let mut rs = build_mixed_graph();
    let base = AutonomousConfig {
        discovery: DiscoveryConfig {
            target_size: 0,  // overridden per size
            sample_count: 200,
            top_m: 10,
            rng_seed: 2024,
            include_meta_in_discovery: false,
        },
        refinement: RefinementConfig { max_tries: 200, rng_seed: 999 },
        naming: NamingPolicy::default(),
        instance_sampling: None,
    };

    println!("RSet before sweep: {} edges", rs.len());
    println!();

    let results = rs.autonomous_sweep(&base, &[2, 3, 4]);

    for (size, outcomes) in &results {
        println!("=== size={} — {} outcomes ===", size, outcomes.len());
        for o in outcomes {
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
    }

    println!("patterns registered after sweep: {}", rs.patterns().len());
    let mut patterns: Vec<&str> = rs.patterns();
    patterns.sort();
    for p in patterns {
        println!("  {}  instances={}", p, rs.instances_of(p).len());
    }
    println!("RSet size: {}", rs.len());
    println!();

    println!("=== Second sweep on same sizes (idempotence) ===");
    let size_before = rs.len();
    let patterns_before = rs.patterns().len();
    let second = rs.autonomous_sweep(&base, &[2, 3, 4]);
    let any_new: usize = second
        .iter()
        .flat_map(|(_, outs)| outs.iter())
        .filter(|o| matches!(o, AutonomousOutcome::NewPattern { .. }))
        .count();
    println!(
        "RSet size: {} → {}  (delta {})",
        size_before,
        rs.len(),
        rs.len() as i64 - size_before as i64
    );
    println!("patterns: {} → {}", patterns_before, rs.patterns().len());
    println!("new patterns in second sweep: {}", any_new);
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
