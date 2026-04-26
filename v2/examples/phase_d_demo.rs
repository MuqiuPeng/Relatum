//! Phase D0+ end-to-end demonstration (ADR 0054 § Verification plan).
//!
//! Constructs a small rset with five named patterns, each carrying
//! the ESTABLISHED_MARKER edge, plus a tiny disconnected data
//! substrate so the M1 subgraph dominates discovery. Then runs the
//! autonomous runtime long enough to see the meta-meta-pattern
//! discovery action fire and name a new pattern from the M1 shape.
//!
//! Captured to `logs/<date>_phase_d_demo.log`.
//!
//! What this proves: the loop closure described in ADR 0054 — M1
//! markers feed `discover_motifs_with_meta_subset`, the resulting
//! candidate is named via Intensional pattern recording, and a
//! concrete new `p_*` id appears in `rset.patterns()` whose
//! canonical is anchored to ESTABLISHED_MARKER.

use relatum_v2::{
    runtime::{ActionKind, AutonomousRuntime, RuleBasedScheduler},
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet,
};

fn main() {
    println!("=== ADR 0054 Phase-D0+ loop-closure demo ===");
    println!();

    // Pre-seed: 5 patterns, each ESTABLISHED. PATTERN_MARKER edges
    // anchor them in `rset.patterns()`; ESTABLISHED_MARKER edges
    // anchor them as M1 subjects.
    let mut rs = RSet::new();
    for p in &["p_a", "p_b", "p_c", "p_d", "p_e"] {
        rs.add(R::new(PATTERN_MARKER, *p));
        rs.add(R::new(*p, ESTABLISHED_MARKER));
    }
    // Disconnected data substrate — enough that the runtime doesn't
    // mistake the rset for empty, but small enough that the M1 cluster
    // dominates the meta-meta sampling pass.
    rs.add(R::new("u", "v"));

    let pre_patterns: Vec<String> =
        rs.patterns().iter().map(|s| s.to_string()).collect();
    let pre_established: Vec<String> = rs
        .right_of(ESTABLISHED_MARKER)
        .iter()
        .map(|r| r.x.clone())
        .collect();

    println!("--- pre-run state ---");
    println!("named patterns: {} {:?}", pre_patterns.len(), pre_patterns);
    println!(
        "ESTABLISHED edges: {} (subjects: {:?})",
        pre_established.len(),
        pre_established
    );
    println!();

    let mut rt = AutonomousRuntime::new(rs);
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(8);

    let post_patterns: Vec<String> = rt
        .rset
        .patterns()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let new_patterns: Vec<&str> = post_patterns
        .iter()
        .filter(|p| !pre_patterns.iter().any(|q| q == *p))
        .map(|s| s.as_str())
        .collect();

    println!("--- post-run state (after {} ticks) ---", rt.tick);
    println!(
        "named patterns: {} {:?}",
        post_patterns.len(),
        post_patterns
    );
    println!("newly named: {:?}", new_patterns);
    println!();

    println!("--- meta-meta episodes ---");
    let mm_episodes: Vec<&_> = rt
        .memory
        .episodes
        .iter()
        .filter(|ep| {
            ep.action_kind == ActionKind::DiscoverMetaMetaPatterns
        })
        .collect();
    println!(
        "DiscoverMetaMetaPatterns episodes: {}",
        mm_episodes.len()
    );
    for (i, ep) in mm_episodes.iter().enumerate() {
        println!(
            "  [{}] tick={} delta={:+.4}",
            i, ep.tick, ep.delta
        );
    }
    println!();

    // Inspect any newly named pattern: its role chain reveals the
    // canonical shape the runtime extracted from M1.
    if let Some(p) = new_patterns.first() {
        println!("--- new pattern intension: {} ---", p);
        let roles: Vec<String> = rt
            .rset
            .pattern_roles(p)
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        println!("roles: {} {:?}", roles.len(), roles);
        // The role-to-role structural edges encode the pattern's
        // topology. Print them for readability.
        let role_set: std::collections::HashSet<&str> =
            roles.iter().map(|s| s.as_str()).collect();
        let mut struct_edges: Vec<(String, String)> = rt
            .rset
            .iter()
            .filter(|r| {
                role_set.contains(r.x.as_str())
                    && role_set.contains(r.y.as_str())
            })
            .map(|r| (r.x.clone(), r.y.clone()))
            .collect();
        struct_edges.sort();
        println!(
            "intension structural edges: {} {:?}",
            struct_edges.len(),
            struct_edges
        );
    } else {
        println!("--- no new pattern named (loop did not close in {} ticks) ---", rt.tick);
    }

    println!();
    println!("--- end ---");
}
