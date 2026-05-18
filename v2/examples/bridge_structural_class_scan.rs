//! Round 6 — structural-class scan.
//!
//! Round 5 finding: v2 at sizes 2-3 on RANDOM-graph families (ER, SBM,
//! synth-DAG) produces essentially identical canonical-form sets
//! (cross-family Jaccard ≈ within-family Jaccard ≈ 0.9). Substrate-
//! sensitivity within the "random" class is null.
//!
//! Open question: does v2 distinguish STRUCTURALLY DIFFERENT classes?
//! Tree (no cycles) and bipartite (no odd cycles) have provably
//! different size-2/3 canonical censuses from generic random graphs:
//!   - tree: no 3-cycle, no bidirectional pair, sparse
//!   - bipartite: no 3-cycle, no self-loop, only even-length paths
//!   - random (ER/SBM/DAG): all motifs present
//!
//! Hypothesis to test (H_class_sensitive):
//!   v2 distinguishes structural classes but not within-random-class:
//!     - within(tree pair) high, cross(tree, random) low
//!     - within(bipartite pair) high, cross(bipartite, random) low
//!     - within(canonical-suite) heterogeneous (Round 4)
//!     - cross(canonical-suite, any random) ≈ 0.1 (Round 5)
//!     - cross(tree, bipartite) low
//!
//! Method: 6 seeds × 3 generated families (TREE, BIPARTITE, synth-DAG)
//! + 4 canonical-suite. All pairwise Jaccards at sizes 2-3 saturation.
//!
//! synth-DAG kept as random-class representative; ER and SBM omitted
//! since Round 5 established they're equivalent to DAG canonical-wise.

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

fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

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

/// Random tree: each node i (i > 0) attaches to a random earlier
/// node as parent. Forms a directed rooted tree of n nodes, n-1 edges.
/// Then add ~80 random forward edges (i → j where i < j) for a
/// "tree + forward DAG noise" structure that still has no cycles
/// but more density. Total ~160 edges (vs synth-DAG's ~270).
fn build_tree(seed: u64) -> Vec<(String, String)> {
    let mut state = seed;
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let token = |i: usize| format!("tree_{:04}", i);

    // Tree backbone: n-1 edges.
    for i in 1..N_NODES {
        let parent = (xorshift(&mut state) as usize) % i;
        edges.insert((token(parent), token(i)));
    }
    // Add forward edges (i < j) to increase density without cycles.
    let extras = 80;
    for _ in 0..extras {
        let i = (xorshift(&mut state) as usize) % (N_NODES - 1);
        let j = i + 1 + ((xorshift(&mut state) as usize) % (N_NODES - i - 1));
        edges.insert((token(i), token(j)));
    }
    edges.into_iter().collect()
}

/// Bipartite directed: split N into two sets L (0..40), R (40..80).
/// Edges only from L→R, density p=0.10 → expected 40*40*0.10 = 160 edges.
/// No L→L, no R→R, no R→L. No self-loops. No 3-cycles possible.
fn build_bipartite(seed: u64) -> Vec<(String, String)> {
    let mut state = seed;
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let p_threshold: u64 = (u64::MAX as f64 * 0.10) as u64;
    let half = N_NODES / 2;
    let token = |i: usize| format!("bp_{:04}", i);
    for i in 0..half {
        for j in half..N_NODES {
            if xorshift(&mut state) < p_threshold {
                edges.insert((token(i), token(j)));
            }
        }
    }
    edges.into_iter().collect()
}

/// Synth-DAG from earlier rounds (layered random with clusters).
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
        run_pass_on_rset(&mut rset, size);
    }
    println!("    [{}] {} R-instances, {} canonicals (build {:.1}s)",
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
        run_pass_on_rset(&mut rset, size);
    }
    let cset = collect_canonicals(&rset);
    println!("    [{}] {} edges, {} canonicals (total {:.1}s)",
             label, edges.len(), cset.len(), t0.elapsed().as_secs_f64());
    use std::io::Write;
    std::io::stdout().flush().ok();
    cset
}

fn jaccard(a: &HashSet<CanonicalForm>, b: &HashSet<CanonicalForm>) -> f64 {
    if a.is_empty() && b.is_empty() { return 1.0; }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    inter as f64 / union as f64
}

