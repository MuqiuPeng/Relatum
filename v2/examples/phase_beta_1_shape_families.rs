//! Phase Beta-1 — Axiom shape family discovery (ADR 0068).
//!
//! Beta-1 is the first slice that lets v2 EXTEND its structural
//! vocabulary at runtime, not just produce instances of pre-defined
//! shapes. Until now, axiom discovery has been bounded by the
//! compile-time `enumerate_axiom_templates` enumerator: the system
//! finds axioms WITHIN that template space but cannot invent a new
//! template SHAPE.
//!
//! Beta-1 introduces the first axiom-shape ABSTRACTION: a "shape
//! family" is a set of registered axioms that share a structural
//! sub-component (initially: identical canonicalized premise).
//! When ≥ N axioms share a premise, the agent mints a new
//! `R(SHAPE_FAMILY_MARKER, shape_<...>)` meta-R object, with
//! `R(shape_id, ax_id)` membership edges.
//!
//! This is "real" auto-extension because the SHAPE_FAMILY_MARKER
//! type was unrealized before the call: `right_of(SHAPE_FAMILY_MARKER)`
//! was empty, then becomes non-empty as a function of the data the
//! agent has seen. The TYPE is discovered, not declared.
//!
//! Empirical question: on OQ#1, t_0's 4 noise axioms share premise
//! `[p0-0, p1-2]`. Does the shape-family mechanism spontaneously
//! capture that as a family, AND does the family correlate with
//! a uniform cross-precision signature?
//!
//! If yes: the system has independently rediscovered "noise
//! family" as a structural type — a vocabulary extension that
//! Phase Alpha couldn't produce.
//!
//! Captured to `logs/<date>_phase_beta_1_shape_families.log`.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    ESTABLISHED_MARKER, PATTERN_MARKER, R, RSet, SHAPE_FAMILY_MARKER,
};
use std::collections::HashSet;

const TICKS_PHASE_0: u64 = 1000;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const SHAPE_MIN_MEMBERS: usize = 2;

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

/// Forward-apply every axiom in `axiom_ids` on `substrate`.
fn predict_all_axioms(substrate: &RSet, axiom_ids: &[String]) -> HashSet<R> {
    let mut total: HashSet<R> = HashSet::new();
    for ax in axiom_ids {
        total.extend(substrate.forward_apply_axiom(ax));
    }
    total
}

fn precision(predicted: &HashSet<R>, actual: &HashSet<R>) -> Option<f64> {
    if predicted.is_empty() {
        return None;
    }
    let inter = predicted.iter().filter(|r| actual.contains(*r)).count();
    Some(inter as f64 / predicted.len() as f64)
}

/// Compute mean cross-precision for a single axiom across all
/// substrates — i.e., "how universal is this axiom?"
/// Mirrors Alpha-7's column-mean calculation but at axiom granularity
/// rather than theory granularity.
fn axiom_mean_cross_precision(
    ax_id: &str,
    substrates: &[RSet],
) -> Option<f64> {
    let single_axiom = vec![ax_id.to_string()];
    let mut sum = 0.0;
    let mut count = 0;
    for sub in substrates {
        let actual: HashSet<R> = sub.iter().cloned().collect();
        let predicted = predict_all_axioms(sub, &single_axiom);
        if let Some(p) = precision(&predicted, &actual) {
            sum += p;
            count += 1;
        }
    }
    if count == 0 {
        None
    } else {
        Some(sum / count as f64)
    }
}

