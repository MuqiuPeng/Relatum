//! Physical-realm bridge — lo-fi PHYRE substitute.
//!
//! The cross-realm test for v3: take a structural mechanism we have
//! a synthetic version of (mechanism A — source-state-conditioned
//! compression of target's reachable region), realise it in a
//! *physical* setting (bouncing ball, walls, velocity, collisions),
//! and check that v3's fingerprint engine recovers a signature in
//! the same neighbourhood. If it does, "mechanism A" is a structural
//! primitive that survives the realm change. If it doesn't, our
//! "structural recovery" is secretly a property of the continuous-
//! random-walk substrate.
//!
//! This is a **substitute** for the real PHYRE benchmark — installing
//! PHYRE requires their Python/sim engine. The shim here is what
//! matters: minimal physics that exhibits the same structural
//! property v3 already detects on synthetic data.

use crate::rng::Rng;
use crate::{Episode, NodeId, Observation};
use std::collections::BTreeMap;

/// Gated bouncing-ball: physical-realm analogue of mechanism A.
///
/// Node `gate` is a slow 1-D random walker. Node `ball` is a 1-D
/// position with velocity and elastic wall bounces. When the gate
/// exceeds `gate_active_threshold`, the ball is confined to a narrow
/// box `[wall_left_active, wall_right_active]` (centered at 0.5);
/// otherwise the box is wide `[wall_left_free, wall_right_free]`.
///
/// Active and free boxes share the same centre (0.5 by default), so
/// the ball's *mean* is the same in both regimes — only its *spread*
/// differs. This is the mechanism-A signature: forward `CE` high,
/// forward `PE` near zero, no latency.
pub struct GatedBouncingBall {
    pub gate: NodeId,
    pub ball: NodeId,
    pub gate_active_threshold: f64,
    pub gate_step_sigma: f64,
    pub initial_velocity: f64,
    pub velocity_noise: f64,
    pub wall_left_active: f64,
    pub wall_right_active: f64,
    pub wall_left_free: f64,
    pub wall_right_free: f64,
}

impl GatedBouncingBall {
    pub fn default_pair(gate: NodeId, ball: NodeId) -> Self {
        GatedBouncingBall {
            gate,
            ball,
            gate_active_threshold: 0.5,
            gate_step_sigma: 0.06,
            initial_velocity: 0.04,
            velocity_noise: 0.015,
            wall_left_active: 0.4,
            wall_right_active: 0.6,
            wall_left_free: 0.0,
            wall_right_free: 1.0,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let mut gate_state: f64 = 0.5;
        let mut ball_x: f64 = 0.5;
        let mut ball_vx: f64 = self.initial_velocity;
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            gate_state =
                (gate_state + rng.normal(0.0, self.gate_step_sigma)).clamp(0.0, 1.0);

            let (wall_l, wall_r) = if gate_state > self.gate_active_threshold {
                (self.wall_left_active, self.wall_right_active)
            } else {
                (self.wall_left_free, self.wall_right_free)
            };

            // If the walls just narrowed, snap the ball back inside.
            ball_x = ball_x.clamp(wall_l, wall_r);

            ball_x += ball_vx;
            if ball_x < wall_l {
                ball_x = wall_l + (wall_l - ball_x);
                ball_vx = -ball_vx;
            }
            if ball_x > wall_r {
                ball_x = wall_r - (ball_x - wall_r);
                ball_vx = -ball_vx;
            }
            ball_x = ball_x.clamp(wall_l, wall_r);

            ball_vx += rng.normal(0.0, self.velocity_noise);
            // Cap velocity so the ball doesn't escape via single-step overshoot.
            ball_vx = ball_vx.clamp(-0.1, 0.1);

            let mut states = BTreeMap::new();
            states.insert(self.gate.clone(), vec![gate_state]);
            states.insert(self.ball.clone(), vec![ball_x]);
            observations.push(Observation { t: t_idx, states });
        }
        Episode {
            id: episode_id.into(),
            nodes: vec![self.gate.clone(), self.ball.clone()],
            observations,
            interventions: vec![],
        }
    }
}

