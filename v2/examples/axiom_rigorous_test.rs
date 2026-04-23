//! Rigorous blind test of axiom discovery (ADR 0027).
//!
//! Constructs a battery of mathematically well-defined R-relations
//! and runs the unmodified `discover_axioms` on each. The library
//! code has no knowledge of which input is which; this file just
//! supplies data + prints raw outputs. Judging whether the
//! discovery matches the mathematical ground truth is done in the
//! experiment log, post-hoc.

use relatum_v2::{AxiomDiscoveryConfig, RSet, R};

fn main() {
    let cases: Vec<(&str, RSet)> = vec![
        ("1. transitive closure of linear chain a<b<c<d<e", case_transitive_closure_chain()),
        ("2. equivalence relation (3 classes)", case_equivalence_relation()),
        ("3. strict partial order (diamond, no self-loops)", case_strict_partial_order()),
        ("4. almost-transitive: closure with ONE violation", case_almost_transitive()),
        ("5. random sparse graph (no designed axiom)", case_random_sparse()),
        ("6. tolerance: reflexive + symmetric, NOT transitive", case_tolerance()),
        ("7. total order on {1..5} with self-loops", case_total_order()),
        ("8. complete bipartite (no back edges, no within-side edges)", case_complete_bipartite()),
    ];

    let config = AxiomDiscoveryConfig::default();
    for (label, rs) in cases {
        println!("=== {} ===", label);
        report(&rs, &config);
        println!();
    }
}

fn report(rs: &RSet, config: &AxiomDiscoveryConfig) {
    println!(
        "  edges: {}  identifiers: {}",
        rs.len(),
        rs.identifiers().len()
    );
    let axioms = rs.discover_axioms(config);
    println!("  discovered axioms (rate=1.0, evidence≥1): {}", axioms.len());
    for ev in &axioms {
        let prem_str: Vec<String> = ev
            .template
            .premise
            .iter()
            .map(|e| format!("R({},{})", e.x_var, e.y_var))
            .collect();
        println!(
            "    [{}] ⇒ R({},{})  bindings={}",
            prem_str.join(" ∧ "),
            ev.template.conclusion.x_var,
            ev.template.conclusion.y_var,
            ev.premise_bindings
        );
    }
    let reflex = rs.check_reflexivity();
    let anti = rs.check_antisymmetry();
    let poset = rs.check_poset();
    println!(
        "  check_reflexivity: {}/{} ({:.0}%)  check_antisymmetry: holds={} (violations={})  check_poset: is_poset={}",
        reflex.self_loops_present,
        reflex.identifiers_total,
        reflex.rate * 100.0,
        anti.holds,
        anti.violations,
        poset.is_poset
    );
}

// ---------- test case constructors ----------

fn case_transitive_closure_chain() -> RSet {
    // a < b < c < d < e, closed under transitivity. No self-loops.
    let mut rs = RSet::new();
    let nodes = ["a", "b", "c", "d", "e"];
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            rs.add(R::new(nodes[i], nodes[j]));
        }
    }
    rs
}

fn case_equivalence_relation() -> RSet {
    // 3 equivalence classes: {a,b}, {c,d,e}, {f}. Full within-class.
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

fn case_strict_partial_order() -> RSet {
    // Diamond without self-loops: a<b, a<c, a<d, b<d, c<d. Plus transitive
    // closure (a<d is already in).
    let mut rs = RSet::new();
    rs.extend([
        R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
        R::new("b", "d"), R::new("c", "d"),
    ]);
    rs
}

fn case_almost_transitive() -> RSet {
    // Transitive closure of a 4-chain a<b<c<d, then REMOVE one closure
    // edge (b<d) so transitivity is violated at exactly one binding.
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

fn case_random_sparse() -> RSet {
    // Deterministically-constructed "random-looking" sparse graph.
    // No particular algebraic structure. Identifiers are letters.
    let mut rs = RSet::new();
    rs.extend([
        R::new("a", "c"), R::new("b", "d"), R::new("c", "e"),
        R::new("d", "f"), R::new("e", "a"), R::new("f", "b"),
        R::new("a", "d"),
    ]);
    rs
}

fn case_tolerance() -> RSet {
    // Reflexive + symmetric but not transitive.
    // Graph: {a,b,c}; edges a~b, b~c, but NOT a~c. Plus self-loops.
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
    // 1 ≤ 2 ≤ 3 ≤ 4 ≤ 5 with transitive closure and self-loops.
    let mut rs = RSet::new();
    let nodes = ["1", "2", "3", "4", "5"];
    for i in 0..nodes.len() {
        rs.add(R::new(nodes[i], nodes[i])); // reflexive
        for j in (i + 1)..nodes.len() {
            rs.add(R::new(nodes[i], nodes[j]));
        }
    }
    rs
}

fn case_complete_bipartite() -> RSet {
    // All edges from A = {a1, a2, a3} to B = {b1, b2, b3}. No reverse,
    // no within-side edges, no self-loops.
    let mut rs = RSet::new();
    for a in ["a1", "a2", "a3"] {
        for b in ["b1", "b2", "b3"] {
            rs.add(R::new(a, b));
        }
    }
    rs
}
