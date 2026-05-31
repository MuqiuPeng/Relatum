//! Print fingerprints for synthetic and physical mechanisms A and B
//! side-by-side. Useful for inspecting cross-realm agreement
//! numerically.

use relatum_v3::physical::{
    CommonDriveReceivers, FrictionGate, GatedBouncingBall, SpringMassFollower,
};
use relatum_v3::similarity::{fingerprint_similarity, fingerprint_similarity_v2};
use relatum_v3::sim::{MechanismA, MechanismB, MechanismC, MechanismD};
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
    let syn_d = MechanismD::default_pair(s.clone(), t.clone()).generate("syn-D", 400, 9);
    let phys_d = FrictionGate::default_pair(s.clone(), t.clone()).generate("phys-D", 1500, 7);

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
    let d_fp = pick(&syn_d);
    let pd_fp = pick(&phys_d);

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
    row("synthetic D", &d_fp);
    row("physical D", &pd_fp);

    println!();
    println!("                    v1      v2");
    println!("cross-realm A:  {:.4}  {:.4}", fingerprint_similarity(&a_fp, &pa_fp), fingerprint_similarity_v2(&a_fp, &pa_fp));
    println!("cross-realm B:  {:.4}  {:.4}", fingerprint_similarity(&b_fp, &pb_fp), fingerprint_similarity_v2(&b_fp, &pb_fp));
    println!("cross-realm C:  {:.4}  {:.4}", fingerprint_similarity(&c_fp, &pc_fp), fingerprint_similarity_v2(&c_fp, &pc_fp));
    println!("cross-realm D:  {:.4}  {:.4}", fingerprint_similarity(&d_fp, &pd_fp), fingerprint_similarity_v2(&d_fp, &pd_fp));

    println!();
    println!("off-mech baselines (synthetic):    v1      v2");
    let syns = [("A", &a_fp), ("B", &b_fp), ("C", &c_fp), ("D", &d_fp)];
    for i in 0..syns.len() {
        for j in (i + 1)..syns.len() {
            println!(
                "  syn_{} vs syn_{}:              {:.4}  {:.4}",
                syns[i].0,
                syns[j].0,
                fingerprint_similarity(syns[i].1, syns[j].1),
                fingerprint_similarity_v2(syns[i].1, syns[j].1)
            );
        }
    }
}
