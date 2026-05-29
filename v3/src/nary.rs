//! n-ary primitives — M4.
//!
//! v3 commitment A4 makes binary projection + irreducibility a
//! build-time obligation. The `NaryMechanism` trait carries the
//! obligation at the type level: any n-ary simulator added to v3
//! must implement it or it is rejected at the compile step. There is
//! no path to ship an n-ary primitive without honoring the contract.
//!
//! M4 ships two ternary mechanisms — the two `arity = 3` entries in
//! the design-notes mechanism library:
//!
//! - `MechanismE` — gating: a gate node `A` controls whether `B → C`
//!   propagation is active. Irreducible because the rule "B drives C
//!   when A is on" is not expressible as a composition of binary R
//!   primitives; it needs a third variable.
//! - `MechanismF` — XOR-style joint determination: `C = (A + B) mod 1`.
//!   Each pairwise marginal `A → C` and `B → C` is null because the
//!   mod-1 boundary destroys the marginal correlation; only the joint
//!   `A, B` predicts `C`. From the pairwise fingerprint engine, F is
//!   indistinguishable from `Independent3`; the irreducibility flag
//!   asserts the hidden joint structure that the substrate cannot
//!   recover at M4.

use crate::rng::Rng;
use crate::{Episode, NodeId, Observation, StateVec};
use std::collections::BTreeMap;

/// Predicted pairwise marginal under the binary projection.
///
/// Used by `NaryMechanism::binary_projection`. Recovery from the
/// generated episode should agree with these on `expected_latency`
/// (exact) and on `effect_size ≤ max_effect_size` (upper bound).
#[derive(Debug, Clone)]
pub struct PredictedPair {
    pub source: NodeId,
    pub target: NodeId,
    pub expected_latency: i32,
    pub max_effect_size: f64,
}

/// A4 build-time contract for n-ary primitives.
pub trait NaryMechanism {
    fn arity(&self) -> usize;

    /// The pairwise marginals the recovery side should see when each
    /// directed pair is examined in isolation. Includes every ordered
    /// pair of distinct nodes the mechanism involves.
    fn binary_projection(&self) -> Vec<PredictedPair>;

    /// Author's assertion that this mechanism cannot be reduced to a
    /// composition of binary R primitives. True for E (gating) and F
    /// (XOR-style joint determination). Recovery from observation
    /// alone may not be able to detect irreducibility at M4 — the
    /// flag is metadata, not an observable.
    fn is_irreducible(&self) -> bool;
}

/// Mechanism E — gating.
///
/// Three nodes: `gate (A)`, `source (B)`, `target (C)`. `A` and `B`
/// are independent random walks. When `A`'s first coordinate exceeds
/// `gate_threshold`, `C` follows `B` at lag `propagation_lag`
/// (mechanism B style). When `A` is below threshold, `C` free-walks
/// (no influence from `B`).
///
/// **Irreducible.** The conditional "B drives C iff A is in state s"
/// cannot be expressed as a composition of binary R(B,C) and R(A,B)
/// primitives; the gating structure requires A as a third slot.
pub struct MechanismE {
    pub gate: NodeId,
    pub source: NodeId,
    pub target: NodeId,
    pub gate_threshold: f64,
    pub propagation_lag: u32,
    pub propagation_noise: f64,
    pub free_step_sigma: f64,
    pub source_step_sigma: f64,
}

