//! Axiom discovery demo (ADR 0027).
//!
//! Constructs a diamond poset, a chain, and a symmetric graph; on
//! each, runs `discover_axioms` (templates) + `check_poset` (combined
//! three-axiom verdict). Demonstrates the extensional→intensional
//! lift: the system reports *which axioms hold*, not just *which
//! subgraph shapes repeat*.

use relatum_v2::{AxiomDiscoveryConfig, RSet, R};

fn main() {
    println!("=== Case 1: diamond poset ===");
    let diamond = build_diamond_poset();
    print_axioms(&diamond);
    print_poset(&diamond);
    println!();

    println!("=== Case 2: raw chain a → b → c (not transitive) ===");
    let mut chain = RSet::new();
    chain.extend([R::new("a", "b"), R::new("b", "c")]);
    print_axioms(&chain);
    print_poset(&chain);
    println!();

    println!("=== Case 3: symmetric graph ===");
    let mut sym = RSet::new();
    sym.extend([
        R::new("a", "b"), R::new("b", "a"),
        R::new("b", "c"), R::new("c", "b"),
    ]);
    print_axioms(&sym);
    print_poset(&sym);
    println!();
}

fn print_axioms(rs: &RSet) {
    let config = AxiomDiscoveryConfig::default();
    let axioms = rs.discover_axioms(&config);
    println!("discovered axioms (rate = 1.0, evidence ≥ 1): {}", axioms.len());
    for ev in axioms {
        println!(
            "  num_vars={}  premise={:?}  conclusion={:?}  bindings={}",
            ev.template.num_vars,
            ev.template
                .premise
                .iter()
                .map(|e| format!("R({}, {})", e.x_var, e.y_var))
                .collect::<Vec<_>>(),
            format!("R({}, {})", ev.template.conclusion.x_var, ev.template.conclusion.y_var),
            ev.premise_bindings,
        );
    }
}

fn print_poset(rs: &RSet) {
    let pc = rs.check_poset();
    println!(
        "check_poset:  is_poset={}  reflexive={:.0}%({}/{})  antisymmetric={}  transitive={}",
        pc.is_poset,
        pc.reflexive.rate * 100.0,
        pc.reflexive.self_loops_present,
        pc.reflexive.identifiers_total,
        pc.antisymmetric.holds,
        pc.transitive.as_ref().map(|e| e.rate == 1.0).unwrap_or(true),
    );
}

fn build_diamond_poset() -> RSet {
    let mut rs = RSet::new();
    for n in ["a", "b", "c", "d"] {
        rs.add(R::new(n, n));
    }
    rs.extend([
        R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
        R::new("b", "d"), R::new("c", "d"),
    ]);
    rs
}
