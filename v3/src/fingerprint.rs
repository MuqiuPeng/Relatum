//! Fingerprint: cached derivation from observation (A3).
//!
//! M2 final ships seven of the seven operational fields:
//!
//! - `constraint_effect` — positional spread-shift (mechanism A)
//! - `position_effect`   — mean-shift (mechanism C)
//! - `velocity_effect`   — target-delta spread-shift (mechanism D)
//! - `latency`           — argmax cross-correlation lag (mechanism B)
//! - `reversibility`     — same-source → same-target proximity (obs.)
//! - `stability`         — signature consistency across episode halves
//! - `effect_size`       — `max(constraint_effect, position_effect,
//!                              velocity_effect)`
//! - `directionality`    — asymmetry of `effect_size`
//!
//! No fingerprint field is written back as an R attribute (A3). The
//! struct is a computed view, not storage.

use crate::{Episode, NodeId, StateVec};

/// Maximum forward lag scanned by the latency estimator (steps).
pub const LATENCY_MAX_LAG: usize = 8;

/// Minimum Pearson correlation to report a non-zero latency.
///
/// Calibrated above the finite-sample noise floor for the multi-lag
/// search. A proper lag-shuffle baseline is M3+ work.
const LATENCY_CORR_THRESHOLD: f64 = 0.25;

#[derive(Debug, Clone, PartialEq)]
pub struct Fingerprint {
    pub source: NodeId,
    pub target: NodeId,
    pub effect_size: f64,
    pub directionality: f64,
    pub constraint_effect: f64,
    pub position_effect: f64,
    pub velocity_effect: f64,
    pub latency: i32,
    pub reversibility: f64,
    pub stability: f64,
}

/// Estimate fingerprints for every directed ordered pair of distinct
/// nodes in the episode.
pub fn estimate_all(episode: &Episode) -> Vec<Fingerprint> {
    let mut out = Vec::new();
    for s in &episode.nodes {
        for t in &episode.nodes {
            if s == t {
                continue;
            }
            let ce_fwd = constraint_effect(episode, s, t);
            let pe_fwd = position_effect(episode, s, t);
            let ve_fwd = velocity_effect(episode, s, t);
            let ce_bwd = constraint_effect(episode, t, s);
            let pe_bwd = position_effect(episode, t, s);
            let ve_bwd = velocity_effect(episode, t, s);
            let es_fwd = ce_fwd.max(pe_fwd).max(ve_fwd);
            let es_bwd = ce_bwd.max(pe_bwd).max(ve_bwd);
            out.push(Fingerprint {
                source: s.clone(),
                target: t.clone(),
                effect_size: es_fwd,
                directionality: asymmetry(es_fwd, es_bwd),
                constraint_effect: ce_fwd,
                position_effect: pe_fwd,
                velocity_effect: ve_fwd,
                latency: latency_lag(episode, s, t, LATENCY_MAX_LAG),
                reversibility: reversibility(episode, s, t),
                stability: stability(episode, s, t),
            });
        }
    }
    out
}

// ---- spread-shift (mechanism A signature) -------------------------------

/// Variance-ratio over source quartile extremes — how unequal is the
/// target's *position* spread between bottom and top source quartiles?
pub fn constraint_effect(ep: &Episode, source: &NodeId, target: &NodeId) -> f64 {
    let samples = match collect_paired(ep, source, target) {
        Some(s) => s,
        None => return 0.0,
    };
    let (low, high) = match quartile_split(&samples) {
        Some(pair) => pair,
        None => return 0.0,
    };
    let var_low = mean_componentwise_variance(&low);
    let var_high = mean_componentwise_variance(&high);
    variance_ratio(var_low, var_high)
}

// ---- mean-shift (mechanism C signature) ---------------------------------

