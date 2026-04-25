//! Phase A verification report (ADR 0052 § Verification plan).
//!
//! Reruns the 8-case rigorous battery (ADR 0027) plus the drip-feed
//! scenario, this time reporting human-readable fingerprints rather
//! than panicking on failure (the assertions live in `src/runtime/
//! mod.rs` `a_verification_*` tests). The output of this example is
//! captured to `logs/<date>_phase_a_verification.log`.

use relatum_v2::{
    AxiomDiscoveryConfig, RSet, R,
    runtime::{
        AutonomousRuntime, Event, RuleBasedScheduler,
        SyntheticStreamEnvironment,
    },
};

fn main() {
    println!("=== ADR 0052 Phase-A verification report ===");
    println!();

    let cfg = AxiomDiscoveryConfig::default();

    println!("--- 8-case rigorous battery (verification #3) ---");
    for (label, rs) in battery() {
        let direct = rs.discover_theory(&cfg);
        let mut direct_members: Vec<String> = direct.member_axiom_ids.clone();
        direct_members.sort();

        let mut rt = AutonomousRuntime::new(rs);
        rt.scheduler = Box::new(RuleBasedScheduler::default());
        rt.run_bounded(60);

        let theories = rt.rset.theories();
        let mut runtime_members: Vec<String> = if theories.is_empty() {
            Vec::new()
        } else {
            rt.rset
                .theory_axioms(theories[0])
                .iter()
                .map(|s| s.to_string())
                .collect()
        };
        runtime_members.sort();

        let agree = runtime_members == direct_members;
        println!(
            "case {}: tick={} lifecycle={:?} match={} \
             direct_axioms={} runtime_axioms={}",
            label,
            rt.tick,
            rt.lifecycle,
            if agree { "OK" } else { "MISMATCH" },
            direct_members.len(),
            runtime_members.len()
        );
        for ax in &runtime_members {
            println!("    - {}", ax);
        }
    }

    println!();
    println!("--- drip-feed diamond (verification #5) ---");
    let schedule = vec![
        (1, Event::AddEdge(R::new("a", "a"))),
        (2, Event::AddEdge(R::new("b", "b"))),
        (3, Event::AddEdge(R::new("c", "c"))),
        (4, Event::AddEdge(R::new("d", "d"))),
        (5, Event::AddEdge(R::new("a", "b"))),
        (6, Event::AddEdge(R::new("a", "c"))),
        (7, Event::AddEdge(R::new("a", "d"))),
        (8, Event::AddEdge(R::new("b", "d"))),
        (9, Event::AddEdge(R::new("c", "d"))),
    ];
    let mut rt = AutonomousRuntime::new(RSet::new());
    rt.environment = Box::new(SyntheticStreamEnvironment::new(schedule));
    rt.scheduler = Box::new(RuleBasedScheduler::default());
    rt.run_bounded(40);
    let poset = rt.rset.check_poset();
    println!(
        "drip_feed: tick={} lifecycle={:?} theories={} is_poset={}",
        rt.tick,
        rt.lifecycle,
        rt.rset.theories().len(),
        poset.is_poset
    );
    if let Some(t_id) = rt.rset.theories().first() {
        let mut members: Vec<String> = rt
            .rset
            .theory_axioms(t_id)
            .iter()
            .map(|s| s.to_string())
            .collect();
        members.sort();
        println!("    named theory: {}", t_id);
        for ax in &members {
            println!("    - {}", ax);
        }
    }

    println!();
    println!("--- end ---");
}

fn battery() -> Vec<(&'static str, RSet)> {
    vec![
        ("transitive_chain", case_transitive_chain()),
        ("equivalence_3_classes", case_equivalence()),
        ("strict_partial_order_diamond", case_strict_partial_order()),
        ("almost_transitive", case_almost_transitive()),
        ("random_sparse", case_random_sparse()),
        ("tolerance", case_tolerance()),
        ("total_order", case_total_order()),
        ("complete_bipartite", case_complete_bipartite()),
    ]
}

fn case_transitive_chain() -> RSet {
    let mut rs = RSet::new();
    let nodes = ["a", "b", "c", "d", "e"];
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            rs.add(R::new(nodes[i], nodes[j]));
        }
    }
    rs
}

fn case_equivalence() -> RSet {
    let mut rs = RSet::new();
    let classes: &[&[&str]] = &[&["a", "b"], &["c", "d", "e"], &["f"]];
    for cls in classes {
        for x in cls.iter() {
            for y in cls.iter() {
                rs.add(R::new(*x, *y));
            }
        }
    }
    rs
}

fn case_strict_partial_order() -> RSet {
    let mut rs = RSet::new();
    rs.extend([
        R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
        R::new("b", "d"), R::new("c", "d"),
    ]);
    rs
}

fn case_almost_transitive() -> RSet {
    let mut rs = RSet::new();
    let nodes = ["a", "b", "c", "d"];
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            rs.add(R::new(nodes[i], nodes[j]));
        }
    }
    rs.remove(&R::new("b", "d"));
    rs
}

fn case_random_sparse() -> RSet {
    let mut rs = RSet::new();
    rs.extend([
        R::new("a", "c"), R::new("b", "d"), R::new("c", "e"),
        R::new("d", "f"), R::new("e", "a"), R::new("f", "b"),
        R::new("a", "d"),
    ]);
    rs
}

fn case_tolerance() -> RSet {
    let mut rs = RSet::new();
    for n in ["a", "b", "c"] {
        rs.add(R::new(n, n));
    }
    rs.extend([
        R::new("a", "b"), R::new("b", "a"),
        R::new("b", "c"), R::new("c", "b"),
    ]);
    rs
}

fn case_total_order() -> RSet {
    let mut rs = RSet::new();
    let nodes = ["1", "2", "3", "4", "5"];
    for i in 0..nodes.len() {
        rs.add(R::new(nodes[i], nodes[i]));
        for j in (i + 1)..nodes.len() {
            rs.add(R::new(nodes[i], nodes[j]));
        }
    }
    rs
}

fn case_complete_bipartite() -> RSet {
    let mut rs = RSet::new();
    for a in ["a1", "a2", "a3"] {
        for b in ["b1", "b2", "b3"] {
            rs.add(R::new(a, b));
        }
    }
    rs
}
