//! Phase D termination empirics (ADR 0054 § Open question #4).
//!
//! Question: if `ESTABLISHED → meta-meta-pattern → C0 promotion →
//! ESTABLISHED` is a real loop, does it terminate, oscillate, or
//! grow without bound?
//!
//! Construction: seed five ESTABLISHED-marked patterns (so the
//! D0 frontier gate fires immediately) plus a tiny disconnected
//! data substrate. NoOp environment — no external events; the
//! only growth comes from the runtime's own actions. Run 500
//! ticks. Snapshot key counters every 50 ticks. At the end,
//! report the trajectory and a verdict.
//!
//! Captured to `logs/<date>_phase_d_termination.log`.

use relatum_v2::{
    runtime::{ActionKind, AutonomousRuntime, RuleBasedScheduler},
    ESTABLISHED_MARKER, PATTERN_MARKER, SHARED_AXIOM_MARKER, R, RSet,
};

const HORIZON: u64 = 500;
const SNAPSHOT_EVERY: u64 = 50;

struct Snapshot {
    tick: u64,
    patterns: usize,
    theories: usize,
    established_edges: usize,
    shared_axiom_edges: usize,
    meta_meta_attempts: u64,
    meta_meta_hits: u64,
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
        meta_meta_attempts: stats
            .action_counts
            .get(&ActionKind::DiscoverMetaMetaPatterns)
            .copied()
            .unwrap_or(0),
        meta_meta_hits: stats
            .action_positive_delta_counts
            .get(&ActionKind::DiscoverMetaMetaPatterns)
            .copied()
            .unwrap_or(0),
        episodes: rt.memory.episodes.len(),
        lifecycle: format!("{:?}", rt.lifecycle),
    }
}

fn main() {
    println!(
        "=== ADR 0054 OQ #4 — termination empirics ({} ticks) ===",
        HORIZON
    );
    println!();

    let mut rs = RSet::new();
    for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
        rs.add(R::new(PATTERN_MARKER, *p));
        rs.add(R::new(*p, ESTABLISHED_MARKER));
    }
    // Tiny disconnected data so the rset isn't pure-meta — gives
    // refresh_stale_prune / the existing prune lane something to
    // consider but no theory to discover.
    rs.add(R::new("u", "v"));

    let mut rt = AutonomousRuntime::new(rs);
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    // Seed an ObjectHistory entry per pattern so C0 has a starting
    // age signal; otherwise first_seen_tick = 0 only fires when the
    // runtime's own dispatch loop touches the pattern, which won't
    // happen for these synthetic registry-only patterns.
    for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
        let h = relatum_v2::runtime::ObjectHistory::new_at(0);
        rt.memory.object_history.patterns.insert(p.to_string(), h);
    }

    let mut snapshots: Vec<Snapshot> = Vec::new();
    snapshots.push(snapshot(&rt));

    let mut consecutive_idle: u32 = 0;
    let mut last_pattern_count = rt.rset.patterns().len();

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

    // Report
    println!(
        "{:>6} {:>9} {:>9} {:>10} {:>10} {:>9} {:>7} {:>9} {}",
        "tick", "patterns", "theories", "estab.edges", "shared.ax",
        "mm.tries", "mm.hits", "episodes", "lifecycle",
    );
    for s in &snapshots {
        println!(
            "{:>6} {:>9} {:>9} {:>10} {:>10} {:>9} {:>7} {:>9} {}",
            s.tick,
            s.patterns,
            s.theories,
            s.established_edges,
            s.shared_axiom_edges,
            s.meta_meta_attempts,
            s.meta_meta_hits,
            s.episodes,
            s.lifecycle,
        );
    }
    println!();

    // Verdict
    let final_state = snapshots.last().unwrap();
    let verdict = if final_state.lifecycle == "Sleeping"
        && consecutive_idle >= 2
    {
        "CONVERGED — runtime is sleeping with no new patterns named \
         in the last 2 windows"
    } else if consecutive_idle >= 2 {
        "QUIESCED — no new patterns in the last 2 windows but \
         lifecycle != Sleeping"
    } else {
        "STILL GROWING — patterns added in the most recent window"
    };
    println!("verdict: {}", verdict);
    println!(
        "final pattern count = {}, final episode count = {}, \
         meta-meta attempts = {}, hits = {}",
        final_state.patterns,
        final_state.episodes,
        final_state.meta_meta_attempts,
        final_state.meta_meta_hits,
    );
    println!();
    println!("--- end ---");
}
