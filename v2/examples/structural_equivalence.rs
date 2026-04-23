//! Structural-equivalence demo (ADR 0004).
//!
//! Runs `equivalence_classes()` on a handful of canonical graphs and prints
//! the partitions. Output is deterministic (identifiers sorted, classes
//! sorted by signature) so it can be diffed across runs.
//!
//! Used to produce `logs/2026-04-23_structural_equivalence.log`.

use relatum_v2::{IdentifierProfile, RSet, SlotPattern, R};

fn main() {
    run("forward chain a1 -> a2 -> a3 -> a4 -> a5", &[
        ("a1", "a2"),
        ("a2", "a3"),
        ("a3", "a4"),
        ("a4", "a5"),
    ]);

    run("bidirectional chain over {a1..a4}", &[
        ("a1", "a2"), ("a2", "a3"), ("a3", "a4"),
        ("a2", "a1"), ("a3", "a2"), ("a4", "a3"),
    ]);

    run("3-cycle a -> b -> c -> a", &[
        ("a", "b"),
        ("b", "c"),
        ("c", "a"),
    ]);

    run("out-star: hub -> {a, b, c}", &[
        ("hub", "a"),
        ("hub", "b"),
        ("hub", "c"),
    ]);

    run("in-star: {a, b, c} -> sink", &[
        ("a", "sink"),
        ("b", "sink"),
        ("c", "sink"),
    ]);

    run("two disjoint chains", &[
        ("p1", "p2"), ("p2", "p3"),
        ("q1", "q2"), ("q2", "q3"),
    ]);
}

fn run(label: &str, edges: &[(&str, &str)]) {
    println!("=== {} ===", label);

    let mut rs = RSet::new();
    for (x, y) in edges {
        rs.add(R::new(*x, *y));
    }

    let mut instances: Vec<&R> = rs.iter().collect();
    instances.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
    println!("R instances ({}):", instances.len());
    for r in &instances {
        println!("  R({}, {})", r.x, r.y);
    }

    let mut ids: Vec<&str> = rs.identifiers().into_iter().collect();
    ids.sort();
    println!("Profiles:");
    for id in &ids {
        let p = rs.profile(id);
        println!(
            "  {}: out={} in={} {}",
            id,
            p.degree_out,
            p.degree_in,
            slot_label(p.slots)
        );
    }

    let classes = rs.equivalence_classes();
    let mut sorted: Vec<(&IdentifierProfile, Vec<&str>)> = classes
        .iter()
        .map(|(sig, members)| {
            let mut m: Vec<&str> = members.iter().copied().collect();
            m.sort();
            (sig, m)
        })
        .collect();
    sorted.sort_by(|a, b| sig_key(a.0).cmp(&sig_key(b.0)));

    println!("Equivalence classes ({}):", sorted.len());
    for (sig, members) in &sorted {
        println!(
            "  (out={} in={} {}): {{{}}}",
            sig.degree_out,
            sig.degree_in,
            slot_label(sig.slots),
            members.join(", ")
        );
    }

    println!();
}

fn slot_label(s: SlotPattern) -> &'static str {
    match s {
        SlotPattern::None => "None",
        SlotPattern::LeftOnly => "LeftOnly",
        SlotPattern::RightOnly => "RightOnly",
        SlotPattern::Both => "Both",
    }
}

fn sig_key(p: &IdentifierProfile) -> (usize, usize, u8) {
    let s = match p.slots {
        SlotPattern::None => 0,
        SlotPattern::LeftOnly => 1,
        SlotPattern::RightOnly => 2,
        SlotPattern::Both => 3,
    };
    (p.degree_out, p.degree_in, s)
}
