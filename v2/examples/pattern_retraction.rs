//! Pattern retraction demo (ADR 0020).
//!
//! Create patterns via autonomous_pass, then retract one and verify
//! the registry state.
//!
//! Used to produce `logs/2026-04-23_pattern_retraction.log`.

use relatum_v2::{
    AutonomousConfig, DiscoveryConfig, NamingPolicy, RSet, RefinementConfig, R,
};

fn main() {
    let mut rs = build_mixed_graph();
    let data_edges_at_start = rs.len();
    println!("RSet before anything: {} edges", data_edges_at_start);
    println!();

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

    println!("=== Step 1 — autonomous_pass names patterns ===");
    rs.autonomous_pass(&config);
    println!("patterns: {}", rs.patterns().len());
    for p in sorted(&rs) {
        println!("  {}  instances={}  participants={:?}", p, rs.instances_of(p).len(), {
            let mut parts: Vec<&str> = rs
                .instances_of(p)
                .iter()
                .flat_map(|i| rs.participants_of(i).into_iter().collect::<Vec<_>>())
                .collect();
            parts.sort();
            parts.dedup();
            parts
        });
    }
    let size_after_naming = rs.len();
    println!("RSet size: {}", size_after_naming);
    println!();

    println!("=== Step 2 — retract p_1 (3-cycle) ===");
    let summary = rs.retract_pattern("p_1").unwrap();
    println!(
        "  pattern_id={}  instances_removed={}  meta_edges_removed={}",
        summary.pattern_id, summary.instances_removed, summary.meta_edges_removed
    );
    println!("patterns remaining: {}", rs.patterns().len());
    for p in sorted(&rs) {
        println!("  {}", p);
    }
    let size_after_retract = rs.len();
    println!("RSet size: {}  (delta {})", size_after_retract,
             size_after_retract as i64 - size_after_naming as i64);
    println!();

    println!("=== Step 3 — data edges intact? ===");
    let data_sample = [
        R::new("c1", "c2"), R::new("k1", "k2"), R::new("s", "sa"),
        R::new("t1", "t2"), R::new("ie1", "ie2"),
    ];
    let still_there = data_sample.iter().filter(|r| rs.contains(r)).count();
    println!(
        "  {} / {} sampled data edges still present",
        still_there, data_sample.len()
    );
    println!();

    println!("=== Step 4 — trying to retract something unknown ===");
    let err = rs.retract_pattern("p_999").unwrap_err();
    println!("  retract_pattern(\"p_999\") → Err({:?})", err);
    println!();

    println!("=== Step 5 — rediscover after retraction ===");
    let before = rs.patterns().len();
    rs.autonomous_pass(&config);
    let after = rs.patterns().len();
    println!("patterns: {} → {}", before, after);
    for p in sorted(&rs) {
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

fn sorted<'a>(rs: &'a RSet) -> Vec<&'a str> {
    let mut v: Vec<&'a str> = rs.patterns();
    v.sort();
    v
}
