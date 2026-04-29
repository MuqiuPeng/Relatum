//! Phase B.5.1 — Scheduler autonomously dispatches shape-family
//! discovery.
//!
//! B.5 added `ActionKind::DiscoverAxiomShapeFamilies` and the
//! execute_action arm. B.5.1 adds:
//!  1. `FrontierKind::ShapeFamilyDiscoveryCandidate`
//!  2. `Frontier::refresh_shape_family_candidates` — surfaces the
//!     candidate when ≥ 2 registered axioms share a premise that's
//!     not yet a family
//!  3. RuleBasedScheduler routes ShapeFamilyDiscoveryCandidate →
//!     ActionKind::DiscoverAxiomShapeFamilies
//!  4. Frontier refresh now calls refresh_shape_family_candidates
//!
//! This slice verifies the autonomous loop discovers shape families
//! WITHOUT external `discover_axiom_shape_families` invocation. We
//! check post-Phase-0 that families exist in rset.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    RSet,
};

const TICKS_PHASE_0: u64 = 1000;

fn main() {
    println!("=== Phase B.5.1 — Scheduler autonomously dispatches shape-family discovery ===");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let families = rt.rset.axiom_shape_families();
    let mut sorted = families.clone();
    sorted.sort();
    println!();
    println!("Post-Phase-0 state:");
    println!("  axioms             : {}", rt.rset.axioms().len());
    println!("  theories           : {}", rt.rset.theories().len());
    println!("  shape families     : {}", sorted.len());
    for f in &sorted {
        let n = rt.rset.shape_family_members(f).len();
        println!("    {}: {} members", f, n);
    }

    // Count DiscoverAxiomShapeFamilies episodes that fired during the run.
    let dsf_episodes = rt.memory.episodes.iter()
        .filter(|e| e.action_kind == relatum_v2::runtime::ActionKind::DiscoverAxiomShapeFamilies)
        .count();
    println!();
    println!("DiscoverAxiomShapeFamilies episodes fired: {}", dsf_episodes);

    println!();
    println!("=== Verdict ===");
    if !sorted.is_empty() && dsf_episodes > 0 {
        println!(
            "  → POSITIVE — scheduler autonomously fired {} DiscoverAxiomShapeFamilies episode(s); rset contains {} shape families post-Phase-0.",
            dsf_episodes, sorted.len(),
        );
    } else if !sorted.is_empty() {
        println!(
            "  → MIXED — families exist ({}) but no DiscoverAxiomShapeFamilies episode fired (perhaps families pre-exist? or scheduler picked them via other path).",
            sorted.len(),
        );
    } else {
        println!("  → NULL — no families discovered; scheduler may not have surfaced the candidate.");
    }
    println!();
    println!("--- end ---");
}
