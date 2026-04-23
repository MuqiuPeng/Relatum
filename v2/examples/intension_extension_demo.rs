//! ADR 0029 demo.
//!
//! Runs the same mixed graph as the ADR 0010 `pattern_naming` example,
//! but names each canonical class under each of the three recording
//! policies (Intensional, InstancesOnly, FullBindings) on a fresh copy
//! of the RSet. Reports meta-R growth per policy so the intension /
//! extension split is visible end-to-end.
//!
//! Used to produce `logs/2026-04-24_intension_extension.log`.

use relatum_v2::{CanonicalForm, PatternRecordingPolicy, RSet, Subgraph, R};
use std::collections::BTreeMap;

fn mixed_graph() -> RSet {
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

fn name_all(rs: &mut RSet, policy: PatternRecordingPolicy) -> usize {
    let before = rs.len();
    let class_subs = rs.compound_class_subgraphs();
    let mut by_canon: BTreeMap<CanonicalForm, Vec<Subgraph>> = BTreeMap::new();
    for (_fp, subs) in class_subs {
        for sub in subs {
            by_canon.entry(sub.canonicalize()).or_default().push(sub);
        }
    }
    for (_canon, subs) in by_canon {
        rs.name_pattern_instances_with_policy(&subs, policy)
            .expect("valid");
    }
    rs.len() - before
}

fn main() {
    println!("ADR 0029 — intension / extension split demo");
    println!("Input graph: ADR 0007 mixed graph, 14 data edges.");
    println!();

    for policy in [
        PatternRecordingPolicy::Intensional,
        PatternRecordingPolicy::InstancesOnly,
        PatternRecordingPolicy::FullBindings,
    ] {
        let mut rs = mixed_graph();
        let data = rs.len();
        let added = name_all(&mut rs, policy);
        let roles = rs.roles().len();
        let insts: usize = rs
            .patterns()
            .iter()
            .map(|p| rs.instances_of(p).len())
            .sum();
        println!(
            "policy = {:?}",
            policy
        );
        println!(
            "  meta-R added: {}    patterns: {}    roles: {}    instances: {}    starting data: {}",
            added,
            rs.patterns().len(),
            roles,
            insts,
            data,
        );
        println!();
    }
}
