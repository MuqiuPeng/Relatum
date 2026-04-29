//! Phase G.7 — Integer arithmetic embedding (constructive scaffold).
//!
//! G.1 + G.6 give the recipes (successor and addition). G.7 combines
//! them with the existing axiom system to express integer ORDER, and
//! documents the boundary where ARITHMETIC EQUIVALENCE (`add(succ(0),
//! succ(0)) ≡ succ(succ(0))`) requires the equality-axiom layer.
//!
//! Three plies:
//!   ply A — Successor chain materialized (G.1)
//!   ply B — Transitivity closure → strict order on chain (Peano "<")
//!   ply C — Addition mints + boundary observation: add(N_a, N_b)
//!           are NEW identifiers; integrating them with the chain
//!           order requires equality axioms (G.7's open boundary)
//!
//! On a 5-step successor chain plus 4 add mints, expect:
//!   - Order: 15 directed pairs from transitivity closure (C(6,2))
//!   - Add mints: 4 NEW ids without order relations to the chain
//!   - Resolution path: equality axiom add(succⁿ, succᵐ) = succⁿ⁺ᵐ

use relatum_v2::{axiom_template_id, AxiomTemplate, EdgeTemplate, R, RSet};
use std::collections::HashSet;

const CHAIN_LEN: usize = 5;
const SEED: &str = "0";

fn mint_succ(t: &str) -> String { format!("succ({})", t) }
fn mint_add(a: &str, b: &str) -> String { format!("add({}, {})", a, b) }

/// Iterate transitivity to fixpoint on the rset.
fn close_transitivity(rset: &mut RSet) {
    let trans = AxiomTemplate {
        num_vars: 3,
        premise: vec![
            EdgeTemplate { x_var: 0, y_var: 1 },
            EdgeTemplate { x_var: 1, y_var: 2 },
        ],
        conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
    };
    let id = axiom_template_id(&trans);
    rset.register_axiom_with_intension(&id);
    loop {
        let pred = rset.forward_apply_axiom(&id);
        let new_edges: Vec<R> = pred.into_iter().filter(|r| !rset.contains(r)).collect();
        if new_edges.is_empty() { break; }
        for r in new_edges { rset.add(r); }
    }
}

