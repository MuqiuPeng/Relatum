//! Phase G.4 — Quality metric for generative axioms.
//!
//! ADR 0069 explicitly excluded generative axioms from cross-precision:
//! cross-precision predicts edges over a fixed substrate, but generative
//! axioms produce identifiers. G.4 specifies the alternative.
//!
//! Approach: **predicate compliance** of the generated structure.
//!
//! For each generative axiom, run it to produce a chain, then check
//! a panel of structural predicates that a "good" generative axiom
//! should satisfy:
//!
//!   1. acyclic — no R(x,y) creates a path back to x via existing edges
//!   2. injective predecessor — every minted id has ≤ 1 predecessor
//!   3. irreflexive — no R(x,x) in the minted chain
//!   4. transitively closes to strict total order on the chain ids
//!   5. fresh — every minted id absent from the seed substrate
//!
//! Compliance rate = fraction of properties satisfied. Quality is a
//! number in [0, 1] per recipe, comparable across recipes — the role
//! cross-precision plays for predicate axioms.
//!
//! This experiment runs two recipes side-by-side:
//!   - **successor** (G.1): mint(t) = format!("succ({})", t)
//!   - **constant**  (broken): mint(t) = "X" — for contrast
//!
//! Expected: successor scores 5/5; constant scores low (fails
//! injectivity, freshness after step 1, anti-collision).

use relatum_v2::{
    axiom_template_id, AxiomTemplate, EdgeTemplate, R, RSet,
};
use std::collections::HashSet;

const SEED: &str = "0";
const CHAIN_LENGTH: usize = 5;

fn run_recipe<F: Fn(&str) -> String>(
    recipe_name: &str,
    mint: F,
    seed: &str,
    chain_length: usize,
) -> (RSet, Vec<String>) {
    let mut rset = RSet::new();
    let mut chain_ids: Vec<String> = vec![seed.to_string()];
    let mut current = seed.to_string();
    for _ in 0..chain_length {
        let next = mint(&current);
        rset.add(R::new(&next, &current));
        chain_ids.push(next.clone());
        current = next;
    }
    println!("[recipe {}]: chain produced ({} ids)", recipe_name, chain_ids.len());
    (rset, chain_ids)
}

/// Property 1: acyclic — no path from any node back to itself
fn check_acyclic(rset: &RSet, chain_ids: &[String]) -> bool {
    // Closure: forward reach from each id
    for start in chain_ids {
        let mut reach: HashSet<String> = HashSet::new();
        let mut frontier: Vec<String> = vec![start.to_string()];
        while let Some(cur) = frontier.pop() {
            for r in rset.left_of(&cur) {
                if r.y == *start {
                    return false; // cycle: start reaches itself
                }
                if reach.insert(r.y.to_string()) {
                    frontier.push(r.y.to_string());
                }
            }
        }
    }
    true
}

/// Property 2: injective predecessor — every minted id has ≤ 1 predecessor
fn check_injective_predecessor(rset: &RSet, chain_ids: &[String]) -> bool {
    for id in chain_ids {
        // Predecessors: edges R(id, ?) where R(id, p) means id is successor of p
        let preds: Vec<&R> = rset.left_of(id);
        if preds.len() > 1 {
            return false;
        }
    }
    true
}

/// Property 3: irreflexive — no self-loops in the minted chain
fn check_irreflexive(rset: &RSet, chain_ids: &[String]) -> bool {
    for id in chain_ids {
        if rset.contains(&R::new(id, id)) {
            return false;
        }
    }
    true
}

/// Property 4: transitive closure → strict total order on chain
fn check_transitive_total_order(rset_in: &RSet, chain_ids: &[String]) -> bool {
    let mut rset = rset_in.clone();
    let trans_template = AxiomTemplate {
        num_vars: 3,
        premise: vec![
            EdgeTemplate { x_var: 0, y_var: 1 },
            EdgeTemplate { x_var: 1, y_var: 2 },
        ],
        conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
    };
    let trans_id = axiom_template_id(&trans_template);
    rset.register_axiom_with_intension(&trans_id);
    // Iterate to fixpoint
    loop {
        let predictions = rset.forward_apply_axiom(&trans_id);
        let new_edges: Vec<R> = predictions.into_iter().filter(|r| !rset.contains(r)).collect();
        if new_edges.is_empty() {
            break;
        }
        for r in new_edges {
            rset.add(r);
        }
    }
    // Strict total order on chain: for every (i, j) with i ≠ j, exactly one
    // of R(chain[i], chain[j]) or R(chain[j], chain[i]) holds.
    for i in 0..chain_ids.len() {
        for j in 0..chain_ids.len() {
            if i == j { continue; }
            let a = rset.contains(&R::new(&chain_ids[i], &chain_ids[j]));
            let b = rset.contains(&R::new(&chain_ids[j], &chain_ids[i]));
            // strict total order: exactly one direction holds (xor)
            if a == b { return false; }
        }
    }
    true
}

