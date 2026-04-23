//! Motif discovery demo (ADR 0016).
//!
//! Sample-score-select on the canonical mixed graph at two sizes.
//! Shows that propose-score-select surfaces the dominant structural
//! motifs without enumerating every possibility.
//!
//! Used to produce `logs/2026-04-23_motif_discovery.log`.

use relatum_v2::{DiscoveryConfig, RSet, R};

fn main() {
    let rs = build_mixed_graph();
    println!("RSet: {} edges", rs.len());
    println!();

    run_with_size(&rs, 2, 200, 5, 2024);
    run_with_size(&rs, 3, 200, 5, 2024);
}

fn run_with_size(rs: &RSet, target_size: usize, sample_count: usize, top_m: usize, seed: u64) {
    let config = DiscoveryConfig {
        target_size,
        sample_count,
        top_m,
        rng_seed: seed,
        include_meta_in_discovery: false,
    };
    println!(
        "=== discover_motifs  target_size={}  sample_count={}  top_m={}  seed={} ===",
        target_size, sample_count, top_m, seed
    );
    let candidates = rs.discover_motifs(&config);
    println!("{} candidate(s) returned:", candidates.len());
    for (i, c) in candidates.iter().enumerate() {
        println!(
            "  #{}  freq={}  canonical={:?}",
            i + 1,
            c.sample_frequency,
            c.canonical
        );
        let mut edges: Vec<&R> = c.representative.edges().collect();
        edges.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
        let rendered: Vec<String> = edges
            .iter()
            .map(|r| format!("R({},{})", r.x, r.y))
            .collect();
        println!("          rep: {{{}}}", rendered.join(", "));
    }
    println!();
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
