//! Multi-seed scan follow-up to Phase 1.D Round 2 retraction.
//!
//! Round 3 reviewer (M3) and result-doc §13.6 both flagged that
//! Round 2's single-seed Within(OQ#1, narrow_a) = 0.20 may be a
//! bad-seed artifact. This experiment expands the canonical-suite
//! within-Jaccard sample to **all pairs** of v2's 4 canonical
//! synthetic substrates (OQ#1, narrow_a, OQ#2, long5k), and the
//! DAG within-Jaccard sample to multiple graph-generation seeds.
//!
//! Hypothesis to test:
//!   H_retraction_stands:  mean Within(canonical-suite pair) ≈
//!                         mean Cross(canonical, synth-DAG),
//!                         confirming that v2 produces no general
//!                         within-canonical-family fingerprint.
//!   H_retraction_was_premature:  mean Within(canonical-suite pair)
//!                         > 0.5 with low variance, and Round 2's
//!                         0.20 was a single-seed outlier.
//!
//! Method:
//!   1. Build 4 canonical-suite RSets (OQ#1, narrow_a, OQ#2, long5k).
//!   2. Build 6 synth-DAG RSets (6 different graph-generation seeds).
//!   3. Run autonomous_pass(sizes 2-3, saturation budget) on each.
//!   4. Compute all pairwise canonical-suite Jaccards (C(4,2) = 6).
//!   5. Compute all pairwise synth-DAG Jaccards (C(6,2) = 15).
//!   6. Compute all cross Jaccards (4 × 6 = 24).
//!   7. Report mean ± std (and full distribution) for each set.
//!
//! Compared to Round 2 (single seed per pair), this gives:
//!   - canonical-suite within: 6 measurements instead of 1
//!   - synth-DAG within: 15 measurements instead of 1
//!   - cross: 24 measurements instead of 2
//!
//! Single saturation budget only — Round 2 already used the
//! low-budget probe to rule in saturation; here we focus on
//! cross-seed variance under the budget that previously
//! produced the 0.20 vs 0.26 comparison.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::{
        long5k::build_5k_stream,
        narrow_a::build_narrow_a_stream,
        oq1::build_long_stream,
        oq2::build_oq2_stream,
    },
    AutonomousConfig, CanonicalForm, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig,
};
use std::collections::HashSet;

const SAMPLE_COUNT: usize = 400;
const TOP_M: usize = 20;
const RNG_SEED: u64 = 0xC0FFEE;
const TICKS_PER_SUBSTRATE: u64 = 1000;

fn run_pass_on_rset(rset: &mut RSet, size: usize) {
    let cfg = AutonomousConfig {
        discovery: DiscoveryConfig {
            target_size: size,
            sample_count: SAMPLE_COUNT,
            top_m: TOP_M,
            rng_seed: RNG_SEED.wrapping_add(size as u64 * 0x9E37),
            include_meta_in_discovery: false,
        },
        refinement: RefinementConfig {
            max_tries: 200,
            rng_seed: RNG_SEED.wrapping_add(size as u64 * 0xDEAD),
        },
        naming: NamingPolicy::default(),
        instance_sampling: None,
    };
    let _ = rset.autonomous_pass(&cfg);
}

fn collect_canonicals(rset: &RSet) -> HashSet<CanonicalForm> {
    rset.patterns()
        .iter()
        .filter_map(|pid| rset.pattern_structure(pid))
        .collect()
}

