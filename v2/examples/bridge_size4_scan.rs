//! Round 7 — sizes 4 scan.
//!
//! Round 5+6 established that v2 at sizes 2-3 saturates to a universal
//! small-motif vocabulary (13-16 canonicals per substrate; cross-class
//! Jaccard ≈ within-class Jaccard for random-graph families).
//!
//! Open question: does this saturation regime hold at size 4? If
//! within-Jaccard DROPS at size 4 (canonical space is bigger, sampling
//! doesn't saturate) AND cross-Jaccard drops MORE THAN within, then
//! substrate-sensitivity at size-4 motifs is supported. If both drop
//! together (within ≈ cross still), retraction extends to size 4.
//!
//! Method: smaller graphs (n=40) to manage discovery time at size 4
//! (size-4 candidate space at n=80 caused BA scaling explosion in
//! Round 5). 3 seeds per family × 4 families (ER, SBM, synth-DAG,
//! BIPARTITE). Saturation budget (sample_count=400, top_m=20).
//! Size 4 only — size 5/6 likely too slow at this scale.

use relatum_v2::{
    AutonomousConfig, CanonicalForm, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig,
};
use std::collections::HashSet;

// Round 8: raised TOP_M from 20 to 100 to remove truncation artifact
// observed at size 4 (Round 7: all random families produced exactly
// 20 canonicals = cap). top_m=100 is large enough that the natural
// canonical census fits within the cap for size-4 random graphs at n=40.
const SAMPLE_COUNT: usize = 400;
const TOP_M: usize = 100;
const RNG_SEED: u64 = 0xC0FFEE;
const N_NODES: usize = 40;
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

