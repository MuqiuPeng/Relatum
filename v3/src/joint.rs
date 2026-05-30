//! Joint-structure recovery — M5 first slice.
//!
//! Detect n-ary irreducibility *from observation alone*, without
//! reading the simulator's `NaryMechanism::is_irreducible` flag. The
//! M4 substrate could only know irreducibility through author
//! metadata; M5 begins to let the recovery side notice the structure
//! by itself.
//!
//! Two complementary detectors land in this slice; each catches a
//! distinct class of irreducibility:
//!
//! - `joint_position_effect` — bin the two source candidates jointly
//!   (`A_bin × B_bin`, four cells) and measure the target's mean
//!   shift across cells. Mechanism F (XOR-style) reads high here
//!   because joint cells have widely different means even though
//!   single-source marginals are null.
//! - `conditional_effect_variance` — split the episode by a
//!   conditioner's quartile and re-compute `position_effect(source →
//!   target)` on each half. The absolute difference catches mechanism
//!   E (gating), where `B → C` effect strength depends on `A`'s
//!   regime.
//!
//! `irreducibility_signal` returns `max(joint_excess, conditional_var)`
//! over both directions: either detector firing is sufficient
//! observational evidence that a binary projection cannot explain the
//! observed structure.

use crate::fingerprint::position_effect;
use crate::{Episode, NodeId, Observation};

/// 4-cell joint eta-squared. Bins both sources at their quartile
/// extremes, drops middle-50% samples, and reports the between-cell
/// variance of the target's first coordinate over its total variance.
///
/// This captures *total* joint structure — including any additive
/// single-source effects that happen to align. For binary-reducible
/// patterns where A predicts B (e.g., chains), this reads high
/// because joint cells provide tighter conditioning than single
/// bins; the lift over single bins is not irreducibility, it is
/// double-counting through the A→B mediation.
///
/// For a clean "irreducibility" detector use
/// `joint_interaction_effect`, which subtracts the additive
/// single-source contributions and reports only the interaction.
pub fn joint_position_effect(
    ep: &Episode,
    src_a: &NodeId,
    src_b: &NodeId,
    target: &NodeId,
) -> f64 {
    let cells = match build_cells(ep, src_a, src_b, target) {
        Some(c) => c,
        None => return 0.0,
    };
    let all: Vec<f64> = cells.iter().flat_map(|c| c.iter().copied()).collect();
    let grand = mean(&all);
    let total_var = variance(&all, grand);
    if total_var < 1e-12 {
        return 0.0;
    }
    let n_total = all.len() as f64;
    let weighted: f64 = cells
        .iter()
        .map(|c| c.len() as f64 * mean(c))
        .sum::<f64>()
        / n_total;
    let between: f64 = cells
        .iter()
        .map(|c| {
            let n = c.len() as f64;
            n * (mean(c) - weighted).powi(2)
        })
        .sum::<f64>()
        / n_total;
    (between / total_var).clamp(0.0, 1.0)
}

