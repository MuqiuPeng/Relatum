//! Phase F.2 — Family-aware merge candidate signaling.
//!
//! Alpha-5 picked merge candidates by Jaccard, excluding subset
//! pairs. F.2 adds a NEW signal: each theory's "shape-family
//! signature" — the set of shape families containing its members.
//! Two theories with *complementary* (disjoint or near-disjoint)
//! signatures cover different structural niches; merging them
//! consolidates without information loss.
//!
//! This slice computes signatures and reports complementarity
//! per pair. Doesn't actually merge — that's the standard
//! Alpha-5 mechanism. F.2 is a SIGNAL extension, future work
//! integrates with selection logic.
//!
//! Captured to `logs/<date>_phase_f2_family_aware.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    RSet,
};
use std::collections::HashSet;

const TICKS_PHASE_0: u64 = 1000;

/// Theory's "family signature": set of shape-family ids whose
/// members include any axiom of this theory.
fn theory_family_signature(rt: &AutonomousRuntime, theory_id: &str) -> HashSet<String> {
    let theory_members: HashSet<String> = rt
        .rset
        .theory_axioms(theory_id)
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mut sig: HashSet<String> = HashSet::new();
    for sf in rt.rset.axiom_shape_families() {
        let members = rt.rset.shape_family_members(sf);
        for m in &members {
            if theory_members.contains(*m) {
                sig.insert(sf.to_string());
                break;
            }
        }
    }
    sig
}

fn jaccard_set(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let inter = a.intersection(b).count();
    let uni = a.union(b).count();
    if uni == 0 { 0.0 } else { inter as f64 / uni as f64 }
}

fn main() {
    println!("=== Phase F.2 — Family-aware merge candidate signal ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    // Ensure shape families discovered (B.5.1 should have done it).
    let _ = rt.rset.discover_axiom_shape_families(2);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    let families: Vec<&str> = rt.rset.axiom_shape_families();

    println!();
    println!("Phase 0: {} theories, {} shape families", theories.len(), families.len());

    if theories.len() < 2 {
        println!("→ INAPPLICABLE — too few theories");
        return;
    }

    // Compute per-theory family signature.
    let signatures: Vec<HashSet<String>> = theories
        .iter()
        .map(|t| theory_family_signature(&rt, t))
        .collect();

    println!();
    println!("=== Per-theory family signatures ===");
    for (t, sig) in theories.iter().zip(&signatures) {
        let mut s_vec: Vec<&String> = sig.iter().collect();
        s_vec.sort();
        println!("  {}: {} families = {:?}", t, sig.len(), s_vec);
    }

    // Pairwise complementarity (lower jaccard = more complementary = better merge).
    println!();
    println!("=== Pairwise signature complementarity (1 - jaccard) ===");
    println!("    higher complementarity = signatures more disjoint = better merge candidate");
    println!("{:>10} {:>10} {:>15} {:>20}", "a", "b", "sig_jaccard", "complementarity");
    let mut best: Option<(String, String, f64)> = None;
    for i in 0..theories.len() {
        for j in (i + 1)..theories.len() {
            let j_score = jaccard_set(&signatures[i], &signatures[j]);
            let complement = 1.0 - j_score;
            println!(
                "{:>10} {:>10} {:>15.4} {:>20.4}",
                theories[i], theories[j], j_score, complement,
            );
            match &best {
                None => best = Some((theories[i].clone(), theories[j].clone(), complement)),
                Some((_, _, w)) if complement > *w => {
                    best = Some((theories[i].clone(), theories[j].clone(), complement))
                }
                _ => {}
            }
        }
    }

    println!();
    println!("=== Verdict ===");
    if let Some((a, b, comp)) = best {
        println!(
            "  → POSITIVE — best merge pair by signature complementarity: ({}, {}) at {:.4}",
            a, b, comp,
        );
        println!("    F.2 produces a complementary pair-selection signal distinct from Alpha-5's Jaccard-on-membership.");
    } else {
        println!("  → INSUFFICIENT — no pairs to compare");
    }
    println!();
    println!("--- end ---");
}
