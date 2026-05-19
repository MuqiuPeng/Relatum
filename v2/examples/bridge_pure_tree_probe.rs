//! Pure-tree probe — Round 6 follow-up.
//!
//! Round 6 (`bridge_structural_class_scan`) found that
//! "tree + forward-DAG noise" overlapped heavily with synth-DAG
//! (cross-Jaccard 0.78 at sizes 2-3, n=80, saturation budget).
//! That experiment's TREE builder added 80 random forward edges
//! on top of a rooted-tree backbone — contaminating the tree
//! signature with DAG-like merge / cluster motifs.
//!
//! This probe asks: does a PURE rooted tree (only n-1 backbone
//! edges, no forward-DAG noise) cleanly pass H1 (within > 0.7,
//! max cross < 0.4) against random-graph baselines?
//!
//! Sizes 2-4 at n=40 + n=80 saturation budget (top_m=100 to
//! avoid Round 7's truncation artifact). 3 seeds per family.

use relatum_v2::{
    AutonomousConfig, CanonicalForm, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig,
};
use std::collections::HashSet;

const SAMPLE_COUNT: usize = 400;
const TOP_M: usize = 100;
const RNG_SEED: u64 = 0xC0FFEE;
const SEEDS_PER_FAMILY: usize = 3;

const SEEDS: [u64; SEEDS_PER_FAMILY] = [
    0xCAFEBABE_DEADBEEF,
    0x12345678_9ABCDEF0,
    0xDEADC0DE_BAADF00D,
];