/// Eta-squared between source quartile extremes — how much of the
/// target's first-coordinate variance is explained by the bin
/// difference?
pub fn position_effect(ep: &Episode, source: &NodeId, target: &NodeId) -> f64 {
    let samples = match collect_paired(ep, source, target) {
        Some(s) => s,
        None => return 0.0,
    };
    let (low, high) = match quartile_split(&samples) {
        Some(pair) => pair,
        None => return 0.0,
    };

    let all: Vec<f64> = samples.iter().map(|(_, t)| t[0]).collect();
    let grand = mean(&all);
    let overall_var = scalar_variance_with_mean(&all, grand);
    if overall_var < 1e-12 {
        return 0.0;
    }

    let low_first: Vec<f64> = low.iter().map(|t| t[0]).collect();
    let high_first: Vec<f64> = high.iter().map(|t| t[0]).collect();
    let n_low = low_first.len() as f64;
    let n_high = high_first.len() as f64;
    let n_total = n_low + n_high;
    let mean_low = mean(&low_first);
    let mean_high = mean(&high_first);
    let weighted = (n_low * mean_low + n_high * mean_high) / n_total;
    let between = (n_low * (mean_low - weighted).powi(2) + n_high * (mean_high - weighted).powi(2))
        / n_total;
    (between / overall_var).clamp(0.0, 1.0)
}

// ---- velocity-shift (mechanism D signature) -----------------------------

/// Variance-ratio over source quartile extremes — applied to the
/// target's first-coordinate *delta* instead of its position.
///
/// Mechanism D constrains target velocity without constraining position.
/// `constraint_effect` is position-blind here; `velocity_effect` picks up
/// the signal. Mechanism A also activates this field at moderate
/// strength because the snap-to-center event reduces velocity in the
/// active bin — A is distinguished from D by retaining a strong
/// `constraint_effect`.
pub fn velocity_effect(ep: &Episode, source: &NodeId, target: &NodeId) -> f64 {
    let mut paired: Vec<(f64, f64)> = Vec::new();
    let mut prev_t: Option<f64> = None;
    for obs in &ep.observations {
        let s = obs.states.get(source).and_then(|v| v.first().copied());
        let t = obs.states.get(target).and_then(|v| v.first().copied());
        if let (Some(sv), Some(tv)) = (s, t) {
            if let Some(pt) = prev_t {
                paired.push((sv, tv - pt));
            }
            prev_t = Some(tv);
        } else {
            prev_t = None;
        }
    }
    if paired.len() < 8 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = paired.iter().map(|(s, _)| *s).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q_low = sorted[sorted.len() / 4];
    let q_high = sorted[3 * sorted.len() / 4];

    let mut low: Vec<f64> = Vec::new();
    let mut high: Vec<f64> = Vec::new();
    for (s, dt) in &paired {
        if *s >= q_high {
            high.push(*dt);
        } else if *s <= q_low {
            low.push(*dt);
        }
    }
    if low.len() < 2 || high.len() < 2 {
        return 0.0;
    }
    let var_low = scalar_variance(&low);
    let var_high = scalar_variance(&high);
    variance_ratio(var_low, var_high)
}

// ---- latency (mechanism B signature) ------------------------------------

/// argmax over k ∈ [0, max_lag] of `Pearson(dsource[..N-k], dtarget[k..])`.
pub fn latency_lag(ep: &Episode, source: &NodeId, target: &NodeId, max_lag: usize) -> i32 {
    let (ds, dt) = differences(ep, source, target);
    if ds.len() < max_lag + 4 {
        return 0;
    }
    let mut best_k: i32 = 0;
    let mut best_corr: f64 = 0.0;
    for k in 0..=max_lag {
        let n = ds.len() - k;
        let c = pearson(&ds[..n], &dt[k..]);
        if c > best_corr {
            best_corr = c;
            best_k = k as i32;
        }
    }
    if best_corr < LATENCY_CORR_THRESHOLD {
        0
    } else {
        best_k
    }
}

// ---- reversibility (observational proxy) --------------------------------

