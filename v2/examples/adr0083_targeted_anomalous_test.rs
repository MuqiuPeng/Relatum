//! ADR 0083 targeted engagement test — construct a substrate with
//! a genuine singleton (instance_count=1) pattern and verify that
//! ApplyRecommendedPatternIntervention dispatches + retracts it.
//!
//! Approach: build a small rset by hand with several distinct
//! size-3 motifs, each appearing exactly once. Then manually
//! invoke autonomous_pass to mint patterns. Each mint will have
//! instance_count = 1 → Anomalous → PatternRetract recommendation.
//! Run the autonomous runtime briefly and confirm ARPI fires.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    R, RSet, Subgraph,
};

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" ADR 0083 targeted ARPI dispatch test");
    println!(" Substrate: hand-built rset with singleton size-3 motifs");
    println!("════════════════════════════════════════════════════════");
    println!();

    // Hand-built rset: 4 distinct size-3 motifs, each appearing
    // exactly once.
    //
    //   motif 1: 3-chain    a -> b -> c
    //   motif 2: 3-star     d -> {e, f, g}? — but star has 3 edges
    //                       on 4 nodes. use d-fork-only e/f
    //   motif 3: triangle   h -> i, i -> j, j -> h
    //   motif 4: V-shape    k -> m, l -> m
    let tsv = "\
# Anomalous-injection substrate
a1\ta1
a2\ta2
a3\ta3
b1\tb1
b2\tb2
b3\tb3
c1\tc1
c2\tc2
c3\tc3
d1\td1
d2\td2
d3\td3
a1\tb1
b1\tc1
a2\tb2
a2\tc2
a3\tb3
b3\tc3
c3\ta3
b2\ta2
d1\td2
d3\td2
";
    let mut rset = RSet::from_text(tsv).expect("from_text");
    eprintln!("Built rset: {} R-instances, {} ids",
              rset.len(), rset.identifiers().len());

    // Directly mint a singleton pattern using name_pattern_instances
    // with exactly ONE subgraph instance. This guarantees
    // instance_count = 1 → Anomalous class (at empty substrates)
    // → PatternRetract recommendation.
    let single_instance = Subgraph::from_edges(vec![
        R::new("a1", "b1"),
        R::new("b1", "c1"),
    ]);
    let pid = rset.name_pattern_instances(&[single_instance])
        .expect("name_pattern_instances");
    eprintln!("Force-minted pattern {} with 1 instance", pid);
    eprintln!("Post-mint: {} patterns", rset.patterns().len());
    for p in rset.patterns() {
        let n = rset.instances_of(p).len();
        eprintln!("  {} → {} instances", p, n);
    }

    // Sanity: directly call recommend_pattern_intervention.
    let substrates: Vec<RSet> = Vec::new();
    let reports = rset.pattern_quality_report_all(&substrates, None);
    eprintln!("Reports generated: {}", reports.len());
    for r in &reports {
        eprintln!("  {} class={:?} inst={} mdl={} overlap={:.2}",
                  r.pattern_id, r.summary_class, r.instance_count,
                  r.mdl_gain, r.overlap_score);
        let others: Vec<_> = reports.iter()
            .filter(|x| x.pattern_id != r.pattern_id).cloned().collect();
        let rec = RSet::recommend_pattern_intervention(r, &others);
        eprintln!("    → recommendation: {:?}", rec);
    }

    // Wrap into runtime and step a few ticks (no stream — just
    // exercise the refresh loop).
    let mut rt = AutonomousRuntime::new(rset);
    rt.environment = Box::new(SyntheticStreamEnvironment::new(Vec::new()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    for tick_target in [1u64, 5, 20, 50, 100] {
        rt.run_bounded(tick_target - rt.tick);
        let arpi = rt.memory.policy_stats.action_counts
            .get(&ActionKind::ApplyRecommendedPatternIntervention)
            .copied().unwrap_or(0);
        let arpi_pos = rt.memory.policy_stats.action_positive_delta_counts
            .get(&ActionKind::ApplyRecommendedPatternIntervention)
            .copied().unwrap_or(0);
        println!(" tick={:>3} | pats={:>2} eps={:>3} | ARPI={} pos={}",
                 tick_target,
                 rt.rset.patterns().len(),
                 rt.memory.episodes.len(),
                 arpi, arpi_pos);
    }

    println!();
    println!(" Final state:");
    println!("   patterns = {}", rt.rset.patterns().len());

    let arpi_eps: Vec<_> = rt.memory.episodes.iter()
        .filter(|ep| ep.action_kind ==
                     ActionKind::ApplyRecommendedPatternIntervention)
        .collect();
    println!();
    println!(" ARPI episodes: {}", arpi_eps.len());
    for ep in arpi_eps.iter().take(20) {
        println!("   tick={} target={:?} delta={}",
                 ep.tick, ep.target, ep.delta);
    }

    println!();
    println!(" All episodes ({} total):", rt.memory.episodes.len());
    for ep in rt.memory.episodes.iter() {
        println!("   tick={} kind={:?} target={:?} delta={}",
                 ep.tick, ep.action_kind, ep.target, ep.delta);
    }
    println!();
    println!(" Frontier items at end:");
    for it in rt.frontier.items.iter() {
        println!("   id={} kind={:?} target={:?} priority={:.2}",
                 it.id, it.kind, it.target, it.priority);
    }
    println!();
    println!(" Lifecycle transitions: {}",
             rt.memory.lifecycle_transitions.len());
    for t in rt.memory.lifecycle_transitions.iter().take(10) {
        println!("   tick={} {:?}→{:?}", t.tick, t.from, t.to);
    }

    println!();
    println!("--- end ---");
}
