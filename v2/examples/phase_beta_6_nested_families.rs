//! Phase B.6 — Nested shape families on OQ#1.
//!
//! Demonstrates two-layer structural abstraction:
//!  1. Layer 1 (Beta-1): axioms grouped into shape families
//!  2. Layer 2 (B.6): families grouped into meta-families by
//!     shared premise edges
//!
//! On OQ#1, after Phase 0 (1000 ticks), we have 6 shape families
//! (3 premise + 3 conclusion). The 3 premise families are:
//!   - shape_premise_p0-0_p1-2 (4 members)
//!   - shape_premise_p0-1 (3 members)
//!   - shape_premise_p0-1_p1-2 (2 members)
//!
//! Shared premise edges:
//!   - p0-0: only in p0-0_p1-2 → no meta
//!   - p0-1: in p0-1 and p0-1_p1-2 → 2 families → MINT meta
//!   - p1-2: in p0-0_p1-2 and p0-1_p1-2 → 2 families → MINT meta
//!
//! Expected: 2 nested families minted.
//!
//! Captured to `logs/<date>_phase_beta_6_nested_families.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    META_SHAPE_FAMILY_MARKER, RSet,
};

const TICKS_PHASE_0: u64 = 1000;

fn main() {
    println!("=== Phase B.6 — Nested shape families ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    // Layer 1: discover shape families.
    let layer1 = rt.rset.discover_axiom_shape_families(2);
    println!();
    println!("=== Layer 1 (Beta-1): {} shape families ===", layer1.len());
    for f in &layer1 {
        println!("  {}", f);
    }

    // Pre-Layer-2 state.
    println!();
    println!(
        "=== Pre-Layer-2: {} nested families (expected 0) ===",
        rt.rset.nested_shape_families().len(),
    );

    // Layer 2: discover nested families.
    let layer2 = rt.rset.discover_nested_shape_families(2);
    println!();
    println!("=== Layer 2 (B.6): {} nested families ===", layer2.len());
    for meta in &layer2 {
        let members = rt.rset.nested_shape_family_members(meta);
        println!("  {}", meta);
        for m in &members {
            println!("    contains {}", m);
        }
    }

    // Constitutional check: META_SHAPE_FAMILY_MARKER queryable.
    println!();
    println!("=== Constitutional check ===");
    let entries: Vec<&str> = rt.rset.left_of(META_SHAPE_FAMILY_MARKER)
        .iter()
        .map(|r| r.y.as_str())
        .collect();
    println!(
        "  R(META_SHAPE_FAMILY_MARKER, ?) → {} entries: {:?}",
        entries.len(),
        entries,
    );

    println!();
    println!("=== Verdict ===");
    if layer2.len() >= 1 {
        println!(
            "  → POSITIVE — {} nested family minted; second-order structural abstraction works.",
            layer2.len(),
        );
        println!(
            "    Layer 1: {} families (axioms grouped by structure)",
            layer1.len(),
        );
        println!(
            "    Layer 2: {} meta-families (families grouped by shared structure)",
            layer2.len(),
        );
    } else {
        println!(
            "  → NULL — no shared premise edges across families on this substrate."
        );
    }
    println!();
    println!("--- end ---");
}
