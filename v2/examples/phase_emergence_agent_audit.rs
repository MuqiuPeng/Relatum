//! ADR 0076 — Micro-agent reframing audit.
//!
//! Runs the standard runtime on 4 canonical substrates and re-reads
//! the resulting episode log under the micro-agent interpretation.
//! Each `(ActionKind, target-kind)` pair is one "agent class"; each
//! Episode is one transient agent's complete observable existence.
//!
//! The point is not new functionality. It's empirical demonstration
//! that v2's existing dispatch system is already a multi-agent
//! cognitive substrate when read this way.

use relatum_v2::{
    runtime::{
        agent_attention_share_recent, agent_classes, ActionKind,
        AgentClassSummary, AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    test_substrates::{
        long5k::build_5k_stream, narrow_a::build_narrow_a_stream,
        oq1::build_long_stream, oq2::build_oq2_stream,
    },
    RSet,
};

fn audit_substrate(
    label: &str,
    stream: Vec<(u64, Event)>,
    ticks: u64,
) -> Vec<AgentClassSummary> {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: {} ({} ticks)", label, ticks);
    println!("════════════════════════════════════════════════════════");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    let total_episodes = rt.memory.episodes.len();
    let classes = agent_classes(&rt.memory.episodes);
    println!(
        " Total episodes: {} | Distinct agent classes: {}",
        total_episodes, classes.len(),
    );

    println!();
    println!(
        " {:<28} {:>6} {:>6} {:>8} {:>8} {:>8} {:>10}",
        "agent class (kind/target)", "eps", "succ", "succ%",
        "first", "last", "mean Δ",
    );
    println!(" {}", "─".repeat(76));
    for c in &classes {
        let kind_str = format!("{:?}", c.action_kind);
        let label_str = format!("{}/{}", kind_str, c.target_label);
        let label_short = if label_str.len() > 28 {
            format!("{}…", &label_str[..27])
        } else {
            label_str
        };
        println!(
            " {:<28} {:>6} {:>6} {:>7.1}% {:>8} {:>8} {:>10.3}",
            label_short,
            c.episode_count,
            c.success_count,
            c.success_rate * 100.0,
            c.first_tick,
            c.last_tick,
            c.mean_delta,
        );
    }

    // Recent attention share for the top three action kinds.
    let total_eps = rt.memory.episodes.len();
    let n_recent = total_eps.min(20);
    if n_recent > 0 {
        println!();
        println!(" Recent-window attention share (last {} episodes):",
                 n_recent);
        let kinds_to_check = [
            ActionKind::DiscoverTheory,
            ActionKind::DiscoverPatterns,
            ActionKind::EvaluatePredictions,
            ActionKind::DiscoverMetaMetaPatterns,
            ActionKind::DiscoverAxiomShapeFamilies,
        ];
        for kind in &kinds_to_check {
            let share = agent_attention_share_recent(
                &rt.memory.episodes, *kind, n_recent,
            );
            if share > 0.0 {
                println!(
                    "   {:<32} {:>5.1}%",
                    format!("{:?}", kind),
                    share * 100.0,
                );
            }
        }
    }

    classes
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" ADR 0076 — Micro-agent audit of v2's dispatch log");
    println!("════════════════════════════════════════════════════════");
    println!(" Re-reads each substrate's episode log under the micro-");
    println!(" agent interpretation: each (ActionKind, target-kind) is");
    println!(" one agent class, each Episode is a transient agent's");
    println!(" complete observable existence. Demonstrates v2 is");
    println!(" already a multi-agent cognitive substrate by query.");

    let _oq1 = audit_substrate("OQ#1", build_long_stream(), 1000);
    let _long5k = audit_substrate("long5k", build_5k_stream(), 1500);
    let _narrow_a = audit_substrate("narrow_a", build_narrow_a_stream(), 500);
    let _oq2 = audit_substrate("OQ#2", build_oq2_stream(), 4500);

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Reading guide");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Each row in the per-substrate tables is one agent class:");
    println!(" a (kind, target-kind) pair that grouped the episodes.");
    println!(" The columns describe that class's complete observable");
    println!(" behaviour over the run:");
    println!();
    println!("   eps     — number of agent instances of this class");
    println!("             that fired (= number of dispatches)");
    println!("   succ    — agents that produced positive abstraction-");
    println!("             score delta (or returned explicit success)");
    println!("   succ%   — class-level success rate (\"agent confidence");
    println!("             of this class\", per ADR 0076)");
    println!("   first/last tick — temporal envelope of the class's");
    println!("             activity (\"agent attention window\")");
    println!("   mean Δ  — average score impact per dispatch");
    println!();
    println!(" The recent-window attention share approximates which");
    println!(" agent class is currently \"holding the floor\" in the");
    println!(" global workspace.");
    println!();
    println!(" Constitution heavy reading is preserved: nothing in");
    println!(" this output is new ontology — these are queries over");
    println!(" Memory::episodes that already exist. Agents are");
    println!(" behaviour patterns we read off the log, not entities");
    println!(" the system declares.");
    println!();
    println!("--- end ---");
}
