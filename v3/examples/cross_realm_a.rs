//! Print fingerprints for synthetic and physical mechanisms A and B
//! side-by-side. Useful for inspecting cross-realm agreement
//! numerically.

use relatum_v3::physical::{CommonDriveReceivers, GatedBouncingBall, SpringMassFollower};
use relatum_v3::similarity::fingerprint_similarity;
use relatum_v3::sim::{MechanismA, MechanismB, MechanismC};
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
    let syn_c = MechanismC::default_pair(s.clone(), t.clone()).generate("syn-C", 400, 5);
    let phys_c =
        CommonDriveReceivers::default_pair(s.clone(), t.clone()).generate("phys-C", 1500, 7);

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
    let c_fp = pick(&syn_c);
    let pc_fp = pick(&phys_c);

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
    row("synthetic C", &c_fp);
    row("physical C", &pc_fp);

    println!();
    println!("cross-realm A:  {:.4}", fingerprint_similarity(&a_fp, &pa_fp));
    println!("cross-realm B:  {:.4}", fingerprint_similarity(&b_fp, &pb_fp));
    println!("cross-realm C:  {:.4}", fingerprint_similarity(&c_fp, &pc_fp));
    println!();
    println!("off-mech baselines (same realm, different mechanism):");
    println!("  syn_A vs syn_B:  {:.4}", fingerprint_similarity(&a_fp, &b_fp));
    println!("  syn_A vs syn_C:  {:.4}", fingerprint_similarity(&a_fp, &c_fp));
    println!("  syn_B vs syn_C:  {:.4}", fingerprint_similarity(&b_fp, &c_fp));
    println!();
    println!("same-realm cross-mech controls (no spurious realm similarity):");
    println!("  phys_A vs phys_B: {:.4}", fingerprint_similarity(&pa_fp, &pb_fp));
    println!("  phys_A vs phys_C: {:.4}", fingerprint_similarity(&pa_fp, &pc_fp));
    println!("  phys_B vs phys_C: {:.4}", fingerprint_similarity(&pb_fp, &pc_fp));
}