/// Spring-mass follower: physical-realm analogue of mechanism B.
///
/// Node `source` is a random walker. Node `target` has position and
/// velocity, with a damped spring force pulling it toward source's
/// current value: `F = spring_k · (source − target) − damping · v`.
/// The target's position chases the source with a phase lag set by
/// the spring + damping parameters; cross-correlation of source-
/// derivative against target-derivative peaks at a positive lag.
///
/// This is the smoothed / physical version of `MechanismB`'s hard
/// fixed-lag propagation. Both belong to the "latency propagation"
/// class — same mechanism, different realm.
pub struct SpringMassFollower {
    pub source: NodeId,
    pub target: NodeId,
    pub source_step_sigma: f64,
    pub spring_k: f64,
    pub damping: f64,
    pub process_noise: f64,
}

impl SpringMassFollower {
    pub fn default_pair(source: NodeId, target: NodeId) -> Self {
        SpringMassFollower {
            source,
            target,
            source_step_sigma: 0.08,
            spring_k: 0.20,
            damping: 0.50,
            process_noise: 0.005,
        }
    }

    pub fn generate(&self, episode_id: impl Into<String>, steps: u64, seed: u64) -> Episode {
        let mut rng = Rng::new(seed);
        let mut s: f64 = 0.5;
        let mut t: f64 = 0.5;
        let mut v_t: f64 = 0.0;
        let mut observations = Vec::with_capacity(steps as usize);
        for t_idx in 0..steps {
            s = (s + rng.normal(0.0, self.source_step_sigma)).clamp(0.0, 1.0);
            let force = self.spring_k * (s - t) - self.damping * v_t;
            v_t += force;
            v_t = v_t.clamp(-0.3, 0.3);
            t += v_t;
            t = (t + rng.normal(0.0, self.process_noise)).clamp(0.0, 1.0);

            let mut states = BTreeMap::new();
            states.insert(self.source.clone(), vec![s]);
            states.insert(self.target.clone(), vec![t]);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::estimate_all;
    use crate::sim::{MechanismA, MechanismB};
    use crate::similarity::fingerprint_similarity;
    use crate::Fingerprint;

    fn forward<'a>(
        fps: &'a [Fingerprint],
        s: &str,
        t: &str,
    ) -> &'a Fingerprint {
        fps.iter()
            .find(|f| f.source == NodeId::new(s) && f.target == NodeId::new(t))
            .expect("forward fp missing")
    }

    /// Structural recovery on the physical realm: the gated-bouncing-
    /// ball episode produces the mechanism-A signature — forward CE
    /// dominates, forward PE stays small, no latency, no spurious
    /// backward signal.
    #[test]
    fn gated_bouncing_ball_shows_mechanism_a_signature() {
        let g = NodeId::new("G");
        let b = NodeId::new("B");
        let sim = GatedBouncingBall::default_pair(g.clone(), b.clone());
        let ep = sim.generate("phys-A", 1500, 7);
        let fps = estimate_all(&ep);
        let fwd = forward(&fps, "G", "B");
        let bwd = forward(&fps, "B", "G");

        assert!(
            fwd.constraint_effect > 0.3,
            "forward CE too low: {}",
            fwd.constraint_effect
        );
        assert!(
            fwd.constraint_effect > bwd.constraint_effect,
            "asymmetry not recovered: fwd CE {} not > bwd CE {}",
            fwd.constraint_effect,
            bwd.constraint_effect
        );
        assert!(
            fwd.position_effect < 0.2,
            "PE should be small (active/free regimes share mean): {}",
            fwd.position_effect
        );
        assert_eq!(fwd.latency, 0, "no propagation lag expected");
    }