fn summary(name: &str, values: &[f64]) {
    if values.is_empty() {
        println!("   {:>30}: (empty)", name);
        return;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let var = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std = var.sqrt();
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    println!("   {:>30}: N={:>3} mean={:.4} std={:.4} [{:.4}, {:.4}]",
             name, values.len(), mean, std, min, max);
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Structural-class scan — Phase 1.D Round 6");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Tests whether v2 at sizes 2-3 distinguishes structural");
    println!(" CLASSES (tree, bipartite, random-DAG, structured-stream)");
    println!(" even though Round 5 showed it cannot distinguish WITHIN");
    println!(" the random-graph class (ER ≈ SBM ≈ DAG).");
    println!();
    println!(" Budget: sample_count={}, top_m={}", SAMPLE_COUNT, TOP_M);
    println!(" Seeds per generated family: {}", SEEDS_PER_FAMILY);
    println!();

    // Canonical-suite (carry over).
    println!(" Building canonical-suite RSets...");
    let canonical_suite: Vec<(&str, HashSet<CanonicalForm>)> = vec![
        ("OQ#1", collect_canonicals(&build_canonical_suite_rset("OQ#1", build_long_stream()))),
        ("narrow_a", collect_canonicals(&build_canonical_suite_rset("narrow_a", build_narrow_a_stream()))),
        ("OQ#2", collect_canonicals(&build_canonical_suite_rset("OQ#2", build_oq2_stream()))),
        ("long5k", collect_canonicals(&build_canonical_suite_rset("long5k", build_5k_stream()))),
    ];

    // TREE family.
    println!();
    println!(" Building TREE instances (rooted tree + forward DAG edges)...");
    let mut tree_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_tree(seed);
        tree_sets.push(build_generated_rset(&format!("TREE_{}", i), &edges));
    }

    // BIPARTITE family.
    println!();
    println!(" Building BIPARTITE instances (40+40, p_cross=0.10)...");
    let mut bp_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_bipartite(seed);
        bp_sets.push(build_generated_rset(&format!("BP_{}", i), &edges));
    }

    // Random class (DAG as representative; Round 5 showed ER/SBM/DAG equivalent).
    println!();
    println!(" Building synth-DAG instances (random-class representative)...");
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
    let cs_only: Vec<HashSet<CanonicalForm>> =
        canonical_suite.iter().map(|(_, s)| s.clone()).collect();
    let within_cs = within(&cs_only);
    let within_tree = within(&tree_sets);
    let within_bp = within(&bp_sets);
    let within_dag = within(&dag_sets);

    // Cross-family.
    fn cross(a: &[HashSet<CanonicalForm>], b: &[HashSet<CanonicalForm>]) -> Vec<f64> {
        let mut out = Vec::new();
        for x in a { for y in b { out.push(jaccard(x, y)); } }
        out
    }
    let cross_cs_tree = cross(&cs_only, &tree_sets);
    let cross_cs_bp = cross(&cs_only, &bp_sets);
    let cross_cs_dag = cross(&cs_only, &dag_sets);
    let cross_tree_bp = cross(&tree_sets, &bp_sets);
    let cross_tree_dag = cross(&tree_sets, &dag_sets);
    let cross_bp_dag = cross(&bp_sets, &dag_sets);

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" WITHIN-class Jaccard distributions");
    println!("════════════════════════════════════════════════════════");
    summary("canonical-suite (C(4,2))", &within_cs);
    summary("TREE (C(6,2))", &within_tree);
    summary("BIPARTITE (C(6,2))", &within_bp);
    summary("synth-DAG (C(6,2))", &within_dag);

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" CROSS-class Jaccard distributions");
    println!("════════════════════════════════════════════════════════");
    summary("canonical × TREE", &cross_cs_tree);
    summary("canonical × BIPARTITE", &cross_cs_bp);
    summary("canonical × synth-DAG", &cross_cs_dag);
    summary("TREE × BIPARTITE", &cross_tree_bp);
    summary("TREE × synth-DAG", &cross_tree_dag);
    summary("BIPARTITE × synth-DAG", &cross_bp_dag);

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" H_class_sensitive test per class");
    println!("════════════════════════════════════════════════════════");
    println!(" Required: within > 0.7 AND max cross < 0.4");
    println!();
    let cases: Vec<(&str, &Vec<f64>, Vec<&Vec<f64>>)> = vec![
        ("TREE", &within_tree,
            vec![&cross_cs_tree, &cross_tree_bp, &cross_tree_dag]),
        ("BIPARTITE", &within_bp,
            vec![&cross_cs_bp, &cross_tree_bp, &cross_bp_dag]),
        ("synth-DAG", &within_dag,
            vec![&cross_cs_dag, &cross_tree_dag, &cross_bp_dag]),
    ];
    for (name, w, xs) in &cases {
        let w_mean = if w.is_empty() { 0.0 } else { w.iter().sum::<f64>() / w.len() as f64 };
        let max_x_mean = xs.iter()
            .map(|x| if x.is_empty() { 0.0 } else { x.iter().sum::<f64>() / x.len() as f64 })
            .fold(0.0f64, f64::max);
        let win_ok = w_mean > 0.7;
        let cross_ok = max_x_mean < 0.4;
        let pass = win_ok && cross_ok;
        println!("   {:>12}: within={:.4} ({}); max_cross={:.4} ({}) → H1: {}",
                 name, w_mean, if win_ok { "✓" } else { "✗" },
                 max_x_mean, if cross_ok { "✓" } else { "✗" },
                 if pass { "SUPPORTED FOR THIS CLASS" } else { "not supported" });
    }
    println!();
    let cs_w_mean = if within_cs.is_empty() { 0.0 } else
        { within_cs.iter().sum::<f64>() / within_cs.len() as f64 };
    println!("   canonical-suite within={:.4} (known low; not a variance-bounded family)",
             cs_w_mean);

    println!();
    println!("--- end ---");
}
