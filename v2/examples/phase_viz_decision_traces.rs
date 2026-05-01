//! Visualization — print decision traces + quality reports for
//! every theory on a substrate. Lets the user SEE how v2 selects
//! interventions, step-by-step.
//!
//! Two views per theory:
//!   - Quality report (the FACTS)
//!   - Decision trace (the WHY of the chosen recommendation)
//!
//! Output: text-only, to stdout. Inspectable in any terminal.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq1::build_long_stream,
    RSet,
};
use std::collections::{HashMap, HashSet};

const TICKS: u64 = 1000;
const NUM_GEN_IDS: usize = 15;
const SEED_DENSITY: f64 = 0.05;
const RNG_SEED_BASE: u64 = 0xC0FFEE_DEADBEEF;
const MIN_AXIOM_PREDICTIONS: u64 = 5;

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" v2 decision-trace visualization on OQ#1 @ {} ticks", TICKS);
    println!("════════════════════════════════════════════════════════");

    // ── Run substrate to convergence ────────────────────────────
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(build_long_stream()));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS);

    let mut theories: Vec<String> =
        rt.rset.theories().into_iter().map(str::to_owned).collect();
    theories.sort();
    let axioms: HashSet<String> =
        rt.rset.axioms().into_iter().map(str::to_owned).collect();

    println!();
    println!("Substrate state:");
    println!("  axioms       : {}", axioms.len());
    println!("  theories     : {} ({:?})", theories.len(), theories);
    println!("  L2 families  : {}", rt.rset.axiom_shape_families().len());

    // ── Build substrates + primary rates ────────────────────────
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
        if let Some(r) =
            rt.memory.prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS)
        {
            primary_rates.insert(ax.clone(), r);
        }
    }

    // ── Build all reports ───────────────────────────────────────
    let reports = rt.rset.theory_quality_report_all(&substrates, &primary_rates);

    // ── Print per-theory: report + decision trace ──────────────
    for r in &reports {
        let others: Vec<_> = reports.iter()
            .filter(|o| o.theory_id != r.theory_id)
            .cloned()
            .collect();
        println!();
        println!("════════════════════════════════════════════════════════");
        println!(" Theory: {}", r.theory_id);
        println!("════════════════════════════════════════════════════════");
        println!();
        print!("{}", RSet::format_quality_report(r));
        println!();
        print!("{}", RSet::format_decision_trace(r, &others));
    }

    // ── Side-by-side recommendation summary ────────────────────
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Recommendation summary");
    println!("════════════════════════════════════════════════════════");
    for r in &reports {
        let others: Vec<_> = reports.iter()
            .filter(|o| o.theory_id != r.theory_id)
            .cloned()
            .collect();
        let rec = RSet::recommend_intervention(r, &others);
        println!("  {} → {:?}", r.theory_id, rec);
    }
    println!();
    println!("--- end ---");
}
