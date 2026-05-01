//! Phase 0072-A — Intervention ablation on long5k.
//!
//! Replicates the OQ#1 ablation
//! (`phase_0072_a_intervention_ablation`) on a structurally
//! distinct substrate to test whether Addendum 2's bug pattern
//! (D-condition cross_min regression) was OQ#1-specific or
//! universal — and therefore whether Addendum 3's quality-floor
//! threshold (0.70) is an OQ#1 artifact or a substrate-agnostic
//! safeguard.
//!
//! Per Phase C.2's finding, long5k @ 1500 ticks produces the
//! same 4-theory structure as OQ#1 (t_0..t_3), with t_2 and t_3
//! at cross-precision 1.0 (Signal), t_1 at ~0.68, and t_0 as
//! the noise-laden theory. The same 5 conditions therefore map
//! cleanly:
//!
//!   A. Baseline                     (no intervention)
//!   B. FamilyDemote                  (noise family retracted)
//!   C. Merge(t_2, t_3)               (HighQualityBoth)
//!   D. Merge(t_1, t_2)               (Complementary — risky)
//!   E. Combined: B + C              (best two together)
//!
//! Falsifiable predictions (against OQ#1 results):
//!   - B improves both primary_mean and cross_mean → if true on
//!     long5k too, FamilyDemote is universally beneficial
//!   - C is cross-min-neutral (~0.0) → if true, harmless merge
//!     pattern generalises
//!   - D regresses cross_min by ≥0.05 → if true, A2's bug is
//!     universal and Addendum 3 is justified across substrates
//!   - E ≥ B and E ≥ C on cross_mean → composability holds
//!
//! Methodology: deterministic substrate; identical Phase 0
//! (1500 ticks); apply intervention; identical Phase 1 (1500
//! more ticks); report.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::long5k::build_5k_stream,
    R, RSet,
};
use std::collections::{HashMap, HashSet};

const TICKS_PHASE_0: u64 = 1500;
const TICKS_PHASE_1: u64 = 1500;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const MIN_AXIOM_PREDICTIONS: u64 = 5;
const NOISE_FAMILY_TARGET: &str = "shape_premise_p0-0_p1-2";

#[derive(Debug, Clone)]
struct Snapshot {
    name: String,
    axiom_count: usize,
    theory_count: usize,
    theories: Vec<String>,
    l2_family_count: usize,
    family_demote_target_resurrected: bool,
    primary_mean: Option<f64>,
    primary_min: Option<f64>,
    primary_qualifying_axioms: usize,
    cross_mean: Option<f64>,
    cross_min: Option<f64>,
    qualifying_theory_count: usize,
}

fn build_runtime() -> AutonomousRuntime {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_5k_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt
}

fn measure(rt: &AutonomousRuntime, name: &str) -> Snapshot {
    let axioms: HashSet<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();
    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    let l2_family_count = rt.rset.axiom_shape_families().len();

    // Was the family-demote target re-discovered?
    let resurrected = rt.rset.is_axiom_shape_family(NOISE_FAMILY_TARGET);

    // Per-axiom primary rates from prediction_state.
    let mut primary_rates: Vec<f64> = Vec::new();
    for ax in &axioms {
        if let Some(r) =
            rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS)
        {
            primary_rates.push(r);
        }
    }
    let primary_qualifying_axioms = primary_rates.len();
    let primary_mean = if primary_qualifying_axioms > 0 {
        Some(primary_rates.iter().sum::<f64>() / primary_qualifying_axioms as f64)
    } else {
        None
    };
    let primary_min = if primary_qualifying_axioms > 0 {
        Some(primary_rates.iter().cloned().fold(f64::INFINITY, f64::min))
    } else {
        None
    };

    // Cross-precision per theory: build substrates, compute mean per axiom, aggregate.
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
    let mut theory_cross_means: Vec<f64> = Vec::new();
    let mut qualifying_theory_count = 0usize;
    for t in &theories {
        let theory_axioms: Vec<String> = rt.rset.theory_axioms(t)
            .into_iter().map(str::to_owned).collect();
        let mut sum = 0.0;
        let mut count = 0;
        for ax in &theory_axioms {
            if let Some(p) = rt.rset.axiom_cross_precision(ax, &substrates) {
                sum += p;
                count += 1;
            }
        }
        if count > 0 {
            theory_cross_means.push(sum / count as f64);
            qualifying_theory_count += 1;
        }
    }
    let cross_mean = if !theory_cross_means.is_empty() {
        Some(theory_cross_means.iter().sum::<f64>() / theory_cross_means.len() as f64)
    } else { None };
    let cross_min = if !theory_cross_means.is_empty() {
        Some(theory_cross_means.iter().cloned().fold(f64::INFINITY, f64::min))
    } else { None };

    Snapshot {
        name: name.to_string(),
        axiom_count: axioms.len(),
        theory_count: theories.len(),
        theories,
        l2_family_count,
        family_demote_target_resurrected: resurrected,
        primary_mean,
        primary_min,
        primary_qualifying_axioms,
        cross_mean,
        cross_min,
        qualifying_theory_count,
    }
}

