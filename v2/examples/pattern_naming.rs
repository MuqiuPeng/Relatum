//! Pattern naming demo (ADR 0010).
//!
//! Runs the full 0007 → 0008 → 0009 pipeline on the ADR 0007 mixed
//! graph, groups subgraph instances by canonical form (as in ADR 0009),
//! and names each group by calling `name_pattern_instances`. Prints the
//! registry of named patterns and the resulting RSet stats.
//!
//! Used to produce `logs/2026-04-23_pattern_naming.log`.

use relatum_v2::{CanonicalForm, RSet, Subgraph, PATTERN_MARKER, R};
use std::collections::BTreeMap;

fn main() {
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

    let edges_before = rs.len();

    // 0007 -> 0008: compound classes split into subgraph instances
    let class_subs = rs.compound_class_subgraphs();

    // 0009: group subgraphs by canonical form
    let mut by_canon: BTreeMap<CanonicalForm, Vec<Subgraph>> = BTreeMap::new();
    for (_fp, subs) in class_subs {
        for sub in subs {
            by_canon.entry(sub.canonicalize()).or_default().push(sub);
        }
    }

    println!("=== Before naming ===");
    println!("R instances: {}", edges_before);
    println!("Canonical-form groups (ADR 0009 pattern classes): {}", by_canon.len());
    for (canon, subs) in &by_canon {
        println!("  {:?}  ({} instance(s))", canon, subs.len());
    }
    println!();

    // 0010: name each canonical group
    println!("=== Naming ===");
    let mut pattern_ids: Vec<(CanonicalForm, String, usize)> = Vec::new();
    for (canon, subs) in by_canon {
        let n = subs.len();
        let pid = rs
            .name_pattern_instances(&subs)
            .expect("valid non-empty, isomorphic instances");
        println!("pattern {}  ({} instance(s))  canonical {:?}", pid, n, canon);
        pattern_ids.push((canon, pid, n));
    }
    println!();

    let edges_after = rs.len();
    println!("=== After naming ===");
    println!(
        "R instances: {} (added {} meta-R entries)",
        edges_after,
        edges_after - edges_before
    );
    println!();

    println!("=== Pattern registry ===");
    let mut patterns: Vec<&str> = rs.patterns();
    patterns.sort();
    for pattern in patterns {
        let mut insts: Vec<&str> = rs.instances_of(pattern);
        insts.sort();
        println!("  {}  ({} instance(s))", pattern, insts.len());
        for inst in insts {
            let mut parts: Vec<&str> = rs.participants_of(inst).into_iter().collect();
            parts.sort();
            println!("    {}  participants: {{{}}}", inst, parts.join(", "));
        }
    }
    println!();

    println!("=== Marker registry view ===");
    let mut marker_edges: Vec<&R> = rs
        .iter()
        .filter(|r| r.x == PATTERN_MARKER)
        .collect();
    marker_edges.sort_by(|a, b| a.y.cmp(&b.y));
    for r in marker_edges {
        println!("  R({}, {})", r.x, r.y);
    }
}
