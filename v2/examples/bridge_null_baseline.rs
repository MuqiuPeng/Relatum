//! Null baseline for ADR 0081 Phase 1.D cross-substrate Jaccard claim.
//!
//! ARIS auto-review-loop Round 1 reviewer demanded: "What is the Jaccard
//! between two INDEPENDENT draws of the same substrate family? Without
//! that, the cross-substrate 0.26 is uninterpretable."
//!
//! Round 1 verdict on the first version of this baseline: 5/10 — the
//! reviewer correctly flagged TWO methodological issues (N1 + N2):
//!
//!   N1: The original OQ#2 within-baseline ran the IDENTICAL deterministic
//!       OQ#2 stream twice and varied only the autonomous_pass sampler RNG.
//!       That measures sampler determinism, not substrate-family variance.
//!
//!   N2: Within-family Jaccard = 1.0 is degenerate evidence: it is consistent
//!       with EITHER (a) genuine within-family convergence (H1) OR
//!       (b) discovery saturation under sample_count=400 (H0 — subgraph
//!       census on small canonical space). The original experiment did not
//!       distinguish these.
//!
//! Round 2 fixes:
//!
//!   N1 fix — Replace OQ#2-self with OQ#1 vs narrow_a (within-canonical-
//!            suite, two genuinely different graph instances from the
//!            v2 canonical synthetic family). Combined with the DAG_A vs
//!            DAG_B test (which was already a legitimate family-variance
//!            test, two different graph-generation seeds), we now have
//!            TWO genuine within-family Jaccards.
//!
//!   N2 fix — Add a saturation probe: re-run all comparisons at a much
//!            lower discovery budget (sample_count=50, top_m=5). If
//!            within-family Jaccard stays high while cross-family drops,
//!            that distinguishes H1 from H0. If within-family also drops,
//!            saturation was the explanation and H1 is not supported.
//!
//! Pre-registration (RETROSPECTIVELY explicit — see Round 2 N3 in
//! `review-stage/AUTO_REVIEW.md`; thresholds chosen before *Round 2's*
//! new measurements but after Round 1's, so the rigor claim is
//! "ex-post-explicit," not formal pre-registration):
//!
//!   - H0 (v2 is just doing subgraph census, no substrate-sensitivity):
//!       At saturation budget, within-family ≈ cross (both ≥0.9 from census).
//!       At low budget, within-family ≈ cross (both noisy and low).
//!
//!   - H1 (substrate-sensitive emergent canonicals):
//!       At saturation budget, within > 0.7 AND cross < 0.4 AND gap > 0.3.
//!       At low budget, within stays > 0.5 AND cross stays < 0.4.
//!       (Critical: within should stay HIGH even when budget can't
//!        saturate — that's what distinguishes "substrate fingerprint"
//!        from "census saturation.")
//!
//! All canonicals compared via direct `CanonicalForm` equality
//! (Vec<(u64, u64)>), NOT hash equality (Round 1 W5 fix).

use relatum_v2::{
    runtime::{
        AutonomousRuntime, RuleBasedScheduler, SyntheticStreamEnvironment,
    },
    test_substrates::{
        narrow_a::build_narrow_a_stream,
        oq1::build_long_stream,
        oq2::build_oq2_stream,
    },
    AutonomousConfig, CanonicalForm, DiscoveryConfig, NamingPolicy,
    RSet, RefinementConfig,
};
use std::collections::HashSet;

#[derive(Clone, Copy)]
struct DiscoveryBudget {
    sample_count: usize,
    top_m: usize,
}

const SATURATION_BUDGET: DiscoveryBudget = DiscoveryBudget {
    sample_count: 400,
    top_m: 20,
};

const LOW_BUDGET: DiscoveryBudget = DiscoveryBudget {
    sample_count: 50,
    top_m: 5,
};

