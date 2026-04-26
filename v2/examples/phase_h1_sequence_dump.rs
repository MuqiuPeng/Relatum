//! Phase H1.0 sequence-stats dump (ADR 0061).
//!
//! Runs each of the F0 battery seeds for HORIZON ticks, then prints
//! the per-seed `[sequence_stats]` content sorted by post-EP-delta
//! mean (descending). Useful for human inspection: which action
//! pairs correlate with positive prediction-error improvement, on
//! which substrates, with what magnitudes?
//!
//! This is the empirical input ADR 0061 calls for before committing
//! to H1.1 (priority bias) or H1.2 (composite dispatch). If the
//! per-seed dumps show no pair with both `count >= 10` and
//! `mean_post_ep_delta > 0.1`, the H1.1 promotion gate would never
//! fire on these seeds; H1.1 / H1.2 design assumptions need
//! revisiting.
//!
//! Captured to `logs/<date>_phase_h1_sequence_dump.log`.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, Event, ObjectHistory,
        RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet,
};

const HORIZON: u64 = 300;

fn run_seed(name: &'static str, mut rt: AutonomousRuntime) {
    rt.run_bounded(HORIZON);
    println!("=== seed: {} ===", name);
    println!(
        "tick={} lifecycle={:?} episodes={} theories={} patterns={}",
        rt.tick,
        rt.lifecycle,
        rt.memory.episodes.len(),
        rt.rset.theories().len(),
        rt.rset.patterns().len(),
    );
    let ss = &rt.memory.sequence_stats;
    let mut rows: Vec<(
        ActionKind,
        ActionKind,
        u64,
        u64,
        f64,
    )> = Vec::new();
    let mut all_pairs: std::collections::HashSet<(ActionKind, ActionKind)> =
        std::collections::HashSet::new();
    for k in ss.pair_counts.keys() {
        all_pairs.insert(*k);
    }
    for k in ss.pair_post_ep_count.keys() {
        all_pairs.insert(*k);
    }
    for (a, b) in all_pairs {
        let count = ss.pair_counts.get(&(a, b)).copied().unwrap_or(0);
        let post_count =
            ss.pair_post_ep_count.get(&(a, b)).copied().unwrap_or(0);
        let mean_delta = ss
            .pair_mean_post_ep_delta((a, b))
            .unwrap_or(0.0);
        rows.push((a, b, count, post_count, mean_delta));
    }
    rows.sort_by(|x, y| {
        y.4.partial_cmp(&x.4).unwrap_or(std::cmp::Ordering::Equal)
    });
    println!(
        "{:>22} {:>22} {:>6} {:>10} {:>14}",
        "from", "to", "count", "post_ep", "mean_post_ep"
    );
    if rows.is_empty() {
        println!("  (no pairs recorded)");
    }
    for (a, b, count, post_count, mean) in rows {
        println!(
            "{:>22} {:>22} {:>6} {:>10} {:>14.4}",
            format!("{:?}", a),
            format!("{:?}", b),
            count,
            post_count,
            mean,
        );
    }
    // H1.1 promotion gate preview, two threshold tiers:
    //   strict: count >= 10 AND mean > 0.1 (ADR 0061's defaults)
    //   relaxed: count >= 5 AND mean > 0.05 (informs threshold tuning)
    let report_promotable = |tag: &str, min_count: u64, min_mean: f64| {
        let pairs: Vec<_> = ss
            .pair_counts
            .iter()
            .filter_map(|(pair, c)| {
                if *c < min_count {
                    return None;
                }
                let mean = ss.pair_mean_post_ep_delta(*pair)?;
                if mean > min_mean {
                    Some((*pair, *c, mean))
                } else {
                    None
                }
            })
            .collect();
        println!(
            "promotable ({}: count>={} AND mean>{:.2}): {}",
            tag,
            min_count,
            min_mean,
            pairs.len()
        );
        for (pair, c, mean) in pairs {
            println!(
                "  ({:?}, {:?}) count={} mean={:.4}",
                pair.0, pair.1, c, mean
            );
        }
    };
    report_promotable("strict", 10, 0.1);
    report_promotable("relaxed", 5, 0.05);
    println!();
}

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
        rs.add(R::new(cluster[0], cluster[1]));
        rs.add(R::new(cluster[1], cluster[2]));
        rs.add(R::new(cluster[2], cluster[0]));
    }
    let mut rt = AutonomousRuntime::new(rs);
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn seed_stream_diamond() -> AutonomousRuntime {
    let mut schedule = Vec::new();
    let phases: [&[&str]; 6] = [
        &["a1", "a2", "a3", "a4"],
        &["b1", "b2", "b3", "b4"],
        &["c1", "c2", "c3", "c4"],
        &["d1", "d2", "d3", "d4"],
        &["e1", "e2", "e3", "e4"],
        &["f1", "f2", "f3", "f4"],
    ];
    let phase_offsets: [u64; 6] = [1, 50, 100, 150, 200, 250];
    for (idx, nodes) in phases.iter().enumerate() {
        let off = phase_offsets[idx];
        schedule.push((off, Event::AddEdge(R::new(nodes[0], nodes[0]))));
        schedule.push((off + 1, Event::AddEdge(R::new(nodes[1], nodes[1]))));
        schedule.push((off + 2, Event::AddEdge(R::new(nodes[2], nodes[2]))));
        schedule.push((off + 3, Event::AddEdge(R::new(nodes[3], nodes[3]))));
        schedule.push((off + 7, Event::AddEdge(R::new(nodes[0], nodes[1]))));
        schedule.push((off + 11, Event::AddEdge(R::new(nodes[0], nodes[2]))));
        schedule.push((off + 15, Event::AddEdge(R::new(nodes[0], nodes[3]))));
        schedule.push((off + 19, Event::AddEdge(R::new(nodes[1], nodes[3]))));
        schedule.push((off + 23, Event::AddEdge(R::new(nodes[2], nodes[3]))));
    }
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(schedule));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn main() {
    println!(
        "=== ADR 0061 H1.0 sequence-stats dump (HORIZON={}) ===",
        HORIZON
    );
    println!();
    run_seed("fan_only", seed_fan_only());
    run_seed("diamond_poset", seed_diamond_poset());
    run_seed("bipartite_2_3", seed_bipartite_2_3());
    run_seed("star_5", seed_star_5());
    run_seed("equivalence_3_classes", seed_equivalence_3_classes());
    run_seed("disconnected_islands", seed_disconnected_islands());
    run_seed("stream_diamond", seed_stream_diamond());
    println!("--- end ---");
}
