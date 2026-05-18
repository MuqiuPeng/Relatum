//! Multi-family scan — Phase 1.D Round 5 follow-up.
//!
//! Round 4 multi-seed scan showed the canonical-suite is too
//! heterogeneous to serve as a within-family baseline
//! (std=0.34, range [0, 1]). This scan extends to additional
//! random-graph families (ER, BA, SBM) to test whether ANY
//! structural-graph family on which v2 mints canonicals produces
//! a within-family Jaccard high enough to anchor a substrate-
//! sensitivity claim.
//!
//! Hypothesis to test:
//!   H_some_family_works:  At least one of {ER, BA, SBM} produces
//!                         within-family Jaccard > 0.7 AND
//!                         cross-Jaccard (to canonical suite or
//!                         other families) < 0.4. If true, the
//!                         substrate-sensitivity claim can be
//!                         restated NARROWLY for that specific family.
//!   H_retraction_global:  All families show the same pattern as
//!                         the canonical suite: within-Jaccard
//!                         broad, cross-Jaccard not meaningfully
//!                         lower. Retraction generalizes; v2 is
//!                         indistinguishable from subgraph census
//!                         across all structural families tested.
//!
//! Method:
//!   1. Build 4 canonical-suite RSets (existing).
//!   2. Build 6 seeds × 4 generative families (ER, BA, SBM, synth-DAG) = 24 RSets.
//!   3. autonomous_pass(sizes 2-3, saturation budget) on each.
//!   4. All pairwise Jaccards.
//!   5. Per-family within-Jaccard distribution (mean, std).
//!   6. All cross-family pair distributions.

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
const N_NODES: usize = 80;
const SEEDS_PER_FAMILY: usize = 6;

const SEEDS: [u64; SEEDS_PER_FAMILY] = [
    0xCAFEBABE_DEADBEEF,
    0x12345678_9ABCDEF0,
    0xDEADC0DE_BAADF00D,
    0xFEEDFACE_8BADF00D,
    0xC0FFEE42_42424242,
    0xABABABAB_CDCDCDCD,
];

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

// xorshift RNG step.
fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Erdős–Rényi G(n, p) directed. Each ordered pair (i, j), i!=j, has
/// independent prob p of having edge i→j. Target ~270 edges to match
/// synth-DAG density: p ≈ 270 / (80*79) ≈ 0.043.
fn build_er(seed: u64) -> Vec<(String, String)> {
    let p_threshold: u64 = (u64::MAX as f64 * 0.043) as u64;
    let token = |i: usize| format!("er_{:04}", i);
    let mut state = seed;
    let mut edges: HashSet<(String, String)> = HashSet::new();
    for i in 0..N_NODES {
        for j in 0..N_NODES {
            if i == j { continue; }
            if xorshift(&mut state) < p_threshold {
                edges.insert((token(i), token(j)));
            }
        }
    }
    edges.into_iter().collect()
}

/// Barabási–Albert directed: each new node forms m=3 outgoing edges
/// to existing nodes chosen proportional to (in-degree + 1).
fn build_ba(seed: u64) -> Vec<(String, String)> {
    let m = 3usize;
    let token = |i: usize| format!("ba_{:04}", i);
    let mut state = seed;
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let mut in_deg: Vec<usize> = vec![1; N_NODES]; // +1 smoothing

    // Seed: m+1 nodes fully connected outgoing to each other.
    for i in 0..=m {
        for j in 0..=m {
            if i != j {
                edges.insert((token(i), token(j)));
                in_deg[j] += 1;
            }
        }
    }

    for i in (m + 1)..N_NODES {
        let mut chosen: HashSet<usize> = HashSet::new();
        while chosen.len() < m {
            let total: usize = (0..i).map(|j| in_deg[j]).sum();
            let pick = (xorshift(&mut state) as usize) % total;
            let mut acc = 0usize;
            for j in 0..i {
                acc += in_deg[j];
                if acc > pick {
                    if chosen.insert(j) {
                        edges.insert((token(i), token(j)));
                        in_deg[j] += 1;
                    }
                    break;
                }
            }
        }
    }
    edges.into_iter().collect()
}