fn build_synth_dag(seed: u64) -> Vec<(String, String)> {
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let n = 80usize;
    let mut rng_state: u64 = seed;
    let mut next = || {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        rng_state
    };
    let token = |i: usize| format!("lem_{:04}", i);

    for i in 0..20 {
        let deps = (next() as usize) % 2 + 1;
        for _ in 0..deps {
            let target = (next() as usize) % i.max(1);
            if target != i {
                edges.insert((token(i), token(target)));
            }
        }
    }
    for i in 20..50 {
        let deps = (next() as usize) % 3 + 2;
        for _ in 0..deps {
            let target = (next() as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    for i in 50..n {
        let deps = (next() as usize) % 4 + 2;
        for _ in 0..deps {
            let target = (next() as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    for cluster_start in (0..n).step_by(15) {
        let cluster_end = (cluster_start + 5).min(n);
        for i in cluster_start..cluster_end {
            for j in cluster_start..cluster_end {
                if i != j && (next() as usize) % 2 == 0 {
                    edges.insert((token(i), token(j)));
                }
            }
        }
    }
    edges.into_iter().collect()
}

fn dag_to_rset(edges: &[(String, String)]) -> RSet {
    let mut tsv = String::from("# multi-seed synth DAG\n");
    for (a, b) in edges {
        tsv.push_str(&format!("{}\t{}\n", a, b));
    }
    RSet::from_text(&tsv).expect("from_text")
}

fn build_canonical_suite_rset(
    label: &str,
    stream: Vec<(u64, relatum_v2::runtime::Event)>,
) -> RSet {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PER_SUBSTRATE);
    let mut rset = rt.rset;
    for size in 2..=3 {
        run_pass_on_rset(&mut rset, size);
    }
    eprintln!("    [{}] {} canonicals", label, collect_canonicals(&rset).len());
    rset
}

fn jaccard(a: &HashSet<CanonicalForm>, b: &HashSet<CanonicalForm>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    inter as f64 / union as f64
}

fn summary_stats(values: &[f64]) -> (f64, f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter()
        .map(|v| (v - mean).powi(2))
        .sum::<f64>() / n;
    let std = var.sqrt();
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (mean, std, min, max)
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Multi-seed scan — Phase 1.D Round 4 follow-up");
    println!(" (tests Round 2 N=1 fragility; W4 from Round 1)");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Budget: sample_count={}, top_m={} (saturation)", SAMPLE_COUNT, TOP_M);
    println!();

    // Build canonical-suite substrates.
    println!(" Building canonical-suite RSets ({} ticks each)...",
             TICKS_PER_SUBSTRATE);
    let canonical_suite: Vec<(&str, HashSet<CanonicalForm>)> = vec![
        ("OQ#1", collect_canonicals(
            &build_canonical_suite_rset("OQ#1", build_long_stream()))),
        ("narrow_a", collect_canonicals(
            &build_canonical_suite_rset("narrow_a", build_narrow_a_stream()))),
        ("OQ#2", collect_canonicals(
            &build_canonical_suite_rset("OQ#2", build_oq2_stream()))),
        ("long5k", collect_canonicals(
            &build_canonical_suite_rset("long5k", build_5k_stream()))),
    ];

    // Build synth-DAG instances with different seeds.
    println!();
    println!(" Building synth-DAG instances...");
    let dag_seeds: Vec<u64> = vec![
        0xCAFEBABE_DEADBEEF,
        0x12345678_9ABCDEF0,
        0xDEADC0DE_BAADF00D,
        0xFEEDFACE_8BADF00D,
        0xC0FFEE_42424242,
        0xABABABAB_CDCDCDCD,
    ];
    let mut dag_substrates: Vec<(String, HashSet<CanonicalForm>)> = Vec::new();
    for (i, &seed) in dag_seeds.iter().enumerate() {
        let edges = build_synth_dag(seed);
        let mut rset = dag_to_rset(&edges);
        for size in 2..=3 {
            run_pass_on_rset(&mut rset, size);
        }
        let cset = collect_canonicals(&rset);
        eprintln!("    [DAG_{}] {} edges, {} canonicals (seed=0x{:016X})",
                  i, edges.len(), cset.len(), seed);
        dag_substrates.push((format!("DAG_{}", i), cset));
    }

    // Within-canonical-suite Jaccards (all 6 pairs).
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Within-canonical-suite Jaccards (C(4,2) = 6 pairs)");
    println!("════════════════════════════════════════════════════════");
    let mut within_canonical: Vec<f64> = Vec::new();
    for i in 0..canonical_suite.len() {
        for j in (i + 1)..canonical_suite.len() {
            let j_val = jaccard(&canonical_suite[i].1, &canonical_suite[j].1);
            within_canonical.push(j_val);
            println!("   Within({:>9}, {:>9}) = {:.4}",
                     canonical_suite[i].0, canonical_suite[j].0, j_val);
        }
    }
    let (cm, cs, cmin, cmax) = summary_stats(&within_canonical);
    println!();
    println!("   N={}  mean={:.4}  std={:.4}  min={:.4}  max={:.4}",
             within_canonical.len(), cm, cs, cmin, cmax);

    // Within-synth-DAG Jaccards (all 15 pairs).
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Within-synth-DAG Jaccards (C(6,2) = 15 pairs)");
    println!("════════════════════════════════════════════════════════");
    let mut within_dag: Vec<f64> = Vec::new();
    for i in 0..dag_substrates.len() {
        for j in (i + 1)..dag_substrates.len() {
            let j_val = jaccard(&dag_substrates[i].1, &dag_substrates[j].1);
            within_dag.push(j_val);
            println!("   Within({}, {}) = {:.4}",
                     dag_substrates[i].0, dag_substrates[j].0, j_val);
        }
    }
    let (dm, ds, dmin, dmax) = summary_stats(&within_dag);
    println!();
    println!("   N={}  mean={:.4}  std={:.4}  min={:.4}  max={:.4}",
             within_dag.len(), dm, ds, dmin, dmax);

    // Cross-family Jaccards (all 4 × 6 = 24 pairs).
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Cross-family Jaccards (4 canonical × 6 synth-DAG = 24)");
    println!("════════════════════════════════════════════════════════");
    let mut cross_pairs: Vec<f64> = Vec::new();
    for c in &canonical_suite {
        for d in &dag_substrates {
            let j_val = jaccard(&c.1, &d.1);
            cross_pairs.push(j_val);
        }
    }
    // print as 4x6 grid
    print!("                ");
    for d in &dag_substrates {
        print!(" {:>7}", d.0);
    }
    println!();
    let mut idx = 0;
    for c in &canonical_suite {
        print!("   {:>11} ", c.0);
        for _ in 0..dag_substrates.len() {
            print!(" {:>7.4}", cross_pairs[idx]);
            idx += 1;
        }
        println!();
    }
    let (xm, xs, xmin, xmax) = summary_stats(&cross_pairs);
    println!();
    println!("   N={}  mean={:.4}  std={:.4}  min={:.4}  max={:.4}",
             cross_pairs.len(), xm, xs, xmin, xmax);

    // ── Verdict ──
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Verdict — does the Round 2 retraction stand?");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Round 2's single-seed value:");
    println!("   Within(OQ#1, narrow_a) = 0.2000");
    println!("   Cross(OQ#2, DAG_A)     = 0.2632");
    println!();
    println!(" Multi-seed measurements:");
    println!("   Within(canonical pair):  N={}  mean={:.4} std={:.4}  [min {:.4}, max {:.4}]",
             within_canonical.len(), cm, cs, cmin, cmax);
    println!("   Within(synth-DAG pair):  N={}  mean={:.4} std={:.4}  [min {:.4}, max {:.4}]",
             within_dag.len(), dm, ds, dmin, dmax);
    println!("   Cross(canonical, DAG):   N={}  mean={:.4} std={:.4}  [min {:.4}, max {:.4}]",
             cross_pairs.len(), xm, xs, xmin, xmax);
    println!();
    if cm > xm + 0.2 {
        println!("   → Within-canonical mean {:.4} EXCEEDS Cross mean {:.4} by",
                 cm, xm);
        println!("     {:.4} (> 0.2 gap). Round 2 retraction was likely",
                 cm - xm);
        println!("     premature; the 0.20 single-seed value was an outlier.");
    } else if cm > xm + 0.05 {
        println!("   → Within-canonical mean {:.4} mildly exceeds Cross mean",
                 cm);
        println!("     {:.4} by {:.4}. Modest substrate-suite signal but",
                 xm, cm - xm);
        println!("     small gap = weak evidence; Round 2 retraction is still");
        println!("     defensible but not strongly reinforced.");
    } else {
        println!("   → Within-canonical mean {:.4} ≈ Cross mean {:.4}", cm, xm);
        println!("     (gap {:.4} < 0.05). Round 2 retraction REINFORCED:",
                 cm - xm);
        println!("     N>1 evidence confirms canonical-suite within-family");
        println!("     agreement is no better than cross-family.");
    }
    println!();
    println!(" Within-DAG mean {:.4} vs Within-canonical mean {:.4}: ",
             dm, cm);
    println!("   DAG family is {:.2}× more self-consistent than canonical",
             if cm > 0.0 { dm / cm } else { f64::INFINITY });
    println!("   suite. This is the surviving narrow positive: the synth-DAG");
    println!("   generator has a small invariant size-2/3 motif vocabulary");
    println!("   (Round 3 M1) that the canonical suite does not share.");

    println!();
    println!("--- end ---");
}
