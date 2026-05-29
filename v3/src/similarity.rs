//! T3 structural-isomorphism: pair similarity between two episodes,
//! and L1 dataset pair structures.
//!
//! Episode similarity is computed by matching fingerprint sets under
//! all node-id bijections and taking the best match. Within a chosen
//! bijection, the episode score is the **minimum** over all directed
//! pair similarities — the worst-matched pair gates the result. This
//! is the structural-isomorphism reading: two episodes are isomorphic
//! iff *every* corresponding pair aligns, not iff they align on
//! average.
//!
//! The earlier mean-pairwise reading failed on Chain-vs-FanOut: both
//! patterns share the same fingerprint *shape* and differ only in
//! which 2 of 6 directed pairs carry the lag signal; the average
//! diluted that structural difference. See
//! `memory/v3_similarity_structure_aware.md` and the M3 entry in
//! `design-notes.md`.
//!
//! `episode_similarity` works for any node count by enumerating `n!`
//! permutations; `episode_similarity_2node` is the 2-node fast-path
//! wrapper. Enumerative search is fine through `n ≤ 7`; beyond,
//! canonical-form pre-filtering and graph-matching heuristics are
//! needed (M4+).
//!
//! By A1, renaming an episode produces bit-identical fingerprints, so
//! `episode_similarity(e, e.rename(...))` is exactly 1.0. Same-
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

/// T5 — chain composition prediction.
///
/// Given the fingerprints of two adjacent edges `A → B` and `B → C`,
/// predict the fingerprint of the indirect edge `A → C` without
/// observing it directly. The composition law is the M3 baseline:
///
/// - `latency`: additive — the only law derivable cleanly from
///   cross-correlation algebra.
/// - `constraint_effect / position_effect / velocity_effect /
///   reversibility`: multiplicative — attenuation through the
///   intermediate node, bounded above by either input.
/// - `stability`: minimum — the chain is no more stable than its
///   least stable edge.
/// - `directionality`: averaged — a coarse first cut.
/// - `effect_size`: max of the multiplicative outputs, matching the
///   single-edge convention.
///
/// Panics if `ab.target != bc.source` (the chain must be well-formed).
pub fn predict_chain_composition(ab: &Fingerprint, bc: &Fingerprint) -> Fingerprint {
    assert_eq!(
        ab.target, bc.source,
        "chain composition requires AB.target == BC.source: {:?} vs {:?}",
        ab.target, bc.source
    );
    let ce = ab.constraint_effect * bc.constraint_effect;
    let pe = ab.position_effect * bc.position_effect;
    let ve = ab.velocity_effect * bc.velocity_effect;
    Fingerprint {
        source: ab.source.clone(),
        target: bc.target.clone(),
        effect_size: ce.max(pe).max(ve),
        directionality: (ab.directionality + bc.directionality) / 2.0,
        constraint_effect: ce,
        position_effect: pe,
        velocity_effect: ve,
        latency: ab.latency + bc.latency,
        reversibility: ab.reversibility * bc.reversibility,
        stability: ab.stability.min(bc.stability),
    }
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

/// Episode similarity for any node count. Enumerates all `n!`
/// node-id bijections between the two episodes and returns the
/// `max-over-bijections of min-over-pairs` of fingerprint similarity.
///
/// The inner `min` makes the score structurally-strict: it equals the
/// similarity of the worst-matched directed pair under the best
/// bijection. Structural isomorphism requires every pair to align;
/// the average reading would let one matching half of the graph
/// compensate for a non-matching other half.
///
/// Panics if the two episodes have different node counts (structurally
/// non-isomorphic by construction).
pub fn episode_similarity(e1: &Episode, e2: &Episode) -> f64 {
    assert_eq!(
        e1.nodes.len(),
        e2.nodes.len(),
        "node counts differ: {} vs {}",
        e1.nodes.len(),
        e2.nodes.len()
    );
    let fps1 = estimate_all(e1);
    let fps2 = estimate_all(e2);
    let perms = permutations(&e2.nodes);
    let mut best: f64 = 0.0;
    for perm in &perms {
        let sim = pairwise_min(&fps1, &e1.nodes, &fps2, perm);
        if sim > best {
            best = sim;
        }
    }
    best
}

/// 2-node fast path / backward compatible wrapper. Same answer as
/// `episode_similarity` for two-node inputs.
pub fn episode_similarity_2node(e1: &Episode, e2: &Episode) -> f64 {
    assert_eq!(e1.nodes.len(), 2, "expected 2 nodes");
    assert_eq!(e2.nodes.len(), 2, "expected 2 nodes");
    episode_similarity(e1, e2)
}

fn pairwise_min(
    fps1: &[Fingerprint],
    nodes1: &[NodeId],
    fps2: &[Fingerprint],
    perm: &[NodeId],
) -> f64 {
    let n = nodes1.len();
    let mut min_sim: f64 = 1.0;
    let mut any = false;
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let f1 = lookup(fps1, &nodes1[i], &nodes1[j]);
            let f2 = lookup(fps2, &perm[i], &perm[j]);
            let s = fingerprint_similarity(f1, f2);
            if s < min_sim {
                min_sim = s;
            }
            any = true;
        }
    }
    if any { min_sim } else { 0.0 }
}

