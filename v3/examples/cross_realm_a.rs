//! Print fingerprints for synthetic and physical mechanisms A and B
//! side-by-side. Useful for inspecting cross-realm agreement
//! numerically.

use relatum_v3::physical::{GatedBouncingBall, SpringMassFollower};
use relatum_v3::similarity::fingerprint_similarity;
use relatum_v3::sim::{MechanismA, MechanismB};
use relatum_v3::{NodeId, estimate_all};

fn main() {
    let s = NodeId::new("S");
    let t = NodeId::new("T");

    let syn_a = MechanismA::default_pair(s.clone(), t.clone()).generate("syn-A", 400, 42);
    let phys_a =
        GatedBouncingBall::default_pair(s.clone(), t.clone()).generate("phys-A", 1500, 7);
    let syn_b = MechanismB::default_pair(s.clone(), t.clone()).generate("syn-B", 500, 11);
    let phys_b =
        SpringMassFollower::default_pair(s.clone(), t.clone()).generate("phys-B", 1500, 7);

    let pick = |ep: &relatum_v3::Episode| {
        let fps = estimate_all(ep);
        fps.iter()
            .find(|f| f.source == s && f.target == t)
            .unwrap()
            .clone()
    };

    let a_fp = pick(&syn_a);
    let pa_fp = pick(&phys_a);
    let b_fp = pick(&syn_b);
    let pb_fp = pick(&phys_b);

    println!(
        "{:>16} {:>6} {:>6} {:>6} {:>5} {:>6} {:>6}",
        "", "CE", "PE", "VE", "lat", "rev", "stab"
    );
    let row = |label, f: &relatum_v3::Fingerprint| {
        println!(
            "{:>16} {:>6.3} {:>6.3} {:>6.3} {:>5} {:>6.3} {:>6.3}",
            label,
            f.constraint_effect,
            f.position_effect,
            f.velocity_effect,
            f.latency,
            f.reversibility,
            f.stability
        )
    };
    row("synthetic A", &a_fp);
    row("physical A", &pa_fp);
    row("synthetic B", &b_fp);
    row("physical B", &pb_fp);

    println!();
    println!("cross-realm A:  similarity(syn_A, phys_A) = {:.4}", fingerprint_similarity(&a_fp, &pa_fp));
    println!("cross-realm B:  similarity(syn_B, phys_B) = {:.4}", fingerprint_similarity(&b_fp, &pb_fp));
    println!("off-mech (A): similarity(syn_A, syn_B)  = {:.4}", fingerprint_similarity(&a_fp, &b_fp));
    println!("off-mech (B): similarity(syn_B, syn_A)  = {:.4}", fingerprint_similarity(&b_fp, &a_fp));
    println!("phys A vs phys B:                       = {:.4}", fingerprint_similarity(&pa_fp, &pb_fp));
}
