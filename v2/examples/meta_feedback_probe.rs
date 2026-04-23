//! Meta-R feedback probe (ADR 0011).
//!
//! Runs compound_class_subgraphs before and after ADR 0010 pattern
//! naming, on the canonical ADR 0007 mixed graph. Classifies each
//! post-naming compound class as data-only / meta-only / mixed and
//! reports canonical forms for any class whose members form a
//! structurally repeating subgraph.
//!
//! Used to produce `logs/2026-04-23_meta_feedback_probe.log`.

use relatum_v2::{
    CanonicalForm, EdgeFingerprint, IdentifierProfile, RSet, SlotPattern, Subgraph,
    PATTERN_MARKER, R,
};
use std::collections::{BTreeMap, HashSet};

fn main() {
    let mut rs = build_mixed_graph();

    println!("========================================");
    println!(" BASELINE — before ADR 0010 pattern naming");
    println!("========================================");
    let baseline = rs.compound_class_subgraphs();
    print_compound_summary(&baseline, &HashSet::new());
    println!();

    // Name all four canonical-form groups via ADR 0009 + ADR 0010.
    let mut by_canon: BTreeMap<CanonicalForm, Vec<Subgraph>> = BTreeMap::new();
    for (_fp, subs) in baseline {
        for sub in subs {
            by_canon.entry(sub.canonicalize()).or_default().push(sub);
        }
    }
    for (_canon, subs) in by_canon {
        rs.name_pattern_instances(&subs).unwrap();
    }

    let meta_ids = collect_meta_ids(&rs);

    println!("========================================");
    println!(" POST-NAMING — ADR 0010 recordings present");
    println!("========================================");
    println!(
        "RSet size: {} edges  (originally 14; added {} meta-R entries)",
        rs.len(),
        rs.len() - 14
    );
    println!("meta-R identifier count: {}", meta_ids.len());
    println!();

    let post = rs.compound_class_subgraphs();
    print_compound_summary(&post, &meta_ids);
    println!();

    println!("========================================");
    println!(" PER-CLASS SUBGRAPH + CANONICAL SUMMARY");
    println!("========================================");
    print_subgraph_canonical_summary(&post, &meta_ids);
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

fn collect_meta_ids(rs: &RSet) -> HashSet<String> {
    let mut meta = HashSet::new();
    meta.insert(PATTERN_MARKER.to_string());
    for p in rs.patterns() {
        meta.insert(p.to_string());
        for inst in rs.instances_of(p) {
            meta.insert(inst.to_string());
        }
    }
    meta
}

enum EdgeKind {
    Data,
    Meta,
    Mixed,
}

fn classify_edge(r: &R, meta_ids: &HashSet<String>) -> EdgeKind {
    let x_meta = meta_ids.contains(&r.x);
    let y_meta = meta_ids.contains(&r.y);
    match (x_meta, y_meta) {
        (false, false) => EdgeKind::Data,
        (true, true) => EdgeKind::Meta,
        _ => EdgeKind::Mixed,
    }
}

fn classify_class(members: &[Subgraph], meta_ids: &HashSet<String>) -> &'static str {
    let mut has_data = false;
    let mut has_meta = false;
    let mut has_mixed = false;
    for sg in members {
        for r in sg.edges() {
            match classify_edge(r, meta_ids) {
                EdgeKind::Data => has_data = true,
                EdgeKind::Meta => has_meta = true,
                EdgeKind::Mixed => has_mixed = true,
            }
        }
    }
    match (has_data, has_meta, has_mixed) {
        (true, false, false) => "data-only",
        (false, true, false) => "meta-only",
        (false, false, true) => "mixed-only",
        _ => "mixed-kind",
    }
}

fn print_compound_summary(
    classes: &std::collections::HashMap<EdgeFingerprint, Vec<Subgraph>>,
    meta_ids: &HashSet<String>,
) {
    println!(
        "compound classes: {}",
        classes.len()
    );
    let total_subs: usize = classes.values().map(|v| v.len()).sum();
    let total_edges: usize = classes
        .values()
        .flat_map(|v| v.iter())
        .map(|s| s.len())
        .sum();
    println!("subgraph instances: {}", total_subs);
    println!("edges across all subgraphs: {}", total_edges);

    // Group sizes histogram
    let mut sizes: Vec<usize> = classes
        .values()
        .map(|v| v.iter().map(|s| s.len()).sum::<usize>())
        .collect();
    sizes.sort_by(|a, b| b.cmp(a));
    println!("class sizes (by total edges, desc): {:?}", sizes);

    // Per-class classification
    println!();
    println!("per-class kind:");
    let mut sorted_classes: Vec<(EdgeFingerprint, Vec<Subgraph>)> = classes
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    sorted_classes.sort_by_key(|(fp, _)| fingerprint_key(fp));
    for (fp, subs) in &sorted_classes {
        let kind = classify_class(subs, meta_ids);
        let edges: usize = subs.iter().map(|s| s.len()).sum();
        let ((x, y), loc) = fp;
        println!(
            "  [{}] {} edges / {} subgraphs:  x=({}) y=({}) loc=({},{},{},{})",
            kind,
            edges,
            subs.len(),
            sig_str(x),
            sig_str(y),
            loc.co_left, loc.co_right, loc.forward, loc.reverse,
        );
    }
}

fn print_subgraph_canonical_summary(
    classes: &std::collections::HashMap<EdgeFingerprint, Vec<Subgraph>>,
    meta_ids: &HashSet<String>,
) {
    // For each class with subgraph count > 1, print the canonical forms
    // of the subgraph instances.
    let mut interesting: Vec<(EdgeFingerprint, Vec<Subgraph>)> = classes
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    interesting.sort_by_key(|(fp, _)| fingerprint_key(fp));

    println!(
        "compound classes with >1 subgraph instance (candidate patterns-of-patterns):"
    );
    if interesting.is_empty() {
        println!("  (none)");
        return;
    }

    for (fp, subs) in interesting {
        let kind = classify_class(&subs, meta_ids);
        let ((x, y), _loc) = &fp;
        println!(
            "  [{}] x=({}) y=({})  subgraphs={}",
            kind,
            sig_str(x),
            sig_str(y),
            subs.len(),
        );
        // Group by canonical form
        let mut by_canon: BTreeMap<CanonicalForm, usize> = BTreeMap::new();
        for sub in &subs {
            *by_canon.entry(sub.canonicalize()).or_insert(0) += 1;
        }
        for (canon, count) in by_canon {
            println!("    canonical {:?}  x {}", canon, count);
        }
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
    fn s(s: SlotPattern) -> u8 {
        match s {
            SlotPattern::None => 0,
            SlotPattern::LeftOnly => 1,
            SlotPattern::RightOnly => 2,
            SlotPattern::Both => 3,
        }
    }
    (
        x.degree_out, x.degree_in, s(x.slots),
        y.degree_out, y.degree_in, s(y.slots),
        loc.co_left, loc.co_right, loc.forward, loc.reverse,
    )
}