/// 2-way ANOVA interaction term, normalised by total variance.
///
/// Decomposes joint cell-variance as
/// `SS_cells = SS_A + SS_B + SS_interaction` (in the balanced case
/// quartile binning approximates) and reports `SS_interaction /
/// SS_total`. This is the part of the joint signal that is **not**
/// expressible as the sum of single-source effects — i.e., the part
/// that demands an n-ary mechanism.
///
/// Mechanism F (XOR) saturates here because both single-source
/// marginals are null but cells differ wildly. Mechanism E (gating)
/// reads positive because A's regime gates B's contribution.
/// ChainBB and other purely-binary reducible patterns read near zero
/// because their joint structure is additive in single-source effects.
pub fn joint_interaction_effect(
    ep: &Episode,
    src_a: &NodeId,
    src_b: &NodeId,
    target: &NodeId,
) -> f64 {
    let cells = match build_cells(ep, src_a, src_b, target) {
        Some(c) => c,
        None => return 0.0,
    };
    // cells indexed as a_bin * 2 + b_bin (a_bin and b_bin in {0,1}).
    let all: Vec<f64> = cells.iter().flat_map(|c| c.iter().copied()).collect();
    let grand = mean(&all);
    let ss_total: f64 = all.iter().map(|x| (x - grand).powi(2)).sum();
    if ss_total < 1e-12 {
        return 0.0;
    }

    let row_samples: [Vec<f64>; 2] = [
        cells[0].iter().chain(cells[1].iter()).copied().collect(),
        cells[2].iter().chain(cells[3].iter()).copied().collect(),
    ];
    let col_samples: [Vec<f64>; 2] = [
        cells[0].iter().chain(cells[2].iter()).copied().collect(),
        cells[1].iter().chain(cells[3].iter()).copied().collect(),
    ];
    let a_means = [mean(&row_samples[0]), mean(&row_samples[1])];
    let b_means = [mean(&col_samples[0]), mean(&col_samples[1])];

    // Weighted residual sum from the additive fit
    // `predicted[ai][bi] = grand + (a_means[ai] − grand) + (b_means[bi] − grand)`.
    // The unbalanced-design-safe interaction term.
    let mut ss_interaction = 0.0;
    for ai in 0..2 {
        for bi in 0..2 {
            let cell = &cells[ai * 2 + bi];
            let n = cell.len() as f64;
            let actual = mean(cell);
            let predicted = a_means[ai] + b_means[bi] - grand;
            let residual = actual - predicted;
            ss_interaction += n * residual * residual;
        }
    }

    (ss_interaction / ss_total).clamp(0.0, 1.0)
}

fn build_cells(
    ep: &Episode,
    src_a: &NodeId,
    src_b: &NodeId,
    target: &NodeId,
) -> Option<[Vec<f64>; 4]> {
    let mut samples: Vec<(f64, f64, f64)> = Vec::new();
    for obs in &ep.observations {
        let a = obs.states.get(src_a).and_then(|v| v.first().copied());
        let b = obs.states.get(src_b).and_then(|v| v.first().copied());
        let t = obs.states.get(target).and_then(|v| v.first().copied());
        if let (Some(a), Some(b), Some(t)) = (a, b, t) {
            samples.push((a, b, t));
        }
    }
    if samples.len() < 16 {
        return None;
    }
    let (a_q_low, a_q_high) = quartile_thresholds(samples.iter().map(|(a, _, _)| *a));
    let (b_q_low, b_q_high) = quartile_thresholds(samples.iter().map(|(_, b, _)| *b));
    let mut cells: [Vec<f64>; 4] = Default::default();
    for (a, b, t) in &samples {
        let a_bin = bin(*a, a_q_low, a_q_high);
        let b_bin = bin(*b, b_q_low, b_q_high);
        if let (Some(ai), Some(bi)) = (a_bin, b_bin) {
            cells[ai * 2 + bi].push(*t);
        }
    }
    if cells.iter().any(|c| c.len() < 2) {
        return None;
    }
    Some(cells)
}

/// `|PE(source→target | conditioner high quartile) − PE(source→target
/// | conditioner low quartile)|`.
///
/// Captures interaction effects: how much does the source's
/// predictive power over the target depend on the conditioner's
/// regime? Mechanism E (gating) reads high here; mechanism F (XOR)
/// stays near zero because its symmetric structure preserves
/// magnitude across conditioner regimes.
pub fn conditional_effect_variance(
    ep: &Episode,
    conditioner: &NodeId,
    source: &NodeId,
    target: &NodeId,
) -> f64 {
    let mut cond_vals: Vec<f64> = ep
        .observations
        .iter()
        .filter_map(|o| o.states.get(conditioner).and_then(|v| v.first().copied()))
        .collect();
    if cond_vals.len() < 16 {
        return 0.0;
    }
    cond_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q_low = cond_vals[cond_vals.len() / 4];
    let q_high = cond_vals[3 * cond_vals.len() / 4];

    let (high_obs, low_obs) = split_by_conditioner(ep, conditioner, q_low, q_high);
    if high_obs.len() < 8 || low_obs.len() < 8 {
        return 0.0;
    }

    let high_ep = sub_episode(ep, high_obs);
    let low_ep = sub_episode(ep, low_obs);
    let pe_high = position_effect(&high_ep, source, target);
    let pe_low = position_effect(&low_ep, source, target);
    (pe_high - pe_low).abs()
}

