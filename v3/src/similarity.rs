//! T3 structural-isomorphism: pair similarity between two episodes,
//! and L1 dataset pair structures.
//!
//! Episode similarity is computed by matching fingerprint sets under
//! all node-id bijections and taking the best match. M2 ships the
//! 2-node case (only 2 bijections: identity and swap); n-node
//! generalisation is M3+ work.
//!
//! By A1, renaming an episode produces bit-identical fingerprints, so
//! `episode_similarity_2node(e, e.rename(...))` is exactly 1.0. Same-
//! mechanism episodes with different seeds are highly similar
//! (> 0.7 typically), different-mechanism episodes lower.

use crate::fingerprint::LATENCY_MAX_LAG;
use crate::{Episode, Fingerprint, NodeId, estimate_all};

/// An L1 dataset pair: two episodes labelled with whether they are
/// expected to be structurally isomorphic (same mechanism + rename).
#[derive(Debug, Clone)]
pub struct L1Pair {
    pub episode_a: Episode,
    pub episode_b: Episode,
    pub is_isomorphic: bool,
    pub label: String,
}

impl L1Pair {
    /// T3 score for this pair.
    pub fn similarity(&self) -> f64 {
        episode_similarity_2node(&self.episode_a, &self.episode_b)
    }
}

/// Pairwise similarity between two fingerprints. Range `[0, 1]`.
///
/// Euclidean distance in the normalised six-dimensional feature space
/// (`CE, PE, VE, latency / LATENCY_MAX_LAG, reversibility, stability`),
/// converted to similarity via `1 − dist / sqrt(6)`.
pub fn fingerprint_similarity(a: &Fingerprint, b: &Fingerprint) -> f64 {
    let va = fingerprint_vec(a);
    let vb = fingerprint_vec(b);
    let dist_sq: f64 = va
        .iter()
        .zip(vb.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum();
    let max_dist = (va.len() as f64).sqrt();
    (1.0 - dist_sq.sqrt() / max_dist).clamp(0.0, 1.0)
}

fn fingerprint_vec(f: &Fingerprint) -> [f64; 6] {
    [
        f.constraint_effect,
        f.position_effect,
        f.velocity_effect,
        (f.latency.max(0) as f64 / LATENCY_MAX_LAG as f64).min(1.0),
        f.reversibility,
        f.stability,
    ]
}

/// Episode similarity for 2-node episodes. Returns the maximum over
/// the two node-id bijections (identity and swap) of the mean
/// pairwise fingerprint similarity.
///
/// Panics if either episode does not have exactly two nodes.
pub fn episode_similarity_2node(e1: &Episode, e2: &Episode) -> f64 {
    assert_eq!(e1.nodes.len(), 2, "only 2-node episodes supported in M2");
    assert_eq!(e2.nodes.len(), 2, "only 2-node episodes supported in M2");
    let fps1 = estimate_all(e1);
    let fps2 = estimate_all(e2);

    let (e1_a, e1_b) = (&e1.nodes[0], &e1.nodes[1]);
    let (e2_a, e2_b) = (&e2.nodes[0], &e2.nodes[1]);
    let f1_ab = lookup(&fps1, e1_a, e1_b);
    let f1_ba = lookup(&fps1, e1_b, e1_a);
    let f2_ab = lookup(&fps2, e2_a, e2_b);
    let f2_ba = lookup(&fps2, e2_b, e2_a);

    let sim_id = (fingerprint_similarity(f1_ab, f2_ab)
        + fingerprint_similarity(f1_ba, f2_ba))
        / 2.0;
    let sim_sw = (fingerprint_similarity(f1_ab, f2_ba)
        + fingerprint_similarity(f1_ba, f2_ab))
        / 2.0;
    sim_id.max(sim_sw)
}

fn lookup<'a>(fps: &'a [Fingerprint], source: &NodeId, target: &NodeId) -> &'a Fingerprint {
    fps.iter()
        .find(|f| &f.source == source && &f.target == target)
        .expect("fingerprint not found")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{MechanismA, MechanismB, MechanismC, MechanismD};
    use std::collections::HashMap;

    fn rename_2(e: &Episode, new_a: &str, new_b: &str) -> Episode {
        let mut map = HashMap::new();
        map.insert(e.nodes[0].clone(), NodeId::new(new_a));
        map.insert(e.nodes[1].clone(), NodeId::new(new_b));
        e.rename(|n| map.get(n).cloned().unwrap())
    }

    /// T3 + A1 guard: renaming an episode produces bit-identical
    /// fingerprints, so similarity is exactly 1.0.
    #[test]
    fn renamed_variant_has_perfect_similarity() {
        let e = MechanismA::default_pair(NodeId::new("N1"), NodeId::new("N2"))
            .generate("E", 400, 7);
        let renamed = rename_2(&e, "X1", "X2");
        let sim = episode_similarity_2node(&e, &renamed);
        assert_eq!(sim.to_bits(), 1.0f64.to_bits(), "sim: {sim}");
    }

    /// Same-mechanism episodes (different seeds) are more similar than
    /// different-mechanism episodes.
    #[test]
    fn same_mechanism_more_similar_than_different() {
        let ea1 = MechanismA::default_pair(NodeId::new("S"), NodeId::new("T"))
            .generate("E", 400, 1);
        let ea2 = MechanismA::default_pair(NodeId::new("S"), NodeId::new("T"))
            .generate("E", 400, 2);
        let ec1 = MechanismC::default_pair(NodeId::new("S"), NodeId::new("T"))
            .generate("E", 400, 1);
        let same = episode_similarity_2node(&ea1, &ea2);
        let diff = episode_similarity_2node(&ea1, &ec1);
        assert!(same > diff, "same={same}, diff={diff}");
        assert!(same > 0.7, "same-mechanism similarity too low: {same}");
    }

    /// Distinguishability matrix on A/B/C/D: for every row i, the
    /// diagonal entry (self-similarity across seeds) dominates every
    /// off-diagonal entry in that row.
    #[test]
    fn distinguishability_matrix_diagonal_dominant() {
        type Gen = fn(u64) -> Episode;
        let mechs: [(&str, Gen); 4] = [
            ("A", |s| {
                MechanismA::default_pair(NodeId::new("N1"), NodeId::new("N2"))
                    .generate("E", 400, s)
            }),
            ("B", |s| {
                MechanismB::default_pair(NodeId::new("N1"), NodeId::new("N2"))
                    .generate("E", 400, s)
            }),
            ("C", |s| {
                MechanismC::default_pair(NodeId::new("N1"), NodeId::new("N2"))
                    .generate("E", 400, s)
            }),
            ("D", |s| {
                MechanismD::default_pair(NodeId::new("N1"), NodeId::new("N2"))
                    .generate("E", 400, s)
            }),
        ];
        for i in 0..4 {
            let (l_i, gen_i) = mechs[i];
            let self_sim = episode_similarity_2node(&gen_i(1), &gen_i(2));
            for j in 0..4 {
                if i == j {
                    continue;
                }
                let (l_j, gen_j) = mechs[j];
                let cross_sim = episode_similarity_2node(&gen_i(1), &gen_j(1));
                assert!(
                    self_sim > cross_sim,
                    "self({l_i}-{l_i})={self_sim} not > cross({l_i}-{l_j})={cross_sim}"
                );
            }
        }
    }

    /// L1Pair end-to-end: positive (same mechanism + rename) reaches
    /// the bit-equal-1.0 ceiling; negative (different mechanisms) is
    /// strictly lower.
    #[test]
    fn l1_pair_positive_above_negative() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let base = MechanismA::default_pair(n1.clone(), n2.clone()).generate("EA", 400, 1);
        let renamed = rename_2(&base, "X1", "X2");
        let other = MechanismC::default_pair(n1, n2).generate("EC", 400, 1);

        let pos = L1Pair {
            episode_a: base.clone(),
            episode_b: renamed,
            is_isomorphic: true,
            label: "A-rename".into(),
        };
        let neg = L1Pair {
            episode_a: base,
            episode_b: other,
            is_isomorphic: false,
            label: "A-vs-C".into(),
        };
        let pos_sim = pos.similarity();
        let neg_sim = neg.similarity();
        assert_eq!(pos_sim.to_bits(), 1.0f64.to_bits(), "positive sim: {pos_sim}");
        assert!(pos_sim > neg_sim, "neg sim: {neg_sim}");
        assert!(pos.is_isomorphic);
        assert!(!neg.is_isomorphic);
    }
}
