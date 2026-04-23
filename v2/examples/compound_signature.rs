//! Compound-signature probe (ADR 0007).
//!
//! A single mixed graph: disjoint union of a 5-chain, a 3-cycle, an
//! out-star of 3 spokes, a small tree (3 edges), and one isolated edge.
//! Computes `edge_fingerprint` for every edge and partitions by value.
//!
//! Used to produce `logs/2026-04-23_compound_signature.log`.

use relatum_v2::{EdgeFingerprint, IdentifierProfile, RSet, SlotPattern, R};
use std::collections::HashMap;

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

    // tree: t1 -> t2, t1 -> t3, t2 -> t4
    rs.extend([
        R::new("t1", "t2"), R::new("t1", "t3"), R::new("t2", "t4"),
    ]);

    // isolated edge
    rs.add(R::new("ie1", "ie2"));

    // Partition edges by compound fingerprint.
    let mut classes: HashMap<EdgeFingerprint, Vec<&R>> = HashMap::new();
    for r in rs.iter() {
        classes.entry(rs.edge_fingerprint(r)).or_default().push(r);
    }

    let mut sorted: Vec<(&EdgeFingerprint, Vec<&R>)> = classes
        .into_iter()
        .map(|(k, mut v)| {
            v.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
            (k, v)
        })
        .map(|(k, v)| {
            // re-borrow: the HashMap moves but we now own Vec<&R>
            (Box::leak(Box::new(k)) as &EdgeFingerprint, v)
        })
        .collect();
    sorted.sort_by_key(|(k, _)| fingerprint_key(k));

    println!("Total edges:           {}", rs.len());
    println!("Total compound classes: {}", sorted.len());
    println!();

    for (fp, members) in &sorted {
        let ((x, y), loc) = fp;
        println!("class |{}|:", members.len());
        println!("  endpoint x: {}", sig_str(x));
        println!("  endpoint y: {}", sig_str(y));
        println!(
            "  locality   co_left={} co_right={} forward={} reverse={}",
            loc.co_left, loc.co_right, loc.forward, loc.reverse
        );
        println!("  members:");
        for r in members {
            println!("    R({}, {})", r.x, r.y);
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