fn main() {
    println!("=== Phase G.7 — Integer arithmetic embedding ===");
    println!();

    // ---- Ply A: successor chain ----
    let mut rset = RSet::new();
    let mut chain: Vec<String> = vec![SEED.to_string()];
    let mut cur = SEED.to_string();
    for _ in 0..CHAIN_LEN {
        let nxt = mint_succ(&cur);
        rset.add(R::new(&nxt, &cur));
        chain.push(nxt.clone());
        cur = nxt;
    }
    println!("--- Ply A: successor chain (Peano natural numbers) ---");
    println!("  chain ids: {:?}", chain);
    println!("  chain edges (R(succⁿ, succⁿ⁻¹)): {}", CHAIN_LEN);

    // ---- Ply B: transitivity → strict order ----
    close_transitivity(&mut rset);
    // Count directed pairs ON THE CHAIN ONLY (ignore axiom intension edges)
    let chain_set: HashSet<&str> = chain.iter().map(|s| s.as_str()).collect();
    let order_pairs: usize = rset.iter()
        .filter(|r| chain_set.contains(r.x.as_str()) && chain_set.contains(r.y.as_str()) && r.x != r.y)
        .count();
    println!();
    println!("--- Ply B: transitivity closure → strict total order on chain ---");
    println!("  directed chain pairs after closure: {}", order_pairs);
    let expected_pairs = chain.len() * (chain.len() - 1) / 2; // C(N, 2)
    println!("  expected: C(6, 2) = {} (full transitive order)", expected_pairs);
    assert_eq!(order_pairs, expected_pairs);

    // Verify total order on chain: for every (i, j), exactly one of R(i,j) or R(j,i)
    let mut order_holds = true;
    for i in 0..chain.len() {
        for j in 0..chain.len() {
            if i == j { continue; }
            let a = rset.contains(&R::new(&chain[i], &chain[j]));
            let b = rset.contains(&R::new(&chain[j], &chain[i]));
            if a == b { order_holds = false; break; }
        }
        if !order_holds { break; }
    }
    assert!(order_holds, "transitivity closure did not produce strict total order");
    println!("  strict total order on chain: ✓");
    println!("  semantic reading: R(succⁿ(0), succᵏ(0)) ↔ \"n is greater than k\"");

    // ---- Ply C: addition mints ----
    let add_pairs = vec![
        ("succ(0)", "succ(0)"),
        ("succ(0)", "succ(succ(0))"),
        ("succ(succ(0))", "succ(succ(0))"),
        ("0", "succ(succ(0))"),
    ];
    let mut add_ids: Vec<String> = Vec::new();
    for (a, b) in &add_pairs {
        let r = mint_add(a, b);
        rset.add(R::new(&r, *a));
        rset.add(R::new(&r, *b));
        add_ids.push(r);
    }
    println!();
    println!("--- Ply C: addition mints + arithmetic boundary ---");
    for (i, (a, b)) in add_pairs.iter().enumerate() {
        println!("  add({}, {}) = {}", a, b, add_ids[i]);
    }

    // Re-close transitivity with new edges
    close_transitivity(&mut rset);
    println!("  transitivity re-closed after addition mints");

    // ---- Boundary observation ----
    // For arithmetic, expect: add(succ(0), succ(0)) ≡ succ(succ(0))
    let add_1_1 = "add(succ(0), succ(0))";
    let succ_2 = "succ(succ(0))";

    // Without equality axiom, are these two identifiers in the same equivalence class
    // structurally? Check: do they have identical neighborhoods?
    let mut nbr_a: HashSet<(String, String)> = HashSet::new();
    let mut nbr_b: HashSet<(String, String)> = HashSet::new();
    for r in rset.left_of(add_1_1) {
        nbr_a.insert(("L".to_string(), r.y.to_string()));
    }
    for r in rset.right_of(add_1_1) {
        nbr_a.insert(("R".to_string(), r.x.to_string()));
    }
    for r in rset.left_of(succ_2) {
        nbr_b.insert(("L".to_string(), r.y.to_string()));
    }
    for r in rset.right_of(succ_2) {
        nbr_b.insert(("R".to_string(), r.x.to_string()));
    }
    let same_neighborhood = nbr_a == nbr_b;

    println!();
    println!("=== Boundary observation ===");
    println!("  add(succ(0), succ(0)) and succ(succ(0)) — same equivalence class?");
    println!("    structural neighborhood equal: {}", same_neighborhood);
    println!("    add(1, 1) neighbors: {} edges", nbr_a.len());
    println!("    succ²(0) neighbors:  {} edges", nbr_b.len());
    if !same_neighborhood {
        println!();
        println!("  → They are NOT structurally equivalent under the current axiom set.");
        println!("    Resolution requires an EQUALITY axiom asserting");
        println!("    `add(x, y) = z when chain-position(x) + chain-position(y) = chain-position(z)`.");
        println!("    This is the existing v2 equality-axiom layer (ADR 0044/0047) —");
        println!("    G.7 establishes the SCAFFOLDING; full arithmetic closure is the next step.");
    }

    // ---- Verdict ----
    println!();
    println!("=== Verdict ===");
    println!("  POSITIVE on the scaffold:");
    println!("    Ply A — successor chain materializes ✓");
    println!("    Ply B — transitivity yields Peano \"<\" relation ✓");
    println!("    Ply C — addition mints integrate as new identifiers ✓");
    println!();
    println!("  OPEN on full arithmetic:");
    println!("    `add(N_a, N_b) ≡ N_(a+b)` requires equality axiom.");
    println!("    G.7 = scaffold; closing the equivalence is G.8/G.9 territory.");
    println!();
    println!("Final RSet: {} edges total", rset.len());
    println!();
    println!("--- end ---");
}
