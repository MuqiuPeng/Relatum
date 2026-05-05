//! Phase Emergence — scheduler integration diagnostic.
//!
//! ADR 0075 piece 2 (preparation). Before promoting
//! `DiscoverPatterns` priority, measure how often the runtime's
//! existing scheduler picks it on each substrate. The kernel-audit
//! data already shows OQ#1/long5k/narrow_a end with 0 patterns
//! and OQ#2 with 2 — but doesn't tell us *why*: were
//! `DiscoverPatterns` actions dispatched zero times, dispatched
//! some but cooled down, or dispatched but produced 0 mints?
//!
//! Reads `policy_stats.action_counts` /
//! `action_positive_delta_counts` after Phase 0 to surface the
//! true scheduler behaviour.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment, Event,
    },
    test_substrates::{
        long5k::build_5k_stream, narrow_a::build_narrow_a_stream,
        oq1::build_long_stream, oq2::build_oq2_stream,
    },
    RSet,
};

#[derive(Debug)]
struct Diagnostic {
    label: String,
    ticks: u64,
    total_episodes: u64,
    discover_patterns_count: u64,
    discover_patterns_positive: u64,
    discover_theory_count: u64,
    discover_theory_positive: u64,
    other_action_counts: Vec<(String, u64)>,
    final_patterns: usize,
    final_theories: usize,
    final_axioms: usize,
}

fn diagnose(label: &str, stream: Vec<(u64, Event)>, ticks: u64) -> Diagnostic {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    let stats = &rt.memory.policy_stats;
    let mut dp_count = 0u64;
    let mut dp_pos = 0u64;
    let mut dt_count = 0u64;
    let mut dt_pos = 0u64;
    let mut other: Vec<(String, u64)> = Vec::new();

    for (kind, count) in &stats.action_counts {
        let pos = stats.action_positive_delta_counts.get(kind).copied().unwrap_or(0);
        match format!("{:?}", kind).as_str() {
            "DiscoverPatterns" => {
                dp_count = *count;
                dp_pos = pos;
            }
            "DiscoverTheory" => {
                dt_count = *count;
                dt_pos = pos;
            }
            other_name => {
                other.push((other_name.to_string(), *count));
            }
        }
    }
    other.sort_by(|a, b| b.1.cmp(&a.1));

    let total: u64 = stats.action_counts.values().sum();

    Diagnostic {
        label: label.to_string(),
        ticks,
        total_episodes: total,
        discover_patterns_count: dp_count,
        discover_patterns_positive: dp_pos,
        discover_theory_count: dt_count,
        discover_theory_positive: dt_pos,
        other_action_counts: other,
        final_patterns: rt.rset.patterns().len(),
        final_theories: rt.rset.theories().len(),
        final_axioms: rt.rset.axioms().len(),
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Scheduler integration diagnostic — DiscoverPatterns frequency");
    println!("════════════════════════════════════════════════════════");
    println!(" Measures how often the standard RuleBasedScheduler picks");
    println!(" DiscoverPatterns vs DiscoverTheory on each substrate at");
    println!(" Phase 0 maturity. Distinguishes \"never dispatched\" from");
    println!(" \"dispatched but produced no mint\" from \"cooled down\".");

    let diags = vec![
        diagnose("OQ#1", build_long_stream(), 1000),
        diagnose("long5k", build_5k_stream(), 1500),
        diagnose("narrow_a", build_narrow_a_stream(), 500),
        diagnose("OQ#2", build_oq2_stream(), 4500),
    ];

    println!();
    println!(" Action-count summary table:");
    println!(
        " {:<10} {:>6} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9} {:>5} {:>5}",
        "substrate", "ticks", "episodes", "DP_count", "DP_pos",
        "DT_count", "DT_pos", "DP_rate", "axs", "ths",
    );
    for d in &diags {
        let dp_rate = if d.discover_patterns_count > 0 {
            d.discover_patterns_positive as f64 / d.discover_patterns_count as f64
        } else { 0.0 };
        println!(
            " {:<10} {:>6} {:>9} {:>9} {:>9} {:>9} {:>9} {:>9.3} {:>5} {:>5}",
            d.label, d.ticks, d.total_episodes,
            d.discover_patterns_count, d.discover_patterns_positive,
            d.discover_theory_count, d.discover_theory_positive,
            dp_rate,
            d.final_axioms, d.final_theories,
        );
    }

    // Per-substrate breakdown.
    println!();
    println!(" Per-substrate detail:");
    for d in &diags {
        println!();
        println!(" {} ({} ticks):", d.label, d.ticks);
        println!("   total episodes:           {}", d.total_episodes);
        println!("   DiscoverPatterns:         {} ({} positive-delta)",
                 d.discover_patterns_count, d.discover_patterns_positive);
        println!("   DiscoverTheory:           {} ({} positive-delta)",
                 d.discover_theory_count, d.discover_theory_positive);
        if !d.other_action_counts.is_empty() {
            println!("   other actions:");
            for (name, c) in &d.other_action_counts {
                println!("     {:<32} {}", name, c);
            }
        }
        println!("   final state: {} axioms, {} theories, {} patterns",
                 d.final_axioms, d.final_theories, d.final_patterns);
    }

    // Verdict.
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Diagnostic verdict");
    println!("════════════════════════════════════════════════════════");
    let total_dp: u64 = diags.iter().map(|d| d.discover_patterns_count).sum();
    let total_dp_positive: u64 = diags.iter().map(|d| d.discover_patterns_positive).sum();
    let total_episodes: u64 = diags.iter().map(|d| d.total_episodes).sum();
    let total_patterns: usize = diags.iter().map(|d| d.final_patterns).sum();

    let dp_share = if total_episodes > 0 {
        total_dp as f64 / total_episodes as f64
    } else { 0.0 };
    let dp_yield = if total_dp > 0 {
        total_dp_positive as f64 / total_dp as f64
    } else { 0.0 };

    println!(" Total episodes across all substrates: {}", total_episodes);
    println!(" Total DiscoverPatterns dispatches:    {} ({:.1}% of episodes)",
             total_dp, dp_share * 100.0);
    println!(" Total positive-delta DP outcomes:     {} ({:.1}% yield rate)",
             total_dp_positive, dp_yield * 100.0);
    println!(" Total final patterns minted by runtime: {}", total_patterns);

    println!();
    if total_dp == 0 {
        println!(" → DP NEVER FIRED. The scheduler's frontier or priority logic");
        println!("   filters DiscoverPatterns out completely. ADR 0075 piece 2 is");
        println!("   strictly needed to make the kernel reachable from runtime.");
    } else if total_patterns == 0 {
        println!(" → DP FIRED BUT PRODUCED NOTHING. The kernel is reachable but");
        println!("   the dispatch parameters (sample_count, top_m, etc.) are too");
        println!("   conservative to mint patterns at small DP counts. Either");
        println!("   raise per-call budget or raise DP frequency.");
    } else if total_dp < total_episodes / 20 {
        println!(" → DP IS RARE (<5% of episodes). The kernel works when called,");
        println!("   but scheduler priority undervalues it. ADR 0075 piece 2");
        println!("   should bump priority so DP runs at, say, 10-20% of episodes.");
    } else {
        println!(" → DP IS ALREADY ACTIVE. The kernel is integrated; further");
        println!("   tuning is fine-grained, not foundational.");
    }

    println!();
    println!("--- end ---");
}