fn run_pass_on_rset(
    rset: &mut RSet,
    size: usize,
    rng_offset: u64,
    budget: DiscoveryBudget,
) {
    let cfg = AutonomousConfig {
        discovery: DiscoveryConfig {
            target_size: size,
            sample_count: budget.sample_count,
            top_m: budget.top_m,
            rng_seed: 0xC0FFEE_u64
                .wrapping_add(rng_offset)
                .wrapping_add(size as u64 * 0x9E37),
            include_meta_in_discovery: false,
        },
        refinement: RefinementConfig {
            max_tries: 200,
            rng_seed: 0xC0FFEE_u64
                .wrapping_add(rng_offset)
                .wrapping_add(size as u64 * 0xDEAD),
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

/// Synth layered random DAG, parameterized by seed (Round 1 W3 fix).
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
    let mut tsv = String::from("# null-baseline synth DAG\n");
    for (a, b) in edges {
        tsv.push_str(&format!("{}\t{}\n", a, b));
    }
    RSet::from_text(&tsv).expect("from_text")
}

fn build_canonical_suite_rset(
    label: &str,
    stream: Vec<(u64, relatum_v2::runtime::Event)>,
    ticks: u64,
) -> RSet {
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);
    eprintln!("    [{}] post-stream rset: {} R-instances, {} axioms",
              label, rt.rset.len(), rt.rset.axioms().len());
    rt.rset
}

fn jaccard(a: &HashSet<CanonicalForm>, b: &HashSet<CanonicalForm>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    inter as f64 / union as f64
}

#[derive(Clone, Copy)]
struct Jaccards {
    canonical_suite_within: f64,
    dag_within: f64,
    cross_oq1_dag: f64,
    cross_oq2_dag: f64,
}

fn run_one_budget(budget: DiscoveryBudget, label: &str) -> Jaccards {
    println!();
    println!(" === Budget: {} (sample_count={}, top_m={}) ===",
             label, budget.sample_count, budget.top_m);

    // Build all four RSets (always identical input graphs).
    let mut rset_oq1 = build_canonical_suite_rset("OQ#1",
        build_long_stream(), 1000);
    let mut rset_narrow_a = build_canonical_suite_rset("narrow_a",
        build_narrow_a_stream(), 1000);
    let mut rset_oq2 = build_canonical_suite_rset("OQ#2",
        build_oq2_stream(), 1000);
    let mut rset_dag_a = dag_to_rset(&build_synth_dag(0xCAFEBABE_DEADBEEF));
    let mut rset_dag_b = dag_to_rset(&build_synth_dag(0x12345678_9ABCDEF0));

    // Run autonomous_pass at this budget.
    for size in 2..=3 {
        run_pass_on_rset(&mut rset_oq1, size, 0, budget);
        run_pass_on_rset(&mut rset_narrow_a, size, 0, budget);
        run_pass_on_rset(&mut rset_oq2, size, 0, budget);
        run_pass_on_rset(&mut rset_dag_a, size, 0, budget);
        run_pass_on_rset(&mut rset_dag_b, size, 0, budget);
    }

    let c_oq1 = collect_canonicals(&rset_oq1);
    let c_narrow_a = collect_canonicals(&rset_narrow_a);
    let c_oq2 = collect_canonicals(&rset_oq2);
    let c_dag_a = collect_canonicals(&rset_dag_a);
    let c_dag_b = collect_canonicals(&rset_dag_b);

    println!("   |canon| OQ#1={} narrow_a={} OQ#2={} DAG_A={} DAG_B={}",
             c_oq1.len(), c_narrow_a.len(), c_oq2.len(),
             c_dag_a.len(), c_dag_b.len());

    let j_canonical_within = jaccard(&c_oq1, &c_narrow_a);
    let j_dag_within = jaccard(&c_dag_a, &c_dag_b);
    let j_cross_oq1_dag = jaccard(&c_oq1, &c_dag_a);
    let j_cross_oq2_dag = jaccard(&c_oq2, &c_dag_a);

    println!("   Within(OQ#1, narrow_a)     = {:.4}", j_canonical_within);
    println!("   Within(DAG_A, DAG_B)       = {:.4}", j_dag_within);
    println!("   Cross(OQ#1, DAG_A)         = {:.4}", j_cross_oq1_dag);
    println!("   Cross(OQ#2, DAG_A)         = {:.4}", j_cross_oq2_dag);

    Jaccards {
        canonical_suite_within: j_canonical_within,
        dag_within: j_dag_within,
        cross_oq1_dag: j_cross_oq1_dag,
        cross_oq2_dag: j_cross_oq2_dag,
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" Null baseline for ADR 0081 Phase 1.D — Round 2");
    println!(" (fixes Round 2 N1 + N2 from ARIS auto-review-loop)");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Pre-registered hypotheses (ex-post-explicit, per N3):");
    println!("   H0: v2 is doing subgraph census →");
    println!("       at saturation: within ≈ cross (both high or both moderate)");
    println!("       at low budget: within ≈ cross (both noisy/low)");
    println!();
    println!("   H1: substrate-sensitive emergence →");
    println!("       at saturation: within > 0.7 AND cross < 0.4");
    println!("       at low budget: within stays > 0.5 AND cross < 0.4");
    println!();
    println!(" KEY DISCRIMINATOR: under LOW budget, does within stay HIGH?");
    println!("   If yes → substrate fingerprint is structurally real (H1)");
    println!("   If no  → it was just census saturation (H0)");

    let sat = run_one_budget(SATURATION_BUDGET, "saturation");
    let low = run_one_budget(LOW_BUDGET, "low (anti-saturation)");

    // ── Verdict ──
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Hypothesis verdict (vs pre-registered ranges)");
    println!("════════════════════════════════════════════════════════");
    println!();
    println!(" Saturation budget:");
    println!("   within(OQ#1,narrow_a)={:.4}  within(DAG_A,DAG_B)={:.4}",
             sat.canonical_suite_within, sat.dag_within);
    println!("   cross(OQ#1,DAG_A)={:.4}      cross(OQ#2,DAG_A)={:.4}",
             sat.cross_oq1_dag, sat.cross_oq2_dag);
    println!();
    println!(" Low budget (anti-saturation):");
    println!("   within(OQ#1,narrow_a)={:.4}  within(DAG_A,DAG_B)={:.4}",
             low.canonical_suite_within, low.dag_within);
    println!("   cross(OQ#1,DAG_A)={:.4}      cross(OQ#2,DAG_A)={:.4}",
             low.cross_oq1_dag, low.cross_oq2_dag);
    println!();

    let sat_within_avg = (sat.canonical_suite_within + sat.dag_within) / 2.0;
    let sat_cross_avg = (sat.cross_oq1_dag + sat.cross_oq2_dag) / 2.0;
    let low_within_avg = (low.canonical_suite_within + low.dag_within) / 2.0;
    let low_cross_avg = (low.cross_oq1_dag + low.cross_oq2_dag) / 2.0;

    println!(" Averaged:");
    println!("   sat: within_mean={:.4} cross_mean={:.4} gap={:.4}",
             sat_within_avg, sat_cross_avg,
             sat_within_avg - sat_cross_avg);
    println!("   low: within_mean={:.4} cross_mean={:.4} gap={:.4}",
             low_within_avg, low_cross_avg,
             low_within_avg - low_cross_avg);
    println!();

    let sat_h1_ok = sat.canonical_suite_within > 0.7
        && sat.dag_within > 0.7
        && sat.cross_oq1_dag < 0.4
        && sat.cross_oq2_dag < 0.4;
    let low_h1_ok = sat_h1_ok
        && low.canonical_suite_within > 0.5
        && low.dag_within > 0.5
        && low.cross_oq1_dag < 0.4
        && low.cross_oq2_dag < 0.4;

    println!(" H1 at saturation budget alone: {}",
             if sat_h1_ok { "supported" } else { "NOT supported" });
    println!(" H1 at BOTH budgets (discriminates H1 from saturation): {}",
             if low_h1_ok { "supported" } else { "NOT supported" });
    println!();

    if low_h1_ok {
        println!("   → H1 SUPPORTED at both budgets. Within-family Jaccard");
        println!("     stays >0.5 even at sample_count=50, top_m=5, so the");
        println!("     substrate-family fingerprint is NOT a saturation");
        println!("     artifact. Round 2 N2 falsifier ruled out for this");
        println!("     configuration.");
    } else if sat_h1_ok && !low_h1_ok {
        println!("   → AMBIGUOUS: H1 thresholds met at saturation budget");
        println!("     but FAIL at low budget — within-family Jaccard");
        println!("     drops, indicating the original H1 'support' was");
        println!("     a saturation artifact (Round 2 N2 confirmed).");
        println!("     The substrate-sensitivity claim is NOT supported");
        println!("     beyond 'two different graphs have different size-3");
        println!("     subgraph censuses.'");
    } else {
        println!("   → H1 NOT SUPPORTED at saturation budget either.");
    }

    println!();
    println!("--- end ---");
}
