//! Gradient-refine probe (ADR 0026).
//!
//! Compares gradient refinement with the existing random-resample
//! refinement on the canonical hard case: a 2-chain canonical whose
//! representative is embedded in a 3-cycle. Gradient descent is
//! expected to either (a) escape to a clean representative or (b)
//! stay stuck — the run makes that concrete.

use relatum_v2::{
    GradientRefineConfig, MotifCandidate, RSet, RefinementConfig, Subgraph, R,
};

fn main() {
    let rs = build_mixed_graph();

    let embedded = Subgraph::from_edges([R::new("k1", "k2"), R::new("k3", "k1")]);
    let canon = embedded.canonicalize();
    let input = MotifCandidate {
        canonical: canon.clone(),
        representative: embedded.clone(),
        sample_frequency: 1,
        score: 1.0,
    };

    println!("=== Input (embedded 2-chain inside 3-cycle) ===");
    println!("  canonical: {:?}", input.canonical);
    println!("  representative: {:?}  clean={}", edges_of(&input.representative), rs.is_clean_subgraph(&input.representative));
    println!();

    // Existing random refine.
    println!("=== ADR 0017 random refine ===");
    let random_cfg = RefinementConfig { max_tries: 200, rng_seed: 42 };
    let random_out = rs.refine_candidates(vec![input.clone()], &random_cfg);
    let random_rep = &random_out[0].representative;
    println!("  representative: {:?}  clean={}", edges_of(random_rep), rs.is_clean_subgraph(random_rep));
    println!();

    // Gradient refine variations.
    let configs = [
        ("default (steps=300, lr=0.5, α=1.0)", GradientRefineConfig::default()),
        ("heavier cleanness weight (α=5.0)", GradientRefineConfig {
            steps: 300, learning_rate: 0.5, cleanness_weight: 5.0, init_scale: 3.0,
        }),
        ("more steps (1000)", GradientRefineConfig {
            steps: 1000, learning_rate: 0.5, cleanness_weight: 1.0, init_scale: 3.0,
        }),
        ("low init_scale (0.5, softer start)", GradientRefineConfig {
            steps: 500, learning_rate: 0.5, cleanness_weight: 2.0, init_scale: 0.5,
        }),
    ];

    for (label, cfg) in configs {
        let out = rs.gradient_refine_candidate(&input, &cfg);
        println!("=== ADR 0026 gradient refine — {} ===", label);
        println!(
            "  representative: {:?}  clean={}  canonical_match={}",
            edges_of(&out.representative),
            rs.is_clean_subgraph(&out.representative),
            out.canonical == canon,
        );
        println!();
    }
}

fn edges_of(sg: &Subgraph) -> Vec<String> {
    let mut v: Vec<&R> = sg.edges().collect();
    v.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
    v.into_iter().map(|r| format!("R({},{})", r.x, r.y)).collect()
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
