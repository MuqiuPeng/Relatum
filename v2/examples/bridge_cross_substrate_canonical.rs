//! Cross-substrate canonical-form comparison: OQ#2 vs synthetic layered random DAG.
//!
//! Phase 1 follow-up to ADR 0081 (vibe-proving bridge Phase 0).
//! Compares the canonical forms (subgraph fingerprints) v2 mints on
//! the canonical OQ#2 substrate vs a synthetic layered random DAG
//! with clique cliques. Tests whether v2's "richer pattern population"
//! is a genuinely different canonical set or just more of the same forms.
//!
//! HONESTY DISCLAIMER (added in ARIS auto-review-loop Round 1, W1+W2+W6 fix):
//!   The second substrate was originally labeled "synthetic Lean dep".
//!   That framing was misleading. It is NOT Lean source. It is NOT a
//!   sample of Mathlib. It is a purely synthetic random DAG with a
//!   layered+clustered structure that loosely *resembles* what a math
//!   library's dependency graph might look like at the level of "small
//!   theorems depend on a few prior small theorems, and topic clusters
//!   form little cliques". The substrate is renamed accordingly here.
//!   Any claim of "Lean substrate sensitivity" requires actual Mathlib
//!   data — see ADR 0081 Phase 1.E.
//!
//! Method:
//!   1. Run OQ#2 to maturity + manual autonomous_pass(sizes 2-3)
//!   2. Same on synthetic layered random DAG
//!   3. Extract canonical forms via `pattern_structure(pid)`
//!   4. Set comparison via direct `CanonicalForm` (= Vec<(u64,u64)>)
//!      equality (W5 fix — was previously truncated 64-bit hash)
//!   5. Render each canonical's shape via `format_pattern_shape`
//!
//! ADR 0075 piece 3 used a similar technique to compare OQ#1 vs
//! OQ#2; this extends it to a synthetic structural family different
//! from the canonical synthetic suite.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq2::build_oq2_stream,
    AutonomousConfig, CanonicalForm, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig,
};
use std::collections::{BTreeSet, HashMap, HashSet};

const RNG_SEED: u64 = 0xC0FFEE;

