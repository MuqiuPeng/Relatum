//! Phase H1 long-run empirics (ADR 0062 retrospective #1).
//!
//! HORIZON=2000 against a richer multi-regime streaming
//! environment (4 regimes × 500 ticks each: diamond posets →
//! bipartite injections → equivalence classes → mixed). The
//! goal is empirical: at scale, are promoted sequences stable?
//! Does demotion fire on regime shift? Does the meta-meta loop
//! produce qualitatively different behaviour vs. the 300-tick
//! F0 battery?
//!
//! Snapshots every 200 ticks. Per-snapshot diff against the
//! prior named-pair / named-triple sets surfaces promotions /
//! demotions as discrete events.
//!
//! Captured to `logs/<date>_phase_h1_long_run.log`.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, DriveABState,
        RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    ESTABLISHED_MARKER, SHARED_AXIOM_MARKER, RSet,
};
use std::collections::HashSet;

const HORIZON: u64 = 2000;
const SNAPSHOT_EVERY: u64 = 200;

#[derive(Clone)]
struct Snapshot {
    tick: u64,
    patterns: usize,
    theories: usize,
    established_edges: usize,
    shared_axiom_edges: usize,
    episodes: usize,
    mm_attempts: u64,
    mm_hits: u64,
    ep_attempts: u64,
    composite_attempts: u64,
    named_pairs: HashSet<(String, String)>,
    named_triples: HashSet<(String, String, String)>,
    lifecycle: String,
    /// ADR 0063 / H2.0 step 2 — DriveMix observability. Active
    /// candidate's weights for the three baseline drives.
    drive_state: String,
    drive_w_compression: f64,
    drive_w_prediction_error: f64,
    drive_w_mode_thrash: f64,
    /// ADR 0063 / H2.0 step 3a — blended drive signal at this tick.
    drive_combined_signal: f64,
}

fn snapshot(rt: &AutonomousRuntime) -> Snapshot {
    let stats = &rt.memory.policy_stats;
    let named_pairs: HashSet<(String, String)> = rt
        .rset
        .action_sequence_pairs()
        .into_iter()
        .map(|(_, p, s)| (p, s))
        .collect();
    let named_triples: HashSet<(String, String, String)> = rt
        .rset
        .action_sequence_triples()
        .into_iter()
        .map(|(_, a, b, c)| (a, b, c))
        .collect();
    Snapshot {
        tick: rt.tick,
        patterns: rt.rset.patterns().len(),
        theories: rt.rset.theories().len(),
        established_edges: rt.rset.right_of(ESTABLISHED_MARKER).len(),
        shared_axiom_edges: rt.rset.right_of(SHARED_AXIOM_MARKER).len(),
        episodes: rt.memory.episodes.len(),
        mm_attempts: stats
            .action_counts
            .get(&ActionKind::DiscoverMetaMetaPatterns)
            .copied()
            .unwrap_or(0),
        mm_hits: stats
            .action_positive_delta_counts
            .get(&ActionKind::DiscoverMetaMetaPatterns)
            .copied()
            .unwrap_or(0),
        ep_attempts: stats
            .action_counts
            .get(&ActionKind::EvaluatePredictions)
            .copied()
            .unwrap_or(0),
        composite_attempts: stats
            .action_counts
            .get(&ActionKind::ExecuteComposite)
            .copied()
            .unwrap_or(0),
        named_pairs,
        named_triples,
        lifecycle: format!("{:?}", rt.lifecycle),
        drive_state: match rt.drive_mix.state {
            DriveABState::TestingA => "A".to_string(),
            DriveABState::TestingB => "B".to_string(),
        },
        drive_w_compression: rt
            .drive_mix
            .active_weights()
            .get("compression")
            .copied()
            .unwrap_or(0.0),
        drive_w_prediction_error: rt
            .drive_mix
            .active_weights()
            .get("prediction_error")
            .copied()
            .unwrap_or(0.0),
        drive_w_mode_thrash: rt
            .drive_mix
            .active_weights()
            .get("mode_thrash")
            .copied()
            .unwrap_or(0.0),
        drive_combined_signal: rt.combined_drive_signal(),
    }
}