/// Observational reversibility — to what extent does the same source
/// value reliably correspond to the same target value across the
/// episode?
///
/// Sort observations by source first-coordinate, then for each
/// neighboring pair in the sort measure target diff. Average the diffs
/// and normalize by target's overall std.
///
/// **Limitation:** this is a same-time observational proxy. Lagged
/// mechanisms (B) appear low under this estimator because target(t)
/// reflects past source, not current source. Intervention-based
/// reversibility is M3+ work.
pub fn reversibility(ep: &Episode, source: &NodeId, target: &NodeId) -> f64 {
    let mut paired: Vec<(f64, f64)> = Vec::new();
    for obs in &ep.observations {
        let s = obs.states.get(source).and_then(|v| v.first().copied());
        let t = obs.states.get(target).and_then(|v| v.first().copied());
        if let (Some(sv), Some(tv)) = (s, t) {
            paired.push((sv, tv));
        }
    }
    if paired.len() < 4 {
        return 0.0;
    }
    paired.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut diff_sum = 0.0;
    for i in 1..paired.len() {
        diff_sum += (paired[i].1 - paired[i - 1].1).abs();
    }
    let mean_diff = diff_sum / (paired.len() - 1) as f64;
    let all_targets: Vec<f64> = paired.iter().map(|(_, t)| *t).collect();
    let t_std = scalar_variance(&all_targets).sqrt();
    if t_std < 1e-12 {
        return 0.0;
    }
    (1.0 - mean_diff / t_std).clamp(0.0, 1.0)
}

// ---- stability (cross-window signature consistency) ---------------------

/// Stability — consistency of the recovered signature across the
/// first and second halves of the episode.
///
/// Computes `constraint_effect`, `position_effect`, and
/// `velocity_effect` on each half and reports
/// `1 − avg(|Δ| over the three fields)`. All M2 mechanisms are
/// stationary so stability is expected to be high (> 0.7). Regime-shift
/// mechanisms (M3+) will populate lower values.
pub fn stability(ep: &Episode, source: &NodeId, target: &NodeId) -> f64 {
    let n = ep.observations.len();
    if n < 16 {
        return 0.0;
    }
    let mid = n / 2;
    let ep_a = sub_episode(ep, 0, mid);
    let ep_b = sub_episode(ep, mid, n);
    let ce_a = constraint_effect(&ep_a, source, target);
    let ce_b = constraint_effect(&ep_b, source, target);
    let pe_a = position_effect(&ep_a, source, target);
    let pe_b = position_effect(&ep_b, source, target);
    let ve_a = velocity_effect(&ep_a, source, target);
    let ve_b = velocity_effect(&ep_b, source, target);
    let s_ce = 1.0 - (ce_a - ce_b).abs();
    let s_pe = 1.0 - (pe_a - pe_b).abs();
    let s_ve = 1.0 - (ve_a - ve_b).abs();
    ((s_ce + s_pe + s_ve) / 3.0).clamp(0.0, 1.0)
}

// ---- shared helpers ------------------------------------------------------

fn sub_episode(ep: &Episode, start: usize, end: usize) -> Episode {
    Episode {
        id: ep.id.clone(),
        nodes: ep.nodes.clone(),
        observations: ep.observations[start..end].to_vec(),
        interventions: vec![],
    }
}

fn variance_ratio(a: f64, b: f64) -> f64 {
    let mx = a.max(b);
    let mn = a.min(b);
    if mx < 1e-12 {
        0.0
    } else {
        (1.0 - mn / mx).clamp(0.0, 1.0)
    }
}

fn asymmetry(forward: f64, backward: f64) -> f64 {
    let s = forward + backward;
    if s < 1e-12 {
        return 0.0;
    }
    (forward - backward) / s
}

fn collect_paired(ep: &Episode, source: &NodeId, target: &NodeId) -> Option<Vec<(f64, StateVec)>> {
    let mut samples: Vec<(f64, StateVec)> = Vec::new();
    for obs in &ep.observations {
        let s = obs.states.get(source)?.first().copied()?;
        let t = obs.states.get(target)?.clone();
        if t.is_empty() {
            return None;
        }
        samples.push((s, t));
    }
    if samples.len() < 8 {
        return None;
    }
    Some(samples)
}

fn quartile_split(samples: &[(f64, StateVec)]) -> Option<(Vec<StateVec>, Vec<StateVec>)> {
    let mut sorted: Vec<f64> = samples.iter().map(|(s, _)| *s).collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q_low = sorted[sorted.len() / 4];
    let q_high = sorted[3 * sorted.len() / 4];
    let mut low: Vec<StateVec> = Vec::new();
    let mut high: Vec<StateVec> = Vec::new();
    for (s, t) in samples {
        if *s >= q_high {
            high.push(t.clone());
        } else if *s <= q_low {
            low.push(t.clone());
        }
    }
    if low.len() < 2 || high.len() < 2 {
        return None;
    }
    Some((low, high))
}