fn main() {
    println!(
        "=== ADR 0068 Phase Beta-1 — Axiom shape family discovery ({} ticks Phase 0) ===",
        TICKS_PHASE_0,
    );
    println!(
        "SHAPE_MIN_MEMBERS={}, NUM_GEN_IDS={}, SEED_DENSITY={}",
        SHAPE_MIN_MEMBERS, NUM_GEN_IDS, SEED_DENSITY,
    );

    // ── Phase 0: discover theories + axioms ─────────────────────
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let axioms_before: Vec<String> = rt
        .rset
        .axioms()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let theories_before: Vec<String> = rt
        .rset
        .theories()
        .into_iter()
        .map(str::to_owned)
        .collect();

    println!();
    println!("=== Pre-Beta-1 state ===");
    println!(
        "  axioms registered  : {}",
        axioms_before.len(),
    );
    println!(
        "  theories registered: {}",
        theories_before.len(),
    );
    println!(
        "  shape families     : {} (expected 0 — type not yet realized)",
        rt.rset.axiom_shape_families().len(),
    );

    // ── Beta-1 step: discover axiom shape families ──────────────
    println!();
    println!("=== Beta-1: discover_axiom_shape_families({}) ===", SHAPE_MIN_MEMBERS);
    let minted: Vec<String> = rt
        .rset
        .discover_axiom_shape_families(SHAPE_MIN_MEMBERS);
    println!(
        "  minted {} new shape families: {:?}",
        minted.len(),
        minted,
    );

    if minted.is_empty() {
        println!();
        println!("=== Verdict ===");
        println!("  → NULL — no axiom premise has ≥ {} members; substrate doesn't expose shape-family auto-extension on this run", SHAPE_MIN_MEMBERS);
        println!();
        println!("--- end ---");
        return;
    }

    // Print each family's members.
    println!();
    println!("=== Family membership ===");
    for shape in &minted {
        let members = rt.rset.shape_family_members(shape);
        println!("  {} ({} members):", shape, members.len());
        for m in &members {
            println!("    - {}", m);
        }
    }

    // ── Cross-precision per axiom: for each member, what is its
    // mean precision across imagined substrates? Members of a real
    // structural family should have similar profiles.
    println!();
    println!("=== Per-axiom cross-precision (for shape family analysis) ===");
    println!("  Generating substrates per theory...");
    let mut substrates: Vec<RSet> = Vec::with_capacity(theories_before.len());
    let mut all_axiom_ids: HashSet<String> = HashSet::new();
    for ax in &axioms_before {
        all_axiom_ids.insert(ax.clone());
    }
    for (i, t) in theories_before.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut gen = match rt
            .rset
            .generate_substrate_from_theory(t, NUM_GEN_IDS, SEED_DENSITY, seed)
        {
            Ok(g) => g,
            Err(_) => continue,
        };
        for ax in &all_axiom_ids {
            gen.register_axiom_with_intension(ax);
        }
        substrates.push(gen);
    }
    println!("  Substrates ready: {}", substrates.len());

    println!();
    println!("=== Family member cross-precision profiles ===");
    for shape in &minted {
        let members: Vec<String> = rt
            .rset
            .shape_family_members(shape)
            .into_iter()
            .map(str::to_owned)
            .collect();
        println!();
        println!("  Family {} ({} members):", shape, members.len());
        let mut profiles: Vec<(String, Option<f64>)> = Vec::new();
        for ax_id in &members {
            let mp = axiom_mean_cross_precision(ax_id, &substrates);
            profiles.push((ax_id.clone(), mp));
        }
        // Print profiles.
        println!("    {:>50} {:>15}", "axiom_id", "mean_xprec");
        for (ax, mp) in &profiles {
            let s = mp.map(|x| format!("{:.4}", x)).unwrap_or_else(|| "—".into());
            println!("    {:>50} {:>15}", ax, s);
        }
        // Family-level diagnostic: are members structurally consistent?
        let valued: Vec<f64> = profiles.iter().filter_map(|(_, p)| *p).collect();
        if valued.is_empty() {
            println!("    (no qualifying predictions for any member)");
            continue;
        }
        let mean = valued.iter().sum::<f64>() / valued.len() as f64;
        let var = valued.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
            / valued.len() as f64;
        let max = valued.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min = valued.iter().cloned().fold(f64::INFINITY, f64::min);
        println!(
            "    family stats: mean={:.4} var={:.6} min={:.4} max={:.4} spread={:.4}",
            mean, var, min, max, max - min,
        );
        // Verdict per family.
        let consistent = (max - min) < 0.10;
        let uniform_low = max < 0.50;
        let uniform_high = min > 0.90;
        if consistent && uniform_low {
            println!(
                "    → STRUCTURAL NOISE FAMILY: all members are consistently low-precision",
            );
        } else if consistent && uniform_high {
            println!(
                "    → STRUCTURAL SIGNAL FAMILY: all members are consistently high-precision",
            );
        } else if consistent {
            println!(
                "    → STRUCTURAL FAMILY (mid-range, consistent profile)",
            );
        } else {
            println!(
                "    → MIXED — members have divergent profiles; family may not capture quality dimension",
            );
        }
    }

    // ── Sanity check: SHAPE_FAMILY_MARKER is queryable as meta-R ──
    println!();
    println!("=== Constitutional check ===");
    let registered_families: Vec<&str> = rt.rset.axiom_shape_families();
    println!(
        "  R(SHAPE_FAMILY_MARKER, ?) → {} entries: {:?}",
        registered_families.len(),
        registered_families,
    );
    // Verify membership edges exist for each family.
    let mut all_edges_present = true;
    for shape in &minted {
        let members: Vec<&str> = rt.rset.shape_family_members(shape);
        for m in members {
            let edge = R::new(shape.to_string(), m.to_string());
            if !rt.rset.contains(&edge) {
                println!("  ✗ MISSING edge: R({}, {})", shape, m);
                all_edges_present = false;
            }
        }
    }
    if all_edges_present {
        println!("  ✓ All membership edges R(shape_id, ax_id) present in rset");
    }
    // Verify SHAPE_FAMILY_MARKER itself.
    let marker_edge = R::new(SHAPE_FAMILY_MARKER.to_string(), minted[0].clone());
    if rt.rset.contains(&marker_edge) {
        println!(
            "  ✓ R(SHAPE_FAMILY_MARKER, {}) present — meta-R class instantiated",
            minted[0],
        );
    }

    // ── Final verdict ──────────────────────────────────────────
    println!();
    println!("=== Verdict ===");
    println!(
        "  Beta-1 minted {} new meta-R object(s) under SHAPE_FAMILY_MARKER",
        minted.len(),
    );
    println!(
        "  These are {} kind(s) of structural abstraction that did not exist in the rset before this call.",
        minted.len(),
    );
    println!(
        "  Constitutional commitment 3 (types as meta-R) realized at a new level: the TYPE itself was discovered structurally, not declared.",
    );

    println!();
    println!("--- end ---");
}
