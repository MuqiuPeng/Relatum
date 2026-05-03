//! Phase 0072-B — MERGE_QUALITY_FLOOR threshold sensitivity scan.
//!
//! Addendum 3 sets MERGE_QUALITY_FLOOR = 0.70 by hand. This scan
//! validates the choice empirically.
//!
//! Method (static check, not full ablation):
//!   1. Run OQ#1 to Phase 0 maturity (1000 ticks)
//!   2. Build TheoryQualityReport for every theory
//!   3. For each candidate floor ∈ {0.55, 0.60, 0.65, 0.70, 0.75,
//!      0.80, 0.85, 0.90}:
//!        - For each Mixed theory, simulate Step 5's quality gate
//!          (focal_primary >= floor AND focal_cross >= floor)
//!        - Record BLOCK or ALLOW
//!   4. Cross-reference against Phase 0072-A's empirical oracle:
//!        - t_1 ALLOW = -0.0907 cross_min regression vs C (BAD)
//!        - t_2/t_3 ALLOW = +0.0938 cross_min improvement (GOOD)
//!   5. Determine the floor band that satisfies both constraints
//!
//! Falsifiable predictions:
//!   - There SHOULD exist a non-empty floor band [lo, hi] where
//!     t_1 is BLOCKED but t_2/t_3 are still ALLOWED to merge —
//!     otherwise Addendum 3 is unrescuable
//!   - 0.70 SHOULD lie within that band — otherwise the chosen
//!     value needs adjustment
//!
//! Note: t_2/t_3 are summary_class = Signal post-A1, so they exit
//! the decision tree at Step 1 (`Signal → None or HighQualityBoth
//! merge`) and don't reach Step 5 at all. The floor only gates
//! Mixed-class focal theories. So the meaningful ALLOW/BLOCK test
//! is purely on Mixed theories (t_1 in OQ#1's case).

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    RSet, TheoryQualityClass,
};
use std::collections::{HashMap, HashSet};

const TICKS_PHASE_0: u64 = 1000;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const MIN_AXIOM_PREDICTIONS: u64 = 5;

const FLOOR_CANDIDATES: &[f64] = &[
    0.55, 0.60, 0.65, 0.70, 0.75, 0.80, 0.85, 0.90,
];

const NEAR_DISJOINT_JACCARD: f64 = 0.50;

fn fmt_opt(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{:.4}", x),
        None => "—".to_string(),
    }
}

