//! ADR 0050 — large-scale sampling-mode benchmark.
//!
//! Re-runs the scale characterization from ADR 0041 but with
//! `instance_sampling` enabled (ADR 0043), pushing to 1000 edges
//! on 100 identifiers — a size range where exhaustive
//! `find_instances_of` would be impractical.
//!
//! Compares two modes side by side at representative sizes so the
//! trade-off (completeness vs. tractability) is visible.

use relatum_v2::{
    AxiomDiscoveryConfig, DiscoveryConfig, DriveConfig, NamingPolicy,
    RefinementConfig, RSet, SamplingMatchConfig, R,
};
use std::time::Instant;

fn main() {
    for &(edges, ids) in &[
        (100, 50),
        (200, 50),
        (500, 100),
        (1000, 100),
    ] {
        run_comparison(edges, ids);
    }
}

fn run_comparison(edge_count: usize, id_count: usize) {
    let rs = build_random_rset(edge_count, id_count, 0xA5A5_5A5A);
    println!("── {} edges / {} ids ──", rs.len(), rs.identifiers().len());

    let base = DriveConfig {
        pattern_sizes: vec![2, 3],
        discovery_config: DiscoveryConfig {
            target_size: 3,
            sample_count: 200,
            top_m: 10,
            rng_seed: 2024,
            include_meta_in_discovery: false,
        },
        refinement_config: RefinementConfig { max_tries: 100, rng_seed: 999 },
        naming_policy: NamingPolicy::default(),
        axiom_config: AxiomDiscoveryConfig::default(),
        max_steps: 6,
        epsilon: 0.0,
        enable_prune: true,
        prune_threshold: 0.0,
        instance_sampling: None,
    };

    let sampled = DriveConfig {
        instance_sampling: Some(SamplingMatchConfig {
            sample_count: 500,
            rng_seed: 777,
        }),
        ..base.clone()
    };

    // Sampling mode always viable at this scale.
    let mut rs_s = rs.clone();
    let t0 = Instant::now();
    let trace_s = rs_s.intrinsic_drive(&sampled);
    let dt_s = t0.elapsed();
    println!(
        "  sampling    : steps={} final_score={:.2}  ({:?})",
        trace_s.steps.len(),
        trace_s.final_score,
        dt_s
    );

    // Exhaustive mode only at small sizes; skip at larger ones to
    // avoid multi-minute runs.
    if edge_count <= 200 {
        let mut rs_e = rs.clone();
        let t0 = Instant::now();
        let trace_e = rs_e.intrinsic_drive(&base);
        let dt_e = t0.elapsed();
        println!(
            "  exhaustive  : steps={} final_score={:.2}  ({:?})",
            trace_e.steps.len(),
            trace_e.final_score,
            dt_e
        );
    } else {
        println!("  exhaustive  : (skipped — too slow at this scale)");
    }
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
