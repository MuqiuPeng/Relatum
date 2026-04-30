//! C.3 prep — chain-pattern predicate + minimal empirical test.
//!
//! Per C.3_prep.md design: this example ships the smallest
//! viable test for C.3a's `is_chain_subgraph` predicate.
//! Validates that the predicate works correctly on the kinds
//! of subgraphs G-series produces.
//!
//! Test cases:
//!   1. POSITIVE: 5-step successor chain from G.1 logic →
//!      should be recognized as chain of length 5
//!   2. NEGATIVE: 3-node star (1 hub, 2 spokes) → should NOT
//!      be recognized as a chain
//!   3. NEGATIVE: 4-node cycle → should NOT be recognized
//!   4. POSITIVE: chain of length 1 (single edge) → length 1
//!
//! The predicate is INLINE here per C.3_prep.md: keep the
//! recipe in the example for now; promote to lib only when
//! C.3a (full chain detection on real substrates) is itself
//! pursued.

use relatum_v2::{R, RSet, Subgraph};
use std::collections::HashSet;

fn mint_successor(token: &str) -> String {
    format!("succ({})", token)
}

/// C.3a predicate — is this subgraph a chain of length N?
///
/// Returns `Some(N)` when the subgraph is a directed chain
/// of length N (N ≥ 1), `None` otherwise. See C.3_prep.md §
/// "C.3a Definition" for the formal characterization.
fn is_chain_subgraph(sg: &Subgraph) -> Option<usize> {
    let edges: Vec<&R> = sg.edges().collect();
    let n_edges = edges.len();
    if n_edges == 0 {
        return None;
    }

    // Build degree maps.
    let mut in_deg: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    let mut out_deg: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    let mut nodes: HashSet<&str> = HashSet::new();
    for e in &edges {
        *out_deg.entry(e.x.as_str()).or_insert(0) += 1;
        *in_deg.entry(e.y.as_str()).or_insert(0) += 1;
        nodes.insert(e.x.as_str());
        nodes.insert(e.y.as_str());
    }

    // Chain of length N: N+1 distinct nodes, N edges.
    if nodes.len() != n_edges + 1 {
        return None;
    }

    // Exactly one source (in-deg 0), one sink (out-deg 0).
    let mut sources: Vec<&str> = Vec::new();
    let mut sinks: Vec<&str> = Vec::new();
    for &node in &nodes {
        let id = *in_deg.get(node).unwrap_or(&0);
        let od = *out_deg.get(node).unwrap_or(&0);
        if id == 0 && od == 1 {
            sources.push(node);
        } else if od == 0 && id == 1 {
            sinks.push(node);
        } else if id == 1 && od == 1 {
            // interior node — fine
        } else {
            // any other degree pattern → not a chain
            return None;
        }
    }
    if sources.len() != 1 || sinks.len() != 1 {
        return None;
    }

    // Walk from source via out-edges; ensure we visit every node.
    // Build adjacency on demand.
    let mut next: std::collections::HashMap<&str, &str> =
        std::collections::HashMap::new();
    for e in &edges {
        // Each non-sink has exactly one out-edge by the degree check.
        next.insert(e.x.as_str(), e.y.as_str());
    }
    let mut cur = sources[0];
    let mut visited: HashSet<&str> = HashSet::new();
    visited.insert(cur);
    while let Some(&nxt) = next.get(cur) {
        if !visited.insert(nxt) {
            // already visited → cycle (shouldn't happen given
            // degree check, but defensive)
            return None;
        }
        cur = nxt;
    }
    // Walk should have visited every node exactly once.
    if visited.len() != nodes.len() {
        return None;
    }

    Some(n_edges)
}