fn permutations(items: &[NodeId]) -> Vec<Vec<NodeId>> {
    let mut current = items.to_vec();
    let mut out = Vec::new();
    permute(&mut current, 0, &mut out);
    out
}

fn permute(arr: &mut Vec<NodeId>, start: usize, out: &mut Vec<Vec<NodeId>>) {
    if start >= arr.len() {
        out.push(arr.clone());
        return;
    }
    for i in start..arr.len() {
        arr.swap(start, i);
        permute(arr, start + 1, out);
        arr.swap(start, i);
    }
}

fn lookup<'a>(fps: &'a [Fingerprint], source: &NodeId, target: &NodeId) -> &'a Fingerprint {
    fps.iter()
        .find(|f| &f.source == source && &f.target == target)
        .expect("fingerprint not found")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{
        ChainBB, FanIn3, FanOut3, Independent3, Loop3, MechanismA, MechanismB, MechanismC,
        MechanismD,
    };
    use std::collections::HashMap;

    fn rename_2(e: &Episode, new_a: &str, new_b: &str) -> Episode {
        let mut map = HashMap::new();
        map.insert(e.nodes[0].clone(), NodeId::new(new_a));
        map.insert(e.nodes[1].clone(), NodeId::new(new_b));
        e.rename(|n| map.get(n).cloned().unwrap())
    }

    fn rename_n(e: &Episode, new_names: &[&str]) -> Episode {
        let mut map = HashMap::new();
        for (orig, new) in e.nodes.iter().zip(new_names.iter()) {
            map.insert(orig.clone(), NodeId::new(*new));
        }
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

    fn forward<'a>(fps: &'a [Fingerprint], s: &str, t: &str) -> &'a Fingerprint {
        fps.iter()
            .find(|f| f.source == NodeId::new(s) && f.target == NodeId::new(t))
            .expect("forward fingerprint missing")
    }

    /// L4 / T5 first slice: chain composition recovery.
    ///
    /// For `A → B → C` with embedded latencies `lag_ab` and `lag_bc`,
    /// the directly-observed `A → C` fingerprint must report the
    /// composed latency `lag_ab + lag_bc`. All backward latencies stay
    /// 0 because the cross-correlation scan is `k ≥ 0`.
    #[test]
    fn chain_bb_recovers_composed_latency() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let chain = ChainBB::default_triple(n1, n2, n3);
        let ep = chain.generate("Ec", 500, 7);
        let fps = estimate_all(&ep);
        let ab = forward(&fps, "N1", "N2");
        let bc = forward(&fps, "N2", "N3");
        let ac = forward(&fps, "N1", "N3");
        assert_eq!(ab.latency, chain.lag_ab as i32, "AB latency");
        assert_eq!(bc.latency, chain.lag_bc as i32, "BC latency");
        assert_eq!(
            ac.latency,
            (chain.lag_ab + chain.lag_bc) as i32,
            "AC composition: expected {}, got {}",
            chain.lag_ab + chain.lag_bc,
            ac.latency
        );
        for &(s, t) in &[("N2", "N1"), ("N3", "N2"), ("N3", "N1")] {
            assert_eq!(forward(&fps, s, t).latency, 0, "backward {s}->{t}");
        }
    }

    /// A1 lifted to 3 nodes: `episode_similarity` over an arbitrary
    /// renaming of a 3-node chain is exactly 1.0 (the rename's
    /// inverse-bijection wins over the 6 permutations).
    #[test]
    fn three_node_rename_similarity_is_one() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let chain = ChainBB::default_triple(n1, n2, n3);
        let ep = chain.generate("E", 300, 11);
        let renamed = rename_n(&ep, &["XX", "YY", "ZZ"]);
        let sim = episode_similarity(&ep, &renamed);
        assert_eq!(sim.to_bits(), 1.0f64.to_bits(), "sim: {sim}");
    }

    // Distinguishability between *chain variants* (same chain
    // structure, different lag values) is not a clean T3 test — both
    // produce the same fingerprint *shape* (forward lag > 0, backward
    // = 0, PE moderate, CE low) and differ only in absolute lag values
    // which weigh 1/6 of the fingerprint vector. Chains-are-chains.
    // Real structural distinguishability needs different patterns,
    // not different parameters of the same pattern (below).

    /// Independent3: every directed pair has no real signal. Latency
    /// stays 0 and `position_effect` is low. `velocity_effect` has a
    /// finite-sample noise floor that depends on quartile bin sizes;
    /// we use 1200 timesteps (300 per quartile) and assert
    /// `velocity_effect < 0.35`.
    #[test]
    fn independent3_signature_is_null_baseline() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let ind = Independent3::default_triple(n1, n2, n3);
        let ep = ind.generate("E", 1200, 3);
        let fps = estimate_all(&ep);
        for f in &fps {
            assert_eq!(f.latency, 0, "{:?}->{:?}", f.source, f.target);
            assert!(f.position_effect < 0.15, "PE: {}", f.position_effect);
            assert!(f.velocity_effect < 0.35, "VE: {}", f.velocity_effect);
        }
    }

    /// FanOut3 recovery:
    ///
    /// - A → B latency = `lag_b`
    /// - A → C latency = `lag_c`
    /// - B → C derived latency = `lag_c − lag_b` (shared-driver lag
    ///   gap; B leads C by that gap because they observe the same
    ///   driver A at different delays)
    /// - all backward latencies = 0 (the scan is `k ≥ 0`)
    #[test]
    fn fanout3_recovers_shared_driver_lag_gap() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let fo = FanOut3::default_triple(n1, n2, n3);
        let ep = fo.generate("Ef", 500, 13);
        let fps = estimate_all(&ep);
        let ab = forward(&fps, "N1", "N2");
        let ac = forward(&fps, "N1", "N3");
        let bc = forward(&fps, "N2", "N3");
        assert_eq!(ab.latency, fo.lag_b as i32, "A->B");
        assert_eq!(ac.latency, fo.lag_c as i32, "A->C");
        assert_eq!(
            bc.latency,
            (fo.lag_c - fo.lag_b) as i32,
            "B->C should be lag_c - lag_b = {}",
            fo.lag_c - fo.lag_b
        );
        for &(s, t) in &[("N2", "N1"), ("N3", "N1"), ("N3", "N2")] {
            assert_eq!(forward(&fps, s, t).latency, 0, "backward {s}->{t}");
        }
    }

    /// Chain vs Independent: Chain self-sim > Chain vs Indep. The
    /// gap is large because Independent has near-zero fingerprints
    /// while Chain has clear forward latencies. This subset of the
    /// distinguishability matrix is robust under the current
    /// average-pairwise metric.
    #[test]
    fn chain_distinguishable_from_independent() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let chain1 = ChainBB::default_triple(n1.clone(), n2.clone(), n3.clone())
            .generate("E", 800, 1);
        let chain2 = ChainBB::default_triple(n1.clone(), n2.clone(), n3.clone())
            .generate("E", 800, 2);
        let indep1 = Independent3::default_triple(n1, n2, n3).generate("E", 800, 1);
        let self_sim = episode_similarity(&chain1, &chain2);
        let cross_sim = episode_similarity(&chain1, &indep1);
        assert!(
            self_sim > cross_sim,
            "self={self_sim}, cross={cross_sim}"
        );
    }

    /// Chain vs FanOut distinguishability under the structure-aware
    /// `min` reading. Both share the same fingerprint shape but
    /// differ in which 2 of 6 directed pairs carry the non-trivial
    /// latencies. The worst-matched pair under any bijection exposes
    /// the structural mismatch; mean-pairwise had averaged it away.
    #[test]
    fn three_node_chain_distinguishable_from_fanout() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let chain1 =
            ChainBB::default_triple(n1.clone(), n2.clone(), n3.clone()).generate("E", 1200, 1);
        let chain2 =
            ChainBB::default_triple(n1.clone(), n2.clone(), n3.clone()).generate("E", 1200, 2);
        let fanout1 = FanOut3::default_triple(n1, n2, n3).generate("E", 1200, 1);
        let self_sim = episode_similarity(&chain1, &chain2);
        let cross_sim = episode_similarity(&chain1, &fanout1);
        assert!(
            self_sim > cross_sim,
            "self={self_sim}, cross={cross_sim}"
        );
    }

    /// FanIn3 recovery: `source → target` pairs carry the embedded
    /// lags, while the two source–source directions stay at 0 (A and
    /// B are independent walks with no shared driver).
    #[test]
    fn fanin3_recovers_source_to_target_lags() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let fi = FanIn3::default_triple(n1, n2, n3);
        let ep = fi.generate("Ei", 1000, 19);
        let fps = estimate_all(&ep);
        let ac = forward(&fps, "N1", "N3");
        let bc = forward(&fps, "N2", "N3");
        let ab = forward(&fps, "N1", "N2");
        let ba = forward(&fps, "N2", "N1");
        assert_eq!(ac.latency, fi.lag_a as i32, "A->C");
        assert_eq!(bc.latency, fi.lag_b as i32, "B->C");
        assert_eq!(ab.latency, 0, "A->B (independent sources)");
        assert_eq!(ba.latency, 0, "B->A (independent sources)");
        for &(s, t) in &[("N3", "N1"), ("N3", "N2")] {
            assert_eq!(forward(&fps, s, t).latency, 0, "backward {s}->{t}");
        }
    }

    /// Loop3 recovery: every directed pair carries a positive forward
    /// latency. Direct hops in the cycle give lag 1; two-hop paths
    /// give lag 2. No pair stays at 0 — Loop3 is the densest 3-node
    /// pattern in the library.
    #[test]
    fn loop3_recovers_all_six_directed_lags() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let lp = Loop3::default_triple(n1, n2, n3);
        let ep = lp.generate("El", 1000, 23);
        let fps = estimate_all(&ep);
        // Direct cycle hops: A→B, B→C, C→A all at lag 1.
        for (s, t) in [("N1", "N2"), ("N2", "N3"), ("N3", "N1")] {
            let f = forward(&fps, s, t);
            assert_eq!(f.latency, 1, "{s}->{t} direct hop");
        }
        // Two-hop paths along the cycle: A→C, B→A, C→B all at lag 2.
        for (s, t) in [("N1", "N3"), ("N2", "N1"), ("N3", "N2")] {
            let f = forward(&fps, s, t);
            assert_eq!(f.latency, 2, "{s}->{t} two-hop");
        }
    }

    /// T5 — chain composition prediction. Predict AC's fingerprint
    /// from AB's and BC's fingerprints alone (no AC observation), then
    /// compare with the observed AC. Latency must compose additively
    /// and exactly; the full predicted vector must be similar enough
    /// to the observed vector.
    #[test]
    fn t5_predicts_chain_composition_matching_observed() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let chain = ChainBB::default_triple(n1, n2, n3);
        let ep = chain.generate("E", 1200, 31);
        let fps = estimate_all(&ep);
        let ab = forward(&fps, "N1", "N2");
        let bc = forward(&fps, "N2", "N3");
        let observed_ac = forward(&fps, "N1", "N3");

        let predicted_ac = predict_chain_composition(ab, bc);
        assert_eq!(
            predicted_ac.latency, observed_ac.latency,
            "latency: predicted={} observed={}",
            predicted_ac.latency, observed_ac.latency
        );
        assert_eq!(
            predicted_ac.latency,
            (chain.lag_ab + chain.lag_bc) as i32,
            "additive law"
        );
        let sim = fingerprint_similarity(&predicted_ac, observed_ac);
        assert!(
            sim > 0.8,
            "predicted vs observed AC similarity: {sim} (predicted={:?}, observed={:?})",
            predicted_ac,
            observed_ac
        );
    }

    /// Full 5-pattern distinguishability matrix on
    /// {Chain, FanOut, FanIn, Loop, Independent}. Every row's
    /// diagonal entry dominates every off-diagonal entry in that row
    /// under the structure-aware `min` metric.
    #[test]
    fn three_node_full_pattern_distinguishability() {
        type Gen = fn(u64) -> Episode;
        let pats: [(&str, Gen); 5] = [
            ("Chain", |s| {
                ChainBB::default_triple(
                    NodeId::new("N1"),
                    NodeId::new("N2"),
                    NodeId::new("N3"),
                )
                .generate("E", 1200, s)
            }),
            ("FanOut", |s| {
                FanOut3::default_triple(
                    NodeId::new("N1"),
                    NodeId::new("N2"),
                    NodeId::new("N3"),
                )
                .generate("E", 1200, s)
            }),
            ("FanIn", |s| {
                FanIn3::default_triple(
                    NodeId::new("N1"),
                    NodeId::new("N2"),
                    NodeId::new("N3"),
                )
                .generate("E", 1200, s)
            }),
            ("Loop", |s| {
                Loop3::default_triple(
                    NodeId::new("N1"),
                    NodeId::new("N2"),
                    NodeId::new("N3"),
                )
                .generate("E", 1200, s)
            }),
            ("Indep", |s| {
                Independent3::default_triple(
                    NodeId::new("N1"),
                    NodeId::new("N2"),
                    NodeId::new("N3"),
                )
                .generate("E", 1200, s)
            }),
        ];
        for i in 0..5 {
            let (l_i, gen_i) = pats[i];
            let self_sim = episode_similarity(&gen_i(1), &gen_i(2));
            for j in 0..5 {
                if i == j {
                    continue;
                }
                let (l_j, gen_j) = pats[j];
                let cross_sim = episode_similarity(&gen_i(1), &gen_j(1));
                assert!(
                    self_sim > cross_sim,
                    "self({l_i}-{l_i})={self_sim} not > cross({l_i}-{l_j})={cross_sim}"
                );
            }
        }
    }
}