    /// **Cross-realm fingerprint agreement.** v3's fingerprint
    /// recovered from synthetic `MechanismA` and from the physical
    /// `GatedBouncingBall` should land in the same neighbourhood —
    /// `fingerprint_similarity` above a meaningful threshold,
    /// substantially above what we'd see comparing a
    /// genuinely-different mechanism (e.g. `MechanismB` latency
    /// propagation) to either.
    #[test]
    fn cross_realm_mechanism_a_fingerprint_agreement() {
        let s_node = NodeId::new("S");
        let t_node = NodeId::new("T");

        let synthetic_a =
            MechanismA::default_pair(s_node.clone(), t_node.clone()).generate("syn-A", 400, 42);
        let physical_a =
            GatedBouncingBall::default_pair(s_node.clone(), t_node.clone()).generate("phys-A", 1500, 7);
        let synthetic_b =
            MechanismB::default_pair(s_node.clone(), t_node.clone()).generate("syn-B", 500, 11);

        let syn_a_fps = estimate_all(&synthetic_a);
        let phys_a_fps = estimate_all(&physical_a);
        let syn_b_fps = estimate_all(&synthetic_b);
        let syn_a_fp = forward(&syn_a_fps, "S", "T");
        let phys_a_fp = forward(&phys_a_fps, "S", "T");
        let syn_b_fp = forward(&syn_b_fps, "S", "T");

        let cross_realm_a = fingerprint_similarity(syn_a_fp, phys_a_fp);
        let off_mechanism = fingerprint_similarity(syn_a_fp, syn_b_fp);

        assert!(
            cross_realm_a > 0.7,
            "cross-realm mechanism-A similarity too low: {cross_realm_a}\n\
             syn_a = {syn_a_fp:?}\n\
             phys_a = {phys_a_fp:?}"
        );
        assert!(
            cross_realm_a > off_mechanism,
            "cross-realm A vs A ({cross_realm_a}) should exceed A vs B ({off_mechanism})"
        );
    }

    /// Structural recovery on the physical-B realm: the spring-mass
    /// follower has a positive forward latency (target chases source
    /// with phase lag) and backward latency stays at 0 (no signal
    /// for negative lags under k ≥ 0 scan).
    #[test]
    fn spring_mass_follower_shows_mechanism_b_signature() {
        let s = NodeId::new("S");
        let t = NodeId::new("T");
        let sim = SpringMassFollower::default_pair(s.clone(), t.clone());
        let ep = sim.generate("phys-B", 1500, 7);
        let fps = estimate_all(&ep);
        let fwd = forward(&fps, "S", "T");
        let bwd = forward(&fps, "T", "S");
        assert!(fwd.latency > 0, "forward latency should be positive: {}", fwd.latency);
        assert_eq!(bwd.latency, 0, "backward latency should be 0: {}", bwd.latency);
    }

    /// **Cross-realm fingerprint agreement on mechanism B.** v3's
    /// fingerprint recovered from synthetic `MechanismB` and from the
    /// physical `SpringMassFollower` should be more similar to each
    /// other than to an off-mechanism control (synthetic `MechanismA`).
    #[test]
    fn cross_realm_mechanism_b_fingerprint_agreement() {
        let s = NodeId::new("S");
        let t = NodeId::new("T");

        let synthetic_b = MechanismB::default_pair(s.clone(), t.clone()).generate("syn-B", 500, 11);
        let physical_b = SpringMassFollower::default_pair(s.clone(), t.clone())
            .generate("phys-B", 1500, 7);
        let synthetic_a = MechanismA::default_pair(s.clone(), t.clone()).generate("syn-A", 400, 42);

        let syn_b_fps = estimate_all(&synthetic_b);
        let phys_b_fps = estimate_all(&physical_b);
        let syn_a_fps = estimate_all(&synthetic_a);
        let syn_b_fp = forward(&syn_b_fps, "S", "T");
        let phys_b_fp = forward(&phys_b_fps, "S", "T");
        let syn_a_fp = forward(&syn_a_fps, "S", "T");

        let cross_realm_b = fingerprint_similarity(syn_b_fp, phys_b_fp);
        let off_mechanism = fingerprint_similarity(syn_b_fp, syn_a_fp);

        assert!(
            cross_realm_b > off_mechanism,
            "cross-realm B vs B ({cross_realm_b}) should exceed B vs A ({off_mechanism})"
        );
    }
}
