//! Cross-substrate canonical-form comparison: OQ#2 vs synthetic Lean dep.
//!
//! Phase 1 follow-up to ADR 0081 (vibe-proving bridge Phase 0).
//! Compares the canonical forms (subgraph fingerprints) v2 mints on
//! the canonical OQ#2 substrate vs the synthetic Lean dep substrate.
//! Tests whether the bridge's "richer pattern population" is a
//! genuinely different canonical set or just more of the same forms.
//!
//! Method:
//!   1. Run OQ#2 to maturity + manual autonomous_pass(sizes 2-3)
//!   2. Same on synthetic Lean dep graph (from bridge_lean_dep_probe)
//!   3. Extract canonical forms via `pattern_structure(pid)`
//!   4. Hash each canonical to a stable 12-hex tag
//!   5. Set comparison: shared / OQ#2-only / Lean-only
//!   6. Render each canonical's shape via `format_pattern_shape`
//!
//! ADR 0075 piece 3 used the same technique to compare OQ#1 vs
//! OQ#2; this extends it across the bridge boundary.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::oq2::build_oq2_stream,
    AutonomousConfig, CanonicalForm, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig,
};
use std::collections::{BTreeMap, HashMap, HashSet};

const RNG_SEED: u64 = 0xC0FFEE;

/// Compact hash of a CanonicalForm to a short hex string for
/// display. Same technique as `phase_emergence_canonical_form_diversity`.
fn canonical_tag(canon: &CanonicalForm) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    canon.hash(&mut h);
    format!("can_{:012x}", h.finish())
}

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

fn collect_canonicals(rset: &RSet) -> Vec<(String, String, CanonicalForm)> {
    // returns (pattern_id, canonical_tag, canonical)
    let mut out = Vec::new();
    for pid in rset.patterns() {
        if let Some(canon) = rset.pattern_structure(pid) {
            let tag = canonical_tag(&canon);
            out.push((pid.to_string(), tag, canon));
        }
    }
    out
}

/// Build the synthetic Lean-style graph from bridge_lean_dep_probe.
/// Duplicated here so this example is self-contained.
fn build_synthetic_lean_dep_graph() -> Vec<(String, String)> {
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
    println!(" OQ#2 vs synthetic Lean dep");
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

    // ── Synthetic Lean dep ─────────────────────────────────
    println!();
    println!(" Building synthetic Lean dep substrate...");
    let edges = build_synthetic_lean_dep_graph();
    let mut tsv = String::from("# synthetic lean-style dep graph\n");
    for (a, b) in &edges {
        tsv.push_str(&format!("{}\t{}\n", a, b));
    }
    let mut rset_lean = match RSet::from_text(&tsv) {
        Ok(rs) => rs,
        Err(e) => {
            println!(" ✗ Lean substrate from_text failed: {:?}", e);
            return;
        }
    };
    for size in 2..=3 {
        run_pass_on_rset(&mut rset_lean, size);
    }
    let lean_canonicals = collect_canonicals(&rset_lean);
    println!(" Synthetic Lean: {} patterns minted", lean_canonicals.len());

    // ── Set comparison ─────────────────────────────────────
    let oq2_tags: HashMap<String, (String, CanonicalForm)> = oq2_canonicals
        .into_iter()
        .map(|(pid, tag, canon)| (tag, (pid, canon)))
        .collect();
    let lean_tags: HashMap<String, (String, CanonicalForm)> = lean_canonicals
        .into_iter()
        .map(|(pid, tag, canon)| (tag, (pid, canon)))
        .collect();

    let oq2_set: HashSet<&String> = oq2_tags.keys().collect();
    let lean_set: HashSet<&String> = lean_tags.keys().collect();
    let shared: BTreeMap<String, ()> = oq2_set
        .intersection(&lean_set)
        .map(|t| ((*t).clone(), ()))
        .collect();
    let oq2_only: BTreeMap<String, ()> = oq2_set
        .difference(&lean_set)
        .map(|t| ((*t).clone(), ()))
        .collect();
    let lean_only: BTreeMap<String, ()> = lean_set
        .difference(&oq2_set)
        .map(|t| ((*t).clone(), ()))
        .collect();

    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Comparison");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Shared canonicals: {}", shared.len());
    for tag in shared.keys() {
        let (oq2_pid, _) = &oq2_tags[tag];
        let (lean_pid, _) = &lean_tags[tag];
        let shape = rt_oq2.rset.format_pattern_shape(oq2_pid);
        let first_line = shape.lines().next().unwrap_or("(empty)");
        println!("   {} (OQ#2 {} / Lean {})", tag, oq2_pid, lean_pid);
        println!("     {}", first_line);
    }
    println!();
    println!(" OQ#2-only canonicals: {}", oq2_only.len());
    for tag in oq2_only.keys() {
        let (pid, _) = &oq2_tags[tag];
        let shape = rt_oq2.rset.format_pattern_shape(pid);
        let first_line = shape.lines().next().unwrap_or("(empty)");
        println!("   {} ({})", tag, pid);
        println!("     {}", first_line);
    }
    println!();
    println!(" Lean-only canonicals: {}", lean_only.len());
    for tag in lean_only.keys() {
        let (pid, _) = &lean_tags[tag];
        let shape = rset_lean.format_pattern_shape(pid);
        let first_line = shape.lines().next().unwrap_or("(empty)");
        println!("   {} ({})", tag, pid);
        println!("     {}", first_line);
    }

    // ── Jaccard ────────────────────────────────────────────
    let union_size = oq2_set.union(&lean_set).count() as f64;
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
    println!(" Total OQ#2 canonicals:   {}", oq2_tags.len());
    println!(" Total Lean canonicals:   {}", lean_tags.len());
    println!(" Shared:                  {}", shared.len());
    println!(" OQ#2-only:               {}", oq2_only.len());
    println!(" Lean-only:               {}", lean_only.len());
    println!(" Jaccard(OQ#2, Lean):     {:.4}", jaccard);
    println!();

    if lean_only.len() > 0 {
        println!(" → Lean dep produces {} structural canonicals that OQ#2",
                 lean_only.len());
        println!("   does NOT. The substrate is genuinely structurally distinct;");
        println!("   v2's pattern path discovers Lean-specific motifs not in");
        println!("   the canonical synthetic suite.");
    } else {
        println!(" → All Lean canonicals appear in OQ#2. The bridge's richer");
        println!("   pattern count is just more INSTANCES of the same canonicals,");
        println!("   not new structural categories.");
    }

    println!();
    println!("--- end ---");
}
