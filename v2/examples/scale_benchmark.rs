//! ADR 0041 — scale benchmark for intrinsic_drive + axiom discovery.
//!
//! Builds random RSets at sizes 50 / 100 / 200 / 400 edges over
//! a bounded identifier universe, runs `intrinsic_drive` and
//! `discover_axioms_minimal`, and measures:
//!
//! - wall time per operation
//! - steps taken by the drive
//! - final abstraction_score
//! - axiom count
//!
//! All RSets are built deterministically from an inline xorshift PRNG
//! seeded by a constant — results are reproducible across runs.
//!
//! Used to produce `logs/2026-04-24_scale_benchmark.log`.

use relatum_v2::{
    AxiomDiscoveryConfig, DiscoveryConfig, DriveConfig, NamingPolicy,
    RefinementConfig, RSet, R,
};
use std::time::Instant;

fn main() {
    // Scales are deliberately modest. An initial run revealed that
    // intrinsic_drive's pattern-discovery subpath, which runs
    // `find_instances_of` exhaustively, grows near-quadratically
    // in the edge count. See log for concrete numbers.
    for &edge_count in &[50, 100, 200, 400] {
        run_one(edge_count, 20);
    }
}

fn run_one(edge_count: usize, id_count: usize) {
    let rs = build_random_rset(edge_count, id_count, 0xA5A5_5A5A);
    println!("── scale: {} edges, {} identifiers ──", rs.len(), rs.identifiers().len());

    // 1. axiom discovery (strict, minimal)
    let cfg_ax = AxiomDiscoveryConfig::default();
    let t0 = Instant::now();
    let raw = rs.discover_axioms(&cfg_ax);
    let t1 = Instant::now();
    let minimal = rs.discover_axioms_minimal(&cfg_ax);
    let t2 = Instant::now();
    println!("  discover_axioms:          raw={:>3}  ({:?})", raw.len(), t1 - t0);
    println!("  discover_axioms_minimal:  min={:>3}  ({:?})", minimal.len(), t2 - t1);

    // 2. full intrinsic drive
    let drive_cfg = DriveConfig {
        pattern_sizes: vec![2, 3],
        discovery_config: DiscoveryConfig {
            target_size: 3,
            sample_count: 100,
            top_m: 10,
            rng_seed: 2024,
            include_meta_in_discovery: false,
        },
        refinement_config: RefinementConfig { max_tries: 100, rng_seed: 999 },
        naming_policy: NamingPolicy::default(),
        axiom_config: AxiomDiscoveryConfig::default(),
        max_steps: 10,
        epsilon: 0.0,
        enable_prune: true,
        prune_threshold: 0.0,
        instance_sampling: None,
    };
    let mut rs_drive = rs.clone();
    let t0 = Instant::now();
    let trace = rs_drive.intrinsic_drive(&drive_cfg);
    let dt_drive = t0.elapsed();
    println!(
        "  intrinsic_drive:          steps={}  final_score={:.2}  ({:?})",
        trace.steps.len(),
        trace.final_score,
        dt_drive
    );

    // 3. to_text / from_text roundtrip timing
    let t0 = Instant::now();
    let text = rs_drive.to_text().unwrap();
    let text_bytes = text.len();
    let t1 = Instant::now();
    let _rs_back = RSet::from_text(&text).unwrap();
    let t2 = Instant::now();
    println!(
        "  to_text:                  bytes={}  ({:?})",
        text_bytes,
        t1 - t0
    );
    println!("  from_text:                                  ({:?})", t2 - t1);
    println!();
}

fn build_random_rset(edge_count: usize, id_count: usize, seed: u64) -> RSet {
    let mut rs = RSet::new();
    let mut state = seed.max(1);
    while rs.len() < edge_count {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let a = (state as usize) % id_count;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let b = (state as usize) % id_count;
        rs.add(R::new(format!("n{}", a), format!("n{}", b)));
    }
    rs
}
