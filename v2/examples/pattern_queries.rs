//! Pattern query demo (ADR 0013).
//!
//! Runs γ default pass on the ADR 0007 mixed graph and then exercises
//! the four new query methods: classify_subgraph, pattern_of,
//! memberships_of, instance_subgraph. Includes a "fresh subgraph
//! classified against the registry" case to show how meta-R pays rent.
//!
//! Used to produce `logs/2026-04-23_pattern_queries.log`.

use relatum_v2::{NamingPolicy, RSet, Subgraph, R};

fn main() {
    let mut rs = build_mixed_graph();
    rs.run_naming_pass(&NamingPolicy::default());

    println!("=== Registered patterns ===");
    let mut patterns: Vec<&str> = rs.patterns();
    patterns.sort();
    for p in &patterns {
        let mut insts: Vec<&str> = rs.instances_of(p);
        insts.sort();
        println!("  {} — {} instance(s)", p, insts.len());
        for inst in insts {
            let sg = rs.instance_subgraph(inst);
            let mut edges: Vec<&R> = sg.edges().collect();
            edges.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
            let edge_strs: Vec<String> = edges.iter().map(|r| format!("R({},{})", r.x, r.y)).collect();
            println!("    {}  sg: {{{}}}  canon: {:?}", inst, edge_strs.join(", "), sg.canonicalize());
        }
    }
    println!();

    println!("=== classify_subgraph ===");
    let cases = [
        (
            "exact 3-cycle reuse",
            Subgraph::from_edges([
                R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
            ]),
        ),
        (
            "fresh 3-cycle with new identifiers",
            Subgraph::from_edges([
                R::new("m1", "m2"), R::new("m2", "m3"), R::new("m3", "m1"),
            ]),
        ),
        (
            "fresh 2-chain with new identifiers",
            Subgraph::from_edges([
                R::new("u", "v"), R::new("v", "w"),
            ]),
        ),
        (
            "novel 2-spoke out-star (no match)",
            Subgraph::from_edges([
                R::new("h", "a"), R::new("h", "b"),
            ]),
        ),
        (
            "single edge (not named by default γ)",
            Subgraph::from_edges([R::new("x", "y")]),
        ),
    ];
    for (label, sg) in cases {
        match rs.classify_subgraph(&sg) {
            Some(pid) => println!("  {:45}  ->  {}", label, pid),
            None => println!("  {:45}  ->  (unmatched)", label),
        }
    }
    println!();

    println!("=== pattern_of ===");
    for pid in patterns.iter().copied() {
        for inst in rs.instances_of(pid) {
            let owner = rs.pattern_of(inst).unwrap_or("(none)");
            println!("  {:16}  ->  {}", inst, owner);
        }
    }
    let bogus = ["k1", "not_an_instance", "__pattern__"];
    for q in bogus {
        let owner = rs.pattern_of(q).unwrap_or("(none)");
        println!("  {:16}  ->  {}", q, owner);
    }
    println!();

    println!("=== memberships_of (each identifier, which patterns it participates in) ===");
    let mut ids: Vec<&str> = rs.identifiers().into_iter().collect();
    ids.sort();
    for id in ids {
        let memberships = rs.memberships_of(id);
        if memberships.is_empty() {
            continue;
        }
        let rendered: Vec<String> = memberships
            .iter()
            .map(|(p, i)| format!("{}@{}", p, i))
            .collect();
        println!("  {:16} -> [{}]", id, rendered.join(", "));
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
