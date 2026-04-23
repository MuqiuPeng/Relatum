//! MDL scoring demo (ADR 0019).
//!
//! Shows sample-frequency ranking vs MDL-gain ranking, and how
//! `min_mdl_gain` in NamingPolicy filters out singleton / low-reuse
//! candidates in `autonomous_pass`.
//!
//! Used to produce `logs/2026-04-23_mdl_scoring.log`.

use relatum_v2::{
    AutonomousConfig, AutonomousOutcome, AutonomousSkip, DiscoveryConfig,
    NamingPolicy, RSet, RefinementConfig, R, SkipReason,
};

fn main() {
    let rs = build_mixed_graph();
    println!("RSet: {} edges", rs.len());
    println!();

    show_scoring_comparison(&rs, 2);
    show_scoring_comparison(&rs, 3);
    show_autonomous_with_and_without_mdl_filter();
}

fn show_scoring_comparison(rs: &RSet, target_size: usize) {
    let config = DiscoveryConfig {
        target_size,
        sample_count: 200,
        top_m: 10,
        rng_seed: 2024,
    };
    let raw = rs.discover_motifs(&config);
    let mdl = rs.score_by_mdl(raw.clone());

    println!("=== target_size={}  comparing rankings ===", target_size);
    println!("{:<45}  sample_freq   mdl_gain", "canonical");
    for (a, b) in raw.iter().zip(mdl.iter()) {
        assert_eq!(a.canonical, b.canonical);
        println!(
            "  {:<43}  {:>10}   {:>8}",
            format!("{:?}", a.canonical),
            a.sample_frequency,
            b.score as usize,
        );
    }
    println!();
}

fn show_autonomous_with_and_without_mdl_filter() {
    let base_config = |min_mdl_gain| AutonomousConfig {
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
        naming: NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
            attach_only: false,
            min_mdl_gain,
        },
    };

    println!("=== autonomous_pass, target_size=3, min_mdl_gain=0 (default) ===");
    let mut rs = build_mixed_graph();
    let outcomes = rs.autonomous_pass(&base_config(0));
    summarize(&outcomes);
    println!("patterns registered: {}", rs.patterns().len());
    println!();

    println!("=== autonomous_pass, target_size=3, min_mdl_gain=1 (MDL filter) ===");
    let mut rs = build_mixed_graph();
    let outcomes = rs.autonomous_pass(&base_config(1));
    summarize(&outcomes);
    println!("patterns registered: {}", rs.patterns().len());
    println!();
}

fn summarize(outcomes: &[AutonomousOutcome]) {
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
                let r = match reason {
                    AutonomousSkip::NoCleanInstance => "NoCleanInstance".to_string(),
                    AutonomousSkip::PolicyFiltered(SkipReason::BelowMinEdges { edges, min }) => {
                        format!("Policy(BelowMinEdges {}<{})", edges, min)
                    }
                    AutonomousSkip::PolicyFiltered(SkipReason::BelowMinInstances {
                        instances,
                        min,
                    }) => {
                        format!("Policy(BelowMinInstances {}<{})", instances, min)
                    }
                    AutonomousSkip::PolicyFiltered(SkipReason::AlreadyKnown) => {
                        "Policy(AlreadyKnown)".to_string()
                    }
                    AutonomousSkip::PolicyFiltered(SkipReason::BelowMdlGain { gain, min }) => {
                        format!("Policy(BelowMdlGain {}<{})", gain, min)
                    }
                };
                println!("  Skipped  {}  canonical={:?}", r, canonical);
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
