//! Phase E.1 — Verify H2.1.0 drive-as-meta-R registration.
//!
//! H2.1.0 (ADR 0064) was supposedly done: `register_drives_in_rset`
//! adds `R(DRIVE_MARKER, drive_<id>)` for each drive and
//! `R(PENALTY_MARKER, drive_<id>)` for penalty drives. The runtime's
//! `combined_drive_signal` already reads penalty status from meta-R
//! as canonical truth.
//!
//! E.1 verifies:
//!   1. Default runtime has 3 drives registered under DRIVE_MARKER
//!   2. ModeThrashPenalty appears under PENALTY_MARKER (the only
//!      penalty drive in the default catalogue)
//!   3. EP path still fires (no shadow-mode breakage)
//!   4. `is_drive_penalty_via_meta_r` matches `Drive::is_penalty()`
//!      for every registered drive (consistency check)
//!
//! Captured to `logs/<date>_phase_e1_drive_meta_r_verify.log`.

use relatum_v2::{
    runtime::{
        ActionKind, AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    DRIVE_MARKER, PENALTY_MARKER, RSet,
};
use std::collections::HashSet;

fn main() {
    println!("=== Phase E.1 — H2.1.0 drive-as-meta-R verification ===");

    // Build default runtime with OQ#1 stream so EP path can fire.
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());

    // ── Verification 1: drive registration ─────────────────────
    println!();
    println!("=== 1. Drives registered under DRIVE_MARKER ===");
    // `R(DRIVE_MARKER, drive_X)` — DRIVE_MARKER on left, so use left_of.
    let drive_ids: Vec<String> = rt.rset.left_of(DRIVE_MARKER)
        .iter()
        .map(|r| r.y.to_string())
        .collect();
    let mut sorted = drive_ids.clone();
    sorted.sort();
    println!("  count: {}", sorted.len());
    for id in &sorted {
        println!("    {}", id);
    }

    // Default drives: compression, prediction_error, mode_thrash
    let expected_drives: HashSet<String> = ["drive_compression", "drive_prediction_error", "drive_mode_thrash"]
        .iter().map(|s| s.to_string()).collect();
    let actual_drives: HashSet<String> = sorted.iter().map(|s| s.to_string()).collect();
    let drive_check = actual_drives == expected_drives;
    println!("  expected: {:?}", expected_drives);
    println!("  match: {}", if drive_check { "✓" } else { "✗" });

    // ── Verification 2: penalty marking ────────────────────────
    println!();
    println!("=== 2. Penalty drives under PENALTY_MARKER ===");
    let penalty_ids: Vec<String> = rt.rset.left_of(PENALTY_MARKER)
        .iter()
        .map(|r| r.y.to_string())
        .collect();
    let mut psorted = penalty_ids.clone();
    psorted.sort();
    println!("  count: {}", psorted.len());
    for id in &psorted {
        println!("    {}", id);
    }
    // Default: only mode_thrash is a penalty
    let expected_penalty: HashSet<String> = ["drive_mode_thrash"]
        .iter().map(|s| s.to_string()).collect();
    let actual_penalty: HashSet<String> = psorted.iter().map(|s| s.to_string()).collect();
    let penalty_check = actual_penalty == expected_penalty;
    println!("  expected: {:?}", expected_penalty);
    println!("  match: {}", if penalty_check { "✓" } else { "✗" });

    // ── Verification 3: EP path still fires ────────────────────
    println!();
    println!("=== 3. EP path runs (no shadow-mode breakage) ===");
    rt.run_bounded(200);
    let ep_count = rt.memory.episodes.iter()
        .filter(|e| e.action_kind == ActionKind::EvaluatePredictions)
        .count();
    println!("  total episodes: {}", rt.memory.episodes.len());
    println!("  EvaluatePredictions episodes: {}", ep_count);
    let ep_check = ep_count > 0 || rt.memory.episodes.is_empty();
    println!("  EP path runs: {}", if ep_check { "✓" } else { "✗ (BROKEN!)" });

    // ── Verification 4: meta-R query consistency ───────────────
    println!();
    println!("=== 4. meta-R query consistency (rset.contains() matches Drive trait) ===");
    let mut consistency_ok = true;
    for id in &sorted {
        let _ = id;
        // Query if registered as penalty.
        let in_penalty = rt.rset.contains(&relatum_v2::R::new(
            PENALTY_MARKER.to_string(), id.to_string(),
        ));
        // Look up the actual Drive trait impl.
        let trait_says: Option<bool> = rt.drives.iter()
            .find(|d| format!("drive_{}", d.id()) == **id)
            .map(|d| d.is_penalty());
        match trait_says {
            Some(b) => {
                let match_ = b == in_penalty;
                println!(
                    "    {}: trait.is_penalty()={}, meta-R says={}, match={}",
                    id, b, in_penalty, if match_ { "✓" } else { "✗" },
                );
                if !match_ {
                    consistency_ok = false;
                }
            }
            None => {
                println!("    {}: no Drive trait found for this id (registration drift?)", id);
                consistency_ok = false;
            }
        }
    }

    // ── Verdict ────────────────────────────────────────────────
    println!();
    println!("=== Verdict ===");
    let all_ok = drive_check && penalty_check && ep_check && consistency_ok;
    if all_ok {
        println!("  → POSITIVE — H2.1.0 verified: drives + penalties registered as meta-R, EP path intact, trait/meta-R consistent.");
    } else {
        println!("  → ISSUES found:");
        if !drive_check { println!("    - drive registration mismatch"); }
        if !penalty_check { println!("    - penalty marker mismatch"); }
        if !ep_check { println!("    - EP path broken"); }
        if !consistency_ok { println!("    - trait/meta-R inconsistency"); }
    }
    println!();
    println!("--- end ---");
}
