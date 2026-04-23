//! Subgraph canonicalization demo (ADR 0009).
//!
//! Applies `canonicalize` to every subgraph instance produced by
//! `compound_class_subgraphs` on the ADR 0007 mixed graph, then groups
//! by canonical form. The resulting groups are "pattern equivalence
//! classes" — subgraph instances that represent the same pattern.
//!
//! Used to produce `logs/2026-04-23_canonicalization.log`.

use relatum_v2::{CanonicalForm, EdgeFingerprint, RSet, Subgraph, R};
use std::collections::{BTreeMap, HashMap};

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

    // Per-compound-class subgraph instances.
    let class_subs: HashMap<EdgeFingerprint, Vec<Subgraph>> = rs.compound_class_subgraphs();

    // Flatten: every subgraph instance, tagged with which compound class
    // it came from. We represent the compound class by a short label so
    // the log is readable.
    let mut compound_labels: HashMap<EdgeFingerprint, String> = HashMap::new();
    let mut next_cls = 0;
    // Deterministic ordering of compound classes by content description.
    let mut ordered_classes: Vec<(EdgeFingerprint, Vec<Subgraph>)> =
        class_subs.into_iter().collect();
    ordered_classes.sort_by_key(|(fp, _)| fingerprint_sort_key(fp));
    for (fp, _) in &ordered_classes {
        compound_labels.insert(fp.clone(), format!("C{}", next_cls));
        next_cls += 1;
    }

    // Group instances by canonical form.
    //
    // Use BTreeMap for deterministic iteration order.
    let mut pattern_classes: BTreeMap<CanonicalForm, Vec<(String, Subgraph)>> = BTreeMap::new();
    for (fp, subs) in ordered_classes {
        let label = compound_labels[&fp].clone();
        for sub in subs {
            let canon = sub.canonicalize();
            pattern_classes
                .entry(canon)
                .or_default()
                .push((label.clone(), sub));
        }
    }

    let total_instances: usize = pattern_classes.values().map(|v| v.len()).sum();

    println!("Total edges:               {}", rs.len());
    println!("Total subgraph instances:  {}", total_instances);
    println!("Total pattern classes (canonical forms): {}", pattern_classes.len());
    println!();

    for (i, (canon, instances)) in pattern_classes.iter().enumerate() {
        let compound_origins: Vec<&str> = {
            let mut v: Vec<&str> = instances.iter().map(|(l, _)| l.as_str()).collect();
            v.sort();
            v.dedup();
            v
        };
        println!(
            "pattern P{}  ({} instances)  canonical edges: {:?}",
            i + 1,
            instances.len(),
            canon
        );
        println!(
            "  compound classes involved: [{}]{}",
            compound_origins.join(", "),
            if compound_origins.len() > 1 {
                "   <-- cross-compound-class pattern"
            } else {
                ""
            }
        );
        for (cls, sub) in instances {
            let mut edges: Vec<&R> = sub.edges().collect();
            edges.sort_by(|a, b| (a.x.as_str(), a.y.as_str()).cmp(&(b.x.as_str(), b.y.as_str())));
            let rendered: Vec<String> = edges
                .iter()
                .map(|r| format!("R({},{})", r.x, r.y))
                .collect();
            println!("  [{}] {{{}}}", cls, rendered.join(", "));
        }
        println!();
    }
}

fn fingerprint_sort_key(
    fp: &EdgeFingerprint,
) -> (usize, usize, u8, usize, usize, u8, usize, usize, usize, usize) {
    use relatum_v2::SlotPattern;
    let ((x, y), loc) = fp;
    fn slot_ord(s: SlotPattern) -> u8 {
        match s {
            SlotPattern::None => 0,
            SlotPattern::LeftOnly => 1,
            SlotPattern::RightOnly => 2,
            SlotPattern::Both => 3,
        }
    }
    (
        x.degree_out, x.degree_in, slot_ord(x.slots),
        y.degree_out, y.degree_in, slot_ord(y.slots),
        loc.co_left, loc.co_right, loc.forward, loc.reverse,
    )
}
