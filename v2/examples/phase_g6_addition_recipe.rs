//! Phase G.6 — Multi-arity generative recipe (addition).
//!
//! G.1 demonstrated a unary generative recipe (successor). G.6 extends
//! to a BINARY recipe — one input pair → one fresh identifier — and
//! verifies ADR 0069's 4-property contract still holds for multi-arity.
//!
//! Recipe:
//!   mint_add(a, b) := format!("add({}, {})", a, b)
//!
//! Materialization: each call writes TWO edges (one per operand):
//!   R(add(a, b), a)   — "add(a,b) connects to a"
//!   R(add(a, b), b)   — "add(a,b) connects to b"
//!
//! Significance:
//!   - First multi-arity generative axiom (ADR 0069 covered it
//!     in principle; G.6 is the empirical confirmation)
//!   - Building block for integer arithmetic (G.7) — successor
//!     gives the chain, addition gives composition
//!   - Materialization uses 2 R edges per mint, still no escape
//!     from commitment 1 (R singular)

use relatum_v2::{R, RSet};
use std::collections::HashSet;

const NUM_MINTS: usize = 5;
const ADD_MARKER: &str = "__add__";

/// Deterministic binary recipe.
fn mint_add(a: &str, b: &str) -> String {
    format!("add({}, {})", a, b)
}

fn main() {
    println!("=== Phase G.6 — Multi-arity generative recipe (addition) ===");
    println!();

    // ---- Property 1: determinism (multi-arity) ----
    let r1 = mint_add("0", "0");
    let r2 = mint_add("0", "0");
    let r3 = mint_add("succ(0)", "succ(0)");
    let r4 = mint_add("0", "succ(0)");
    let r5 = mint_add("succ(0)", "0");
    assert_eq!(r1, r2, "non-deterministic on (a, a)");
    assert_ne!(r4, r5, "recipe is symmetric (R doesn't distinguish operand order)");
    assert_eq!(r3, "add(succ(0), succ(0))");
    println!("[1] determinism: mint_add(\"0\", \"0\") == \"{}\" (3/3 calls match)", r1);
    println!("    mint_add NOT symmetric: \"add(0, succ(0))\" != \"add(succ(0), 0)\" — orientation preserved");

    // ---- Setup ----
    let seed_pool: Vec<String> = (0..3).map(|i| format!("seed_{}", i)).collect();
    let mut rset = RSet::new();
    for s in &seed_pool {
        rset.add(R::new(s, s));
    }
    let seed_ids: HashSet<String> =
        rset.identifiers().iter().map(|s| (*s).to_string()).collect();
    println!();
    println!("seed RSet: {} edges, {} identifiers", rset.len(), seed_ids.len());

    // ---- Apply mint_add to a sequence of (a, b) pairs ----
    println!();
    println!("=== Minting via addition recipe ===");
    let pairs: Vec<(String, String)> = vec![
        ("seed_0".to_string(), "seed_1".to_string()),
        ("seed_0".to_string(), "seed_2".to_string()),
        ("seed_1".to_string(), "seed_2".to_string()),
        ("seed_0".to_string(), "seed_0".to_string()),
        ("seed_1".to_string(), "seed_1".to_string()),
    ];
    let mut minted_ids: Vec<String> = Vec::new();
    let mut minted_edges: Vec<(R, R)> = Vec::new();
    for (a, b) in &pairs {
        let result = mint_add(a, b);
        let ea = R::new(&result, a);
        let eb = R::new(&result, b);
        let em = R::new(ADD_MARKER, &result);
        rset.add(ea.clone());
        rset.add(eb.clone());
        rset.add(em);
        minted_edges.push((ea, eb));
        minted_ids.push(result.clone());
        println!("  mint_add({}, {}) = {}", a, b, result);
    }

    // ---- Property 2: freshness ----
    let mut fresh = 0;
    for id in &minted_ids {
        if !seed_ids.contains(id.as_str()) { fresh += 1; }
    }
    assert_eq!(fresh, NUM_MINTS, "minted ids collided with seed");
    println!();
    println!("[2] freshness: {}/{} minted ids absent from seed RSet", fresh, NUM_MINTS);

    // ---- Property 3: anti-collision (output disjoint from input space) ----
    // Outputs all start with "add(" — disjoint from seed pool by construction
    let mut all_distinct = true;
    let unique: HashSet<&str> = minted_ids.iter().map(|s| s.as_str()).collect();
    if unique.len() != minted_ids.len() { all_distinct = false; }
    let mut overlap_with_seeds = false;
    for id in &minted_ids {
        if seed_ids.contains(id.as_str()) {
            overlap_with_seeds = true;
            break;
        }
    }
    assert!(all_distinct, "minted ids contain duplicates");
    assert!(!overlap_with_seeds, "minted ids overlap with seed pool");
    println!(
        "[3] anti-collision: all {} mints distinct AND disjoint from input space",
        minted_ids.len(),
    );

    // ---- Property 4: materializability (2 edges per mint) ----
    let mut materialized = 0;
    for (ea, eb) in &minted_edges {
        if rset.contains(ea) { materialized += 1; }
        if rset.contains(eb) { materialized += 1; }
    }
    assert_eq!(materialized, 2 * NUM_MINTS);
    println!(
        "[4] materializability: {}/{} edges present in RSet ({} per mint × {} mints)",
        materialized, 2 * NUM_MINTS, 2, NUM_MINTS,
    );

    // ---- Property 5: persistence safety ----
    let text = rset.to_text().expect("to_text failed");
    let restored = RSet::from_text(&text).expect("from_text failed");
    let round_trip_ok = rset == restored;
    assert!(round_trip_ok, "round-trip lost information");
    println!(
        "[5] persistence: round-trip {} bytes; restored == original; minted ids preserved byte-for-byte",
        text.len(),
    );

    // ---- Probe: backwards walk via R(add(a,b), ?) ----
    println!();
    println!("=== Backwards walk: from minted id to operands ===");
    for id in &minted_ids {
        let outs: Vec<&R> = rset.left_of(id);
        // Filter the operand edges (R(id, *) where the right is a seed or another mint)
        let operands: Vec<&str> = outs
            .iter()
            .map(|r| r.y.as_str())
            .filter(|y| *y != id.as_str()) // skip self-loops if any
            .collect();
        println!("  {} → operands {:?}", id, operands);
    }

    // ---- Verdict ----
    println!();
    println!("=== Verdict ===");
    println!("  POSITIVE — multi-arity recipe satisfies all 5 ADR-0069 properties:");
    println!("    1. determinism (binary)        ✓");
    println!("    2. freshness                    ✓");
    println!("    3. anti-collision               ✓");
    println!("    4. materializability (2 edges)  ✓");
    println!("    5. persistence safety           ✓");
    println!();
    println!("Final RSet: {} edges (3 seed loops + {} chain edges + {} marker = {})",
             rset.len(), 2 * NUM_MINTS, NUM_MINTS, 3 + 3 * NUM_MINTS);
    println!();
    println!("--- end ---");
}
