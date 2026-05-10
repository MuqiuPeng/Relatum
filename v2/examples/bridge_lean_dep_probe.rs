//! Phase 0 of vibe-proving bridge (ADR 0081).
//!
//! Synthesizes a Lean-style lemma dependency graph in-process,
//! anonymizes ids to `lem_NNNN` tokens, writes TSV per ADR 0038,
//! loads via `RSet::from_text`, then runs the standard v2
//! discovery pipeline (autonomous_pass / discover_axioms_minimal /
//! discover_theory).
//!
//! Reports what v2 says about this non-canonical substrate vs the
//! OQ#1-clade synthetic suite that v2 has been tested against so
//! far.
//!
//! No external dependencies — synthetic data substitutes for a
//! real Mathlib extraction until a checkout is available (Phase 1
//! territory).

use relatum_v2::{
    AutonomousConfig, AxiomDiscoveryConfig, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig,
};

/// Build a synthetic Lean-style dependency graph:
///   - 80 lemma nodes (lem_0000 .. lem_0079)
///   - layered structure: lower-indexed lemmas are "base", higher
///     are "derived"
///   - each derived lemma depends on 2-4 random earlier lemmas
///   - some "clusters" where lemmas in close index range
///     interlinkedly cite each other (simulates topic-coherent
///     bundles in Mathlib like Group / Ring / Topology)
fn build_synthetic_lean_dep_graph() -> Vec<(String, String)> {
    use std::collections::HashSet;
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let n = 80usize;

    // Simple deterministic pseudo-random (xorshift64 over u64).
    let mut rng_state: u64 = 0xCAFEBABE_DEADBEEF;
    let mut next = || {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        rng_state
    };

    let token = |i: usize| format!("lem_{:04}", i);

    // Base layer (0..20): some pairwise dependencies for foundation
    for i in 0..20 {
        let deps = (next() as usize) % 2 + 1;
        for _ in 0..deps {
            let target = (next() as usize) % i.max(1);
            if target != i {
                edges.insert((token(i), token(target)));
            }
        }
    }
    // Middle layer (20..50): more dependencies, mixed across base
    for i in 20..50 {
        let deps = (next() as usize) % 3 + 2;
        for _ in 0..deps {
            let target = (next() as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    // Derived layer (50..n): heaviest dependency count
    for i in 50..n {
        let deps = (next() as usize) % 4 + 2;
        for _ in 0..deps {
            let target = (next() as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    // Clusters: each cluster of 5 lemmas interlinked (simulates
    // topic-coherent bundles)
    for cluster_start in (0..n).step_by(15) {
        let cluster_end = (cluster_start + 5).min(n);
        for i in cluster_start..cluster_end {
            for j in cluster_start..cluster_end {
                if i != j && (next() as usize) % 2 == 0 {
                    edges.insert((token(i), token(j)));
                }
            }
        }
    }

    edges.into_iter().collect()
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" ADR 0081 Phase 0 — vibe-proving bridge (synthetic Lean dep)");
    println!("════════════════════════════════════════════════════════");

    // 1. Generate synthetic graph.
    let edges = build_synthetic_lean_dep_graph();
    println!();
    println!(" Synthetic Lean dep graph: {} edges over up to 80 lemmas",
             edges.len());

    // 2. Write TSV per ADR 0038 format.
    let mut tsv = String::new();
    tsv.push_str("# synthetic lean-style dep graph\n");
    tsv.push_str("# 80 lemmas, layered + clustered structure\n");
    tsv.push_str("# extracted 2026-05-11 by bridge_lean_dep_probe v0.1\n");
    for (a, b) in &edges {
        tsv.push_str(&format!("{}\t{}\n", a, b));
    }
    println!(" TSV body: {} bytes", tsv.len());

    // 3. Load via RSet::from_text.
    let mut rset = match RSet::from_text(&tsv) {
        Ok(rs) => rs,
        Err(e) => {
            println!(" ✗ from_text failed: {:?}", e);
            return;
        }
    };
    let loaded_edges = rset.iter().count();
    println!(" Loaded into RSet: {} edges (sanity check vs {})",
             loaded_edges, edges.len());

    // 4. Run autonomous_pass for sizes 2-3, count mints.
    // (size 4-5 deferred — find_instances_of is O(data^k) and
    // sizes 4-5 on a 270-edge graph exceeded reasonable budget
    // in pilot runs. Size 2-3 already produced rich signal.)
    println!();
    println!(" === autonomous_pass sizes 2..=3 ===");
    for size in 2..=3 {
        let cfg = AutonomousConfig {
            discovery: DiscoveryConfig {
                target_size: size,
                sample_count: 400,
                top_m: 20,
                rng_seed: 0xC0FFEE_u64.wrapping_add(size as u64 * 0x9E37),
                include_meta_in_discovery: false,
            },
            refinement: RefinementConfig {
                max_tries: 200,
                rng_seed: 0xC0FFEE_u64.wrapping_add(size as u64 * 0xDEAD),
            },
            naming: NamingPolicy::default(),
            instance_sampling: None,
        };
        let outcomes = rset.autonomous_pass(&cfg);
        let new_patterns: Vec<&str> = outcomes.iter()
            .filter_map(|o| match o {
                relatum_v2::AutonomousOutcome::NewPattern {
                    pattern_id, ..
                } => Some(pattern_id.as_str()),
                _ => None,
            })
            .collect();
        let existing = outcomes.iter()
            .filter(|o| matches!(
                o,
                relatum_v2::AutonomousOutcome::Existing { .. }
            ))
            .count();
        let skipped = outcomes.iter()
            .filter(|o| matches!(
                o,
                relatum_v2::AutonomousOutcome::Skipped { .. }
            ))
            .count();
        println!("   size={}: {} new patterns ({:?}), {} existing, {} skipped",
                 size, new_patterns.len(), new_patterns, existing, skipped);
    }

    let patterns = rset.patterns();
    let pattern_count = patterns.len();
    println!();
    println!(" Total patterns minted: {}", pattern_count);
    for pid in patterns.iter().take(10) {
        let shape = rset.format_pattern_shape(pid);
        let first_line = shape.lines().next().unwrap_or("(empty)");
        println!("   {}", first_line);
    }

    // 5. Run discover_axioms_minimal.
    println!();
    println!(" === discover_axioms_minimal ===");
    let ax_cfg = AxiomDiscoveryConfig::default();
    let axioms = rset.discover_axioms_minimal(&ax_cfg);
    println!("   {} axioms discovered", axioms.len());
    for ax in axioms.iter().take(5) {
        println!(
            "   rate={:.3} support={} → {:?}",
            ax.rate, ax.premise_bindings, ax.template,
        );
    }

    // 6. Run discover_theory.
    println!();
    println!(" === discover_theory ===");
    let th = rset.discover_theory(&ax_cfg);
    println!("   theory candidate axiom set: {:?}", th.member_axiom_ids);

    // 7. Summary verdict.
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Go/no-go verdict");
    println!("════════════════════════════════════════════════════════");
    let any_pattern_minted = pattern_count > 0;
    let any_axiom_holds = !axioms.is_empty();
    let theory_has_axioms = !th.member_axiom_ids.is_empty();

    println!();
    println!("   Pattern minted on synthetic-Lean substrate: {}",
             if any_pattern_minted { "✓" } else { "✗" });
    println!("   Axiom holds (rate >= min_rate): {}",
             if any_axiom_holds { "✓" } else { "✗" });
    println!("   Theory candidate non-empty: {}",
             if theory_has_axioms { "✓" } else { "✗" });

    println!();
    if any_pattern_minted || any_axiom_holds || theory_has_axioms {
        println!(" → GO. Phase 0 produced non-trivial output on a substrate");
        println!("   that v2 has never seen before. The bridge mechanism");
        println!("   works; Phase 1 with real Mathlib data is the natural");
        println!("   follow-up.");
    } else {
        println!(" → NO-GO. All outputs degenerate. v2's discovery machinery");
        println!("   doesn't engage on this Lean-style substrate. Either the");
        println!("   substrate generation needs tuning or v2's machinery is");
        println!("   calibrated to a class of structure that Lean dependency");
        println!("   graphs don't exemplify. Either way, write up as a result.");
    }

    println!();
    println!("--- end ---");
}
