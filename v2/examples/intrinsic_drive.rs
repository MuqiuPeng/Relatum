//! ADR 0031 — intrinsic drive demo.
//!
//! The system self-triggers: on each call to `intrinsic_drive`, the
//! RSet explores its own action space (pattern discovery at various
//! sizes; theory discovery) and greedily applies the action that most
//! improves the global abstraction score. No external instruction
//! tells it which mechanism to run or when to stop — the value signal
//! does that.

use relatum_v2::{DriveConfig, RSet, R};

fn main() {
    let cases: Vec<(&str, RSet)> = vec![
        ("mixed graph (pattern-rich)", mixed_graph()),
        ("equivalence relation (theory-rich)", equivalence()),
        ("strict partial order (theory-rich, structure-thin)", poset()),
        ("random sparse graph (low-abstraction input)", random()),
    ];

    let cfg = DriveConfig::default();

    for (label, mut rs) in cases {
        println!("=== {} ===", label);
        println!("  data edges: {}", rs.len());
        println!("  score before: {:.2}", rs.abstraction_score());

        let trace = rs.intrinsic_drive(&cfg);

        println!("  steps taken: {}", trace.steps.len());
        for (i, step) in trace.steps.iter().enumerate() {
            let kind = match &step.action {
                relatum_v2::DriveAction::DiscoverPatterns(c) => {
                    format!("DiscoverPatterns(size={})", c.discovery.target_size)
                }
                relatum_v2::DriveAction::DiscoverTheory(_) => {
                    "DiscoverTheory".to_string()
                }
                relatum_v2::DriveAction::Prune(threshold) => {
                    format!("Prune(threshold={:.2})", threshold)
                }
            };
            println!(
                "    step {}: {}  Δ={:+.2}  (score {:.2} → {:.2})",
                i + 1,
                kind,
                step.delta,
                step.score_before,
                step.score_after
            );
        }
        println!("  final score: {:.2}", trace.final_score);
        println!(
            "  named: {} pattern(s), {} theory(ies), {} axiom(s) registered",
            rs.patterns().len(),
            rs.theories().len(),
            rs.axioms().len()
        );
        println!();
    }
}

fn mixed_graph() -> RSet {
    let mut rs = RSet::new();
    rs.extend([
        R::new("c1", "c2"), R::new("c2", "c3"),
        R::new("c3", "c4"), R::new("c4", "c5"),
    ]);
    rs.extend([R::new("k1", "k2"), R::new("k2", "k3"), R::new("k3", "k1")]);
    rs.extend([R::new("s", "sa"), R::new("s", "sb"), R::new("s", "sc")]);
    rs.extend([R::new("t1", "t2"), R::new("t1", "t3"), R::new("t2", "t4")]);
    rs.add(R::new("ie1", "ie2"));
    rs
}

fn equivalence() -> RSet {
    let mut rs = RSet::new();
    let classes: &[&[&str]] = &[&["a", "b"], &["c", "d", "e"]];
    for cls in classes {
        for x in cls.iter() {
            for y in cls.iter() {
                rs.add(R::new(*x, *y));
            }
        }
    }
    rs
}

fn poset() -> RSet {
    let mut rs = RSet::new();
    let nodes = ["a", "b", "c", "d"];
    for n in &nodes {
        rs.add(R::new(*n, *n));
    }
    rs.extend([
        R::new("a", "b"), R::new("a", "c"), R::new("a", "d"),
        R::new("b", "d"), R::new("c", "d"),
    ]);
    rs
}

fn random() -> RSet {
    let mut rs = RSet::new();
    rs.extend([
        R::new("a", "c"), R::new("b", "d"), R::new("c", "e"),
        R::new("d", "f"), R::new("e", "a"), R::new("f", "b"),
        R::new("a", "d"),
    ]);
    rs
}