fn class_label(c: TheoryQualityClass) -> &'static str {
    match c {
        TheoryQualityClass::Signal => "Signal",
        TheoryQualityClass::Mixed => "Mixed",
        TheoryQualityClass::Noise => "Noise",
        TheoryQualityClass::Indeterminate => "Indet.",
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Phase 0072-B — MERGE_QUALITY_FLOOR threshold scan");
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: OQ#1 ({} ticks)", TICKS_PHASE_0);
    println!(" Floor candidates: {:?}", FLOOR_CANDIDATES);

    // ── Phase 0 setup ───────────────────────────────────────────
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PHASE_0);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    let axioms: HashSet<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();

    println!();
    println!(" Post Phase 0: {} axioms / {} theories ({:?})",
             axioms.len(), theories.len(), theories);

    // ── Build substrates + primary_rates ────────────────────────
    let mut substrates: Vec<RSet> = Vec::new();
    for (i, t) in theories.iter().enumerate() {
        let seed = RNG_SEED_BASE.wrapping_add(i as u64 * 0x9E3779B97F4A7C15);
        let mut g = match rt.rset.generate_substrate_from_theory(
            t, NUM_GEN_IDS, SEED_DENSITY, seed,
        ) {
            Ok(g) => g, Err(_) => continue,
        };
        for ax in &axioms { g.register_axiom_with_intension(ax); }
        substrates.push(g);
    }
    let mut primary_rates: HashMap<String, f64> = HashMap::new();
    for ax in &axioms {
        if let Some(r) = rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS) {
            primary_rates.insert(ax.clone(), r);
        }
    }

    let reports = rt.rset.theory_quality_report_all(&substrates, &primary_rates);

    // ── Per-theory summary ──────────────────────────────────────
    println!();
    println!(" Per-theory quality summary:");
    println!("   {:<6} {:>8} {:>10} {:>10}", "theory", "class", "p_mean", "c_mean");
    for r in &reports {
        println!("   {:<6} {:>8} {:>10} {:>10}",
                 r.theory_id, class_label(r.summary_class),
                 fmt_opt(r.primary_rate_mean), fmt_opt(r.cross_precision_mean));
    }

    // ── Identify Mixed theories (only ones gated by Step 5) ─────
    let mixed_reports: Vec<_> = reports.iter()
        .filter(|r| r.summary_class == TheoryQualityClass::Mixed)
        .collect();

    println!();
    println!(" Mixed theories (subject to Step 5 quality gate):");
    if mixed_reports.is_empty() {
        println!("   (none)");
        println!(" → INAPPLICABLE: no Mixed theories means Step 5 never fires.");
        println!("   Floor scan is moot on this substrate at this horizon.");
        return;
    }
    for r in &mixed_reports {
        println!("   {} primary={} cross={}",
                 r.theory_id,
                 fmt_opt(r.primary_rate_mean), fmt_opt(r.cross_precision_mean));
    }

    // ── Find candidate Signal partners with Jaccard ≤ 0.50 ─────
    println!();
    println!(" Identifying Signal partners with near-disjoint Jaccard:");
    println!("   {:<6} {:<10} {:<10} {:>8}", "focal", "partner", "p_class", "jaccard");

    fn jaccard(a: &HashSet<&str>, b: &HashSet<&str>) -> f64 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        let inter = a.intersection(b).count() as f64;
        let union = a.union(b).count() as f64;
        if union == 0.0 { 1.0 } else { inter / union }
    }

    type Partnership<'a> = (&'a String, &'a String, f64);
    let mut active_partners: Vec<Partnership> = Vec::new();
    for focal in &mixed_reports {
        let focal_fams: HashSet<&str> = focal.family_memberships.iter()
            .map(|m| m.family_id.as_str()).collect();
        for partner in reports.iter() {
            if partner.theory_id == focal.theory_id {
                continue;
            }
            if partner.summary_class != TheoryQualityClass::Signal {
                continue;
            }
            let partner_fams: HashSet<&str> = partner.family_memberships.iter()
                .map(|m| m.family_id.as_str()).collect();
            let j = jaccard(&focal_fams, &partner_fams);
            println!("   {:<6} {:<10} {:<10} {:>8.4}",
                     focal.theory_id, partner.theory_id,
                     class_label(partner.summary_class), j);
            if j <= NEAR_DISJOINT_JACCARD {
                active_partners.push((&focal.theory_id, &partner.theory_id, j));
            }
        }
    }
    println!();
    println!(" Active (focal, partner) pairs reaching Step 5:");
    for (f, p, j) in &active_partners {
        println!("   ({}, {}) jaccard={:.4}", f, p, j);
    }
    if active_partners.is_empty() {
        println!("   (none) — Step 5 never fires regardless of floor.");
        println!(" → INAPPLICABLE on this substrate.");
        return;
    }

    // ── Threshold scan: for each (focal_theory, floor), BLOCK or ALLOW? ─
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Threshold scan — BLOCK (B) vs ALLOW (A) per (focal, floor)");
    println!("════════════════════════════════════════════════════════");
    print!(" {:<6} {:>10} {:>10}", "focal", "p_mean", "c_mean");
    for f in FLOOR_CANDIDATES { print!("  ≥{:.2}", f); }
    println!();

    for focal in &mixed_reports {
        let p = focal.primary_rate_mean.unwrap_or(0.0);
        let c = focal.cross_precision_mean.unwrap_or(0.0);
        print!(" {:<6} {:>10} {:>10}",
                 focal.theory_id, fmt_opt(focal.primary_rate_mean),
                 fmt_opt(focal.cross_precision_mean));
        for floor in FLOOR_CANDIDATES {
            let allow = p >= *floor && c >= *floor;
            print!("    {}  ", if allow { "A" } else { "B" });
        }
        println!();
    }

    // ── Oracle cross-reference ─────────────────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Oracle from Phase 0072-A ablation (empirical truth)");
    println!("════════════════════════════════════════════════════════");
    println!(" t_1 ALLOW (Merge with t_2):");
    println!("   - Empirical effect: cross_min -0.0907 vs C, cross_mean -0.0427 vs C");
    println!("   - Verdict: HARMFUL — t_1 should be BLOCKED");
    println!();
    println!(" Note: t_2/t_3 are Signal-class, so they exit at Step 1 (HighQualityBoth)");
    println!(" and never reach the Step 5 gate. The floor only affects Mixed theories.");

    // ── Recommended floor band ─────────────────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Recommended floor band");
    println!("════════════════════════════════════════════════════════");

    // Smallest floor that BLOCKs every Mixed theory in active_partners.
    // (We want all current Mixed theories to be blocked; if more
    // empirical evidence reveals some are safe, this band would shift.)
    let mut max_focal_min: f64 = 0.0;
    for focal in &mixed_reports {
        // The focal is BLOCKED when floor > min(p, c).
        let p = focal.primary_rate_mean.unwrap_or(0.0);
        let c = focal.cross_precision_mean.unwrap_or(0.0);
        let focal_min = p.min(c);
        if focal_min > max_focal_min {
            max_focal_min = focal_min;
        }
    }
    let lower_bound = max_focal_min;
    println!(" Lower bound (floor must EXCEED this to BLOCK harmful merges):");
    println!("   {:.4} (max over all Mixed theories of min(p_mean, c_mean))",
             lower_bound);

    // No clear upper bound from this scan alone — the upper bound is
    // determined by what merges we *want* to allow but which are
    // close to the floor. With current data, no Mixed theory is
    // "safe enough to merge" empirically; once one appears, its
    // min(p, c) sets the upper bound.
    println!(" Upper bound: no Mixed theory is empirically safe-to-merge yet");
    println!("   so any floor > {:.4} is acceptable from BLOCK perspective", lower_bound);
    println!();

    let chosen_floor: f64 = 0.70;
    let band_ok = chosen_floor > lower_bound;
    println!(" Currently shipped floor: {:.2}", chosen_floor);
    println!(" Within recommended band ({:.4} < floor)? {}",
             lower_bound, if band_ok { "✓" } else { "✗" });

    println!();
    println!(" Per-floor BLOCK behaviour for each focal theory:");
    println!("   {:<6} {:<10}", "focal", "min(p,c)");
    for focal in &mixed_reports {
        let p = focal.primary_rate_mean.unwrap_or(0.0);
        let c = focal.cross_precision_mean.unwrap_or(0.0);
        let mn = p.min(c);
        println!("   {:<6} {:.4} → BLOCKED at floor > {:.4}",
                 focal.theory_id, mn, mn);
    }

    // ── Verdict ────────────────────────────────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Verdict");
    println!("════════════════════════════════════════════════════════");

    let t1_blocked_at_070 = mixed_reports.iter().any(|r| {
        if r.theory_id != "t_1" { return false; }
        let p = r.primary_rate_mean.unwrap_or(0.0);
        let c = r.cross_precision_mean.unwrap_or(0.0);
        !(p >= chosen_floor && c >= chosen_floor)
    });

    if t1_blocked_at_070 {
        println!(" ✓ At floor=0.70, t_1 (Phase 0072-A's known-harmful focal) is BLOCKED.");
        println!("   Addendum 3's threshold correctly excludes the empirically validated");
        println!("   pollution case.");
    } else {
        println!(" ✗ At floor=0.70, t_1 is NOT blocked. Addendum 3 needs adjustment.");
    }

    println!();
    println!(" Lowest floor that blocks t_1: > {:.4}", lower_bound);
    println!();

    println!(" Caveats:");
    println!("   - This scan only covers Mixed focal theories present at this horizon.");
    println!("   - Future Mixed theories with p_mean or c_mean in the (0.55..0.84) range");
    println!("     would tighten the lower bound; structurally distinct substrates");
    println!("     (narrow_a, OQ#2 at maturity) are needed to expand sample size.");
    println!("   - The upper bound on the floor is currently unconstrained — no Mixed");
    println!("     theory is empirically safe-to-merge yet.");
    println!();
    println!("--- end ---");
}