fn diff_strs<T: std::fmt::Debug>(items: &[T]) -> String {
    if items.is_empty() {
        return "none".to_string();
    }
    items
        .iter()
        .map(|i| format!("{:?}", i))
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_snapshot_row(s: &Snapshot) {
    println!(
        "{:>5} {:>4} {:>4} {:>5} {:>5} {:>6} {:>6} {:>5} {:>6} {:>5} {:>6} {:>4} {:>3} {:>5.2} {:>5.2} {:>5.2} {:>6.3} {}",
        s.tick,
        s.patterns,
        s.theories,
        s.established_edges,
        s.shared_axiom_edges,
        s.episodes,
        s.ep_attempts,
        s.mm_attempts,
        s.mm_hits,
        s.composite_attempts,
        s.named_pairs.len(),
        s.named_triples.len(),
        s.drive_state,
        s.drive_w_compression,
        s.drive_w_prediction_error,
        s.drive_w_mode_thrash,
        s.drive_combined_signal,
        s.lifecycle,
    );
}

fn print_diff_against(prev: &Snapshot, curr: &Snapshot) {
    let promoted_pairs: Vec<_> = curr
        .named_pairs
        .difference(&prev.named_pairs)
        .cloned()
        .collect();
    let demoted_pairs: Vec<_> = prev
        .named_pairs
        .difference(&curr.named_pairs)
        .cloned()
        .collect();
    let promoted_triples: Vec<_> = curr
        .named_triples
        .difference(&prev.named_triples)
        .cloned()
        .collect();
    let demoted_triples: Vec<_> = prev
        .named_triples
        .difference(&curr.named_triples)
        .cloned()
        .collect();
    if !promoted_pairs.is_empty() {
        println!(
            "  +pair: {}",
            diff_strs(&promoted_pairs)
        );
    }
    if !demoted_pairs.is_empty() {
        println!("  -pair: {}", diff_strs(&demoted_pairs));
    }
    if !promoted_triples.is_empty() {
        println!(
            "  +triple: {}",
            diff_strs(&promoted_triples)
        );
    }
    if !demoted_triples.is_empty() {
        println!(
            "  -triple: {}",
            diff_strs(&demoted_triples)
        );
    }
}

// ─── Multi-regime streaming environment ──────────────────────────

/// 4 regimes × 500 ticks. Each regime injects a different
/// substrate family at staggered offsets. The runtime sees a
/// non-stationary stream — promoted sequences from regime N
/// should either stay productive into regime N+1 (stable) or
/// degrade enough that H1.3 demotes them (regime-sensitive).
use relatum_v2::test_substrates::oq1::build_long_stream;

fn build_runtime() -> AutonomousRuntime {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment =
        Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn main() {
    println!(
        "=== ADR 0062 retrospective #1 — long-run empirics (HORIZON={}, multi-regime) ===",
        HORIZON
    );
    println!();
    println!(
        "Regimes: A=diamonds (1–490) | B=bipartite (501–990) | C=cliques (1001–1490) | D=diamonds+patterns (1501–1990)"
    );
    println!();

    let mut rt = build_runtime();
    let initial = snapshot(&rt);
    println!(
        "{:>5} {:>4} {:>4} {:>5} {:>5} {:>6} {:>6} {:>5} {:>6} {:>5} {:>6} {:>4} {:>3} {:>5} {:>5} {:>5} {:>6} {}",
        "tick",
        "pat",
        "thy",
        "est",
        "shAx",
        "epis",
        "ep",
        "mmA",
        "mmH",
        "comp",
        "pairs",
        "tri",
        "ab",
        "wC",
        "wPE",
        "wMT",
        "sig",
        "lifecycle",
    );
    print_snapshot_row(&initial);

    let mut history: Vec<Snapshot> = vec![initial];
    while rt.tick < HORIZON {
        rt.run_bounded(SNAPSHOT_EVERY);
        let s = snapshot(&rt);
        let prev = history.last().unwrap().clone();
        print_snapshot_row(&s);
        print_diff_against(&prev, &s);
        history.push(s);
    }

    println!();
    let final_s = history.last().unwrap();
    let initial_s = &history[0];

    // Stability: of all pairs ever named, how many ended named.
    let mut ever_named_pairs: HashSet<(String, String)> = HashSet::new();
    let mut ever_named_triples: HashSet<(String, String, String)> =
        HashSet::new();
    for h in &history {
        for p in &h.named_pairs {
            ever_named_pairs.insert(p.clone());
        }
        for t in &h.named_triples {
            ever_named_triples.insert(t.clone());
        }
    }

    let pair_promotions = ever_named_pairs.len();
    let triple_promotions = ever_named_triples.len();
    let pair_demotions = pair_promotions - final_s.named_pairs.len();
    let triple_demotions =
        triple_promotions - final_s.named_triples.len();

    println!("=== summary ===");
    println!(
        "horizon: {} ticks, {} snapshots",
        HORIZON,
        history.len() - 1
    );
    println!(
        "patterns: {} → {} (Δ {})",
        initial_s.patterns,
        final_s.patterns,
        final_s.patterns as i64 - initial_s.patterns as i64,
    );
    println!(
        "theories: {} → {} (Δ {})",
        initial_s.theories,
        final_s.theories,
        final_s.theories as i64 - initial_s.theories as i64,
    );
    println!(
        "episodes: {} → {} (Δ {})",
        initial_s.episodes,
        final_s.episodes,
        final_s.episodes as i64 - initial_s.episodes as i64,
    );
    println!(
        "EP attempts: {}, composite attempts: {}",
        final_s.ep_attempts, final_s.composite_attempts,
    );
    println!(
        "MM attempts: {}, MM hits: {}",
        final_s.mm_attempts, final_s.mm_hits,
    );
    println!(
        "pair promotions ever: {} | currently named: {} | demotions: {}",
        pair_promotions,
        final_s.named_pairs.len(),
        pair_demotions,
    );
    println!(
        "triple promotions ever: {} | currently named: {} | demotions: {}",
        triple_promotions,
        final_s.named_triples.len(),
        triple_demotions,
    );
    println!();
    println!("currently-named pairs:");
    if final_s.named_pairs.is_empty() {
        println!("  (none)");
    } else {
        for p in &final_s.named_pairs {
            println!("  ({:?}, {:?})", p.0, p.1);
        }
    }
    println!("currently-named triples:");
    if final_s.named_triples.is_empty() {
        println!("  (none)");
    } else {
        for t in &final_s.named_triples {
            println!("  ({:?}, {:?}, {:?})", t.0, t.1, t.2);
        }
    }
    println!();
    println!("DriveMix final state (ADR 0063 / H2.0 step 2):");
    println!(
        "  state: {} | window_size: {} | stage_start_episode_count: {} | rng_state: {}",
        final_s.drive_state,
        rt.drive_mix.window_size,
        rt.drive_mix.stage_start_episode_count,
        rt.drive_mix.rng_state,
    );
    println!("  candidate_a:");
    let mut a_keys: Vec<&String> =
        rt.drive_mix.candidate_a.keys().collect();
    a_keys.sort();
    for k in a_keys {
        let v = rt.drive_mix.candidate_a.get(k).copied().unwrap_or(0.0);
        println!("    {}: {:.3}", k, v);
    }
    println!("  candidate_b:");
    let mut b_keys: Vec<&String> =
        rt.drive_mix.candidate_b.keys().collect();
    b_keys.sort();
    for k in b_keys {
        let v = rt.drive_mix.candidate_b.get(k).copied().unwrap_or(0.0);
        println!("    {}: {:.3}", k, v);
    }
    println!();
    println!("--- end ---");
}
