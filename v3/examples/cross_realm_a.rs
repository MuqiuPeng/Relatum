//! Print fingerprints for synthetic and physical mechanism-A
//! episodes side-by-side. Useful for inspecting cross-realm
//! agreement numerically.

use relatum_v3::physical::GatedBouncingBall;
use relatum_v3::similarity::fingerprint_similarity;
use relatum_v3::sim::{MechanismA, MechanismB};
use relatum_v3::{NodeId, estimate_all};

fn main() {
    let s = NodeId::new("S");
    let t = NodeId::new("T");

    let syn_a = MechanismA::default_pair(s.clone(), t.clone()).generate("syn-A", 400, 42);
    let phys_a = GatedBouncingBall::default_pair(s.clone(), t.clone()).generate("phys-A", 1500, 7);
    let syn_b = MechanismB::default_pair(s.clone(), t.clone()).generate("syn-B", 500, 11);

    let syn_a_fps = estimate_all(&syn_a);
    let phys_a_fps = estimate_all(&phys_a);
    let syn_b_fps = estimate_all(&syn_b);

    let pick = |fps: &[relatum_v3::Fingerprint]| {
        fps.iter()
            .find(|f| f.source == s && f.target == t)
            .unwrap()
            .clone()
    };

    let a_fp = pick(&syn_a_fps);
    let p_fp = pick(&phys_a_fps);
    let b_fp = pick(&syn_b_fps);

    println!("{:>16} {:>6} {:>6} {:>6} {:>5} {:>6} {:>6}", "", "CE", "PE", "VE", "lat", "rev", "stab");
    println!(
        "{:>16} {:>6.3} {:>6.3} {:>6.3} {:>5} {:>6.3} {:>6.3}",
        "synthetic A",
        a_fp.constraint_effect,
        a_fp.position_effect,
        a_fp.velocity_effect,
        a_fp.latency,
        a_fp.reversibility,
        a_fp.stability
    );
    println!(
        "{:>16} {:>6.3} {:>6.3} {:>6.3} {:>5} {:>6.3} {:>6.3}",
        "physical A",
        p_fp.constraint_effect,
        p_fp.position_effect,
        p_fp.velocity_effect,
        p_fp.latency,
        p_fp.reversibility,
        p_fp.stability
    );
    println!(
        "{:>16} {:>6.3} {:>6.3} {:>6.3} {:>5} {:>6.3} {:>6.3}",
        "synthetic B",
        b_fp.constraint_effect,
        b_fp.position_effect,
        b_fp.velocity_effect,
        b_fp.latency,
        b_fp.reversibility,
        b_fp.stability
    );

    println!();
    println!("similarity(syn A, phys A) = {:.4}  <-- cross-realm A agreement", fingerprint_similarity(&a_fp, &p_fp));
    println!("similarity(syn A, syn B)  = {:.4}  <-- off-mechanism baseline", fingerprint_similarity(&a_fp, &b_fp));
    println!("similarity(phys A, syn B) = {:.4}", fingerprint_similarity(&p_fp, &b_fp));
}
