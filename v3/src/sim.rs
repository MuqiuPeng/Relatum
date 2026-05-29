//! Synthetic mechanisms for v3.
//!
//! The simulator embeds hidden mechanisms in generated episodes. The
//! fingerprint recovery side (`fingerprint.rs`) must work from
//! observation alone — it has no access to which mechanism produced
//! which episode (A2).
//!
//! M1 shipped mechanism A. M2 adds B (latency propagation), C
//! (synchronization via a shared hidden trajectory), and D (target
//! velocity suppression). M3 adds `ChainBB`, a 3-node composition
//! (two stacked B mechanisms). E and F (n-ary primitives, A4) are
//! still TODO.

use crate::rng::Rng;
use crate::{Episode, NodeId, Observation, StateVec};
use std::collections::BTreeMap;

/// Mechanism A — source change compresses target's reachable state set.
///
/// When the source's first coordinate is above `active_threshold`, the
/// target is forced near `compressed_center` with tight noise. Otherwise
/// the target performs a free random walk. The source itself is an
/// independent random walk.
///
/// This is binary, so A4 (binary projection + irreducibility) does not
/// apply. The first n-ary mechanism (E or F in M4) will need to
/// implement that contract.
pub struct MechanismA {
    pub source: NodeId,
    pub target: NodeId,
    pub active_threshold: f64,
    pub compressed_sigma: f64,
    pub free_sigma: f64,
    pub source_step_sigma: f64,
    pub compressed_center: StateVec,
}

