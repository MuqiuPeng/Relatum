//! sample_instances_of vs find_instances_of (ADR 0024).
//!
//! Compare the two variants on a handful of target canonicals at
//! different sample budgets. Exhaustive hits every clean instance;
//! sampling hits a subset that grows with sample_count.

use relatum_v2::{CanonicalForm, NamingPolicy, RSet, SamplingMatchConfig, R};

fn main() {
    let mut rs = build_mixed_graph();
    rs.run_naming_pass(&NamingPolicy::default());

    let targets: &[(&str, CanonicalForm)] = &[
        ("2-chain       ", vec![(1, 2), (2, 0)]),
        ("2-star        ", vec![(1, 0), (1, 0)]),
        ("3-chain       ", vec![(1, 3), (2, 0), (3, 2)]),
        ("3-cycle       ", vec![(0, 0), (0, 0), (0, 0)]),
        ("3-star        ", vec![(1, 0), (1, 0), (1, 0)]),
        ("3-tree        ", vec![(2, 0), (3, 1), (3, 2)]),
    ];
    let budgets = [50usize, 200, 1000];

    println!(
        "{:<16} {:>9}  {:>9}  {:>9}  {:>9}",
        "target", "exact", "N=50", "N=200", "N=1000"
    );
    for (label, canon) in targets {
        let exact = rs.find_instances_of(canon).len();
        let mut row = String::new();
        for b in budgets {
            let got = rs.sample_instances_of(
                canon,
                &SamplingMatchConfig { sample_count: b, rng_seed: 2024 },
            );
            row.push_str(&format!("  {:>9}", got.len()));
        }
        println!("{} {:>9}{}", label, exact,row);
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
