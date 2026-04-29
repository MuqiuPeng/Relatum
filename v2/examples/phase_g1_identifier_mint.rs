//! Phase G.1 — Identifier minting proof-of-concept.
//!
//! Smallest mechanism that derives NEW identifiers from existing
//! ones via deterministic application of a recipe — bridging from
//! Beta-1 (new structures over fixed identifiers) toward G.2+
//! (generative rules integrated with the R primitive).
//!
//! Recipe:
//!   mint_successor(token) := format!("succ({})", token)
//!
//! Properties verified:
//!   1. Determinism — mint_successor("0") == "succ(0)" every time.
//!      External code with the same recipe produces token-equal
//!      identifiers (commitment 4 compliance).
//!   2. Freshness — each minted identifier was NOT in the seed RSet.
//!   3. Anti-collision — minted ids form an injection of the chain
//!      length (5 distinct strings).
//!   4. Materializability — the chain expresses cleanly as R edges:
//!      R(succ(0), 0), R(succ(succ(0)), succ(0)), ...
//!      Each edge means "left is the successor of right".
//!
//! Significance: first runtime step that grows v2's identifier
//! space (not just structure space). Beta-1 added L2/L3/L4
//! abstractions over a fixed identifier pool; G.1 starts producing
//! new identifiers from old. Token-deterministic minting means
//! commitment 4 is preserved — minted strings are externally
//! reproducible.

use relatum_v2::{R, RSet};
use std::collections::HashSet;

const SEED: &str = "0";
const CHAIN_LENGTH: usize = 5;
const SUCC_MARKER: &str = "__successor__";

/// Deterministic minting recipe. Pure function of input string.
fn mint_successor(token: &str) -> String {
    format!("succ({})", token)
}

fn main() {
    println!("=== Phase G.1 — Identifier minting proof-of-concept ===");
    println!();

    // ---- Property 1: determinism ----
    let a = mint_successor(SEED);
    let b = mint_successor(SEED);
    let c = mint_successor("0");
    assert_eq!(a, b, "non-deterministic mint");
    assert_eq!(a, c, "external recipe call differs from internal");
    println!("[1] determinism: mint_successor(\"0\") == \"{}\" (3/3 calls match)", a);

    // ---- Setup: seed RSet contains only the seed identifier as a self-loop ----
    let mut rset = RSet::new();
    rset.add(R::new(SEED, SEED));
    let seed_ids: HashSet<String> =
        rset.identifiers().iter().map(|s| (*s).to_string()).collect();
    println!();
    println!("seed RSet: {} edges, {} identifiers", rset.len(), seed_ids.len());

    // ---- Trace: apply mint_successor iteratively, materialize each step as R ----
    println!();
    println!("=== Minting chain ===");
    let mut current = SEED.to_string();
    let mut chain_ids: Vec<String> = vec![current.clone()];
    let mut chain_edges: Vec<R> = Vec::new();
    for step in 1..=CHAIN_LENGTH {
        let next = mint_successor(&current);
        // R(next, current) := "next is the successor of current"
        let edge = R::new(&next, &current);
        // R(SUCC_MARKER, next) := "next is a successor-derived identifier"
        let marker_edge = R::new(SUCC_MARKER, &next);
        rset.add(edge.clone());
        rset.add(marker_edge);
        chain_edges.push(edge);
        chain_ids.push(next.clone());
        println!("  step {}: {} -> {}", step, current, next);
        current = next;
    }

    // ---- Property 2: freshness ----
    let mut fresh_count = 0;
    for id in chain_ids.iter().skip(1) {
        if !seed_ids.contains(id.as_str()) {
            fresh_count += 1;
        }
    }
    assert_eq!(fresh_count, CHAIN_LENGTH, "some minted ids collided with seed");
    println!();
    println!("[2] freshness: {}/{} minted ids absent from seed RSet", fresh_count, CHAIN_LENGTH);

    // ---- Property 3: anti-collision ----
    let unique: HashSet<&str> = chain_ids.iter().map(|s| s.as_str()).collect();
    assert_eq!(unique.len(), chain_ids.len(), "chain has duplicates");
    println!("[3] anti-collision: {} unique ids in chain (length {})", unique.len(), chain_ids.len());

    // ---- Property 4: materializability ----
    let materialized = chain_edges.iter().filter(|e| rset.contains(e)).count();
    assert_eq!(materialized, CHAIN_LENGTH);
    println!("[4] materializability: {}/{} chain edges present in RSet",
             materialized, CHAIN_LENGTH);

    // ---- Probe: structural query — what identifiers carry the SUCC_MARKER? ----
    let succ_derived: Vec<&R> = rset.left_of(SUCC_MARKER);
    println!();
    println!("=== Meta-R query: R(__successor__, ?) ===");
    println!("  {} successor-marked identifiers:", succ_derived.len());
    for r in &succ_derived {
        println!("    {}", r.y);
    }

    // ---- Probe: walk the chain backwards via right_of ----
    println!();
    println!("=== Walk backwards via R(?, current) ===");
    let mut cur = current.clone();
    for _ in 0..CHAIN_LENGTH {
        // The chain edge is R(next, prev). To walk from `cur` back to its
        // predecessor, we look for R(cur, prev) — `cur` on the LEFT.
        let outs: Vec<&R> = rset
            .left_of(&cur)
            .into_iter()
            .filter(|r| r.y != cur && !r.y.starts_with("__"))
            .collect();
        if outs.is_empty() {
            println!("  reached terminus: {}", cur);
            break;
        }
        let pred = outs[0].y.to_string();
        println!("  {} -> predecessor {}", cur, pred);
        cur = pred;
    }

    // ---- Verdict ----
    println!();
    println!("=== Verdict ===");
    println!("  POSITIVE — all 4 properties hold:");
    println!("    1. determinism  ✓");
    println!("    2. freshness    ✓");
    println!("    3. anti-collision ✓");
    println!("    4. materializability ✓");
    println!();
    println!("Final RSet: {} edges (seed self-loop + {} chain + {} marker = {})",
             rset.len(), CHAIN_LENGTH, CHAIN_LENGTH, 1 + 2 * CHAIN_LENGTH);
    println!();
    println!("--- end ---");
}
