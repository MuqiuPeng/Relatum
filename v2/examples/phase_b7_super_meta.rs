//! Phase B.7 — Layer-3 super-meta nested abstraction.
//!
//! Beta-1 (L2) groups axioms into shape families.
//! B.6 (L3) groups premise-shape families into nested families
//! by shared individual premise edge.
//! B.7 (L4) groups nested families themselves by shared member
//! shape family — super-meta abstraction.
//!
//! On OQ#1 after Phase 0:
//!   - L2: 6 shape families (3 premise + 3 conclusion)
//!   - L3: 2 nested families (meta_premise_p0-1, meta_premise_p1-2)
//!   - L4: ?  Both meta families contain shape_premise_p0-1_p1-2,
//!     so B.7 should mint 1 super-meta containing both metas.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    SUPER_META_SHAPE_FAMILY_MARKER, RSet,
};

const TICKS_PHASE_0: u64 = 1000;

fn main() {
    println!("=== Phase B.7 — Super-meta layer-3 abstraction ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    // Layer 2.
    let l2 = rt.rset.discover_axiom_shape_families(2);
    println!();
    println!("L2 shape families: {} minted", l2.len());

    // Layer 3.
    let l3 = rt.rset.discover_nested_shape_families(2);
    println!("L3 nested families: {} minted", l3.len());
    for n in &l3 {
        let members = rt.rset.nested_shape_family_members(n);
        println!("  {} → {:?}", n, members);
    }

    // Layer 4 (B.7).
    let l4 = rt.rset.discover_super_meta_shape_families(2);
    println!();
    println!("L4 super-meta families: {} minted", l4.len());
    for s in &l4 {
        let members = rt.rset.super_meta_shape_family_members(s);
        println!("  {} contains:", s);
        for m in &members {
            println!("    {} (which contains: {:?})", m, rt.rset.nested_shape_family_members(m));
        }
    }

    println!();
    println!("=== Constitutional check ===");
    let registered: Vec<&str> = rt.rset.left_of(SUPER_META_SHAPE_FAMILY_MARKER)
        .iter().map(|r| r.y.as_str()).collect();
    println!("  R(SUPER_META_SHAPE_FAMILY_MARKER, ?) → {} entries", registered.len());

    println!();
    println!("=== Verdict ===");
    if !l4.is_empty() {
        println!(
            "  → POSITIVE — Layer-4 abstraction works. Hierarchy:"
        );
        println!("     L2 = {} families", l2.len());
        println!("     L3 = {} nested families", l3.len());
        println!("     L4 = {} super-meta families", l4.len());
        println!("    Recursive structural derivation across 4 layers.");
    } else {
        println!(
            "  → NULL — no Layer-4 candidates (no nested family pair shares a member)."
        );
    }
    println!();
    println!("--- end ---");
}