/// Stochastic Block Model directed: 4 blocks of 20 nodes each.
/// p_within = 0.10, p_cross = 0.02. Target ~248 edges.
fn build_sbm(seed: u64) -> Vec<(String, String)> {
    let block_size = 20usize;
    let n_blocks = N_NODES / block_size;
    let p_within: u64 = (u64::MAX as f64 * 0.10) as u64;
    let p_cross: u64 = (u64::MAX as f64 * 0.02) as u64;
    let token = |i: usize| format!("sbm_{:04}", i);
    let block_of = |i: usize| i / block_size;
    let mut state = seed;
    let mut edges: HashSet<(String, String)> = HashSet::new();
    for i in 0..N_NODES {
        for j in 0..N_NODES {
            if i == j { continue; }
            let threshold = if block_of(i) == block_of(j) { p_within } else { p_cross };
            if xorshift(&mut state) < threshold {
                edges.insert((token(i), token(j)));
            }
        }
    }
    let _ = n_blocks;
    edges.into_iter().collect()
}

/// Synth-DAG (from Round 1+), parameterized.
fn build_synth_dag(seed: u64) -> Vec<(String, String)> {
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let mut state = seed;
    let token = |i: usize| format!("dag_{:04}", i);

    for i in 0..20 {
        let deps = (xorshift(&mut state) as usize) % 2 + 1;
        for _ in 0..deps {
            let target = (xorshift(&mut state) as usize) % i.max(1);
            if target != i {
                edges.insert((token(i), token(target)));
            }
        }
    }
    for i in 20..50 {
        let deps = (xorshift(&mut state) as usize) % 3 + 2;
        for _ in 0..deps {
            let target = (xorshift(&mut state) as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    for i in 50..N_NODES {
        let deps = (xorshift(&mut state) as usize) % 4 + 2;
        for _ in 0..deps {
            let target = (xorshift(&mut state) as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    for cluster_start in (0..N_NODES).step_by(15) {
        let cluster_end = (cluster_start + 5).min(N_NODES);
        for i in cluster_start..cluster_end {
            for j in cluster_start..cluster_end {
                if i != j && (xorshift(&mut state) as usize) % 2 == 0 {
                    edges.insert((token(i), token(j)));
                }
            }
        }
    }
    edges.into_iter().collect()
}

fn edges_to_rset(label: &str, edges: &[(String, String)]) -> RSet {
    let mut tsv = format!("# {} substrate\n", label);
    for (a, b) in edges {
        tsv.push_str(&format!("{}\t{}\n", a, b));
    }
    RSet::from_text(&tsv).expect("from_text")
}

fn build_canonical_suite_rset(
    label: &str,
    stream: Vec<(u64, relatum_v2::runtime::Event)>,
) -> RSet {
    let t0 = std::time::Instant::now();
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(TICKS_PER_SUBSTRATE);
    let t_stream = t0.elapsed();
    let mut rset = rt.rset;
    for size in 2..=3 {
        let t1 = std::time::Instant::now();
        run_pass_on_rset(&mut rset, size);
        println!("    [{}] size={} pass took {:.1}s",
                 label, size, t1.elapsed().as_secs_f64());
    }
    println!("    [{}] {} R-instances, {} canonicals (stream={:.1}s)",
             label, rset.len(), collect_canonicals(&rset).len(),
             t_stream.as_secs_f64());
    use std::io::Write;
    std::io::stdout().flush().ok();
    rset
}

fn build_generated_rset(label: &str, edges: &[(String, String)]) -> HashSet<CanonicalForm> {
    let t0 = std::time::Instant::now();
    let mut rset = edges_to_rset(label, edges);
    for size in 2..=3 {
        let t1 = std::time::Instant::now();
        run_pass_on_rset(&mut rset, size);
        println!("    [{}] size={} pass took {:.1}s",
                 label, size, t1.elapsed().as_secs_f64());
    }
    let cset = collect_canonicals(&rset);
    println!("    [{}] {} edges, {} canonicals (total {:.1}s)",
             label, edges.len(), cset.len(), t0.elapsed().as_secs_f64());
    use std::io::Write;
    std::io::stdout().flush().ok();
    cset
}

fn jaccard(a: &HashSet<CanonicalForm>, b: &HashSet<CanonicalForm>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    inter as f64 / union as f64
}

fn summary(name: &str, values: &[f64]) {
    if values.is_empty() {
        println!("   {:>22}: (empty)", name);
        return;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std = var.sqrt();
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    println!("   {:>22}: N={:>3} mean={:.4} std={:.4} [{:.4}, {:.4}]",
             name, values.len(), mean, std, min, max);
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Multi-family scan — Phase 1.D Round 5 follow-up");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Budget: sample_count={}, top_m={}", SAMPLE_COUNT, TOP_M);
    println!(" Families: canonical-suite (4 fixed) + ER/BA/SBM/synth-DAG × {} seeds",
             SEEDS_PER_FAMILY);
    println!();

    // Canonical suite.
    println!(" Building canonical-suite RSets...");
    let canonical_suite: Vec<(&str, HashSet<CanonicalForm>)> = vec![
        ("OQ#1", collect_canonicals(&build_canonical_suite_rset("OQ#1", build_long_stream()))),
        ("narrow_a", collect_canonicals(&build_canonical_suite_rset("narrow_a", build_narrow_a_stream()))),
        ("OQ#2", collect_canonicals(&build_canonical_suite_rset("OQ#2", build_oq2_stream()))),
        ("long5k", collect_canonicals(&build_canonical_suite_rset("long5k", build_5k_stream()))),
    ];

    // Generative families.
    println!();
    println!(" Building ER instances (n={}, p~0.043)...", N_NODES);
    let mut er_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_er(seed);
        er_sets.push(build_generated_rset(&format!("ER_{}", i), &edges));
    }

    println!();
    println!(" BA instances SKIPPED at n={} m=3: single-instance timing in",
             N_NODES);
    println!(" prior partial run showed size=3 autonomous_pass took >25 min");
    println!(" per BA instance due to power-law hub structure exploding the");
    println!(" subgraph-sampling space. Documented as separate finding; see");
    println!(" result doc §N. v2's pattern discovery does NOT scale on");
    println!(" power-law graphs at saturation budget on n=80.");
    let ba_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    let _ = build_ba; // silence unused-fn warning

    println!();
    println!(" Building SBM instances (n={}, 4 blocks of 20)...", N_NODES);
    let mut sbm_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_sbm(seed);
        sbm_sets.push(build_generated_rset(&format!("SBM_{}", i), &edges));
    }

    println!();
    println!(" Building synth-DAG instances (layered+clustered)...");
    let mut dag_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_synth_dag(seed);
        dag_sets.push(build_generated_rset(&format!("DAG_{}", i), &edges));
    }

    // Within-family Jaccards.
    fn within(sets: &[HashSet<CanonicalForm>]) -> Vec<f64> {
        let mut out = Vec::new();
        for i in 0..sets.len() {
            for j in (i + 1)..sets.len() {
                out.push(jaccard(&sets[i], &sets[j]));
            }
        }
        out
    }
    let within_canonical: Vec<f64> = {
        let mut out = Vec::new();
        for i in 0..canonical_suite.len() {
            for j in (i + 1)..canonical_suite.len() {
                out.push(jaccard(&canonical_suite[i].1, &canonical_suite[j].1));
            }
        }
        out
    };
    let within_er = within(&er_sets);
    let within_ba = within(&ba_sets);
    let within_sbm = within(&sbm_sets);
    let within_dag = within(&dag_sets);

    // Cross-family Jaccards (all pairs).
    fn cross(a: &[HashSet<CanonicalForm>], b: &[HashSet<CanonicalForm>]) -> Vec<f64> {
        let mut out = Vec::new();
        for x in a {
            for y in b {
                out.push(jaccard(x, y));
            }
        }
        out
    }
    let cs_only: Vec<HashSet<CanonicalForm>> =
        canonical_suite.iter().map(|(_, s)| s.clone()).collect();
    let cross_cs_er = cross(&cs_only, &er_sets);
    let cross_cs_ba = cross(&cs_only, &ba_sets);
    let cross_cs_sbm = cross(&cs_only, &sbm_sets);
    let cross_cs_dag = cross(&cs_only, &dag_sets);
    let cross_er_ba = cross(&er_sets, &ba_sets);
    let cross_er_sbm = cross(&er_sets, &sbm_sets);
    let cross_er_dag = cross(&er_sets, &dag_sets);
    let cross_ba_sbm = cross(&ba_sets, &sbm_sets);
    let cross_ba_dag = cross(&ba_sets, &dag_sets);
    let cross_sbm_dag = cross(&sbm_sets, &dag_sets);

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Per-family WITHIN-Jaccard distributions");
    println!("════════════════════════════════════════════════════════");
    summary("canonical-suite (C(4,2))", &within_canonical);
    summary("ER (C(6,2))", &within_er);
    summary("BA (C(6,2))", &within_ba);
    summary("SBM (C(6,2))", &within_sbm);
    summary("synth-DAG (C(6,2))", &within_dag);

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" CROSS-family Jaccard distributions");
    println!("════════════════════════════════════════════════════════");
    summary("canonical × ER", &cross_cs_er);
    summary("canonical × BA", &cross_cs_ba);
    summary("canonical × SBM", &cross_cs_sbm);
    summary("canonical × DAG", &cross_cs_dag);
    summary("ER × BA", &cross_er_ba);
    summary("ER × SBM", &cross_er_sbm);
    summary("ER × DAG", &cross_er_dag);
    summary("BA × SBM", &cross_ba_sbm);
    summary("BA × DAG", &cross_ba_dag);
    summary("SBM × DAG", &cross_sbm_dag);

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" H_some_family_works test per generative family");
    println!("════════════════════════════════════════════════════════");
    println!(" Required for H1 on a family: within > 0.7 AND all crosses < 0.4");
    println!();
    let families: Vec<(&str, &Vec<f64>, Vec<&Vec<f64>>)> = vec![
        ("ER", &within_er,
            vec![&cross_cs_er, &cross_er_ba, &cross_er_sbm, &cross_er_dag]),
        ("BA", &within_ba,
            vec![&cross_cs_ba, &cross_er_ba, &cross_ba_sbm, &cross_ba_dag]),
        ("SBM", &within_sbm,
            vec![&cross_cs_sbm, &cross_er_sbm, &cross_ba_sbm, &cross_sbm_dag]),
        ("synth-DAG", &within_dag,
            vec![&cross_cs_dag, &cross_er_dag, &cross_ba_dag, &cross_sbm_dag]),
    ];
    for (name, within, crosses) in &families {
        let w_mean = if within.is_empty() { 0.0 }
            else { within.iter().sum::<f64>() / within.len() as f64 };
        let max_cross_mean = crosses.iter()
            .map(|c| if c.is_empty() { 0.0 } else { c.iter().sum::<f64>() / c.len() as f64 })
            .fold(0.0f64, f64::max);
        let within_ok = w_mean > 0.7;
        let cross_ok = max_cross_mean < 0.4;
        let pass = within_ok && cross_ok;
        println!("   {:>10}: within_mean={:.4} (>{:.1}: {}); max_cross_mean={:.4} (<{:.1}: {}) → H1: {}",
                 name, w_mean, 0.7, if within_ok { "✓" } else { "✗" },
                 max_cross_mean, 0.4, if cross_ok { "✓" } else { "✗" },
                 if pass { "supported FOR THIS FAMILY" } else { "not supported" });
    }
    println!();
    println!(" canonical-suite verdict (for reference):");
    let cs_w_mean = within_canonical.iter().sum::<f64>() / within_canonical.len() as f64;
    println!("   within_mean={:.4} → already known to fail H1 threshold (Round 4)",
             cs_w_mean);

    println!();
    println!("--- end ---");
}