fn run_pass_on_size(rt: &mut AutonomousRuntime, size: usize) {
    let cfg = AutonomousConfig {
        discovery: DiscoveryConfig {
            target_size: size,
            sample_count: 400,
            top_m: 20,
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
    let _ = rt.rset.autonomous_pass(&cfg);
}

fn run_pass_on_rset(rset: &mut RSet, size: usize) {
    let cfg = AutonomousConfig {
        discovery: DiscoveryConfig {
            target_size: size,
            sample_count: 400,
            top_m: 20,
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

/// Returns (pattern_id, canonical_form). Uses direct CanonicalForm
/// (Vec<(u64,u64)>) rather than hashing — hash collisions at 64 bits
/// are unlikely but the W5 reviewer's point was right: structural
/// equality is the correct primitive for set operations on canonicals.
fn collect_canonicals(rset: &RSet) -> Vec<(String, CanonicalForm)> {
    let mut out = Vec::new();
    for pid in rset.patterns() {
        if let Some(canon) = rset.pattern_structure(pid) {
            out.push((pid.to_string(), canon));
        }
    }
    out
}

/// Synthetic layered random DAG with clique-style clusters.
///
/// Renamed from `build_synthetic_lean_dep_graph` (W1 fix). The graph
/// is parameterized purely by the RNG seed below; no Lean source,
/// no Mathlib snapshot, no proof-engine sequence is involved. The
/// LOOSE INTUITION was: math-library dep graphs tend to have small
/// theorems depending on a few earlier small theorems, with topic
/// clusters forming little cliques. We replicate THAT STRUCTURAL
/// FLAVOR — not Lean itself.
fn build_synthetic_layered_random_dag() -> Vec<(String, String)> {
    let mut edges: HashSet<(String, String)> = HashSet::new();
    let n = 80usize;
    let mut rng_state: u64 = 0xCAFEBABE_DEADBEEF;
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

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Cross-substrate canonical-form comparison (ADR 0081 P1.D)");
    println!(" OQ#2 vs synthetic layered random DAG");
    println!(" (NOT Lean — see W1/W2/W6 disclaimer in source header)");
    println!("════════════════════════════════════════════════════════");

    // ── OQ#2 ────────────────────────────────────────────────
    println!();
    println!(" Building OQ#2 runtime state...");
    let mut rt_oq2 = AutonomousRuntime::new(RSet::new());
    rt_oq2.environment = Box::new(SyntheticStreamEnvironment::new(build_oq2_stream()));
    rt_oq2.scheduler = Box::new(RuleBasedScheduler::default());
    // Reduced from 4500 to 1000 ticks: 1000 is well past OQ#2's
    // Phase 0 maturity (~250 ticks); the additional 500-4500
    // range with post-ADR-0080 sustained mode + LP gating costs
    // ~minutes without changing pattern outcome. Use manual
    // autonomous_pass below to fully populate canonical set.
    rt_oq2.run_bounded(1000);
    // Manually invoke autonomous_pass sizes 2-3 to populate pattern set.
    for size in 2..=3 {
        run_pass_on_size(&mut rt_oq2, size);
    }
    let oq2_canonicals = collect_canonicals(&rt_oq2.rset);
    println!(" OQ#2: {} patterns minted", oq2_canonicals.len());

    // ── Synthetic layered random DAG ───────────────────────
    println!();
    println!(" Building synthetic layered random DAG substrate...");
    let edges = build_synthetic_layered_random_dag();
    let mut tsv = String::from("# synthetic layered random DAG\n");
    for (a, b) in &edges {
        tsv.push_str(&format!("{}\t{}\n", a, b));
    }
    let mut rset_dag = match RSet::from_text(&tsv) {
        Ok(rs) => rs,
        Err(e) => {
            println!(" ✗ DAG substrate from_text failed: {:?}", e);
            return;
        }
    };
    for size in 2..=3 {
        run_pass_on_rset(&mut rset_dag, size);
    }
    let dag_canonicals = collect_canonicals(&rset_dag);
    println!(" Synthetic layered random DAG: {} patterns minted",
             dag_canonicals.len());

    // ── Set comparison via direct CanonicalForm equality (W5 fix) ──
    // Previously: hashed each canonical to a truncated 64-bit tag
    // and operated on tag strings. Reviewer's point: structural
    // equality (Vec<(u64,u64)>) IS the canonical primitive — no
    // need for a hash truncation that can collide.
    let oq2_canon_set: HashSet<CanonicalForm> = oq2_canonicals
        .iter()
        .map(|(_, c)| c.clone())
        .collect();
    let dag_canon_set: HashSet<CanonicalForm> = dag_canonicals
        .iter()
        .map(|(_, c)| c.clone())
        .collect();

    // Build pid lookup for display.
    let oq2_canon_to_pid: HashMap<CanonicalForm, String> = oq2_canonicals
        .iter()
        .map(|(pid, c)| (c.clone(), pid.clone()))
        .collect();
    let dag_canon_to_pid: HashMap<CanonicalForm, String> = dag_canonicals
        .iter()
        .map(|(pid, c)| (c.clone(), pid.clone()))
        .collect();

    // BTreeSet to get stable display order.
    let shared: BTreeSet<CanonicalForm> = oq2_canon_set
        .intersection(&dag_canon_set)
        .cloned()
        .collect();
    let oq2_only: BTreeSet<CanonicalForm> = oq2_canon_set
        .difference(&dag_canon_set)
        .cloned()
        .collect();
    let dag_only: BTreeSet<CanonicalForm> = dag_canon_set
        .difference(&oq2_canon_set)
        .cloned()
        .collect();

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Comparison (set ops via direct CanonicalForm equality)");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Shared canonicals: {}", shared.len());
    for canon in &shared {
        let oq2_pid = &oq2_canon_to_pid[canon];
        let dag_pid = &dag_canon_to_pid[canon];
        let shape = rt_oq2.rset.format_pattern_shape(oq2_pid);
        let first_line = shape.lines().next().unwrap_or("(empty)");
        println!("   (OQ#2 {} / DAG {}) edges={}",
                 oq2_pid, dag_pid, canon.len());
        println!("     {}", first_line);
    }
    println!();
    println!(" OQ#2-only canonicals: {}", oq2_only.len());
    for canon in &oq2_only {
        let pid = &oq2_canon_to_pid[canon];
        let shape = rt_oq2.rset.format_pattern_shape(pid);
        let first_line = shape.lines().next().unwrap_or("(empty)");
        println!("   ({}) edges={}", pid, canon.len());
        println!("     {}", first_line);
    }
    println!();
    println!(" Synth-DAG-only canonicals: {}", dag_only.len());
    for canon in &dag_only {
        let pid = &dag_canon_to_pid[canon];
        let shape = rset_dag.format_pattern_shape(pid);
        let first_line = shape.lines().next().unwrap_or("(empty)");
        println!("   ({}) edges={}", pid, canon.len());
        println!("     {}", first_line);
    }

    // ── Jaccard ────────────────────────────────────────────
    let union_size = oq2_canon_set.union(&dag_canon_set).count() as f64;
    let inter_size = shared.len() as f64;
    let jaccard = if union_size > 0.0 {
        inter_size / union_size
    } else {
        0.0
    };

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Verdict");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Total OQ#2 canonicals:        {}", oq2_canon_set.len());
    println!(" Total synth-DAG canonicals:   {}", dag_canon_set.len());
    println!(" Shared:                       {}", shared.len());
    println!(" OQ#2-only:                    {}", oq2_only.len());
    println!(" Synth-DAG-only:               {}", dag_only.len());
    println!(" Jaccard(OQ#2, synth-DAG):     {:.4}", jaccard);
    println!();

    if !dag_only.is_empty() {
        println!(" → Synth-DAG produces {} structural canonicals that OQ#2",
                 dag_only.len());
        println!("   does NOT. The substrate is structurally distinct from the");
        println!("   canonical synthetic suite; v2's pattern path discovers");
        println!("   substrate-specific motifs.");
        println!();
        println!("   Interpretation guard: this distinguishes \"layered random");
        println!("   DAG with clusters\" from \"OQ#2-style sequence\". It does");
        println!("   NOT establish substrate-sensitivity for *Lean*, which");
        println!("   would require an actual Mathlib snapshot (Phase 1.E).");
    } else {
        println!(" → All synth-DAG canonicals appear in OQ#2. The richer");
        println!("   pattern count is just more INSTANCES of the same canonicals,");
        println!("   not new structural categories.");
    }

    println!();
    println!("--- end ---");
}