/// Observational evidence for n-ary irreducibility involving nodes
/// `a`, `b`, `c`. Returns `max(joint_excess, conditional_variance)`,
/// where `joint_excess = max(0, joint_PE − max(PE(a→c), PE(b→c)))`
/// and conditional variance is taken over both `a-conditions-(b,c)`
/// and `b-conditions-(a,c)` orderings.
///
/// Either detector triggering is sufficient observational evidence
/// that a binary projection cannot explain the observed structure.
/// Mechanism F fires through `joint_excess`; mechanism E fires
/// through `conditional_variance`.
/// Minimum interaction threshold below which conditional-variance
/// contributions are ignored. The interaction term is the gatekeeper:
/// if the joint cells are additive in single-source effects (chain
/// composition mediates through B), no amount of conditional-PE
/// variance should be called irreducibility.
const INTERACTION_GATE: f64 = 0.05;

pub fn irreducibility_signal(
    ep: &Episode,
    a: &NodeId,
    b: &NodeId,
    c: &NodeId,
) -> f64 {
    let interaction = joint_interaction_effect(ep, a, b, c);
    if interaction < INTERACTION_GATE {
        // No joint structure beyond additive — irreducibility unsupported.
        // Any conditional-PE variance is mediation noise, not gating.
        return 0.0;
    }
    let cond_a = conditional_effect_variance(ep, a, b, c);
    let cond_b = conditional_effect_variance(ep, b, a, c);
    interaction.max(cond_a).max(cond_b)
}

/// The class of irreducibility detected for nodes `(a, b)` jointly
/// driving `c`. Reported by `classify_irreducibility`.
///
/// Beyond the binary reducible/irreducible split of
/// `irreducibility_signal`, this distinguishes the two M4 n-ary
/// families. The discriminator is the *shape* of the
/// `(interaction, cond_a, cond_b)` triple:
///
/// - XOR-like: interaction dominates, both conditional variances
///   stay low (symmetric joint, no regime asymmetry).
/// - Gating: interaction modest, one conditional variance dominates
///   (the conditioner whose regime gates the other's effect).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrreducibilityClass {
    /// Joint structure is additive in single-source effects — chains,
    /// fan-out, fan-in, loops, and independent walks all land here.
    Reducible,
    /// Joint cells deviate from the additive fit symmetrically; both
    /// conditional-PE variances are low. Matches `MechanismF`.
    XorLike,
    /// Joint cells deviate from the additive fit asymmetrically; one
    /// conditioner's regime controls whether the other's effect on
    /// the target manifests. The `gate` field names that conditioner.
    /// Matches `MechanismE`.
    Gating { gate: NodeId },
}

/// Classify the joint structure spanning `(a, b)` → `c` from
/// observation alone. Substrate-side counterpart to the M4
/// `NaryMechanism::is_irreducible` author flag.
///
/// Robust to argument order: passing `(a, b, c)` vs `(b, a, c)`
/// agrees on the class and identifies the same gate node in
/// `Gating` cases.
pub fn classify_irreducibility(
    ep: &Episode,
    a: &NodeId,
    b: &NodeId,
    c: &NodeId,
) -> IrreducibilityClass {
    let interaction = joint_interaction_effect(ep, a, b, c);
    if interaction < INTERACTION_GATE {
        return IrreducibilityClass::Reducible;
    }
    let cond_a = conditional_effect_variance(ep, a, b, c);
    let cond_b = conditional_effect_variance(ep, b, a, c);
    let cond_max = cond_a.max(cond_b);

    if interaction >= cond_max {
        IrreducibilityClass::XorLike
    } else {
        let gate = if cond_a > cond_b { a.clone() } else { b.clone() };
        IrreducibilityClass::Gating { gate }
    }
}

// ---- helpers ------------------------------------------------------------

fn quartile_thresholds(values: impl Iterator<Item = f64>) -> (f64, f64) {
    let mut sorted: Vec<f64> = values.collect();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q_low = sorted[sorted.len() / 4];
    let q_high = sorted[3 * sorted.len() / 4];
    (q_low, q_high)
}

