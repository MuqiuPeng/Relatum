//! ADR 0076 phase 2 — episode-log enrichments audit.
//!
//! Builds on phase 1's agent_audit by adding the three richer
//! query helpers:
//!   - outcome distribution per agent class (delta histogram)
//!   - temporal density per agent class (when did it fire?)
//!   - target overlap per action kind (which specific instances?)
//!
//! No new lib mechanism beyond `agent_view`. This example wires
//! the helpers into a single multi-perspective audit on OQ#1
//! and OQ#2 (the two empirically distinct substrates per the
//! diversity probe).

use relatum_v2::{
    runtime::{
        agent_classes, agent_outcome_distribution, agent_target_overlap,
        agent_temporal_density, ActionKind, AutonomousRuntime, Event,
        RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::{oq1::build_long_stream, oq2::build_oq2_stream},
    RSet,
};

fn run_substrate_with_phase2_audit(
    label: &str,
    stream: Vec<(u64, Event)>,
    ticks: u64,
    n_temporal_windows: usize,
) {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: {} ({} ticks, {} temporal windows)",
             label, ticks, n_temporal_windows);
    println!("════════════════════════════════════════════════════════");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    let classes = agent_classes(&rt.memory.episodes);

    // Per-class outcome distribution.
    println!();
    println!(" === Outcome distribution per agent class ===");
    println!(
        " {:<32} {:>6} {:>4} {:>4} {:>4} {:>8} {:>8} {:>8}",
        "agent class", "eps", "neg", "zer", "pos",
        "min Δ", "med Δ", "max Δ",
    );
    for c in &classes {
        let dist = agent_outcome_distribution(
            &rt.memory.episodes, c.action_kind, &c.target_label,
        );
        let class_str = format!("{:?}/{}", c.action_kind, c.target_label);
        let class_short = if class_str.len() > 32 {
            format!("{}…", &class_str[..31])
        } else {
            class_str
        };
        println!(
            " {:<32} {:>6} {:>4} {:>4} {:>4} {:>8.3} {:>8.3} {:>8.3}",
            class_short,
            dist.episode_count,
            dist.negative_count,
            dist.zero_count,
            dist.positive_count,
            dist.min_delta,
            dist.median_delta,
            dist.max_delta,
        );
    }

    // Per-class temporal density (peak window only, for compactness).
    println!();
    println!(" === Temporal density (peak window per class) ===");
    println!(
        " {:<32} {:>6} {:>14} {:>10}",
        "agent class", "total", "peak window", "peak count",
    );
    for c in &classes {
        let dens = agent_temporal_density(
            &rt.memory.episodes,
            c.action_kind,
            &c.target_label,
            n_temporal_windows,
            ticks,
        );
        let class_str = format!("{:?}/{}", c.action_kind, c.target_label);
        let class_short = if class_str.len() > 32 {
            format!("{}…", &class_str[..31])
        } else {
            class_str
        };
        match dens.peak_window_idx {
            Some(idx) => {
                let (start, end, count) = dens.windows[idx];
                println!(
                    " {:<32} {:>6} {:>5}-{:<8} {:>10}",
                    class_short,
                    dens.total_episodes,
                    start, end,
                    count,
                );
            }
            None => {
                println!(
                    " {:<32} {:>6} {:>14} {:>10}",
                    class_short, 0, "—", "—",
                );
            }
        }
    }

    // Target overlap for kinds that carry an id.
    println!();
    println!(" === Target overlap (kinds with id-bearing targets) ===");
    let id_bearing_kinds = [
        ActionKind::PruneLowValueObjects,
        ActionKind::Declarativize,
        ActionKind::ExecuteComposite,
        ActionKind::RetractShapeFamily,
    ];
    for kind in &id_bearing_kinds {
        let overlap = agent_target_overlap(&rt.memory.episodes, *kind);
        if overlap.total_episodes == 0 {
            continue;
        }
        println!();
        println!(
            "   {:?}: {} episodes over {} distinct targets",
            kind, overlap.total_episodes, overlap.distinct_targets,
        );
        println!("     modal: {}", overlap.modal_target.as_deref().unwrap_or("—"));
        println!("     top entries (descending count):");
        for (target, count) in overlap.target_counts.iter().take(5) {
            println!("       {:>4} × {}", count, target);
        }
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" ADR 0076 phase 2 — Episode-log enrichments audit");
    println!("════════════════════════════════════════════════════════");
    println!(" Three richer queries over the same Memory::episodes:");
    println!("   - Outcome distribution: per-class delta histogram");
    println!("   - Temporal density: when did each class fire?");
    println!("   - Target overlap: which specific instances did the");
    println!("     class act on (for id-bearing target kinds)?");
    println!();
    println!(" All read-only; no new state. Continues path C of");
    println!(" ADR 0076 — no agent token registered, no agent attribute");
    println!(" named.");

    run_substrate_with_phase2_audit(
        "OQ#1", build_long_stream(), 1000, 5,
    );
    run_substrate_with_phase2_audit(
        "OQ#2", build_oq2_stream(), 4500, 5,
    );

    println!();
    println!("--- end ---");
}
