//! Statistical helpers used by axiom evidence and motif scoring:
//! Wilson 95% CI on a binomial proportion + null-baseline probability
//! under iid Bernoulli edges. ADR 0045.

/// Wilson score 95% confidence interval on the binomial proportion
/// `s / n`. Returns `(lower, upper)`. ADR 0045.
///
/// Wilson score is an interval-estimator that's better than the
/// normal approximation for small `n` and extreme `s / n`. Formula
/// with `z = 1.96`:
///
/// ```text
/// p_hat = s / n
/// denom = 1 + z² / n
/// center = (p_hat + z² / (2n)) / denom
/// halfwidth = z × sqrt(p_hat(1 − p_hat)/n + z²/(4n²)) / denom
/// ```
pub fn wilson_score_95(successes: usize, n: usize) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let z = 1.96_f64;
    let z2 = z * z;
    let n_f = n as f64;
    let p_hat = successes as f64 / n_f;
    let denom = 1.0 + z2 / n_f;
    let center = (p_hat + z2 / (2.0 * n_f)) / denom;
    let halfwidth =
        (z * (p_hat * (1.0 - p_hat) / n_f + z2 / (4.0 * n_f * n_f)).sqrt()) / denom;
    (
        (center - halfwidth).max(0.0),
        (center + halfwidth).min(1.0),
    )
}

/// Null-baseline probability of a template's result under iid
/// Bernoulli edges with density `p` = data_edges / |ids|².
/// ADR 0045. If premise holds on N bindings and conclusion
/// satisfied on all N, returns `p_conclusion^N` — the chance
/// of this observation under random edges. A small value = the
/// observed rate is surprising and less likely to be accidental.
pub fn null_baseline_probability(
    bindings: usize,
    satisfied: usize,
    p_edge: f64,
) -> f64 {
    if p_edge <= 0.0 || bindings == 0 {
        return 1.0;
    }
    if p_edge >= 1.0 {
        return 1.0;
    }
    if satisfied < bindings {
        return 1.0; // not a rate-1.0 claim; no significance discount
    }
    p_edge.powi(bindings as i32)
}