fn bin(v: f64, q_low: f64, q_high: f64) -> Option<usize> {
    if v >= q_high {
        Some(1)
    } else if v <= q_low {
        Some(0)
    } else {
        None
    }
}

fn split_by_conditioner(
    ep: &Episode,
    conditioner: &NodeId,
    q_low: f64,
    q_high: f64,
) -> (Vec<Observation>, Vec<Observation>) {
    let mut high = Vec::new();
    let mut low = Vec::new();
    for obs in &ep.observations {
        if let Some(v) = obs.states.get(conditioner).and_then(|v| v.first().copied()) {
            if v >= q_high {
                high.push(obs.clone());
            } else if v <= q_low {
                low.push(obs.clone());
            }
        }
    }
    (high, low)
}

fn sub_episode(parent: &Episode, observations: Vec<Observation>) -> Episode {
    Episode {
        id: parent.id.clone(),
        nodes: parent.nodes.clone(),
        observations,
        interventions: vec![],
    }
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        0.0
    } else {
        xs.iter().sum::<f64>() / xs.len() as f64
    }
}

fn variance(xs: &[f64], m: f64) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / xs.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nary::{MechanismE, MechanismF};
    use crate::sim::{ChainBB, Independent3};

    /// Mechanism F (XOR): joint_PE saturates near 1, single PEs near 0.
    /// The substrate sees joint structure without the metadata flag.
    #[test]
    fn mechanism_f_joint_position_effect_saturates() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let m = MechanismF::default_triple(n1.clone(), n2.clone(), n3.clone());
        let ep = m.generate("E", 2400, 5);
        let joint = joint_position_effect(&ep, &n1, &n2, &n3);
        let pe_a = position_effect(&ep, &n1, &n3);
        let pe_b = position_effect(&ep, &n2, &n3);
        assert!(joint > 0.7, "F joint_PE too low: {joint}");
        assert!(pe_a < 0.2, "F pe_a leaked: {pe_a}");
        assert!(pe_b < 0.2, "F pe_b leaked: {pe_b}");
    }

    /// Mechanism E (gating): conditional_effect_variance reads high
    /// when A is the conditioner — `B → C` strength differs between
    /// gate-open and gate-closed regimes.
    #[test]
    fn mechanism_e_conditional_variance_catches_gating() {
        let m = MechanismE::default_triple(
            NodeId::new("A"),
            NodeId::new("B"),
            NodeId::new("C"),
        );
        let ep = m.generate("E", 1500, 7);
        let cond = conditional_effect_variance(&ep, &m.gate, &m.source, &m.target);
        assert!(cond > 0.1, "E conditional variance too low: {cond}");
    }

    /// Class recovery: F → XorLike, E → Gating{gate=A},
    /// ChainBB → Reducible, Independent3 → Reducible.
    #[test]
    fn class_recovery_separates_xor_from_gating_and_reducible() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");

        let f_ep =
            MechanismF::default_triple(n1.clone(), n2.clone(), n3.clone()).generate("F", 2400, 5);
        let e_ep = MechanismE::default_triple(n1.clone(), n2.clone(), n3.clone())
            .generate("E", 1500, 7);
        let ch_ep =
            ChainBB::default_triple(n1.clone(), n2.clone(), n3.clone()).generate("Ch", 1500, 7);
        let in_ep =
            Independent3::default_triple(n1.clone(), n2.clone(), n3.clone()).generate("I", 1500, 5);

        assert_eq!(
            classify_irreducibility(&f_ep, &n1, &n2, &n3),
            IrreducibilityClass::XorLike
        );
        assert_eq!(
            classify_irreducibility(&e_ep, &n1, &n2, &n3),
            IrreducibilityClass::Gating { gate: n1.clone() }
        );
        assert_eq!(
            classify_irreducibility(&ch_ep, &n1, &n2, &n3),
            IrreducibilityClass::Reducible
        );
        assert_eq!(
            classify_irreducibility(&in_ep, &n1, &n2, &n3),
            IrreducibilityClass::Reducible
        );
    }

    /// Gate identification survives argument permutation. The gate
    /// node is the same regardless of whether it is passed as `a` or
    /// `b` to the classifier.
    #[test]
    fn gating_class_identifies_gate_under_argument_swap() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        // gate = N2 (not N1 — explicit swap to test classifier
        // doesn't have a positional bias).
        let m = MechanismE {
            gate: n2.clone(),
            source: n1.clone(),
            target: n3.clone(),
            gate_threshold: 0.5,
            propagation_lag: 2,
            propagation_noise: 0.03,
            free_step_sigma: 0.12,
            source_step_sigma: 0.08,
        };
        let ep = m.generate("E", 1500, 7);

        assert_eq!(
            classify_irreducibility(&ep, &n1, &n2, &n3),
            IrreducibilityClass::Gating { gate: n2.clone() }
        );
        assert_eq!(
            classify_irreducibility(&ep, &n2, &n1, &n3),
            IrreducibilityClass::Gating { gate: n2.clone() }
        );
    }

    /// All 3-node binary patterns from the M3 library classify as
    /// Reducible — Chain, FanOut, FanIn, Loop, and Independent.
    #[test]
    fn all_binary_patterns_classify_as_reducible() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");
        let chains: Vec<(&str, Episode)> = vec![
            (
                "Chain",
                ChainBB::default_triple(n1.clone(), n2.clone(), n3.clone())
                    .generate("E", 1500, 11),
            ),
            (
                "FanOut",
                crate::sim::FanOut3::default_triple(n1.clone(), n2.clone(), n3.clone())
                    .generate("E", 1500, 11),
            ),
            (
                "FanIn",
                crate::sim::FanIn3::default_triple(n1.clone(), n2.clone(), n3.clone())
                    .generate("E", 1500, 11),
            ),
            (
                "Loop",
                crate::sim::Loop3::default_triple(n1.clone(), n2.clone(), n3.clone())
                    .generate("E", 1500, 11),
            ),
            (
                "Indep",
                Independent3::default_triple(n1.clone(), n2.clone(), n3.clone())
                    .generate("E", 1500, 11),
            ),
        ];
        for (label, ep) in chains {
            let class = classify_irreducibility(&ep, &n1, &n2, &n3);
            assert_eq!(
                class,
                IrreducibilityClass::Reducible,
                "{label} should be reducible, got {:?}",
                class
            );
        }
    }

    /// F's irreducibility signal saturates (joint pathway).
    /// E's signal is positive (conditional pathway).
    /// ChainBB (binary composition) stays low — pairwise reduction works.
    /// Independent3 stays near zero — no joint structure at all.
    #[test]
    fn irreducibility_signal_separates_n_ary_from_binary() {
        let n1 = NodeId::new("N1");
        let n2 = NodeId::new("N2");
        let n3 = NodeId::new("N3");

        let f_ep =
            MechanismF::default_triple(n1.clone(), n2.clone(), n3.clone()).generate("F", 2400, 5);
        let e_ep = {
            let m = MechanismE::default_triple(n1.clone(), n2.clone(), n3.clone());
            m.generate("E", 1500, 7)
        };
        let chain_ep =
            ChainBB::default_triple(n1.clone(), n2.clone(), n3.clone()).generate("Ch", 1500, 7);
        let indep_ep =
            Independent3::default_triple(n1.clone(), n2.clone(), n3.clone()).generate("I", 1500, 5);

        let f_sig = irreducibility_signal(&f_ep, &n1, &n2, &n3);
        let e_sig = irreducibility_signal(&e_ep, &n1, &n2, &n3);
        let ch_sig = irreducibility_signal(&chain_ep, &n1, &n2, &n3);
        let in_sig = irreducibility_signal(&indep_ep, &n1, &n2, &n3);

        assert!(f_sig > 0.4, "F irreducibility: {f_sig}");
        assert!(e_sig > 0.1, "E irreducibility: {e_sig}");
        assert!(ch_sig < 0.2, "Chain (reducible) leaked: {ch_sig}");
        assert!(in_sig < 0.15, "Independent3 leaked: {in_sig}");
        assert!(f_sig > ch_sig);
        assert!(e_sig > ch_sig);
    }
}