/// ER directed at p=0.05 → ~78 edges expected for n=40.
fn build_er(seed: u64) -> Vec<(String, String)> {
    let p_threshold: u64 = (u64::MAX as f64 * 0.05) as u64;
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

/// SBM: 4 blocks of 10, p_within=0.15, p_cross=0.03.
fn build_sbm(seed: u64) -> Vec<(String, String)> {
    let block_size = 10usize;
    let p_within: u64 = (u64::MAX as f64 * 0.15) as u64;
    let p_cross: u64 = (u64::MAX as f64 * 0.03) as u64;
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
    edges.into_iter().collect()
}

fn build_synth_dag(seed: u64) -> Vec<(String, String)> {
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let mut state = seed;
    let token = |i: usize| format!("dag_{:04}", i);
    let phase1 = N_NODES / 4;
    let phase2 = N_NODES / 2;
    for i in 0..phase1 {
        let deps = (xorshift(&mut state) as usize) % 2 + 1;
        for _ in 0..deps {
            let target = (xorshift(&mut state) as usize) % i.max(1);
            if target != i {
                edges.insert((token(i), token(target)));
            }
        }
    }
    for i in phase1..phase2 {
        let deps = (xorshift(&mut state) as usize) % 3 + 2;
        for _ in 0..deps {
            let target = (xorshift(&mut state) as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    for i in phase2..N_NODES {
        let deps = (xorshift(&mut state) as usize) % 4 + 2;
        for _ in 0..deps {
            let target = (xorshift(&mut state) as usize) % i;
            edges.insert((token(i), token(target)));
        }
    }
    let step = N_NODES / 5;
    for cluster_start in (0..N_NODES).step_by(step.max(1)) {
        let cluster_end = (cluster_start + 3).min(N_NODES);
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

fn build_bipartite(seed: u64) -> Vec<(String, String)> {
    let mut state = seed;
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let p_threshold: u64 = (u64::MAX as f64 * 0.15) as u64;
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

fn edges_to_rset(label: &str, edges: &[(String, String)]) -> RSet {
    let mut tsv = format!("# {} substrate\n", label);
    for (a, b) in edges {
        tsv.push_str(&format!("{}\t{}\n", a, b));
    }
    RSet::from_text(&tsv).expect("from_text")
}

fn build_generated(label: &str, edges: &[(String, String)]) -> HashSet<CanonicalForm> {
    let t0 = std::time::Instant::now();
    let mut rset = edges_to_rset(label, edges);
    // size 2, 3, 4 — include size 2-3 baseline for comparison
    for size in 2..=4 {
        let t1 = std::time::Instant::now();
        run_pass_on_rset(&mut rset, size);
        println!("    [{}] size={} pass {:.1}s", label, size, t1.elapsed().as_secs_f64());
        use std::io::Write;
        std::io::stdout().flush().ok();
    }
    let cset = collect_canonicals(&rset);
    println!("    [{}] {} edges, {} canonicals total (build {:.1}s)",
             label, edges.len(), cset.len(), t0.elapsed().as_secs_f64());
    use std::io::Write;
    std::io::stdout().flush().ok();
    cset
}

fn build_generated_size_only(label: &str, edges: &[(String, String)], size: usize) -> HashSet<CanonicalForm> {
    let t0 = std::time::Instant::now();
    let mut rset = edges_to_rset(label, edges);
    run_pass_on_rset(&mut rset, size);
    let cset = collect_canonicals(&rset);
    println!("    [{}] size {} only: {} edges, {} canonicals ({:.1}s)",
             label, size, edges.len(), cset.len(), t0.elapsed().as_secs_f64());
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

fn run_size(size: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>,
                              Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    println!();
    println!(" === size = {} ===", size);
    println!();
    let mut er_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_er(seed);
        er_sets.push(build_generated_size_only(&format!("ER_{}", i), &edges, size));
    }
    let mut sbm_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_sbm(seed);
        sbm_sets.push(build_generated_size_only(&format!("SBM_{}", i), &edges, size));
    }
    let mut dag_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_synth_dag(seed);
        dag_sets.push(build_generated_size_only(&format!("DAG_{}", i), &edges, size));
    }
    let mut bp_sets: Vec<HashSet<CanonicalForm>> = Vec::new();
    for (i, &seed) in SEEDS.iter().enumerate() {
        let edges = build_bipartite(seed);
        bp_sets.push(build_generated_size_only(&format!("BP_{}", i), &edges, size));
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
    (within(&er_sets), within(&sbm_sets), within(&dag_sets), within(&bp_sets),
     cross(&er_sets, &sbm_sets), cross(&er_sets, &dag_sets), cross(&er_sets, &bp_sets),
     cross(&sbm_sets, &dag_sets), cross(&sbm_sets, &bp_sets), cross(&dag_sets, &bp_sets))
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Round 7 — sizes 4 scan");
    println!("════════════════════════════════════════════════════════");
    println!(" n_nodes={}  sample_count={}  top_m={}  seeds_per_family={}",
             N_NODES, SAMPLE_COUNT, TOP_M, SEEDS_PER_FAMILY);
    println!(" Compares size=2-3 (saturation regime) to size=4");
    println!();

    for &size in &[2usize, 3, 4] {
        let (er_w, sbm_w, dag_w, bp_w,
             er_sbm, er_dag, er_bp,
             sbm_dag, sbm_bp, dag_bp) = run_size(size);
        println!();
        println!(" --- size={} WITHIN ---", size);
        summary(&format!("ER size={}", size), &er_w);
        summary(&format!("SBM size={}", size), &sbm_w);
        summary(&format!("DAG size={}", size), &dag_w);
        summary(&format!("BP size={}", size), &bp_w);
        println!();
        println!(" --- size={} CROSS ---", size);
        summary(&format!("ER × SBM size={}", size), &er_sbm);
        summary(&format!("ER × DAG size={}", size), &er_dag);
        summary(&format!("ER × BP size={}", size), &er_bp);
        summary(&format!("SBM × DAG size={}", size), &sbm_dag);
        summary(&format!("SBM × BP size={}", size), &sbm_bp);
        summary(&format!("DAG × BP size={}", size), &dag_bp);

        // Compactly: within mean across all 4 families vs cross mean across all pairs.
        let within_all: Vec<f64> = er_w.iter().chain(sbm_w.iter())
            .chain(dag_w.iter()).chain(bp_w.iter()).cloned().collect();
        let cross_all: Vec<f64> = er_sbm.iter().chain(er_dag.iter())
            .chain(er_bp.iter()).chain(sbm_dag.iter())
            .chain(sbm_bp.iter()).chain(dag_bp.iter())
            .cloned().collect();
        let w_mean = if within_all.is_empty() { 0.0 } else
            { within_all.iter().sum::<f64>() / within_all.len() as f64 };
        let c_mean = if cross_all.is_empty() { 0.0 } else
            { cross_all.iter().sum::<f64>() / cross_all.len() as f64 };
        let gap = w_mean - c_mean;
        println!();
        println!(" *** size={} OVERALL: within_mean={:.4} cross_mean={:.4} gap={:.4} ***",
                 size, w_mean, c_mean, gap);
    }

    println!();
    println!("--- end ---");
}