impl MechanismE {
    pub fn default_triple(gate: NodeId, source: NodeId, target: NodeId) -> Self {
        MechanismE {
            gate,
            source,
            target,
            gate_threshold: 0.5,
            propagation_lag: 2,
            propagation_noise: 0.03,
            free_step_sigma: 0.12,
            source_step_sigma: 0.08,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let lag = self.propagation_lag as usize;
        let mut a = vec![0.5; d];
        let mut b = vec![0.5; d];
        let mut c = vec![0.5; d];
        let mut b_history: Vec<StateVec> = vec![vec![0.5; d]; lag + 1];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                a[i] = (a[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
                b[i] = (b[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
            }
            b_history.push(b.clone());
            let b_past = &b_history[b_history.len() - 1 - lag];
            if a[0] > self.gate_threshold {
                for i in 0..d {
                    c[i] = (b_past[i] + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0);
                }
            } else {
                for i in 0..d {
                    c[i] = (c[i] + rng.normal(0.0, self.free_step_sigma)).clamp(0.0, 1.0);
                }
            }
            let mut states = BTreeMap::new();
            states.insert(self.gate.clone(), a.clone());
            states.insert(self.source.clone(), b.clone());
            states.insert(self.target.clone(), c.clone());
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![self.gate.clone(), self.source.clone(), self.target.clone()],
            observations,
            interventions: vec![],
        }
    }
}

impl NaryMechanism for MechanismE {
    fn arity(&self) -> usize {
        3
    }

    fn binary_projection(&self) -> Vec<PredictedPair> {
        let lag = self.propagation_lag as i32;
        vec![
            // A → B: independent walks, null.
            PredictedPair {
                source: self.gate.clone(),
                target: self.source.clone(),
                expected_latency: 0,
                max_effect_size: 0.35,
            },
            // A → C: gating creates an A-binned spread asymmetry on
            // C (active vs free regimes), so CE / VE are visible even
            // though A doesn't directly drive C. Expect substantial
            // effect_size but no lag.
            PredictedPair {
                source: self.gate.clone(),
                target: self.target.clone(),
                expected_latency: 0,
                max_effect_size: 0.95,
            },
            // B → C: real propagation, but only when gate is open
            // (~50% of timesteps). Pearson at lag k = propagation_lag
            // is the half-strength average; still above the
            // recovery threshold of 0.25.
            PredictedPair {
                source: self.source.clone(),
                target: self.target.clone(),
                expected_latency: lag,
                max_effect_size: 0.85,
            },
            // B → A: independent, null.
            PredictedPair {
                source: self.source.clone(),
                target: self.gate.clone(),
                expected_latency: 0,
                max_effect_size: 0.35,
            },
            // C → A: weak / noise.
            PredictedPair {
                source: self.target.clone(),
                target: self.gate.clone(),
                expected_latency: 0,
                max_effect_size: 0.45,
            },
            // C → B: backward latency stays 0 (scan is `k ≥ 0`),
            // but PE is substantial. C ≈ B(t-lag) during open windows,
            // and B's own autocorrelation makes C's quartile a strong
            // predictor of B's current mean. Loose bound.
            PredictedPair {
                source: self.target.clone(),
                target: self.source.clone(),
                expected_latency: 0,
                max_effect_size: 0.85,
            },
        ]
    }

    fn is_irreducible(&self) -> bool {
        true
    }
}

/// Mechanism F — discrete XOR joint determination.
///
/// `A` and `B` are independent random walks on `[0, 1]`. Threshold
/// each at `0.5` to a boolean, XOR them, and set `C` to a high
/// center (`0.75`) when the XOR is true, a low center (`0.25`)
/// otherwise, plus small observation noise.
///
/// Discrete XOR is the cleanest pair-null joint mechanism: each
/// marginal `A → C` and `B → C` reads zero because conditioning on
/// only one source leaves the XOR's output 50/50 (mean ≈ 0.5,
/// variance equal across bins). The earlier continuous form
/// `(A + B) mod 1` leaked through the boundary-crossing dynamics on
/// `velocity_effect`; discrete XOR has clean step boundaries that
/// are equally frequent in every source quartile.
///
/// **Irreducible.** The mechanism is observationally indistinguishable
/// from `Independent3` at the pairwise level. The irreducibility flag
/// asserts the joint structure the M4 pairwise substrate cannot
/// recover.
pub struct MechanismF {
    pub source_a: NodeId,
    pub source_b: NodeId,
    pub target: NodeId,
    pub source_step_sigma: f64,
    pub propagation_noise: f64,
    pub threshold: f64,
    pub center_low: f64,
    pub center_high: f64,
}

impl MechanismF {
    pub fn default_triple(a: NodeId, b: NodeId, target: NodeId) -> Self {
        MechanismF {
            source_a: a,
            source_b: b,
            target,
            source_step_sigma: 0.08,
            propagation_noise: 0.02,
            threshold: 0.5,
            center_low: 0.25,
            center_high: 0.75,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let mut a = vec![0.5; d];
        let mut b = vec![0.5; d];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                a[i] = (a[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
                b[i] = (b[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
            }
            let c: StateVec = (0..d)
                .map(|i| {
                    let xor = (a[i] > self.threshold) ^ (b[i] > self.threshold);
                    let center = if xor { self.center_high } else { self.center_low };
                    (center + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0)
                })
                .collect();
            let mut states = BTreeMap::new();
            states.insert(self.source_a.clone(), a.clone());
            states.insert(self.source_b.clone(), b.clone());
            states.insert(self.target.clone(), c);
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![
                self.source_a.clone(),
                self.source_b.clone(),
                self.target.clone(),
            ],
            observations,
            interventions: vec![],
        }
    }
}

impl NaryMechanism for MechanismF {
    fn arity(&self) -> usize {
        3
    }

    fn binary_projection(&self) -> Vec<PredictedPair> {
        let nodes = [&self.source_a, &self.source_b, &self.target];
        let mut out = Vec::new();
        for s in &nodes {
            for t in &nodes {
                if s == t {
                    continue;
                }
                out.push(PredictedPair {
                    source: (*s).clone(),
                    target: (*t).clone(),
                    expected_latency: 0,
                    max_effect_size: 0.25,
                });
            }
        }
        out
    }

    fn is_irreducible(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::estimate_all;

    fn forward<'a>(
        fps: &'a [crate::Fingerprint],
        source: &NodeId,
        target: &NodeId,
    ) -> &'a crate::Fingerprint {
        fps.iter()
            .find(|f| &f.source == source && &f.target == target)
            .expect("fingerprint not found")
    }

    /// Mechanism E meets its own binary projection: latencies match
    /// exactly and effect sizes stay under the published bounds.
    #[test]
    fn mechanism_e_recovery_matches_binary_projection() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let m = MechanismE::default_triple(n1, n2, n3);
        let ep = m.generate("E", 1200, 7);
        let fps = estimate_all(&ep);
        assert_eq!(m.arity(), 3);
        assert!(m.is_irreducible());
        for pred in m.binary_projection() {
            let actual = forward(&fps, &pred.source, &pred.target);
            assert_eq!(
                actual.latency, pred.expected_latency,
                "{:?} -> {:?} lat: got {}, expected {}",
                pred.source, pred.target, actual.latency, pred.expected_latency
            );
            assert!(
                actual.effect_size <= pred.max_effect_size,
                "{:?} -> {:?} effect_size {} exceeds projection bound {}",
                pred.source,
                pred.target,
                actual.effect_size,
                pred.max_effect_size
            );
        }
    }

    /// Mechanism F is observationally indistinguishable from
    /// `Independent3` at the pairwise level: every directed pair
    /// fingerprint stays near zero. The irreducibility flag is the
    /// only signal of the joint XOR structure.
    #[test]
    fn mechanism_f_pairwise_null_irreducibility_metadata_only() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let m = MechanismF::default_triple(n1, n2, n3);
        let ep = m.generate("Ef", 2400, 5);
        let fps = estimate_all(&ep);
        assert_eq!(m.arity(), 3);
        assert!(m.is_irreducible());
        for pred in m.binary_projection() {
            let actual = forward(&fps, &pred.source, &pred.target);
            assert_eq!(actual.latency, 0, "{:?} -> {:?}", pred.source, pred.target);
            assert!(
                actual.effect_size <= pred.max_effect_size,
                "{:?} -> {:?} effect_size {} (XOR should be pair-null)",
                pred.source,
                pred.target,
                actual.effect_size
            );
        }
    }

    /// Joint structure check on F: even though pairwise A→C and B→C
    /// are null, the *joint* `(A_bool, B_bool) → C` reconstructs C
    /// to within noise. We do not use this as a recovery primitive
    /// yet — recovery of joint structure is an M4+ research item —
    /// but the simulator must satisfy the joint law it claims to
    /// embed.
    #[test]
    fn mechanism_f_joint_law_reconstructs_target() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let m = MechanismF::default_triple(n1.clone(), n2.clone(), n3.clone());
        let ep = m.generate("Ef", 200, 3);
        let mut residuals = Vec::new();
        for obs in &ep.observations {
            let a = obs.states.get(&n1).unwrap()[0];
            let b = obs.states.get(&n2).unwrap()[0];
            let c = obs.states.get(&n3).unwrap()[0];
            let xor = (a > m.threshold) ^ (b > m.threshold);
            let predicted = if xor { m.center_high } else { m.center_low };
            residuals.push((c - predicted).abs());
        }
        let mean_residual: f64 = residuals.iter().sum::<f64>() / residuals.len() as f64;
        // Should be on the order of `propagation_noise`.
        assert!(
            mean_residual < 0.05,
            "joint XOR law mismatch: mean residual = {mean_residual}"
        );
    }
}