impl MechanismA {
    /// Sensible defaults for a `d = 2` system. Tuned so M1 fingerprint
    /// recovery has clear directional signal.
    pub fn default_pair(source: NodeId, target: NodeId) -> Self {
        MechanismA {
            source,
            target,
            active_threshold: 0.5,
            compressed_sigma: 0.04,
            free_sigma: 0.18,
            source_step_sigma: 0.08,
            compressed_center: vec![0.5, 0.5],
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = self.compressed_center.len();
        let mut s = vec![0.5; d];
        let mut t = vec![0.5; d];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                s[i] = (s[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
            }
            if s[0] > self.active_threshold {
                for i in 0..d {
                    t[i] = (self.compressed_center[i] + rng.normal(0.0, self.compressed_sigma))
                        .clamp(0.0, 1.0);
                }
            } else {
                for i in 0..d {
                    t[i] = (t[i] + rng.normal(0.0, self.free_sigma)).clamp(0.0, 1.0);
                }
            }
            let mut states = BTreeMap::new();
            states.insert(self.source.clone(), s.clone());
            states.insert(self.target.clone(), t.clone());
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![self.source.clone(), self.target.clone()],
            observations,
            interventions: vec![],
        }
    }
}

/// Mechanism B — source change propagates to target after fixed latency.
///
/// Source random walks. Target's state at time `t` equals source's state
/// at time `t - latency`, plus small observation noise. The forward
/// direction has a clear lag; the backward direction has none.
///
/// Binary, no n-ary obligations.
pub struct MechanismB {
    pub source: NodeId,
    pub target: NodeId,
    pub latency: u32,
    pub source_step_sigma: f64,
    pub propagation_noise: f64,
}

impl MechanismB {
    pub fn default_pair(source: NodeId, target: NodeId) -> Self {
        MechanismB {
            source,
            target,
            latency: 2,
            source_step_sigma: 0.08,
            propagation_noise: 0.03,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let mut s = vec![0.5; d];
        let lag = self.latency as usize;
        let mut history: Vec<StateVec> = Vec::with_capacity(steps as usize + lag + 1);
        for _ in 0..=lag {
            history.push(s.clone());
        }
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                s[i] = (s[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
            }
            history.push(s.clone());
            let past = &history[history.len() - 1 - lag];
            let t: StateVec = past
                .iter()
                .map(|x| (x + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0))
                .collect();
            let mut states = BTreeMap::new();
            states.insert(self.source.clone(), s.clone());
            states.insert(self.target.clone(), t);
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![self.source.clone(), self.target.clone()],
            observations,
            interventions: vec![],
        }
    }
}

/// Mechanism C — symmetric synchronization via a shared hidden trajectory.
///
/// Both nodes are noisy observations of the same drifting hidden state.
/// They co-vary strongly with no causal direction. The forward and
/// backward fingerprints should be nearly identical (directionality ≈ 0).
///
/// Binary, no n-ary obligations.
pub struct MechanismC {
    pub left: NodeId,
    pub right: NodeId,
    pub hidden_drift_sigma: f64,
    pub observation_noise: f64,
}

impl MechanismC {
    pub fn default_pair(left: NodeId, right: NodeId) -> Self {
        MechanismC {
            left,
            right,
            hidden_drift_sigma: 0.10,
            observation_noise: 0.03,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let mut hidden = vec![0.5; d];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                hidden[i] = (hidden[i] + rng.normal(0.0, self.hidden_drift_sigma)).clamp(0.0, 1.0);
            }
            let l: StateVec = hidden
                .iter()
                .map(|h| (h + rng.normal(0.0, self.observation_noise)).clamp(0.0, 1.0))
                .collect();
            let r: StateVec = hidden
                .iter()
                .map(|h| (h + rng.normal(0.0, self.observation_noise)).clamp(0.0, 1.0))
                .collect();
            let mut states = BTreeMap::new();
            states.insert(self.left.clone(), l);
            states.insert(self.right.clone(), r);
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![self.left.clone(), self.right.clone()],
            observations,
            interventions: vec![],
        }
    }
}

/// Mechanism D — source suppresses target's change magnitude.
///
/// When the source's first coordinate is above `active_threshold`, the
/// target's step noise drops to `suppressed_sigma` (nearly frozen).
/// Otherwise the target free-walks with `free_sigma`. Unlike A (which
/// snaps the target to a fixed center), D leaves the target's *position*
/// alone — only its *velocity* is constrained.
///
/// Binary, no n-ary obligations.
pub struct MechanismD {
    pub source: NodeId,
    pub target: NodeId,
    pub active_threshold: f64,
    pub suppressed_sigma: f64,
    pub free_sigma: f64,
    pub source_step_sigma: f64,
}

impl MechanismD {
    pub fn default_pair(source: NodeId, target: NodeId) -> Self {
        MechanismD {
            source,
            target,
            active_threshold: 0.5,
            suppressed_sigma: 0.005,
            free_sigma: 0.15,
            source_step_sigma: 0.08,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let mut s = vec![0.5; d];
        let mut t = vec![0.5; d];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                s[i] = (s[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
            }
            let sig = if s[0] > self.active_threshold {
                self.suppressed_sigma
            } else {
                self.free_sigma
            };
            for i in 0..d {
                t[i] = (t[i] + rng.normal(0.0, sig)).clamp(0.0, 1.0);
            }
            let mut states = BTreeMap::new();
            states.insert(self.source.clone(), s.clone());
            states.insert(self.target.clone(), t.clone());
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![self.source.clone(), self.target.clone()],
            observations,
            interventions: vec![],
        }
    }
}

/// 3-node chain composition: A → B → C, where each hop is a latency
/// propagation (mechanism B). Used for L4 / T5 — the direct AC
/// fingerprint should report a composed latency of `lag_ab + lag_bc`.
///
/// Composition of *binary* mechanisms. Not an n-ary primitive; A4 does
/// not apply.
pub struct ChainBB {
    pub a: NodeId,
    pub b: NodeId,
    pub c: NodeId,
    pub lag_ab: u32,
    pub lag_bc: u32,
    pub source_step_sigma: f64,
    pub propagation_noise: f64,
}

impl ChainBB {
    pub fn default_triple(a: NodeId, b: NodeId, c: NodeId) -> Self {
        ChainBB {
            a,
            b,
            c,
            lag_ab: 2,
            lag_bc: 3,
            source_step_sigma: 0.08,
            propagation_noise: 0.03,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let lag_ab = self.lag_ab as usize;
        let lag_bc = self.lag_bc as usize;
        let mut a = vec![0.5; d];
        let mut a_history: Vec<StateVec> = vec![vec![0.5; d]; lag_ab + 1];
        let mut b_history: Vec<StateVec> = vec![vec![0.5; d]; lag_bc + 1];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                a[i] = (a[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
            }
            a_history.push(a.clone());
            let a_past = &a_history[a_history.len() - 1 - lag_ab];
            let b: StateVec = a_past
                .iter()
                .map(|x| (x + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0))
                .collect();
            b_history.push(b.clone());
            let b_past = &b_history[b_history.len() - 1 - lag_bc];
            let c: StateVec = b_past
                .iter()
                .map(|x| (x + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0))
                .collect();
            let mut states = BTreeMap::new();
            states.insert(self.a.clone(), a.clone());
            states.insert(self.b.clone(), b);
            states.insert(self.c.clone(), c);
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![self.a.clone(), self.b.clone(), self.c.clone()],
            observations,
            interventions: vec![],
        }
    }
}

/// 3 independent random walkers — no coupling. The null hypothesis
/// baseline for 3-node pattern distinguishability.
pub struct Independent3 {
    pub a: NodeId,
    pub b: NodeId,
    pub c: NodeId,
    pub step_sigma: f64,
}

impl Independent3 {
    pub fn default_triple(a: NodeId, b: NodeId, c: NodeId) -> Self {
        Independent3 {
            a,
            b,
            c,
            step_sigma: 0.08,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let mut a = vec![0.5; d];
        let mut b = vec![0.5; d];
        let mut c = vec![0.5; d];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                a[i] = (a[i] + rng.normal(0.0, self.step_sigma)).clamp(0.0, 1.0);
                b[i] = (b[i] + rng.normal(0.0, self.step_sigma)).clamp(0.0, 1.0);
                c[i] = (c[i] + rng.normal(0.0, self.step_sigma)).clamp(0.0, 1.0);
            }
            let mut states = BTreeMap::new();
            states.insert(self.a.clone(), a.clone());
            states.insert(self.b.clone(), b.clone());
            states.insert(self.c.clone(), c.clone());
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![self.a.clone(), self.b.clone(), self.c.clone()],
            observations,
            interventions: vec![],
        }
    }
}

/// Fan-out: A → B and A → C, both via latency propagation. The shared
/// driver makes B and C appear correlated with a *derived* lag of
/// `lag_c − lag_b` (positive when `lag_c > lag_b`). This is the
/// "explainable indirect correlation" pattern — useful for testing
/// that the fingerprint engine reports the *observed* B → C lag
/// rather than incorrectly inferring direct propagation.
pub struct FanOut3 {
    pub source: NodeId,
    pub target_b: NodeId,
    pub target_c: NodeId,
    pub lag_b: u32,
    pub lag_c: u32,
    pub source_step_sigma: f64,
    pub propagation_noise: f64,
}

impl FanOut3 {
    pub fn default_triple(source: NodeId, b: NodeId, c: NodeId) -> Self {
        FanOut3 {
            source,
            target_b: b,
            target_c: c,
            lag_b: 2,
            lag_c: 3,
            source_step_sigma: 0.08,
            propagation_noise: 0.03,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let lag_b = self.lag_b as usize;
        let lag_c = self.lag_c as usize;
        let max_lag = lag_b.max(lag_c);
        let mut a = vec![0.5; d];
        let mut a_history: Vec<StateVec> = vec![vec![0.5; d]; max_lag + 1];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                a[i] = (a[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
            }
            a_history.push(a.clone());
            let a_past_b = &a_history[a_history.len() - 1 - lag_b];
            let b: StateVec = a_past_b
                .iter()
                .map(|x| (x + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0))
                .collect();
            let a_past_c = &a_history[a_history.len() - 1 - lag_c];
            let c: StateVec = a_past_c
                .iter()
                .map(|x| (x + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0))
                .collect();
            let mut states = BTreeMap::new();
            states.insert(self.source.clone(), a.clone());
            states.insert(self.target_b.clone(), b);
            states.insert(self.target_c.clone(), c);
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![
                self.source.clone(),
                self.target_b.clone(),
                self.target_c.clone(),
            ],
            observations,
            interventions: vec![],
        }
    }
}

/// Fan-in: A → C and B → C, where A and B are independent random
/// walkers and C is a noisy mixture of their lagged values. Distinct
/// from FanOut by having `A → B` carry no signal (the two sources do
/// not share a driver), and by having only 2 of 6 directed pairs
/// active (`A → C`, `B → C`).
pub struct FanIn3 {
    pub source_a: NodeId,
    pub source_b: NodeId,
    pub target: NodeId,
    pub lag_a: u32,
    pub lag_b: u32,
    pub source_step_sigma: f64,
    pub propagation_noise: f64,
}

impl FanIn3 {
    pub fn default_triple(a: NodeId, b: NodeId, target: NodeId) -> Self {
        FanIn3 {
            source_a: a,
            source_b: b,
            target,
            lag_a: 2,
            lag_b: 3,
            source_step_sigma: 0.08,
            propagation_noise: 0.03,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let lag_a = self.lag_a as usize;
        let lag_b = self.lag_b as usize;
        let mut a = vec![0.5; d];
        let mut b = vec![0.5; d];
        let mut a_history: Vec<StateVec> = vec![vec![0.5; d]; lag_a + 1];
        let mut b_history: Vec<StateVec> = vec![vec![0.5; d]; lag_b + 1];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            for i in 0..d {
                a[i] = (a[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
                b[i] = (b[i] + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
            }
            a_history.push(a.clone());
            b_history.push(b.clone());
            let a_past = &a_history[a_history.len() - 1 - lag_a];
            let b_past = &b_history[b_history.len() - 1 - lag_b];
            let c: StateVec = (0..d)
                .map(|i| {
                    let mix = 0.5 * a_past[i] + 0.5 * b_past[i];
                    (mix + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0)
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

/// 3-cycle: A → B → C → A with one-step propagation noise per hop.
/// All six directed pairs carry a positive forward latency under the
/// `k ≥ 0` scan: direct hops at lag 1 (`A → B`, `B → C`, `C → A`) and
/// two-hop paths at lag 2 (`A → C`, `B → A`, `C → B`). Structurally
/// the most "dense" 3-node pattern.
pub struct Loop3 {
    pub a: NodeId,
    pub b: NodeId,
    pub c: NodeId,
    pub propagation_noise: f64,
}

impl Loop3 {
    pub fn default_triple(a: NodeId, b: NodeId, c: NodeId) -> Self {
        Loop3 {
            a,
            b,
            c,
            propagation_noise: 0.04,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let d = 2;
        let mut a = vec![0.5; d];
        let mut b = vec![0.5; d];
        let mut c = vec![0.5; d];
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            let prev_a = a.clone();
            let prev_b = b.clone();
            let prev_c = c.clone();
            for i in 0..d {
                a[i] = (prev_c[i] + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0);
                b[i] = (prev_a[i] + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0);
                c[i] = (prev_b[i] + rng.normal(0.0, self.propagation_noise)).clamp(0.0, 1.0);
            }
            let mut states = BTreeMap::new();
            states.insert(self.a.clone(), a.clone());
            states.insert(self.b.clone(), b.clone());
            states.insert(self.c.clone(), c.clone());
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![self.a.clone(), self.b.clone(), self.c.clone()],
            observations,
            interventions: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mechanism_a_generates_episode_with_declared_nodes() {
        let m = MechanismA::default_pair(NodeId::new("S"), NodeId::new("T"));
        let ep = m.generate("E1", 50, 1);
        assert_eq!(ep.nodes.len(), 2);
        assert_eq!(ep.observations.len(), 50);
        for obs in &ep.observations {
            assert!(obs.states.contains_key(&NodeId::new("S")));
            assert!(obs.states.contains_key(&NodeId::new("T")));
        }
    }

    /// Determinism guard: same seed reproduces the same trajectory bit-for-bit.
    /// The rename-invariance benchmark depends on this.
    #[test]
    fn mechanism_a_is_deterministic_under_seed() {
        let m = MechanismA::default_pair(NodeId::new("S"), NodeId::new("T"));
        let a = m.generate("E", 30, 7);
        let b = m.generate("E", 30, 7);
        for (oa, ob) in a.observations.iter().zip(b.observations.iter()) {
            for (k, va) in &oa.states {
                let vb = ob.states.get(k).unwrap();
                for (x, y) in va.iter().zip(vb.iter()) {
                    assert_eq!(x.to_bits(), y.to_bits());
                }
            }
        }
    }
}
