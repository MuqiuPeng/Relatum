//! Phase Alpha-1 — UCB1 composite selection A/B comparison
//! (ADR 0065).
//!
//! Two 2000-tick runs over the same multi-regime substrate
//! (the OQ #1 streaming env). Run A uses the baseline
//! `RuleBasedScheduler`. Run B wraps the same scheduler in
//! `UcbCompositeScheduler` so composite-candidate selection
//! uses UCB1 instead of greedy priority. Captures and prints:
//!
//! - episode count, EP attempts, composite attempts
//! - pairs / triples named at end
//! - final candidate weights (DriveMix state)
//! - per-snapshot diff between runs
//!
//! Captured to `logs/<date>_phase_alpha_composite_ucb.log`.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, RuleBasedScheduler,
        Scheduler, SyntheticStreamEnvironment, UcbCompositeScheduler,
    }, RSet,
};

const HORIZON: u64 = 2000;
const SNAPSHOT_EVERY: u64 = 200;

#[derive(Clone, Default)]
struct Snap {
    tick: u64,
    episodes: usize,
    ep_attempts: u64,
    composite_attempts: u64,
    pairs_named: usize,
    triples_named: usize,
}

fn snap(rt: &AutonomousRuntime) -> Snap {
    Snap {
        tick: rt.tick,
        episodes: rt.memory.episodes.len(),
        ep_attempts: rt
            .memory
            .policy_stats
            .action_counts
            .get(&ActionKind::EvaluatePredictions)
            .copied()
            .unwrap_or(0),
        composite_attempts: rt
            .memory
            .policy_stats
            .action_counts
            .get(&ActionKind::ExecuteComposite)
            .copied()
            .unwrap_or(0),
        pairs_named: rt.rset.action_sequence_pairs().len(),
        triples_named: rt.rset.action_sequence_triples().len(),
    }
}
use relatum_v2::test_substrates::oq1::build_long_stream;

fn build_baseline_runtime() -> AutonomousRuntime {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment =
        Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn build_ucb_runtime() -> AutonomousRuntime {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment =
        Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    let inner: Box<dyn Scheduler> =
        Box::new(RuleBasedScheduler::default());
    rt.scheduler = Box::new(UcbCompositeScheduler::new(inner));
    rt
}

fn run_with(label: &str, mut rt: AutonomousRuntime) -> Vec<Snap> {
    println!();
    println!("=== run: {} ===", label);
    println!(
        "{:>5} {:>6} {:>6} {:>5} {:>6} {:>4}",
        "tick", "epis", "ep", "comp", "pairs", "tri",
    );
    let mut snaps = vec![snap(&rt)];
    print_snap(&snaps[0]);
    while rt.tick < HORIZON {
        rt.run_bounded(SNAPSHOT_EVERY);
        snaps.push(snap(&rt));
        print_snap(snaps.last().unwrap());
    }
    snaps
}

fn print_snap(s: &Snap) {
    println!(
        "{:>5} {:>6} {:>6} {:>5} {:>6} {:>4}",
        s.tick,
        s.episodes,
        s.ep_attempts,
        s.composite_attempts,
        s.pairs_named,
        s.triples_named,
    );
}

fn main() {
    println!(
        "=== ADR 0065 Phase Alpha-1 — UCB1 composite selection A/B (HORIZON={}) ===",
        HORIZON
    );

    let baseline = run_with("baseline (greedy)", build_baseline_runtime());
    let ucb = run_with("ucb1", build_ucb_runtime());

    println!();
    println!("=== final-snapshot comparison ===");
    let b = baseline.last().unwrap();
    let u = ucb.last().unwrap();
    let row = |label: &str, b: u64, u: u64| {
        let delta = u as i64 - b as i64;
        let sign = if delta == 0 {
            "="
        } else if delta > 0 {
            "+"
        } else {
            "-"
        };
        println!(
            "{:>20}  baseline={:>5}  ucb1={:>5}  Δ={}{}",
            label,
            b,
            u,
            sign,
            delta.abs()
        );
    };
    row("episodes", b.episodes as u64, u.episodes as u64);
    row("ep_attempts", b.ep_attempts, u.ep_attempts);
    row("composite_attempts", b.composite_attempts, u.composite_attempts);
    row("pairs_named", b.pairs_named as u64, u.pairs_named as u64);
    row("triples_named", b.triples_named as u64, u.triples_named as u64);

    println!();
    println!("=== per-snapshot trajectory delta (ucb1 - baseline) ===");
    println!(
        "{:>5} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "tick", "Δepis", "Δep", "Δcomp", "Δpairs", "Δtri",
    );
    for i in 0..baseline.len().min(ucb.len()) {
        let bs = &baseline[i];
        let us = &ucb[i];
        println!(
            "{:>5} {:>8} {:>8} {:>8} {:>8} {:>8}",
            bs.tick,
            us.episodes as i64 - bs.episodes as i64,
            us.ep_attempts as i64 - bs.ep_attempts as i64,
            us.composite_attempts as i64 - bs.composite_attempts as i64,
            us.pairs_named as i64 - bs.pairs_named as i64,
            us.triples_named as i64 - bs.triples_named as i64,
        );
    }

    println!();
    println!("--- end ---");
}
