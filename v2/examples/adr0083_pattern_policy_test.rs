//! ADR 0083 empirical test — pattern-side policy execution loop
//! on OQ#2 (substrate that mints many patterns).
//!
//! Predicted: patterns with instance_count=1 and no cross-substrate
//! evidence get retracted via ApplyRecommendedPatternIntervention.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    test_substrates::oq2::build_oq2_stream,
    RSet,
};

const HORIZON_TICKS: u64 = 1500;

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" ADR 0083 — OQ#2 pattern policy execution loop test");
    println!(" Predicted: instance=1 anomalous patterns get retracted");
    println!("════════════════════════════════════════════════════════");
    println!();

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(
        build_oq2_stream(),
    ));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    let snap = HORIZON_TICKS / 10;
    let mut tick = 0u64;
    while tick < HORIZON_TICKS {
        let next = (tick + snap).min(HORIZON_TICKS);
        rt.run_bounded(next - tick);
        tick = next;
        let arpi_count = rt.memory.policy_stats.action_counts
            .get(&ActionKind::ApplyRecommendedPatternIntervention)
            .copied().unwrap_or(0);
        let arpi_pos = rt.memory.policy_stats.action_positive_delta_counts
            .get(&ActionKind::ApplyRecommendedPatternIntervention)
            .copied().unwrap_or(0);
        let ari_count = rt.memory.policy_stats.action_counts
            .get(&ActionKind::ApplyRecommendedIntervention)
            .copied().unwrap_or(0);
        println!(
            " tick={:>4} | axs={:>2} ths={:>2} pats={:>3} eps={:>4} | ARPI={}/pos={} ARI={}",
            tick,
            rt.rset.axioms().len(),
            rt.rset.theories().len(),
            rt.rset.patterns().len(),
            rt.memory.episodes.len(),
            arpi_count, arpi_pos, ari_count,
        );
    }

    println!();
    println!(" Final state:");
    println!("   axioms = {}", rt.rset.axioms().len());
    println!("   theories = {}", rt.rset.theories().len());
    println!("   patterns = {}", rt.rset.patterns().len());

    let arpi_eps: Vec<_> = rt.memory.episodes.iter()
        .filter(|ep| ep.action_kind ==
                     ActionKind::ApplyRecommendedPatternIntervention)
        .collect();
    println!();
    println!(" ApplyRecommendedPatternIntervention episodes: {}",
             arpi_eps.len());
    for ep in arpi_eps.iter().take(10) {
        println!("   tick={} target={:?} delta={}",
                 ep.tick, ep.target, ep.delta);
    }

    println!();
    println!("--- end ---");
}
