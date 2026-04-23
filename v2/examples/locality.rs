//! Locality-profile demo (ADR 0006).
//!
//! Shows `locality_profile` on the same six canonical graphs, with an
//! emphasis on the cycle-vs-star separation (the stated motivation) and
//! on the known chain-middle / cycle-edge 1-hop collision.
//!
//! Used to produce `logs/2026-04-23_locality.log`.

use relatum_v2::{LocalityProfile, RSet, R};

fn main() {
    run("forward chain a1 -> a2 -> a3 -> a4 -> a5", &[
        ("a1", "a2"), ("a2", "a3"), ("a3", "a4"), ("a4", "a5"),
    ]);

    run("bidirectional chain over {a1..a4}", &[
        ("a1", "a2"), ("a2", "a3"), ("a3", "a4"),
        ("a2", "a1"), ("a3", "a2"), ("a4", "a3"),
    ]);

    run("3-cycle a -> b -> c -> a", &[
        ("a", "b"), ("b", "c"), ("c", "a"),
    ]);

    run("out-star: hub -> {a, b, c}", &[
        ("hub", "a"), ("hub", "b"), ("hub", "c"),
    ]);

    run("in-star: {a, b, c} -> sink", &[
        ("a", "sink"), ("b", "sink"), ("c", "sink"),
    ]);

    run("two disjoint chains", &[
        ("p1", "p2"), ("p2", "p3"),
        ("q1", "q2"), ("q2", "q3"),
    ]);

    println!("--- Collision check: chain-middle vs cycle-edge ---");
    {
        let mut chain = RSet::new();
        chain.extend([R::new("a", "b"), R::new("b", "c"), R::new("c", "d")]);
        let chain_mid = chain.locality_profile(&R::new("b", "c"));

        let mut cycle = RSet::new();
        cycle.extend([R::new("x", "y"), R::new("y", "z"), R::new("z", "x")]);
        let cycle_edge = cycle.locality_profile(&R::new("x", "y"));

        println!("  chain-middle R(b,c):  {}", fmt(&chain_mid));
        println!("  cycle-edge  R(x,y):   {}", fmt(&cycle_edge));
        println!(
            "  equal? {}  (1-hop collision, deferred per ADR 0006)",
            chain_mid == cycle_edge
        );
    }
}

fn run(label: &str, edges: &[(&str, &str)]) {
    println!("=== {} ===", label);
    let mut rs = RSet::new();
    for (x, y) in edges {
        rs.add(R::new(*x, *y));
    }

    let mut sorted_edges: Vec<&R> = rs.iter().collect();
    sorted_edges.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
    for r in sorted_edges {
        let p = rs.locality_profile(r);
        println!("  R({}, {}):  {}", r.x, r.y, fmt(&p));
    }
    println!();
}

fn fmt(p: &LocalityProfile) -> String {
    format!(
        "co_left={} co_right={} forward={} reverse={}",
        p.co_left, p.co_right, p.forward, p.reverse
    )
}
