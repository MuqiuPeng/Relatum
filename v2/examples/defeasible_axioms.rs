//! ADR 0033 demo — defeasible (rate < 1.0) axiom discovery.
//!
//! The "almost-transitive" input is a 4-chain transitive closure
//! with one closure edge removed. Strict discovery (rate = 1.0)
//! returns nothing — transitivity fails at one binding, which is
//! enough to suppress the rule under ADR 0027's all-or-nothing
//! criterion. Defeasible discovery (min_rate ≤ 0.5) surfaces the
//! *same* template with a reported rate under 1.0, plus support
//! counts, so a caller can see "transitivity nearly holds — it
//! holds on 2 of 3 premise bindings" on this input.

use relatum_v2::{AxiomDiscoveryConfig, RSet, R};

fn main() {
    let mut rs = RSet::new();
    let nodes = ["a", "b", "c", "d"];
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            rs.add(R::new(nodes[i], nodes[j]));
        }
    }
    rs.remove(&R::new("b", "d"));

    println!("Input: 4-chain transitive closure minus edge (b,d).");
    println!("Edges: {}  Identifiers: {}", rs.len(), rs.identifiers().len());
    println!();

    for threshold in [1.0, 0.95, 0.8, 0.5, 0.1] {
        let cfg = AxiomDiscoveryConfig {
            min_rate: threshold,
            ..AxiomDiscoveryConfig::default()
        };
        let found = rs.discover_axioms(&cfg);
        println!("min_rate = {:.2}   found: {}", threshold, found.len());
        for ev in &found {
            let prem_str: Vec<String> = ev
                .template
                .premise
                .iter()
                .map(|e| format!("R({},{})", e.x_var, e.y_var))
                .collect();
            println!(
                "  [{}] ⇒ R({},{})   rate={:.3}  support={}/{}",
                prem_str.join(" ∧ "),
                ev.template.conclusion.x_var,
                ev.template.conclusion.y_var,
                ev.rate,
                ev.conclusion_satisfied,
                ev.premise_bindings
            );
        }
        println!();
    }
}
