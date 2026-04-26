//! Phase D verification battery (ADR 0056 § Phase F0).
//!
//! Six seeds × HORIZON ticks each, against `RuleBasedScheduler::default()`.
//! Per-seed trajectory snapshots every SNAPSHOT_EVERY ticks plus a
//! verdict (CONVERGED / STILL GROWING / ANOMALOUS). Battery summary
//! at the end.
//!
//! Captured to `logs/<date>_phase_d_battery.log`, analogous to
//! `phase_a_verification.log`.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, ObjectHistory, RuleBasedScheduler,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, SHARED_AXIOM_MARKER, R, RSet,
};

const HORIZON: u64 = 300;
const SNAPSHOT_EVERY: u64 = 50;

struct Snapshot {
    tick: u64,
    patterns: usize,
    theories: usize,
    established_edges: usize,
    shared_axiom_edges: usize,
    mm_attempts: u64,
    mm_hits: u64,
    episodes: usize,
    lifecycle: String,
}

fn snapshot(rt: &AutonomousRuntime) -> Snapshot {
    let stats = &rt.memory.policy_stats;
    Snapshot {
        tick: rt.tick,
        patterns: rt.rset.patterns().len(),
        theories: rt.rset.theories().len(),
        established_edges: rt.rset.right_of(ESTABLISHED_MARKER).len(),
        shared_axiom_edges: rt.rset.right_of(SHARED_AXIOM_MARKER).len(),
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
        episodes: rt.memory.episodes.len(),
        lifecycle: format!("{:?}", rt.lifecycle),
    }
}

struct SeedResult {
    name: &'static str,
    snapshots: Vec<Snapshot>,
    verdict: &'static str,
    new_patterns: usize,
}

fn run_seed(name: &'static str, mut rt: AutonomousRuntime) -> SeedResult {
    let initial = snapshot(&rt);
    let mut snapshots: Vec<Snapshot> = vec![initial];
    let mut last_pattern_count = snapshots[0].patterns;
    let mut consecutive_idle: u32 = 0;

    while rt.tick < HORIZON {
        rt.run_bounded(SNAPSHOT_EVERY);
        let s = snapshot(&rt);
        if s.patterns == last_pattern_count {
            consecutive_idle += 1;
        } else {
            consecutive_idle = 0;
        }
        last_pattern_count = s.patterns;
        snapshots.push(s);
    }

    let final_state = snapshots.last().unwrap();
    let theory_or_mm =
        final_state.theories > 0 || final_state.mm_attempts > 0;
    let verdict: &'static str = if final_state.lifecycle == "Sleeping"
        && consecutive_idle >= 2
    {
        "CONVERGED"
    } else if consecutive_idle >= 2 {
        "QUIESCED"
    } else if !theory_or_mm
        && final_state.patterns == snapshots[0].patterns
    {
        "ANOMALOUS"
    } else {
        "STILL GROWING"
    };
    let new_patterns =
        final_state.patterns.saturating_sub(snapshots[0].patterns);
    SeedResult { name, snapshots, verdict, new_patterns }
}

fn print_seed(result: &SeedResult) {
    println!("=== seed: {} ===", result.name);
    println!(
        "{:>6} {:>9} {:>9} {:>11} {:>9} {:>9} {:>7} {:>9} {}",
        "tick", "patterns", "theories", "established", "shared.ax",
        "mm.tries", "mm.hits", "episodes", "lifecycle",
    );
    for s in &result.snapshots {
        println!(
            "{:>6} {:>9} {:>9} {:>11} {:>9} {:>9} {:>7} {:>9} {}",
            s.tick,
            s.patterns,
            s.theories,
            s.established_edges,
            s.shared_axiom_edges,
            s.mm_attempts,
            s.mm_hits,
            s.episodes,
            s.lifecycle,
        );
    }
    println!(
        "verdict: {} (new patterns named: {})",
        result.verdict, result.new_patterns
    );
    println!();
}

// ─── Seed builders ──────────────────────────────────────────────

fn seed_fan_only() -> AutonomousRuntime {
    let mut rs = RSet::new();
    for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
        rs.add(R::new(PATTERN_MARKER, *p));
        rs.add(R::new(*p, ESTABLISHED_MARKER));
    }
    rs.add(R::new("u", "v"));
    let mut rt = AutonomousRuntime::new(rs);
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
        rt.memory
            .object_history
            .patterns
            .insert(p.to_string(), ObjectHistory::new_at(0));
    }
    rt
}

fn seed_diamond_poset() -> AutonomousRuntime {
    let mut rs = RSet::new();
    for n in &["a", "b", "c", "d"] {
        rs.add(R::new(*n, *n));
    }
    rs.extend([
        R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
        R::new("b", "d"), R::new("c", "d"),
    ]);
    let mut rt = AutonomousRuntime::new(rs);
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn seed_bipartite_2_3() -> AutonomousRuntime {
    let mut rs = RSet::new();
    for l in &["l1", "l2"] {
        for r in &["r1", "r2", "r3"] {
            rs.add(R::new(*l, *r));
        }
    }
    let mut rt = AutonomousRuntime::new(rs);
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn seed_star_5() -> AutonomousRuntime {
    let mut rs = RSet::new();
    for leaf in &["x1", "x2", "x3", "x4", "x5"] {
        rs.add(R::new("center", *leaf));
    }
    let mut rt = AutonomousRuntime::new(rs);
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn seed_equivalence_3_classes() -> AutonomousRuntime {
    let mut rs = RSet::new();
    let classes: &[&[&str]] = &[&["a", "b"], &["c", "d", "e"], &["f"]];
    for cls in classes {
        for x in cls.iter() {
            for y in cls.iter() {
                rs.add(R::new(*x, *y));
            }
        }
    }
    let mut rt = AutonomousRuntime::new(rs);
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn seed_disconnected_islands() -> AutonomousRuntime {
    let mut rs = RSet::new();
    for cluster in &[
        ["a1", "a2", "a3"],
        ["b1", "b2", "b3"],
        ["c1", "c2", "c3"],
    ] {
        // 3-cycle in each cluster.
        rs.add(R::new(cluster[0], cluster[1]));
        rs.add(R::new(cluster[1], cluster[2]));
        rs.add(R::new(cluster[2], cluster[0]));
    }
    let mut rt = AutonomousRuntime::new(rs);
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn main() {
    println!(
        "=== ADR 0056 Phase F0 — D-battery (HORIZON={}) ===",
        HORIZON
    );
    println!();

    let seeds: Vec<(&'static str, fn() -> AutonomousRuntime)> = vec![
        ("fan_only", seed_fan_only),
        ("diamond_poset", seed_diamond_poset),
        ("bipartite_2_3", seed_bipartite_2_3),
        ("star_5", seed_star_5),
        ("equivalence_3_classes", seed_equivalence_3_classes),
        ("disconnected_islands", seed_disconnected_islands),
    ];

    let mut results: Vec<SeedResult> = Vec::new();
    for (name, builder) in &seeds {
        let rt = builder();
        let result = run_seed(name, rt);
        print_seed(&result);
        results.push(result);
    }

    println!("=== battery summary ===");
    println!(
        "{:>22} {:>15} {:>14}",
        "seed", "verdict", "new patterns"
    );
    for r in &results {
        println!("{:>22} {:>15} {:>14}", r.name, r.verdict, r.new_patterns);
    }
    println!();
    println!("--- end ---");
}
