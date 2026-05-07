//! ADR 0078 — Pattern-aware drive signal audit.
//!
//! Runs each canonical substrate to maturity and prints the
//! drive signal: how much R is unexplained, what canonical
//! shapes it forms, what the modal canonical is.
//!
//! Comparison: drive at maturity (after the runtime's
//! initialization phase, before any further pattern work) vs
//! drive after manually invoking autonomous_pass for sizes 2-5.
//! The shrinkage shows how much "drive pressure" pattern minting
//! actually relieves.

use relatum_v2::{
    runtime::{
        AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
    test_substrates::{
        narrow_a::build_narrow_a_stream, oq1::build_long_stream,
        oq2::build_oq2_stream,
    },
    AutonomousConfig, DiscoveryConfig, NamingPolicy, RSet,
    RefinementConfig, UnexplainedDriveSignal,
};

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

fn print_signal(label: &str, sig: &UnexplainedDriveSignal) {
    println!();
    println!(" --- {} ---", label);
    println!(
        "   total data edges: {}, unexplained: {} ({:.1}%)",
        sig.total_data_edges,
        sig.unexplained_count,
        sig.unexplained_ratio * 100.0,
    );
    println!(
        "   distinct canonicals: {}, modal-bucket count: {}",
        sig.distinct_canonicals,
        sig.modal_count(),
    );
    if sig.canonical_buckets.is_empty() {
        println!("   (no buckets — drive is silent)");
        return;
    }
    println!("   {:<6} {:>8} {:>8} {:>14}",
             "rank", "components", "edges", "canonical_size");
    for (i, b) in sig.canonical_buckets.iter().take(8).enumerate() {
        println!(
            "   #{:<5} {:>8} {:>8} {:>14}",
            i + 1,
            b.component_count,
            b.edge_count,
            b.canonical.len(),
        );
        // Print up to 3 example edges.
        for r in b.example_edges.iter().take(3) {
            println!("       e.g. R({}, {})", r.x, r.y);
        }
    }
    if sig.canonical_buckets.len() > 8 {
        println!("   ... and {} more buckets",
                 sig.canonical_buckets.len() - 8);
    }
}

fn audit(label: &str, stream: Vec<(u64, Event)>, ticks: u64) {
    println!();
    println!("════════════════════════════════════════════════════════");
    println!(" Substrate: {} ({} ticks to maturity)", label, ticks);
    println!("════════════════════════════════════════════════════════");

    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(stream));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(ticks);

    println!();
    println!(" Phase 0 state: {} axioms, {} theories, {} patterns",
             rt.rset.axioms().len(),
             rt.rset.theories().len(),
             rt.rset.patterns().len());

    // Drive signal at maturity (just after Phase 0).
    let sig_pre = rt.rset.unexplained_drive_signal();
    print_signal("Drive at maturity (post Phase 0)", &sig_pre);

    // Manually invoke autonomous_pass for sizes 2-5 and re-measure.
    for size in [2usize, 3, 4, 5] {
        run_pass_on_size(&mut rt, size);
    }
    let sig_post = rt.rset.unexplained_drive_signal();
    print_signal("Drive after autonomous_pass(sizes 2-5)", &sig_post);

    // Delta.
    println!();
    println!(" Drive delta:");
    println!("   patterns: {} → {}",
             rt.rset.patterns().len() - sig_post.distinct_canonicals,
             rt.rset.patterns().len());
    println!("   unexplained_count: {} → {} ({:+})",
             sig_pre.unexplained_count, sig_post.unexplained_count,
             sig_post.unexplained_count as i64
                 - sig_pre.unexplained_count as i64);
    println!("   distinct canonicals: {} → {} ({:+})",
             sig_pre.distinct_canonicals,
             sig_post.distinct_canonicals,
             sig_post.distinct_canonicals as i64
                 - sig_pre.distinct_canonicals as i64);
}

fn main() {
    println!("════════════════════════════════════════════════════════");
    println!(" ADR 0078 — Pattern-aware drive signal audit");
    println!("════════════════════════════════════════════════════════");
    println!(" Drive = unexplained R organized by canonical-form");
    println!(" buckets. Constitution-compliant: no per-token");
    println!(" signature bucketing. Subgraph-level structural keys");
    println!(" only.");

    audit("OQ#1", build_long_stream(), 1000);
    audit("narrow_a", build_narrow_a_stream(), 500);
    audit("OQ#2", build_oq2_stream(), 4500);

    println!();
    println!("--- end ---");
}
