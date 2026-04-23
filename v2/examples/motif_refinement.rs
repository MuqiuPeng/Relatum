//! Motif refinement demo (ADR 0017).
//!
//! Runs `discover_motifs` then `refine_candidates` on the canonical
//! mixed graph. Shows the representative-quality improvement when a
//! clean alternative exists (e.g., the 2-chain candidate's embedded
//! representative moves to a clean instance).
//!
//! Used to produce `logs/2026-04-23_motif_refinement.log`.

use relatum_v2::{
    DiscoveryConfig, MotifCandidate, RSet, RefinementConfig, R,
};

fn main() {
    let rs = build_mixed_graph();
    println!("RSet: {} edges", rs.len());
    println!();

    run(&rs, 2);
    run(&rs, 3);
    demonstrate_explicit_refinement(&rs);
}

// Manually construct a non-clean candidate to show refinement
// visibly replacing the embedded representative with a clean one.
fn demonstrate_explicit_refinement(rs: &RSet) {
    use relatum_v2::Subgraph;
    println!("=== Explicit refinement on a non-clean 2-chain rep ===");
    let embedded = Subgraph::from_edges([R::new("k1", "k2"), R::new("k3", "k1")]);
    let canon = embedded.canonicalize();
    let seeded = vec![MotifCandidate {
        canonical: canon.clone(),
        representative: embedded.clone(),
        sample_frequency: 1,
        score: 1.0,
    }];
    println!(
        "  before:  clean={}  rep={}",
        rs.is_clean_subgraph(&seeded[0].representative),
        render_rep(&seeded[0]),
    );
    let refined = rs.refine_candidates(
        seeded,
        &RefinementConfig { max_tries: 200, rng_seed: 42 },
    );
    println!(
        "  after:   clean={}  rep={}",
        rs.is_clean_subgraph(&refined[0].representative),
        render_rep(&refined[0]),
    );
}

fn run(rs: &RSet, target_size: usize) {
    let disc_config = DiscoveryConfig {
        target_size,
        sample_count: 200,
        top_m: 5,
        rng_seed: 2024,
            include_meta_in_discovery: false,
    };
    let raw = rs.discover_motifs(&disc_config);

    let refine_config = RefinementConfig {
        max_tries: 200,
        rng_seed: 999,
    };
    let refined = rs.refine_candidates(raw.clone(), &refine_config);

    println!(
        "=== target_size={}  sample_count=200  top_m=5  seeds: disc=2024 refine=999 ===",
        target_size
    );
    for (r, refined_c) in raw.iter().zip(refined.iter()) {
        println!(
            "  canonical {:?}  freq={}",
            r.canonical, r.sample_frequency
        );
        let raw_clean = rs.is_clean_subgraph(&r.representative);
        let refined_clean = rs.is_clean_subgraph(&refined_c.representative);
        println!(
            "    raw rep      {}:  {}",
            if raw_clean { "clean  " } else { "EMBEDDED" },
            render_rep(&r),
        );
        println!(
            "    refined rep  {}:  {}{}",
            if refined_clean { "clean  " } else { "EMBEDDED" },
            render_rep(refined_c),
            if raw_clean != refined_clean { "   <-- improved" } else { "" },
        );
    }
    println!();
}

fn render_rep(c: &MotifCandidate) -> String {
    let mut edges: Vec<&R> = c.representative.edges().collect();
    edges.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
    let rendered: Vec<String> = edges
        .iter()
        .map(|r| format!("R({},{})", r.x, r.y))
        .collect();
    format!("{{{}}}", rendered.join(", "))
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
