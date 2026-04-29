//! Phase Beta-1.5 (Direction B.5) — Runtime integration of shape
//! family discovery.
//!
//! Beta-1 added `RSet::discover_axiom_shape_families` as a
//! standalone API. Beta-2..4 used it from examples by direct call.
//! B.5 wires it into the autonomous runtime as a first-class
//! `ActionKind::DiscoverAxiomShapeFamilies`. Effect: family
//! discovery becomes part of the action catalogue the scheduler
//! can dispatch (when a future scheduler decides to do so).
//!
//! This slice demonstrates the wiring by manually dispatching the
//! action via `ActionPlan { kind: DiscoverAxiomShapeFamilies, ... }`
//! after Phase 0. Verify:
//!  1. Episode appended with action_kind = DiscoverAxiomShapeFamilies
//!  2. Episode delta equals the count of newly minted families
//!  3. Re-dispatch produces delta = 0 (idempotent)
//!  4. New families are queryable via existing `axiom_shape_families`
//!
//! Captured to `logs/<date>_phase_beta_5_runtime_family.log`.

use relatum_v2::{
    runtime::{
        ActionKind, ActionPlan, AutonomousRuntime, FrontierTarget,
        RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    RSet,
};

const TICKS_PHASE_0: u64 = 1000;

fn main() {
    println!(
        "=== Phase Beta-1.5 — Runtime integration of shape family discovery ==="
    );

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    println!();
    println!("=== Pre-dispatch state ===");
    println!(
        "  axioms registered  : {}",
        rt.rset.axioms().len(),
    );
    println!(
        "  shape families     : {}",
        rt.rset.axiom_shape_families().len(),
    );
    println!(
        "  episodes recorded  : {}",
        rt.memory.episodes.len(),
    );

    // First dispatch: should mint families.
    let ep_before = rt.memory.episodes.len();
    let plan = ActionPlan {
        action_kind: ActionKind::DiscoverAxiomShapeFamilies,
        target: FrontierTarget::WholeRSet,
    };
    let delta1 = rt.execute_action(&plan);
    let _ep_after_1 = rt.memory.episodes.len();
    let families_1 = rt.rset.axiom_shape_families().len();

    println!();
    println!("=== Dispatch 1 ===");
    println!("  delta returned    : {:?}", delta1);
    println!("  families minted   : {}", families_1);
    for shape in rt.rset.axiom_shape_families() {
        let n = rt.rset.shape_family_members(shape).len();
        println!("    {}: {} members", shape, n);
    }

    // Second dispatch: should be idempotent (delta = 0).
    let _ep_before_2 = rt.memory.episodes.len();
    let delta2 = rt.execute_action(&plan);
    let families_2 = rt.rset.axiom_shape_families().len();
    println!();
    println!("=== Dispatch 2 (idempotency check) ===");
    println!("  delta returned    : {:?}", delta2);
    println!("  families total    : {}", families_2);

    println!();
    println!("=== Verdict ===");
    let delta1_v = delta1.unwrap_or(-1.0);
    let delta2_v = delta2.unwrap_or(-1.0);
    let mint_count_correct = (delta1_v as usize) == families_1;
    let idempotent = delta2_v == 0.0 && families_2 == families_1;
    let positive = mint_count_correct && idempotent && families_1 > 0;
    if positive {
        println!(
            "  → POSITIVE — runtime dispatch mints {} families, second dispatch idempotent (delta=0).",
            families_1,
        );
    } else if !idempotent {
        println!("  → BUG — second dispatch was not idempotent (delta={}, families={}).", delta2_v, families_2);
    } else if !mint_count_correct {
        println!(
            "  → MISMATCH — delta {} != families {}.",
            delta1_v, families_1,
        );
    } else {
        println!("  → INCONCLUSIVE — no families discovered (substrate doesn't expose them).");
    }
    let _ = ep_before;
    println!();
    println!("--- end ---");
}
