//! Hierarchical discovery probe (ADR 0025).
//!
//! Compare `discover_motifs` on the post-autonomous mixed graph
//! with `include_meta_in_discovery` OFF and ON. OFF recovers the
//! ADR 0018 discovery; ON samples from data + meta-R and may
//! surface higher-order canonicals.

use relatum_v2::{
    AutonomousConfig, DiscoveryConfig, NamingPolicy, RSet, RefinementConfig, R,
};

fn main() {
    let mut rs = build_mixed_graph();
    let cfg_autonomous = AutonomousConfig {
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
    rs.autonomous_pass(&cfg_autonomous);

    println!("=== Baseline: discover_motifs (meta excluded) ===");
    let base_cfg = DiscoveryConfig {
        target_size: 3,
        sample_count: 500,
        top_m: 20,
        rng_seed: 7,
        include_meta_in_discovery: false,
    };
    let baseline = rs.discover_motifs(&base_cfg);
    print_candidates(&baseline, &rs);
    println!();

    println!("=== Probe: discover_motifs (meta included) ===");
    let probe_cfg = DiscoveryConfig {
        target_size: 3,
        sample_count: 500,
        top_m: 20,
        rng_seed: 7,
        include_meta_in_discovery: true,
    };
    let probe = rs.discover_motifs(&probe_cfg);
    print_candidates(&probe, &rs);
}

fn print_candidates(cands: &[relatum_v2::MotifCandidate], rs: &RSet) {
    let meta_ids = build_meta_ids(rs);
    println!("{} candidates:", cands.len());
    for c in cands {
        let touches_meta = c
            .representative
            .edges()
            .any(|r| meta_ids.contains(&r.x) || meta_ids.contains(&r.y));
        let marker = if touches_meta { "META" } else { "data" };
        println!(
            "  [{}]  freq={}  canonical={:?}",
            marker, c.sample_frequency, c.canonical
        );
    }
}

fn build_meta_ids(rs: &RSet) -> std::collections::HashSet<String> {
    let mut s = std::collections::HashSet::new();
    s.insert(relatum_v2::PATTERN_MARKER.to_string());
    for p in rs.patterns() {
        s.insert(p.to_string());
        for inst in rs.instances_of(p) {
            s.insert(inst.to_string());
        }
    }
    s
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
