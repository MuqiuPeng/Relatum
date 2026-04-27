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
        ActionKind, AutonomousRuntime, Event, RuleBasedScheduler,
        Scheduler, SyntheticStreamEnvironment, UcbCompositeScheduler,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet,
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

fn build_long_stream() -> Vec<(u64, Event)> {
    let mut schedule = Vec::new();

    let regime_a_phases: [&[&str]; 5] = [
        &["a1", "a2", "a3", "a4"],
        &["a5", "a6", "a7", "a8"],
        &["a9", "a10", "a11", "a12"],
        &["a13", "a14", "a15", "a16"],
        &["a17", "a18", "a19", "a20"],
    ];
    for (i, ns) in regime_a_phases.iter().enumerate() {
        let off = 1 + (i as u64) * 100;
        for k in 0..4 {
            schedule.push((off + k as u64, Event::AddEdge(R::new(ns[k], ns[k]))));
        }
        schedule.push((off + 7, Event::AddEdge(R::new(ns[0], ns[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(ns[0], ns[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(ns[0], ns[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(ns[1], ns[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(ns[2], ns[3]))));
    }

    let regime_b_phases: [(&[&str], &[&str]); 5] = [
        (&["bL1", "bL2"], &["bR1", "bR2", "bR3"]),
        (&["bL3", "bL4"], &["bR4", "bR5", "bR6"]),
        (&["bL5", "bL6"], &["bR7", "bR8", "bR9"]),
        (&["bL7", "bL8"], &["bR10", "bR11", "bR12"]),
        (&["bL9", "bL10"], &["bR13", "bR14", "bR15"]),
    ];
    for (i, (lefts, rights)) in regime_b_phases.iter().enumerate() {
        let off = 501 + (i as u64) * 100;
        let mut t = 0u64;
        for l in lefts.iter() {
            for r in rights.iter() {
                schedule.push((off + t, Event::AddEdge(R::new(*l, *r))));
                t += 2;
            }
        }
    }

    let regime_c_phases: [&[&[&str]]; 5] = [
        &[&["c_a1", "c_a2"], &["c_b1", "c_b2", "c_b3"]],
        &[&["c_a3", "c_a4"], &["c_b4", "c_b5", "c_b6"]],
        &[&["c_a5", "c_a6"], &["c_b7", "c_b8", "c_b9"]],
        &[&["c_a7", "c_a8"], &["c_b10", "c_b11", "c_b12"]],
        &[&["c_a9", "c_a10"], &["c_b13", "c_b14", "c_b15"]],
    ];
    for (i, classes) in regime_c_phases.iter().enumerate() {
        let off = 1001 + (i as u64) * 100;
        let mut t = 0u64;
        for cls in classes.iter() {
            for x in cls.iter() {
                for y in cls.iter() {
                    schedule.push((off + t, Event::AddEdge(R::new(*x, *y))));
                    t += 1;
                }
            }
        }
    }

    let regime_d_phases: [&[&str]; 5] = [
        &["d1", "d2", "d3", "d4"],
        &["d5", "d6", "d7", "d8"],
        &["d9", "d10", "d11", "d12"],
        &["d13", "d14", "d15", "d16"],
        &["d17", "d18", "d19", "d20"],
    ];
    for (i, ns) in regime_d_phases.iter().enumerate() {
        let off = 1501 + (i as u64) * 100;
        for k in 0..4 {
            schedule.push((off + k as u64, Event::AddEdge(R::new(ns[k], ns[k]))));
        }
        schedule.push((off + 5, Event::AddEdge(R::new(PATTERN_MARKER, ns[0]))));
        schedule.push((off + 6, Event::AddEdge(R::new(ns[0], ESTABLISHED_MARKER))));
        schedule.push((off + 7, Event::AddEdge(R::new(ns[0], ns[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(ns[0], ns[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(ns[0], ns[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(ns[1], ns[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(ns[2], ns[3]))));
    }

    schedule
}

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