fn xorshift(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn run_pass(rset: &mut RSet, size: usize) {
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

/// Pure rooted tree: n-1 edges, each non-root attaches to a single
/// random earlier parent. No forward-DAG noise. Acyclic by
/// construction; merge motifs only appear if multiple kids share
/// parent (which DOES happen randomly).
fn build_pure_tree(seed: u64, n: usize) -> Vec<(String, String)> {
    let mut state = seed;
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let token = |i: usize| format!("tree_{:04}", i);
    for i in 1..n {
        let parent = (xorshift(&mut state) as usize) % i;
        edges.insert((token(parent), token(i)));
    }
    edges.into_iter().collect()
}

/// Synth-DAG (random class representative).
fn build_synth_dag(seed: u64, n: usize) -> Vec<(String, String)> {
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let mut state = seed;
    let token = |i: usize| format!("dag_{:04}", i);
    let p1 = n / 4;
    let p2 = n / 2;
    for i in 0..p1 {
        let deps = (xorshift(&mut state) as usize) % 2 + 1;
        for _ in 0..deps {
            let target = (xorshift(&mut state) as usize) % i.max(1);
            if target != i {
                edges.insert((token(i), token(target)));
            }
        }
    }
    for i in p1..p2 {
        let deps = (xorshift(&mut state) as usize) % 3 + 2;
        for _ in 0..deps {
            let target = (xorshift(&mut state) as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    for i in p2..n {
        let deps = (xorshift(&mut state) as usize) % 4 + 2;
        for _ in 0..deps {
            let target = (xorshift(&mut state) as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    let step = (n / 5).max(1);
    for cluster_start in (0..n).step_by(step) {
        let cluster_end = (cluster_start + 3).min(n);
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

/// Bipartite (Round 6 reference — first H1-passing family).
fn build_bipartite(seed: u64, n: usize) -> Vec<(String, String)> {
    let mut state = seed;
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let p_threshold: u64 = (u64::MAX as f64 * 0.15) as u64;
    let half = n / 2;
    let token = |i: usize| format!("bp_{:04}", i);
    for i in 0..half {
        for j in half..n {
            if xorshift(&mut state) < p_threshold {
                edges.insert((token(i), token(j)));
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

fn build_at_size(
    label: &str,
    edges: &[(String, String)],
    size: usize,
) -> HashSet<CanonicalForm> {
    let t0 = std::time::Instant::now();
    let mut rset = edges_to_rset(label, edges);
    run_pass(&mut rset, size);
    let cset = collect_canonicals(&rset);
    println!("    [{}] size={} {} edges, {} canonicals ({:.1}s)",
             label, size, edges.len(), cset.len(),
             t0.elapsed().as_secs_f64());
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
    println!("   {:>30}: N={:>2} mean={:.4} std={:.4} [{:.4}, {:.4}]",
             name, values.len(), mean, std, min, max);
}

fn run_at(n: usize, size: usize) {
    println!();
    println!(" === n={} size={} ===", n, size);
    let mut tree_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_pure_tree(seed, n);
        tree_sets.push(build_at_size(&format!("TREE_{}", i), &edges, size));
    }
    let mut dag_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_synth_dag(seed, n);
        dag_sets.push(build_at_size(&format!("DAG_{}", i), &edges, size));
    }
    let mut bp_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_bipartite(seed, n);
        bp_sets.push(build_at_size(&format!("BP_{}", i), &edges, size));
    }
    fn within(sets: &[HashSet<CanonicalForm>]) -> Vec<f64> {
        let mut out = Vec::new();
        for i in 0..sets.len() {
            for j in (i + 1)..sets.len() {
                out.push(jaccard(&sets[i], &sets[j]));
            }
        }
        out
    }
    fn cross(a: &[HashSet<CanonicalForm>], b: &[HashSet<CanonicalForm>]) -> Vec<f64> {
        let mut out = Vec::new();
        for x in a { for y in b { out.push(jaccard(x, y)); } }
        out
    }
    let w_tree = within(&tree_sets);
    let w_dag = within(&dag_sets);
    let w_bp = within(&bp_sets);
    let c_tree_dag = cross(&tree_sets, &dag_sets);
    let c_tree_bp = cross(&tree_sets, &bp_sets);
    let c_dag_bp = cross(&dag_sets, &bp_sets);
    println!();
    summary("WITHIN pure-TREE", &w_tree);
    summary("WITHIN synth-DAG", &w_dag);
    summary("WITHIN BIPARTITE", &w_bp);
    summary("CROSS  TREE × DAG", &c_tree_dag);
    summary("CROSS  TREE × BP", &c_tree_bp);
    summary("CROSS  DAG × BP", &c_dag_bp);
    // H1 verdict per family.
    let w_tree_mean = if w_tree.is_empty() { 0.0 } else { w_tree.iter().sum::<f64>() / w_tree.len() as f64 };
    let c_tree_dag_mean = if c_tree_dag.is_empty() { 0.0 } else { c_tree_dag.iter().sum::<f64>() / c_tree_dag.len() as f64 };
    let c_tree_bp_mean = if c_tree_bp.is_empty() { 0.0 } else { c_tree_bp.iter().sum::<f64>() / c_tree_bp.len() as f64 };
    let max_tree_cross = c_tree_dag_mean.max(c_tree_bp_mean);
    let tree_h1 = w_tree_mean > 0.7 && max_tree_cross < 0.4;
    println!();
    println!(" PURE-TREE H1 at n={} size={}: within={:.4} max_cross={:.4} → {}",
             n, size, w_tree_mean, max_tree_cross,
             if tree_h1 { "SUPPORTED ✓" } else { "not supported" });
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Pure-tree probe — Round 6 follow-up");
    println!("════════════════════════════════════════════════════════");
    println!(" Tests whether a PURE rooted tree (no forward-DAG noise)");
    println!(" passes H1 cleanly. Round 6's TREE-with-noise had");
    println!(" TREE × synth-DAG = 0.78 (heavy overlap).");
    println!();
    println!(" Budget: sample_count={}, top_m={}", SAMPLE_COUNT, TOP_M);
    println!(" Seeds per family: {}", SEEDS_PER_FAMILY);

    for &n in &[40usize, 80] {
        for &size in &[2usize, 3, 4] {
            run_at(n, size);
        }
    }

    println!();
    println!("--- end ---");
}
