//! ADR 0030 demo — theory discovery on the 8-case rigorous battery.
//!
//! For each input: run `discover_theory`, then `name_theory`, then
//! print the member axioms. This exercises the conjunctive-concept
//! step: not "here is an axiom that holds" but "here is the full
//! bundle of axioms that jointly describe this RSet."

use relatum_v2::{AxiomDiscoveryConfig, RSet, R};

fn main() {
    let cases: Vec<(&str, RSet)> = vec![
        ("1. transitive closure of linear chain", case_transitive_chain()),
        ("2. equivalence relation (3 classes)", case_equivalence()),
        ("3. strict partial order (diamond, no self-loops)", case_strict_poset()),
        ("4. almost-transitive: closure with ONE violation", case_broken_transitive()),
        ("5. random sparse graph (no designed axiom)", case_random()),
        ("6. tolerance: reflexive + symmetric, NOT transitive", case_tolerance()),
        ("7. total order on {1..5} with self-loops", case_total_order()),
        ("8. complete bipartite", case_bipartite()),
    ];

    let config = AxiomDiscoveryConfig::default();
    for (label, rs) in cases {
        let mut rs = rs;
        let th = rs.discover_theory(&config);
        println!("=== {} ===", label);
        println!("  theory member count: {}", th.member_axiom_ids.len());
        for id in &th.member_axiom_ids {
            println!("    · {}", id);
        }
        if th.member_axiom_ids.is_empty() {
            println!("    (no axioms hold at rate 1.0 — theory is empty)");
            continue;
        }
        // Try to persist it.
        let ids: Vec<&str> = th.member_axiom_ids.iter().map(|s| s.as_str()).collect();
        match rs.name_theory(&ids) {
            Ok(t_id) => {
                println!("  named theory: {}", t_id);
                let members: Vec<&str> = rs.theory_axioms(&t_id);
                println!("  members in meta-R: {:?}", members);
            }
            Err(e) => {
                println!("  name_theory rejected: {:?}", e);
            }
        }
        println!();
    }
}

fn case_transitive_chain() -> RSet {
    let mut rs = RSet::new();
    let nodes = ["a", "b", "c", "d", "e"];
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            rs.add(R::new(nodes[i], nodes[j]));
        }
    }
    rs
}

fn case_equivalence() -> RSet {
    let mut rs = RSet::new();
    let classes: &[&[&str]] = &[&["a", "b"], &["c", "d", "e"], &["f"]];
    for cls in classes {
        for x in cls.iter() {
            for y in cls.iter() {
                rs.add(R::new(*x, *y));
            }
        }
    }
    rs
}

fn case_strict_poset() -> RSet {
    let mut rs = RSet::new();
    rs.extend([
        R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
        R::new("b", "d"), R::new("c", "d"),
    ]);
    rs
}

fn case_broken_transitive() -> RSet {
    let mut rs = RSet::new();
    let nodes = ["a", "b", "c", "d"];
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            rs.add(R::new(nodes[i], nodes[j]));
        }
    }
    rs.remove(&R::new("b", "d"));
    rs
}

fn case_random() -> RSet {
    let mut rs = RSet::new();
    rs.extend([
        R::new("a", "c"), R::new("b", "d"), R::new("c", "e"),
        R::new("d", "f"), R::new("e", "a"), R::new("f", "b"),
        R::new("a", "d"),
    ]);
    rs
}

fn case_tolerance() -> RSet {
    let mut rs = RSet::new();
    for n in ["a", "b", "c"] {
        rs.add(R::new(n, n));
    }
    rs.extend([
        R::new("a", "b"), R::new("b", "a"),
        R::new("b", "c"), R::new("c", "b"),
    ]);
    rs
}

fn case_total_order() -> RSet {
    let mut rs = RSet::new();
    let nodes = ["1", "2", "3", "4", "5"];
    for i in 0..nodes.len() {
        rs.add(R::new(nodes[i], nodes[i]));
        for j in (i + 1)..nodes.len() {
            rs.add(R::new(nodes[i], nodes[j]));
        }
    }
    rs
}

fn case_bipartite() -> RSet {
    let mut rs = RSet::new();
    for a in ["a1", "a2", "a3"] {
        for b in ["b1", "b2", "b3"] {
            rs.add(R::new(a, b));
        }
    }
    rs
}