fn main() {
    println!("=== C.3 prep — chain-pattern predicate empirical test ===");

    // ── Test 1: G.1-style 5-step successor chain ────────────
    println!();
    println!("[1] POSITIVE: 5-step successor chain (G.1 style)");
    let mut chain_edges: Vec<R> = Vec::new();
    let mut cur = "0".to_string();
    for _ in 0..5 {
        let nxt = mint_successor(&cur);
        chain_edges.push(R::new(&nxt, &cur)); // R(succ, prev) — successor → predecessor
        cur = nxt;
    }
    let chain_sg = Subgraph::from_edges(chain_edges.clone());
    let chain_result = is_chain_subgraph(&chain_sg);
    println!("    edges:  {} (chain[i+1] → chain[i])", chain_edges.len());
    println!("    nodes:  {}", chain_sg.edges()
        .flat_map(|e| [e.x.as_str(), e.y.as_str()])
        .collect::<HashSet<_>>().len());
    println!("    is_chain_subgraph → {:?}", chain_result);
    let test1_pass = chain_result == Some(5);
    println!("    expected: Some(5) → {}", if test1_pass { "✓" } else { "✗" });

    // ── Test 2: 3-node star (NEGATIVE) ─────────────────────
    println!();
    println!("[2] NEGATIVE: 3-node out-star (hub → 2 spokes)");
    let star_edges = vec![
        R::new("hub", "leaf_a"),
        R::new("hub", "leaf_b"),
    ];
    let star_sg = Subgraph::from_edges(star_edges);
    let star_result = is_chain_subgraph(&star_sg);
    println!("    is_chain_subgraph → {:?}", star_result);
    let test2_pass = star_result.is_none();
    println!("    expected: None → {}", if test2_pass { "✓" } else { "✗" });

    // ── Test 3: 3-node cycle (NEGATIVE) ────────────────────
    println!();
    println!("[3] NEGATIVE: 3-node directed cycle");
    let cycle_edges = vec![
        R::new("a", "b"),
        R::new("b", "c"),
        R::new("c", "a"),
    ];
    let cycle_sg = Subgraph::from_edges(cycle_edges);
    let cycle_result = is_chain_subgraph(&cycle_sg);
    println!("    is_chain_subgraph → {:?}", cycle_result);
    let test3_pass = cycle_result.is_none();
    println!("    expected: None → {}", if test3_pass { "✓" } else { "✗" });

    // ── Test 4: chain of length 1 (POSITIVE) ───────────────
    println!();
    println!("[4] POSITIVE: chain of length 1 (single edge)");
    let single_edge_sg = Subgraph::from_edges(vec![R::new("a", "b")]);
    let single_result = is_chain_subgraph(&single_edge_sg);
    println!("    is_chain_subgraph → {:?}", single_result);
    let test4_pass = single_result == Some(1);
    println!("    expected: Some(1) → {}", if test4_pass { "✓" } else { "✗" });

    // ── Test 5: branching chain (NEGATIVE) ─────────────────
    println!();
    println!("[5] NEGATIVE: branching chain (a→b, b→c, b→d)");
    let branching_sg = Subgraph::from_edges(vec![
        R::new("a", "b"),
        R::new("b", "c"),
        R::new("b", "d"),
    ]);
    let branching_result = is_chain_subgraph(&branching_sg);
    println!("    is_chain_subgraph → {:?}", branching_result);
    let test5_pass = branching_result.is_none();
    println!("    expected: None → {}", if test5_pass { "✓" } else { "✗" });

    // ── Verdict ─────────────────────────────────────────────
    let pass_count = [test1_pass, test2_pass, test3_pass, test4_pass, test5_pass]
        .iter()
        .filter(|p| **p)
        .count();
    println!();
    println!("════════════════════════════════════════════════════════");
    println!("  Predicate test results: {}/5 pass", pass_count);
    if pass_count == 5 {
        println!();
        println!("  → POSITIVE: is_chain_subgraph predicate is well-defined");
        println!("    and produces correct results on all test cases.");
        println!();
        println!("  Next steps for C.3a (DEFERRED per C.3_prep.md):");
        println!("    1. Build a chain-rich substrate (engineered or otherwise)");
        println!("    2. Run motif discovery on it");
        println!("    3. Apply this predicate to discovered motifs");
        println!("    4. Verify that motif discovery surfaces chains as patterns");
    } else {
        println!("  → MIXED: predicate has issues; see per-test status above");
    }
    println!();
    println!("--- end ---");
    let _ = RSet::new(); // keep import used
}