fn fmt_opt(v: Option<f64>) -> String {
    match v {
        Some(x) => format!("{:.4}", x),
        None => "—".to_string(),
    }
}

fn run_condition(name: &str, intervention_label: &str,
                 apply: impl FnOnce(&mut AutonomousRuntime)) -> Snapshot {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Condition {}: {}", name, intervention_label);
    println!("════════════════════════════════════════════════════════");

    let mut rt = build_runtime();
    rt.run_bounded(TICKS_PHASE_0);
    let pre = measure(&rt, &format!("{}_pre", name));
    println!("  Phase 0 ({} ticks, identical baseline):", TICKS_PHASE_0);
    println!("    axioms={}  theories={:?}  L2 fams={}",
             pre.axiom_count, pre.theories, pre.l2_family_count);
    println!("    primary mean={} min={}",
             fmt_opt(pre.primary_mean), fmt_opt(pre.primary_min));
    println!("    cross   mean={} min={}",
             fmt_opt(pre.cross_mean), fmt_opt(pre.cross_min));

    println!("  → applying intervention: {}", intervention_label);
    apply(&mut rt);

    let between = measure(&rt, &format!("{}_post_intervention", name));
    println!("    post-intervention: axioms={}  theories={:?}  L2 fams={}",
             between.axiom_count, between.theories, between.l2_family_count);

    rt.run_bounded(TICKS_PHASE_1);
    let post = measure(&rt, name);
    println!("  Phase 1 ({} more ticks):", TICKS_PHASE_1);
    println!("    axioms={}  theories={:?}  L2 fams={}",
             post.axiom_count, post.theories, post.l2_family_count);
    println!("    primary mean={} min={}",
             fmt_opt(post.primary_mean), fmt_opt(post.primary_min));
    println!("    cross   mean={} min={}",
             fmt_opt(post.cross_mean), fmt_opt(post.cross_min));
    println!("    family-demote target rediscovered: {}",
             post.family_demote_target_resurrected);
    post
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Phase 0072-A — Intervention ablation (long5k)");
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: long5k (5000-tick stream, 5 regimes × 10 phases)");
    println!(" Method: 5 conditions; identical Phase 0 ({} ticks),",
             TICKS_PHASE_0);
    println!(" different intervention, identical Phase 1 ({} ticks).",
             TICKS_PHASE_1);
    println!(" Noise family target: {}", NOISE_FAMILY_TARGET);

    // Condition A — Baseline (no intervention)
    let a = run_condition("A", "no intervention", |_rt| {});

    // Condition B — FamilyDemote on the noise family
    let b = run_condition("B",
        &format!("FamilyDemote({})", NOISE_FAMILY_TARGET),
        |rt| {
            match rt.rset.retract_shape_family(NOISE_FAMILY_TARGET) {
                Ok(s) => {
                    println!("    retraction summary: layer={:?}, axioms_retracted={}, theory_detachments={}",
                             s.layer, s.axioms_globally_retracted, s.theory_memberships_detached);
                }
                Err(e) => println!("    ✗ retract failed: {:?}", e),
            }
        });

    // Condition C — Merge(t_2, t_3) HighQualityBoth
    let c = run_condition("C", "Merge(t_2, t_3) HighQualityBoth", |rt| {
        match rt.rset.merge_theories("t_2", "t_3") {
            Ok(merged_id) => println!("    merged → {}", merged_id),
            Err(e) => println!("    ✗ merge failed: {:?}", e),
        }
    });

    // Condition D — Merge(t_1, t_2) Complementary (risky)
    let d = run_condition("D", "Merge(t_1, t_2) Complementary", |rt| {
        match rt.rset.merge_theories("t_1", "t_2") {
            Ok(merged_id) => println!("    merged → {}", merged_id),
            Err(e) => println!("    ✗ merge failed: {:?}", e),
        }
    });

    // Condition E — Combined: FamilyDemote + Merge(t_2, t_3)
    let e = run_condition("E", "FamilyDemote + Merge(t_2, t_3)", |rt| {
        match rt.rset.retract_shape_family(NOISE_FAMILY_TARGET) {
            Ok(s) => println!("    retracted: {} axioms",
                              s.axioms_globally_retracted),
            Err(e) => println!("    ✗ retract failed: {:?}", e),
        }
        match rt.rset.merge_theories("t_2", "t_3") {
            Ok(merged_id) => println!("    merged → {}", merged_id),
            Err(e) => println!("    ✗ merge failed: {:?}", e),
        }
    });

    // ── Comparison table ───────────────────────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Comparison (after Phase 1, {} total ticks)",
             TICKS_PHASE_0 + TICKS_PHASE_1);
    println!("════════════════════════════════════════════════════════");
    println!(
        " {:<3} {:<40} {:>4} {:>4} {:>4} {:>10} {:>10} {:>10} {:>10}",
        "id", "intervention",
        "axs", "ths", "fam",
        "p_mean", "p_min", "c_mean", "c_min",
    );
    let snaps = [&a, &b, &c, &d, &e];
    let labels = [
        "no intervention",
        "FamilyDemote(noise family)",
        "Merge(t_2, t_3) HighQualityBoth",
        "Merge(t_1, t_2) Complementary",
        "FamilyDemote + Merge(t_2, t_3)",
    ];
    for (s, label) in snaps.iter().zip(labels.iter()) {
        println!(
            " {:<3} {:<40} {:>4} {:>4} {:>4} {:>10} {:>10} {:>10} {:>10}",
            s.name, label,
            s.axiom_count, s.theory_count, s.l2_family_count,
            fmt_opt(s.primary_mean), fmt_opt(s.primary_min),
            fmt_opt(s.cross_mean), fmt_opt(s.cross_min),
        );
    }

    // ── Resurrection check ─────────────────────────────────────
    println!();
    println!(" family-demote target resurrected after Phase 1?");
    for s in snaps.iter() {
        println!("   {}: {}", s.name, s.family_demote_target_resurrected);
    }

    // ── Pairwise verdicts (compared to OQ#1 ablation) ──────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Verdicts vs OQ#1 baseline expectations");
    println!("════════════════════════════════════════════════════════");

    fn delta(post: Option<f64>, base: Option<f64>) -> String {
        match (post, base) {
            (Some(p), Some(b)) => format!("{:+.4}", p - b),
            _ => "—".to_string(),
        }
    }

    println!();
    println!(" Q1: Does FamilyDemote (B) improve both primary_mean AND cross_mean?");
    println!("     B vs A primary_mean: {} (post {} vs base {})",
             delta(b.primary_mean, a.primary_mean),
             fmt_opt(b.primary_mean), fmt_opt(a.primary_mean));
    println!("     B vs A cross_mean:   {} (post {} vs base {})",
             delta(b.cross_mean, a.cross_mean),
             fmt_opt(b.cross_mean), fmt_opt(a.cross_mean));
    println!("     B axiom delta:       {} (B: {}, A: {})",
             b.axiom_count as i64 - a.axiom_count as i64,
             b.axiom_count, a.axiom_count);
    println!("     OQ#1 reference: B improved primary_mean by +0.2227 and cross_mean by +0.0380");

    println!();
    println!(" Q2: Is Merge(t_2, t_3) (C) cross-min-neutral?");
    println!("     C vs A cross_mean: {}", delta(c.cross_mean, a.cross_mean));
    println!("     C vs A cross_min:  {}", delta(c.cross_min, a.cross_min));
    println!("     C theory delta: {} (C: {}, A: {})",
             c.theory_count as i64 - a.theory_count as i64,
             c.theory_count, a.theory_count);
    println!("     OQ#1 reference: C had cross_mean +0.0234, cross_min +0.0938 — harmless");

    println!();
    println!(" Q3: Does Merge(t_1, t_2) (D) regress cross_min by ≥0.05?  ← THE BUG TEST");
    println!("     D vs A cross_mean: {}", delta(d.cross_mean, a.cross_mean));
    println!("     D vs A cross_min:  {}  ← dilution metric",
             delta(d.cross_min, a.cross_min));
    println!("     D vs C cross_min:  {}  ← directly compare merge targets",
             delta(d.cross_min, c.cross_min));
    println!("     OQ#1 reference: D had cross_min -0.0907 vs C, cross_mean -0.0427 vs C — POLLUTION");

    println!();
    println!(" Q4: Does the demoted family resurrect after Phase 1?");
    println!("     B condition: target_resurrected = {}",
             b.family_demote_target_resurrected);
    println!("     E condition: target_resurrected = {}",
             e.family_demote_target_resurrected);

    println!();
    println!(" Q5 (composability): Does combined intervention E dominate B and C?");
    println!("     E vs B cross_mean: {}", delta(e.cross_mean, b.cross_mean));
    println!("     E vs C cross_mean: {}", delta(e.cross_mean, c.cross_mean));

    // ── Verdict on the universality question ───────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Universality verdict: is Addendum 3 (quality floor 0.70) justified?");
    println!("════════════════════════════════════════════════════════");

    let d_cross_min_regression = match (d.cross_min, c.cross_min) {
        (Some(d_cmin), Some(c_cmin)) => d_cmin - c_cmin,
        _ => 0.0,
    };
    let d_vs_a_regression = match (d.cross_min, a.cross_min) {
        (Some(d_cmin), Some(a_cmin)) => d_cmin - a_cmin,
        _ => 0.0,
    };

    println!();
    println!(" D condition cross_min vs C: {:+.4}", d_cross_min_regression);
    println!(" D condition cross_min vs A: {:+.4}", d_vs_a_regression);

    let bug_universal = d_cross_min_regression < -0.05 || d_vs_a_regression < -0.05;
    if bug_universal {
        println!();
        println!(" → CONFIRMED — D-condition pollution reproduces on long5k.");
        println!("   Addendum 3 (focal quality floor 0.70) is justified across substrates,");
        println!("   not an OQ#1 artifact. Threshold may merit later tuning, but the");
        println!("   underlying bug pattern is universal.");
    } else {
        println!();
        println!(" → NOT REPRODUCED — D-condition does NOT pollute t_2 on long5k.");
        println!("   Addendum 3's threshold may be OQ#1-specific. Investigate substrate");
        println!("   structural differences (e.g. t_1's specificity / overlap with t_2)");
        println!("   before generalising the rule.");
    }

    println!();
    println!("--- end ---");
    let _ = HashMap::<String, f64>::new();
    let _ = std::marker::PhantomData::<R>;
}