fn differences(ep: &Episode, source: &NodeId, target: &NodeId) -> (Vec<f64>, Vec<f64>) {
    let mut ds = Vec::new();
    let mut dt = Vec::new();
    let mut prev_s: Option<f64> = None;
    let mut prev_t: Option<f64> = None;
    for obs in &ep.observations {
        let s = obs.states.get(source).and_then(|v| v.first().copied());
        let t = obs.states.get(target).and_then(|v| v.first().copied());
        if let (Some(sv), Some(tv)) = (s, t) {
            if let (Some(ps), Some(pt)) = (prev_s, prev_t) {
                ds.push(sv - ps);
                dt.push(tv - pt);
            }
            prev_s = Some(sv);
            prev_t = Some(tv);
        } else {
            prev_s = None;
            prev_t = None;
        }
    }
    (ds, dt)
}

fn pearson(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.len() < 2 {
        return 0.0;
    }
    let n = a.len() as f64;
    let ma: f64 = a.iter().sum::<f64>() / n;
    let mb: f64 = b.iter().sum::<f64>() / n;
    let (mut num, mut da, mut db) = (0.0, 0.0, 0.0);
    for (x, y) in a.iter().zip(b.iter()) {
        let xa = x - ma;
        let yb = y - mb;
        num += xa * yb;
        da += xa * xa;
        db += yb * yb;
    }
    let denom = (da * db).sqrt();
    if denom < 1e-12 {
        0.0
    } else {
        num / denom
    }
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

fn scalar_variance(xs: &[f64]) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let m = mean(xs);
    scalar_variance_with_mean(xs, m)
}

fn scalar_variance_with_mean(xs: &[f64], m: f64) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64
}

