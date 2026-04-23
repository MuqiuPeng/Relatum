//! Subgraph extraction demo (ADR 0008).
//!
//! Runs `compound_class_subgraphs` on the same mixed graph used by the
//! ADR 0007 probe, so the subgraph split can be compared against the
//! compound-class result directly.
//!
//! Used to produce `logs/2026-04-23_subgraph_extraction.log`.

use relatum_v2::{EdgeFingerprint, IdentifierProfile, RSet, SlotPattern, Subgraph, R};

fn main() {
    let mut rs = RSet::new();

    // chain c1 -> c2 -> c3 -> c4 -> c5
    rs.extend([
        R::new("c1", "c2"), R::new("c2", "c3"),
        R::new("c3", "c4"), R::new("c4", "c5"),
    ]);
    // 3-cycle k1 -> k2 -> k3 -> k1
    rs.extend([
        R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1"),
    ]);
    // out-star s -> {sa, sb, sc}
    rs.extend([
        R::new("s", "sa"), R::new("s", "sb"), R::new("s", "sc"),
    ]);
    // small tree t1 -> t2, t1 -> t3, t2 -> t4
    rs.extend([
        R::new("t1", "t2"), R::new("t1", "t3"), R::new("t2", "t4"),
    ]);
    // isolated edge
    rs.add(R::new("ie1", "ie2"));

    let classes = rs.compound_class_subgraphs();

    let mut sorted: Vec<(EdgeFingerprint, Vec<Subgraph>)> = classes.into_iter().collect();
    sorted.sort_by_key(|(fp, _)| fingerprint_key(fp));

    println!("Total edges:            {}", rs.len());
    println!("Total compound classes: {}", sorted.len());
    println!(
        "Total subgraph instances: {}",
        sorted.iter().map(|(_, subs)| subs.len()).sum::<usize>()
    );
    println!();

    for (fp, mut subs) in sorted {
        let ((x, y), loc) = &fp;
        let total_members: usize = subs.iter().map(|s| s.len()).sum();
        println!("compound class  members={}  subgraphs={}", total_members, subs.len());
        println!("  endpoint x: {}", sig_str(x));
        println!("  endpoint y: {}", sig_str(y));
        println!(
            "  locality   co_left={} co_right={} forward={} reverse={}",
            loc.co_left, loc.co_right, loc.forward, loc.reverse
        );

        // Sort subgraphs deterministically: by size desc, then by the
        // lexicographically-smallest edge they contain.
        subs.sort_by(|a, b| {
            let a_min = min_edge_key(a);
            let b_min = min_edge_key(b);
            b.len().cmp(&a.len()).then(a_min.cmp(&b_min))
        });

        for (i, sub) in subs.iter().enumerate() {
            println!("  subgraph #{}  ({} edges)", i + 1, sub.len());
            let mut edges: Vec<&R> = sub.edges().collect();
            edges.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
            for r in edges {
                println!("    R({}, {})", r.x, r.y);
            }
        }
        println!();
    }
}

fn sig_str(p: &IdentifierProfile) -> String {
    format!(
        "out={} in={} {}",
        p.degree_out,
        p.degree_in,
        match p.slots {
            SlotPattern::None => "None",
            SlotPattern::LeftOnly => "LeftOnly",
            SlotPattern::RightOnly => "RightOnly",
            SlotPattern::Both => "Both",
        }
    )
}

fn fingerprint_key(
    fp: &EdgeFingerprint,
) -> (usize, usize, u8, usize, usize, u8, usize, usize, usize, usize) {
    let ((x, y), loc) = fp;
    (
        x.degree_out, x.degree_in, slot_ord(x.slots),
        y.degree_out, y.degree_in, slot_ord(y.slots),
        loc.co_left, loc.co_right, loc.forward, loc.reverse,
    )
}

fn slot_ord(s: SlotPattern) -> u8 {
    match s {
        SlotPattern::None => 0,
        SlotPattern::LeftOnly => 1,
        SlotPattern::RightOnly => 2,
        SlotPattern::Both => 3,
    }
}

fn min_edge_key(s: &Subgraph) -> (String, String) {
    s.edges()
        .map(|r| (r.x.clone(), r.y.clone()))
        .min()
        .unwrap_or_default()
}