/// Property 5: freshness — every minted id (after seed) absent from seed substrate
fn check_freshness(rset: &RSet, chain_ids: &[String], seed_substrate: &HashSet<String>) -> bool {
    for id in chain_ids.iter().skip(1) {
        if seed_substrate.contains(id) { return false; }
    }
    // Bonus: chain ids unique among themselves
    let unique: HashSet<&str> = chain_ids.iter().map(|s| s.as_str()).collect();
    if unique.len() != chain_ids.len() { return false; }
    true
}

fn evaluate_recipe<F: Fn(&str) -> String>(
    name: &str,
    mint: F,
    seed_substrate: &HashSet<String>,
) -> f64 {
    println!();
    println!("====== Recipe: {} ======", name);
    let (rset, chain_ids) = run_recipe(name, &mint, SEED, CHAIN_LENGTH);

    let p1 = check_acyclic(&rset, &chain_ids);
    let p2 = check_injective_predecessor(&rset, &chain_ids);
    let p3 = check_irreflexive(&rset, &chain_ids);
    let p4 = check_transitive_total_order(&rset, &chain_ids);
    let p5 = check_freshness(&rset, &chain_ids, seed_substrate);

    println!("  [1] acyclic:                {}", if p1 { "✓" } else { "✗" });
    println!("  [2] injective predecessor:  {}", if p2 { "✓" } else { "✗" });
    println!("  [3] irreflexive:            {}", if p3 { "✓" } else { "✗" });
    println!("  [4] transitive total order: {}", if p4 { "✓" } else { "✗" });
    println!("  [5] freshness:              {}", if p5 { "✓" } else { "✗" });

    let satisfied = [p1, p2, p3, p4, p5].iter().filter(|x| **x).count();
    let rate = satisfied as f64 / 5.0;
    println!("  → compliance rate: {}/5 = {:.4}", satisfied, rate);
    rate
}

fn main() {
    println!("=== Phase G.4 — Generative-axiom quality via predicate compliance ===");

    // Seed substrate (small, fixed) — could be drawn from a real run.
    let seed_substrate: HashSet<String> = ["0", "X", "Y"]
        .iter().map(|s| s.to_string()).collect();
    println!();
    println!("seed substrate: {:?}", seed_substrate);

    let succ_rate = evaluate_recipe("successor", |t| format!("succ({})", t), &seed_substrate);
    let const_rate = evaluate_recipe("constant",  |_| "X".to_string(), &seed_substrate);

    // A third recipe: "double-prefix" — produces unique tokens but is symmetric
    // (mint(mint(t)) creates new id, so chain is acyclic, but format unusual).
    let dbl_rate = evaluate_recipe("dbl_prefix", |t| format!("p_{}", t), &seed_substrate);

    println!();
    println!("=== Verdict ===");
    println!(
        "  successor:  {:.4} (expected 1.0 — Peano-style chain satisfies all)",
        succ_rate
    );
    println!(
        "  constant:   {:.4} (expected ~0.2 — collisions break injectivity, freshness, total order)",
        const_rate
    );
    println!(
        "  dbl_prefix: {:.4} (expected 1.0 — same shape as successor)",
        dbl_rate
    );
    println!();
    if succ_rate >= 0.99 && const_rate < 0.5 && dbl_rate >= 0.99 {
        println!(
            "  POSITIVE — predicate-compliance metric discriminates good generative recipes from broken ones."
        );
        println!(
            "  This is the cross-precision analog for generative axioms: a per-axiom quality scalar."
        );
    } else {
        println!(
            "  NULL — metric does not discriminate as expected; see compliance vectors above."
        );
    }
    println!();
    println!("--- end ---");
}