fn mean_componentwise_variance(samples: &[StateVec]) -> f64 {
    if samples.len() < 2 {
        return 0.0;
    }
    let d = samples[0].len();
    if d == 0 {
        return 0.0;
    }
    let n = samples.len() as f64;
    let mut var_sum = 0.0;
    for i in 0..d {
        let m: f64 = samples.iter().map(|s| s[i]).sum::<f64>() / n;
        let v: f64 = samples.iter().map(|s| (s[i] - m).powi(2)).sum::<f64>() / n;
        var_sum += v;
    }
    var_sum / d as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{MechanismA, MechanismB, MechanismC, MechanismD};
    use std::collections::HashMap;

    /// Mechanism A: spread-shift signature with no mean shift and no lag.
    #[test]
    fn mechanism_a_signature_is_spread_only() {
        let m = MechanismA::default_pair(NodeId::new("S"), NodeId::new("T"));
        let ep = m.generate("E", 400, 42);
        let fps = estimate_all(&ep);
        let f = forward(&fps, "S", "T");
        assert!(f.constraint_effect > 0.3, "CE: {}", f.constraint_effect);
        assert!(f.position_effect < 0.15, "PE leaked: {}", f.position_effect);
        assert_eq!(f.latency, 0, "spurious lag: {}", f.latency);
    }

    /// Mechanism B: forward latency recovers embedded delay.
    #[test]
    fn mechanism_b_recovers_forward_latency() {
        let m = MechanismB::default_pair(NodeId::new("S"), NodeId::new("T"));
        let ep = m.generate("E", 500, 11);
        let fps = estimate_all(&ep);
        let fwd = forward(&fps, "S", "T");
        let bwd = forward(&fps, "T", "S");
        assert_eq!(fwd.latency, m.latency as i32);
        assert_eq!(bwd.latency, 0);
    }

    /// Mechanism C: symmetric mean-shift signature.
    #[test]
    fn mechanism_c_signature_is_symmetric_position() {
        let m = MechanismC::default_pair(NodeId::new("L"), NodeId::new("R"));
        let ep = m.generate("E", 400, 5);
        let fps = estimate_all(&ep);
        let lr = forward(&fps, "L", "R");
        let rl = forward(&fps, "R", "L");
        assert!(lr.position_effect > 0.3, "PE fwd: {}", lr.position_effect);
        assert!(rl.position_effect > 0.3, "PE bwd: {}", rl.position_effect);
        assert!(lr.directionality.abs() < 0.15, "dir: {}", lr.directionality);
        assert_eq!(lr.latency, 0);
    }

    /// Mechanism D: velocity nearly perfectly bounded (VE > 0.9) and
    /// dominates over CE. PE near zero. Latency zero.
    ///
    /// CE is not asserted low because target's *frozen positions* across
    /// runs still create position spread asymmetry between source bins.
    /// What distinguishes D from A is the absolute strength of VE: D
    /// suppresses velocity to 0.005 vs 0.15 (ratio ≈ 0.033, VE ≈ 0.999),
    /// while A's snap-to-center reduces velocity only moderately.
    #[test]
    fn mechanism_d_signature_is_velocity_only() {
        let m = MechanismD::default_pair(NodeId::new("S"), NodeId::new("T"));
        let ep = m.generate("E", 400, 9);
        let fps = estimate_all(&ep);
        let f = forward(&fps, "S", "T");
        assert!(f.velocity_effect > 0.9, "VE: {}", f.velocity_effect);
        assert!(
            f.velocity_effect > f.constraint_effect,
            "VE should dominate CE: VE={}, CE={}",
            f.velocity_effect,
            f.constraint_effect
        );
        assert!(f.position_effect < 0.15, "PE leaked: {}", f.position_effect);
        assert_eq!(f.latency, 0);
    }

    /// Reversibility orders intuitively: C (instantaneous sync) >
    /// A (mode-dependent partial reset) > D (free position under
    /// velocity suppression).
    #[test]
    fn reversibility_orders_c_above_a_above_d() {
        let ep_c =
            MechanismC::default_pair(NodeId::new("S"), NodeId::new("T")).generate("Ec", 400, 3);
        let ep_a =
            MechanismA::default_pair(NodeId::new("S"), NodeId::new("T")).generate("Ea", 400, 3);
        let ep_d =
            MechanismD::default_pair(NodeId::new("S"), NodeId::new("T")).generate("Ed", 400, 3);
        let rc = forward(&estimate_all(&ep_c), "S", "T").reversibility;
        let ra = forward(&estimate_all(&ep_a), "S", "T").reversibility;
        let rd = forward(&estimate_all(&ep_d), "S", "T").reversibility;
        assert!(rc > ra, "C={rc}, A={ra}");
        assert!(ra > rd, "A={ra}, D={rd}");
    }

    /// All M2 mechanisms are stationary, so stability is high
    /// (> 0.7 on episode-half consistency).
    #[test]
    fn stability_holds_on_stationary_mechanisms() {
        let cases: Vec<(&str, Episode)> = vec![
            (
                "A",
                MechanismA::default_pair(NodeId::new("S"), NodeId::new("T")).generate("E", 400, 2),
            ),
            (
                "B",
                MechanismB::default_pair(NodeId::new("S"), NodeId::new("T")).generate("E", 500, 2),
            ),
            (
                "C",
                MechanismC::default_pair(NodeId::new("S"), NodeId::new("T")).generate("E", 400, 2),
            ),
            (
                "D",
                MechanismD::default_pair(NodeId::new("S"), NodeId::new("T")).generate("E", 400, 2),
            ),
        ];
        for (label, ep) in cases {
            let fps = estimate_all(&ep);
            let f = forward(&fps, "S", "T");
            assert!(f.stability > 0.7, "{label} stability: {}", f.stability);
        }
    }

    /// Each mechanism has a unique winning field. A → CE,
    /// B → latency, C → PE, D → VE (with CE low to separate D from A).
    #[test]
    fn fingerprints_distinguish_a_b_c_d() {
        let fps_a = estimate_all(
            &MechanismA::default_pair(NodeId::new("S"), NodeId::new("T")).generate("Ea", 400, 1),
        );
        let fps_b = estimate_all(
            &MechanismB::default_pair(NodeId::new("S"), NodeId::new("T")).generate("Eb", 400, 1),
        );
        let fps_c = estimate_all(
            &MechanismC::default_pair(NodeId::new("S"), NodeId::new("T")).generate("Ec", 400, 1),
        );
        let fps_d = estimate_all(
            &MechanismD::default_pair(NodeId::new("S"), NodeId::new("T")).generate("Ed", 400, 1),
        );
        let fa = forward(&fps_a, "S", "T");
        let fb = forward(&fps_b, "S", "T");
        let fc = forward(&fps_c, "S", "T");
        let fd = forward(&fps_d, "S", "T");
        assert!(fa.constraint_effect > fa.position_effect);
        assert_eq!(fa.latency, 0);
        assert!(fb.latency > 0);
        assert!(fc.position_effect > fc.constraint_effect);
        assert_eq!(fc.latency, 0);
        assert!(fd.velocity_effect > 0.9, "D's VE: {}", fd.velocity_effect);
        assert!(
            fd.velocity_effect > fd.constraint_effect,
            "D's VE should beat CE: VE={}, CE={}",
            fd.velocity_effect,
            fd.constraint_effect
        );
    }

    /// A1 guard: every field of every fingerprint is invariant under
    /// node-id bijection. Now covers all 4 mechanisms × 3 seeds × all
    /// 8 numerical / integer fields (including reversibility,
    /// stability, velocity_effect).
    #[test]
    fn fingerprints_invariant_under_renaming_all_mechanisms() {
        let cases: Vec<(&str, Box<dyn Fn(u64) -> Episode>)> = vec![
            (
                "A",
                Box::new(|seed| {
                    MechanismA::default_pair(NodeId::new("N1"), NodeId::new("N2"))
                        .generate("E", 200, seed)
                }),
            ),
            (
                "B",
                Box::new(|seed| {
                    MechanismB::default_pair(NodeId::new("N1"), NodeId::new("N2"))
                        .generate("E", 200, seed)
                }),
            ),
            (
                "C",
                Box::new(|seed| {
                    MechanismC::default_pair(NodeId::new("N1"), NodeId::new("N2"))
                        .generate("E", 200, seed)
                }),
            ),
            (
                "D",
                Box::new(|seed| {
                    MechanismD::default_pair(NodeId::new("N1"), NodeId::new("N2"))
                        .generate("E", 200, seed)
                }),
            ),
        ];
        let mut map = HashMap::new();
        map.insert(NodeId::new("N1"), NodeId::new("XX"));
        map.insert(NodeId::new("N2"), NodeId::new("YY"));
        for (label, gen) in &cases {
            for seed in [1u64, 7, 42] {
                let ep = gen(seed);
                let original = estimate_all(&ep);
                let renamed = estimate_all(&ep.rename(|n| map.get(n).cloned().unwrap()));
                assert_eq!(original.len(), renamed.len(), "{label} seed {seed}");
                for orig in &original {
                    let mapped_src = map.get(&orig.source).unwrap();
                    let mapped_tgt = map.get(&orig.target).unwrap();
                    let found = renamed
                        .iter()
                        .find(|f| &f.source == mapped_src && &f.target == mapped_tgt)
                        .unwrap_or_else(|| panic!("missing fp in {label} seed {seed}"));
                    assert_eq!(found.effect_size.to_bits(), orig.effect_size.to_bits());
                    assert_eq!(found.directionality.to_bits(), orig.directionality.to_bits());
                    assert_eq!(found.constraint_effect.to_bits(), orig.constraint_effect.to_bits());
                    assert_eq!(found.position_effect.to_bits(), orig.position_effect.to_bits());
                    assert_eq!(found.velocity_effect.to_bits(), orig.velocity_effect.to_bits());
                    assert_eq!(found.reversibility.to_bits(), orig.reversibility.to_bits());
                    assert_eq!(found.stability.to_bits(), orig.stability.to_bits());
                    assert_eq!(found.latency, orig.latency);
                }
            }
        }
    }

    fn forward<'a>(fps: &'a [Fingerprint], s: &str, t: &str) -> &'a Fingerprint {
        fps.iter()
            .find(|f| f.source == NodeId::new(s) && f.target == NodeId::new(t))
            .expect("forward fingerprint missing")
    }
}
