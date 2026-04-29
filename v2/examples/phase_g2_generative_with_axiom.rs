//! Phase G.2 — Generative-rule output integrated with the existing axiom system.
//!
//! G.1 showed minted identifiers materialize as R edges. G.2 asks the
//! integration question: does the existing `forward_apply_axiom`
//! machinery accept those edges as ordinary data, *without any
//! special handling*?
//!
//! Procedure:
//!   1. Build a 5-step minted chain (R(succ(x), x) for x in chain).
//!   2. Register transitivity (`ax_tpl_v3_p0-1_p1-2_c0-2`).
//!   3. forward_apply_axiom — get the transitive closure.
//!   4. Verify: R(succ⁵(0), "0") ∈ closure.
//!   5. Round-trip rset via to_text / from_text — confirm
//!      minted identifiers are persistence-safe.
//!
//! The 6-id chain has 6 choose 2 = 15 directed (i > j) pairs.
//! Direct edges = 5 (chain). Transitivity should add 10. Plus the
//! 5 chain edges themselves (forward_apply emits all edges that
//! satisfy the conclusion, not just NEW ones).
//!
//! Significance: closes the loop. G.1 = "minting works"; G.2 =
//! "minted output is first-class data — existing axiom processing
//! handles it transparently". No new code path; constitution
//! commitments preserved end-to-end.

use relatum_v2::{
    axiom_template_id, AxiomTemplate, EdgeTemplate, R, RSet,
};
use std::collections::HashSet;

const SEED: &str = "0";
const CHAIN_LENGTH: usize = 5;

fn mint_successor(token: &str) -> String {
    format!("succ({})", token)
}

fn main() {
    println!("=== Phase G.2 — Generative output × existing axiom system ===");
    println!();

    // ---- Build chain ----
    let mut rset = RSet::new();
    let mut chain_ids: Vec<String> = vec![SEED.to_string()];
    let mut current = SEED.to_string();
    for _ in 0..CHAIN_LENGTH {
        let next = mint_successor(&current);
        rset.add(R::new(&next, &current));
        chain_ids.push(next.clone());
        current = next;
    }
    println!("chain ids: {:?}", chain_ids);
    println!("seed rset: {} edges", rset.len());

    // ---- Register transitivity ----
    // R(0,1) ∧ R(1,2) → R(0,2)
    let trans_template = AxiomTemplate {
        num_vars: 3,
        premise: vec![
            EdgeTemplate { x_var: 0, y_var: 1 },
            EdgeTemplate { x_var: 1, y_var: 2 },
        ],
        conclusion: EdgeTemplate { x_var: 0, y_var: 2 },
    };
    let trans_id = axiom_template_id(&trans_template);
    println!("transitivity axiom id: {}", trans_id);
    let registered = rset.register_axiom_with_intension(&trans_id);
    assert!(registered, "axiom registration failed");

    // ---- Forward-apply (iterate to fixpoint) ----
    // forward_apply_axiom is single-step. Transitive closure needs
    // iteration: apply, add new edges, re-apply, until no new edges.
    let before_total = rset.len();
    let mut rounds = 0;
    let mut last_round_added = usize::MAX;
    while last_round_added > 0 {
        rounds += 1;
        let predictions = rset.forward_apply_axiom(&trans_id);
        let new_edges: Vec<R> = predictions.into_iter().filter(|r| !rset.contains(r)).collect();
        last_round_added = new_edges.len();
        for r in new_edges {
            rset.add(r);
        }
        println!("  round {}: +{} edges (rset = {})", rounds, last_round_added, rset.len());
    }
    let after_total = rset.len();
    println!();
    println!(
        "fixpoint reached after {} rounds (rset {} → {}, +{} inferred)",
        rounds, before_total, after_total, after_total - before_total,
    );

    // ---- Verify: full transitive closure on the chain ids ----
    // Expected: all (i > j) pairs among the 6 chain ids → 15 directed edges,
    // 5 of which are chain originals → 10 inferred.
    let mut expected: HashSet<R> = HashSet::new();
    for i in 0..chain_ids.len() {
        for j in 0..i {
            expected.insert(R::new(&chain_ids[i], &chain_ids[j]));
        }
    }
    println!("expected closure pairs: {}", expected.len());

    let missing: Vec<&R> = expected.iter().filter(|e| !rset.contains(e)).collect();
    println!("  missing from final rset: {}", missing.len());
    for m in &missing {
        println!("    {} -> {}", m.x, m.y);
    }

    // The CRITICAL edge: succ⁵(0) → 0 (transitivity bridges full chain).
    let critical = R::new(&chain_ids[CHAIN_LENGTH], SEED);
    let critical_present = rset.contains(&critical);
    println!();
    println!("=== Critical edge ===");
    println!(
        "  R({}, {}) ∈ rset: {}",
        critical.x, critical.y, critical_present,
    );
    assert!(critical_present, "transitivity did not bridge full chain");

    // ---- Round-trip persistence ----
    let text = rset.to_text().expect("to_text failed");
    let restored = RSet::from_text(&text).expect("from_text failed");
    let round_trip_ok = rset == restored;
    println!();
    println!("=== Round-trip via to_text / from_text ===");
    println!("  serialized {} bytes", text.len());
    println!("  restored == original: {}", round_trip_ok);
    assert!(round_trip_ok, "round-trip lost information");

    // ---- Verify minted ids survive the round-trip with byte equality ----
    let original_ids: HashSet<&str> = rset.identifiers();
    let restored_ids: HashSet<&str> = restored.identifiers();
    let id_match = original_ids == restored_ids;
    println!("  identifier set identical: {}", id_match);
    let succ_ids: Vec<&&str> = original_ids
        .iter()
        .filter(|s| s.starts_with("succ("))
        .collect();
    println!("  minted ids preserved through serialization: {}", succ_ids.len());

    // ---- Verdict ----
    let all_ok =
        missing.is_empty() && critical_present && round_trip_ok && id_match;
    println!();
    println!("=== Verdict ===");
    if all_ok {
        println!("  POSITIVE — generative output integrates with existing axiom processing.");
        println!("  Transitivity closes the chain; persistence preserves minted ids byte-for-byte.");
        println!("  No new code path required; commitment 4 holds end-to-end.");
    } else {
        println!("  NEGATIVE — integration broke somewhere; see details above.");
    }
    println!();
    println!("--- end ---");
}
