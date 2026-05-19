//! ADR 0082 empirical test — does the policy execution loop
//! fire the predicted FamilyDemote on OQ#1's t_0 noise family?
//!
//! Predicted (per ADR 0082 §Empirical predictions):
//!   On OQ#1: t_0 has noise family `shape_premise_p0-0_p1-2`.
//!   Scheduler should fire ApplyRecommendedIntervention(t_0)
//!   → FamilyDemote on that family. Theory count and axiom
//!   count change as the noise family axioms are removed.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    RSet,
};

const HORIZON_TICKS: u64 = 1500;

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" ADR 0082 empirical test — OQ#1 policy execution loop");
    println!(" Predicted: t_0 → FamilyDemote on noise family");
    println!("════════════════════════════════════════════════════════");
    println!();

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(
        build_long_stream(),
    ));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    println!(" Initial state: {} axioms, {} theories, {} families",
             rt.rset.axioms().len(),
             rt.rset.theories().len(),
             rt.rset.axiom_shape_families().len());

    let snap = HORIZON_TICKS / 10;
    let mut tick = 0u64;
    while tick < HORIZON_TICKS {
        let next = (tick + snap).min(HORIZON_TICKS);
        rt.run_bounded(next - tick);
        tick = next;
        let ari_count = rt.memory.policy_stats.action_counts
            .get(&ActionKind::ApplyRecommendedIntervention)
            .copied().unwrap_or(0);
        let ari_pos = rt.memory.policy_stats.action_positive_delta_counts
            .get(&ActionKind::ApplyRecommendedIntervention)
            .copied().unwrap_or(0);
        let rsf_count = rt.memory.policy_stats.action_counts
            .get(&ActionKind::RetractShapeFamily)
            .copied().unwrap_or(0);
        println!(
            " tick={:>4} | axs={:>2} ths={:>2} fams={:>2} eps={:>4} | ARI={}/{}/pos={} RSF={}",
            tick,
            rt.rset.axioms().len(),
            rt.rset.theories().len(),
            rt.rset.axiom_shape_families().len(),
            rt.memory.episodes.len(),
            ari_count, ari_count, ari_pos, rsf_count,
        );
    }

    println!();
    println!(" Final state:");
    println!("   axioms = {}", rt.rset.axioms().len());
    println!("   theories = {} ({:?})",
             rt.rset.theories().len(),
             rt.rset.theories());
    println!("   shape families = {}", rt.rset.axiom_shape_families().len());

    // Find policy episodes.
    let policy_episodes: Vec<_> = rt.memory.episodes.iter()
        .filter(|ep| ep.action_kind ==
                     ActionKind::ApplyRecommendedIntervention)
        .collect();
    println!();
    println!(" ApplyRecommendedIntervention episodes: {}",
             policy_episodes.len());
    for ep in policy_episodes.iter().take(10) {
        println!("   tick={} target={:?} delta={}",
                 ep.tick, ep.target, ep.delta);
    }

    println!();
    println!("--- end ---");
}
