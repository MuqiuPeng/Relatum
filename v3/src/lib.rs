//! Relatum v3 — world-model substrate.
//!
//! Working proposition: recover de-named relation structure from
//! anonymous state sequences.
//!
//! Primitives: `state(node, t)` and `transition(node, t → t+1)`.
//! `R(x, y)` exists in v3 only as a derived layer recovered from state
//! dynamics — it is *not* the primitive.
//!
//! Ontological commitments: see `docs/constitution.md`.

pub mod causal;
pub mod fingerprint;
pub mod joint;
pub mod nary;
pub mod physical;
pub mod rng;
pub mod scheduling;
pub mod sim;
pub mod similarity;

pub use causal::{
    CausalDirection, CausalPair, EvalSummary, PairMeta, evaluate, iid_directionality, judge_pair,
    load_tubingen_dataset, pair_from_meta, pair_to_episode, parse_pair_data,
    parse_pairmeta_line,
};
pub use fingerprint::{Fingerprint, estimate_all};
pub use joint::{
    IrreducibilityClass, classify_irreducibility, conditional_effect_variance,
    irreducibility_signal, joint_interaction_effect, joint_position_effect,
};
pub use nary::{MechanismE, MechanismF, NaryMechanism, PredictedPair};
pub use physical::{
    CommonDriveReceivers, FrictionGate, GatedBouncingBall, SpringMassFollower,
};
pub use scheduling::{DriveSeekingScheduler, RoundRobinScheduler, Scheduler};
pub use sim::{
    ChainBB, FanIn3, FanOut3, Independent3, Loop3, MechanismA, MechanismB, MechanismC, MechanismD,
};
pub use similarity::{
    L1Pair, episode_similarity, episode_similarity_2node, fingerprint_similarity,
    fingerprint_similarity_v2, predict_chain_composition,
};

use std::collections::BTreeMap;

/// A node identifier.
///
/// Within an episode, two equal `NodeId`s denote the same node (v2
/// commitment #4, narrowed to episode scope by v3). Across episodes,
/// node identity is *structural* — do not compare `NodeId`s across
/// episodes (A1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub String);

impl NodeId {
    pub fn new(s: impl Into<String>) -> Self {
        NodeId(s.into())
    }
}

/// A state vector. v3 takes states as opaque numeric vectors at the
/// primitive layer; structural meaning comes from how states co-vary
/// across nodes and time, not from the contents.
pub type StateVec = Vec<f64>;

/// One observation at one timestep: every node's state vector at `t`.
#[derive(Debug, Clone)]
pub struct Observation {
    pub t: u64,
    pub states: BTreeMap<NodeId, StateVec>,
}

/// An external intervention applied at a specific timestep.
#[derive(Debug, Clone)]
pub struct Intervention {
    pub t: u64,
    pub target: NodeId,
    pub set_state: StateVec,
}

/// One episode — the v3 training unit (see `docs/design-notes.md`).
#[derive(Debug, Clone)]
pub struct Episode {
    pub id: String,
    pub nodes: Vec<NodeId>,
    pub observations: Vec<Observation>,
    pub interventions: Vec<Intervention>,
}

impl Episode {
    /// Apply a node-relabeling bijection.
    ///
    /// A1 requires that any objective be invariant under this operation
    /// up to the same bijection on outputs. This method exists to make
    /// that invariance testable.
    pub fn rename<F>(&self, f: F) -> Episode
    where
        F: Fn(&NodeId) -> NodeId,
    {
        let nodes = self.nodes.iter().map(&f).collect();
        let observations = self
            .observations
            .iter()
            .map(|o| Observation {
                t: o.t,
                states: o.states.iter().map(|(n, s)| (f(n), s.clone())).collect(),
            })
            .collect();
        let interventions = self
            .interventions
            .iter()
            .map(|i| Intervention {
                t: i.t,
                target: f(&i.target),
                set_state: i.set_state.clone(),
            })
            .collect();
        Episode {
            id: self.id.clone(),
            nodes,
            observations,
            interventions,
        }
    }

    /// Coarse canonical form invariant under node-id bijection.
    ///
    /// For each timestep, returns the *multiset* of state vectors with
    /// node identities stripped. Two episodes that are equal up to
    /// node-relabeling have equal canonical forms.
    ///
    /// This is intentionally weak: it preserves no temporal-node
    /// correspondence and so is not the full fingerprint. It is the
    /// minimum needed to write the rename-invariance guard for A1; the
    /// real fingerprint engine will replace it.
    pub fn canonical_form(&self) -> Vec<(u64, Vec<StateVec>)> {
        let mut out: Vec<(u64, Vec<StateVec>)> = self
            .observations
            .iter()
            .map(|o| {
                let mut states: Vec<StateVec> = o.states.values().cloned().collect();
                states.sort_by(|a, b| {
                    a.iter()
                        .zip(b.iter())
                        .map(|(x, y)| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal))
                        .find(|o| *o != std::cmp::Ordering::Equal)
                        .unwrap_or_else(|| a.len().cmp(&b.len()))
                });
                (o.t, states)
            })
            .collect();
        out.sort_by_key(|(t, _)| *t);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn toy_episode() -> Episode {
        let mut t0 = BTreeMap::new();
        t0.insert(NodeId::new("N1"), vec![0.1, 0.7]);
        t0.insert(NodeId::new("N2"), vec![0.4, 0.2]);
        let mut t1 = BTreeMap::new();
        t1.insert(NodeId::new("N1"), vec![0.8, 0.6]);
        t1.insert(NodeId::new("N2"), vec![0.2, 0.2]);
        Episode {
            id: "toy".into(),
            nodes: vec![NodeId::new("N1"), NodeId::new("N2")],
            observations: vec![
                Observation { t: 0, states: t0 },
                Observation { t: 1, states: t1 },
            ],
            interventions: vec![],
        }
    }

    /// A1 guard: the canonical form is invariant under any node-id
    /// bijection. If this test ever fails, some structural attribute
    /// has leaked through the node-name channel.
    #[test]
    fn rename_invariance_preserves_canonical_form() {
        let e = toy_episode();
        let mut map = HashMap::new();
        map.insert(NodeId::new("N1"), NodeId::new("X7"));
        map.insert(NodeId::new("N2"), NodeId::new("A3"));
        let renamed = e.rename(|n| map.get(n).cloned().unwrap_or_else(|| n.clone()));
        assert_eq!(e.canonical_form(), renamed.canonical_form());
    }

    /// v2 #4 narrowed to episode scope: within one episode, equal
    /// `NodeId`s denote the same node. Verified by the BTreeMap refusing
    /// to hold two separate entries for the same id.
    #[test]
    fn within_episode_token_identity() {
        let mut states = BTreeMap::new();
        states.insert(NodeId::new("N1"), vec![0.1]);
        states.insert(NodeId::new("N1"), vec![0.9]);
        assert_eq!(states.len(), 1);
        assert_eq!(states.get(&NodeId::new("N1")).unwrap(), &vec![0.9]);
    }

    /// Well-formedness: every observed state and every intervention
    /// references a declared node. A foundation for any later
    /// fingerprint computation.
    #[test]
    fn episode_wellformedness_holds() {
        let e = toy_episode();
        let declared: std::collections::HashSet<_> = e.nodes.iter().collect();
        for obs in &e.observations {
            for n in obs.states.keys() {
                assert!(declared.contains(n), "undeclared node {:?}", n);
            }
        }
        for iv in &e.interventions {
            assert!(declared.contains(&iv.target));
        }
    }
}
