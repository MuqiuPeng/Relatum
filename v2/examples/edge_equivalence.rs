//! Edge-level equivalence demo (ADR 0005).
//!
//! Runs `r_equivalence_classes()` on the same canonical graphs as the
//! identifier-level demo so results can be compared side by side.
//! Deterministic output (sorted) for logging.
//!
//! Used to produce `logs/2026-04-23_edge_equivalence.log`.

use relatum_v2::{IdentifierProfile, RSet, SlotPattern, R, RSignature};

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

    let classes = rs.r_equivalence_classes();
    let mut sorted: Vec<(&RSignature, Vec<&R>)> = classes
        .iter()
        .map(|(sig, members)| {
            let mut m: Vec<&R> = members.iter().copied().collect();
            m.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
            (sig, m)
        })
        .collect();
    sorted.sort_by(|a, b| rsig_key(a.0).cmp(&rsig_key(b.0)));

    println!("R-equivalence classes ({}):", sorted.len());
    for (sig, members) in &sorted {
        let (lx, ly) = (sig_str(&sig.0), sig_str(&sig.1));
        println!("  class ({}) -> ({}):", lx, ly);
        for r in members {
            println!("    R({}, {})", r.x, r.y);
        }
    }

    println!();
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

fn rsig_key(sig: &RSignature) -> (usize, usize, u8, usize, usize, u8) {
    let (ax, ay) = (&sig.0, &sig.1);
    let sa = slot_ord(ax.slots);
    let sb = slot_ord(ay.slots);
    (ax.degree_out, ax.degree_in, sa, ay.degree_out, ay.degree_in, sb)
}

fn slot_ord(s: SlotPattern) -> u8 {
    match s {
        SlotPattern::None => 0,
        SlotPattern::LeftOnly => 1,
        SlotPattern::RightOnly => 2,
        SlotPattern::Both => 3,
    }
}
