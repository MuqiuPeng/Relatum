//! Tübingen Cause-Effect Pairs — cross-realm bridge.
//!
//! The first external-realm entry into v3. Each Tübingen pair is two
//! iid sample series `X` and `Y` plus a ground-truth direction
//! `X → Y` or `Y → X`. The benchmark question is whether observed
//! marginals carry enough asymmetry to recover the direction.
//!
//! v3's binary-mechanism fingerprint engine was calibrated on
//! continuous random walks, so two of its seven fields do not transfer
//! to iid data:
//!
//! - `velocity_effect` — depends on `target[t] − target[t−1]`. With
//!   iid samples, "t" is just sample index; the differences carry no
//!   structural signal.
//! - `latency` — cross-correlation of derivatives. Same problem.
//!
//! The other four fields (`constraint_effect`, `position_effect`,
//! `reversibility`, `stability`) are quartile-binning summary
//! statistics that apply just as well to iid samples as to time
//! series. `iid_directionality` computes asymmetry from `CE` and `PE`
//! only.
//!
//! Calling convention: each pair becomes a 2-node `Episode`, with
//! sample index playing the role of timestep. `judge_pair` produces
//! a `CausalDirection` from `iid_directionality` and compares with
//! the pair's `ground_truth` for evaluation. Real Tübingen pairs are
//! ingested by populating `CausalPair.source_series` /
//! `target_series` from the dataset's column layout (loader code is
//! out of scope for this slice — the substrate-side translation is
//! what matters here).

use crate::fingerprint::{constraint_effect, position_effect};
use crate::{Episode, NodeId, Observation};
use std::collections::BTreeMap;

/// Ground-truth causal direction for a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalDirection {
    /// Variable in the `source` slot is the cause.
    SourceCausesTarget,
    /// Variable in the `target` slot is the cause.
    TargetCausesSource,
}

/// A causal-pair sample in Tübingen-style format.
///
/// Both series should be the same length; the shorter one truncates.
/// Values can be on any scale — `pair_to_episode` normalises each
/// series to `[0, 1]` independently.
#[derive(Debug, Clone)]
pub struct CausalPair {
    pub source_label: String,
    pub target_label: String,
    pub source_series: Vec<f64>,
    pub target_series: Vec<f64>,
    pub ground_truth: CausalDirection,
}

impl CausalPair {
    pub fn n_samples(&self) -> usize {
        self.source_series.len().min(self.target_series.len())
    }
}

/// Translate a `CausalPair` into a 2-node Episode.
///
/// Each sample becomes one `Observation` with `t` set to the sample
/// index. Both series are min-max normalised into `[0, 1]`. Node ids
/// are fixed strings `"X"` and `"Y"` (A1 guarantees this doesn't
/// matter for the recovered structure).
pub fn pair_to_episode(pair: &CausalPair, episode_id: impl Into<String>) -> Episode {
    let source = NodeId::new("X");
    let target = NodeId::new("Y");
    let s_norm = normalise(&pair.source_series);
    let t_norm = normalise(&pair.target_series);
    let n = s_norm.len().min(t_norm.len());
    let mut observations = Vec::with_capacity(n);
    for i in 0..n {
        let mut states = BTreeMap::new();
        states.insert(source.clone(), vec![s_norm[i]]);
        states.insert(target.clone(), vec![t_norm[i]]);
        observations.push(Observation {
            t: i as u64,
            states,
        });
    }
    Episode {
        id: episode_id.into(),
        nodes: vec![source, target],
        observations,
        interventions: vec![],
    }
}

fn normalise(xs: &[f64]) -> Vec<f64> {
    if xs.is_empty() {
        return Vec::new();
    }
    let min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range < 1e-12 {
        return vec![0.5; xs.len()];
    }
    xs.iter().map(|x| (x - min) / range).collect()
}

/// iid-only causal direction estimator. Range `[-1, 1]`.
///
/// Returns `(strength(X → Y) − strength(Y → X)) / sum`, where
/// `strength` is `max(constraint_effect, position_effect)` in each
/// direction. Positive → `X` causes `Y`; negative → `Y` causes `X`;
/// near zero → no detectable directional asymmetry.
///
/// **Why CE+PE and not the full fingerprint:** `velocity_effect` and
/// `latency` need consecutive-sample structure that iid pairs do not
/// have. Including them would inject noise that does not encode
/// causal direction.
pub fn iid_directionality(ep: &Episode, x: &NodeId, y: &NodeId) -> f64 {
    let ce_xy = constraint_effect(ep, x, y);
    let pe_xy = position_effect(ep, x, y);
    let ce_yx = constraint_effect(ep, y, x);
    let pe_yx = position_effect(ep, y, x);
    let s_xy = ce_xy.max(pe_xy);
    let s_yx = ce_yx.max(pe_yx);
    let sum = s_xy + s_yx;
    if sum < 1e-12 {
        return 0.0;
    }
    (s_xy - s_yx) / sum
}

/// v3's judgment on a pair: which variable does the substrate
/// believe is the cause?
pub fn judge_pair(pair: &CausalPair) -> CausalDirection {
    let ep = pair_to_episode(pair, "P");
    let x = &ep.nodes[0];
    let y = &ep.nodes[1];
    let dir = iid_directionality(&ep, x, y);
    if dir >= 0.0 {
        CausalDirection::SourceCausesTarget
    } else {
        CausalDirection::TargetCausesSource
    }
}

// ---- Tübingen file-format loader -----------------------------------------

/// One row of `pairmeta.txt`, parsed.
///
/// Tübingen format: `<id> <c_first> <c_last> <e_first> <e_last> <weight>`,
/// columns 1-indexed, both endpoints inclusive. The first variable's
/// column range is the cause; the second is the effect. The weight
/// is `1.0` for full-confidence pairs and lower (e.g. `0.5`) for
/// pairs the dataset authors flag as ambiguous or contested.
#[derive(Debug, Clone, PartialEq)]
pub struct PairMeta {
    pub id: String,
    pub cause_cols: (usize, usize),
    pub effect_cols: (usize, usize),
    pub weight: f64,
}

/// Parse one whitespace-separated line of `pairmeta.txt`. Returns
/// `None` for malformed lines (blanks, comments, parse errors).
pub fn parse_pairmeta_line(line: &str) -> Option<PairMeta> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let mut parts = trimmed.split_whitespace();
    let id = parts.next()?.to_string();
    let c_first: usize = parts.next()?.parse().ok()?;
    let c_last: usize = parts.next()?.parse().ok()?;
    let e_first: usize = parts.next()?.parse().ok()?;
    let e_last: usize = parts.next()?.parse().ok()?;
    let weight: f64 = parts.next()?.parse().ok()?;
    Some(PairMeta {
        id,
        cause_cols: (c_first, c_last),
        effect_cols: (e_first, e_last),
        weight,
    })
}

/// Parse a Tübingen pair data file's contents — whitespace-separated
/// numeric columns, one observation per line. Lines that fail to
/// parse cleanly (header lines, blank lines) are skipped.
pub fn parse_pair_data(content: &str) -> Vec<Vec<f64>> {
    content
        .lines()
        .filter_map(|line| {
            let cols: Vec<f64> = line
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if cols.is_empty() {
                None
            } else {
                Some(cols)
            }
        })
        .collect()
}

/// Build a `CausalPair` from parsed metadata and parsed rows.
///
/// **Bivariate only.** Returns `None` if the cause or effect column
/// range spans more than one column (multivariate pairs are out of
/// scope for this slice — `CausalPair` carries scalar `Vec<f64>`
/// series, not multidim states). Roughly 90 of the 108 Tübingen
/// pairs are bivariate.
pub fn pair_from_meta(meta: &PairMeta, rows: &[Vec<f64>]) -> Option<CausalPair> {
    if meta.cause_cols.0 != meta.cause_cols.1 || meta.effect_cols.0 != meta.effect_cols.1 {
        return None;
    }
    let c_idx = meta.cause_cols.0.checked_sub(1)?;
    let e_idx = meta.effect_cols.0.checked_sub(1)?;
    let mut source_series = Vec::with_capacity(rows.len());
    let mut target_series = Vec::with_capacity(rows.len());
    for row in rows {
        if c_idx >= row.len() || e_idx >= row.len() {
            continue;
        }
        source_series.push(row[c_idx]);
        target_series.push(row[e_idx]);
    }
    if source_series.len() < 8 {
        return None;
    }
    Some(CausalPair {
        source_label: format!("pair{}_col{}", meta.id, meta.cause_cols.0),
        target_label: format!("pair{}_col{}", meta.id, meta.effect_cols.0),
        source_series,
        target_series,
        ground_truth: CausalDirection::SourceCausesTarget,
    })
}

/// Load the full Tübingen dataset from disk.
///
/// `pairmeta_path` is the path to `pairmeta.txt`. `data_dir` is the
/// directory containing `pair0001.txt`, `pair0002.txt`, … files (the
/// canonical 4-digit zero-padded naming). Multivariate, malformed,
/// and too-short pairs are silently skipped.
///
/// The download URL the user will most likely want is
/// <https://webdav.tuebingen.mpg.de/cause-effect/>.
pub fn load_tubingen_dataset(
    pairmeta_path: &std::path::Path,
    data_dir: &std::path::Path,
) -> std::io::Result<Vec<CausalPair>> {
    let meta_content = std::fs::read_to_string(pairmeta_path)?;
    let mut pairs = Vec::new();
    for line in meta_content.lines() {
        let meta = match parse_pairmeta_line(line) {
            Some(m) => m,
            None => continue,
        };
        let data_path = data_dir.join(format!("pair{}.txt", meta.id));
        let content = match std::fs::read_to_string(&data_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let rows = parse_pair_data(&content);
        if let Some(pair) = pair_from_meta(&meta, &rows) {
            pairs.push(pair);
        }
    }
    Ok(pairs)
}

// ---- evaluation summary --------------------------------------------------

/// Batch accuracy on a slice of pairs.
#[derive(Debug, Clone, Copy)]
pub struct EvalSummary {
    pub n: usize,
    pub correct: usize,
    pub accuracy: f64,
}

pub fn evaluate(pairs: &[CausalPair]) -> EvalSummary {
    if pairs.is_empty() {
        return EvalSummary {
            n: 0,
            correct: 0,
            accuracy: 0.0,
        };
    }
    let correct = pairs
        .iter()
        .filter(|p| judge_pair(p) == p.ground_truth)
        .count();
    EvalSummary {
        n: pairs.len(),
        correct,
        accuracy: correct as f64 / pairs.len() as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    /// Synthetic pair built around mechanism A's iid analogue:
    /// `X ∼ Uniform[0, 1]`; when `X > 0.5`, `Y` is compressed near
    /// `0.5`; otherwise `Y` is free uniform. This is the cleanest
    /// asymmetric marginal-binning signature v3 has, so the iid
    /// pipeline should report `X → Y` correctly.
    #[test]
    fn synthetic_compression_x_causes_y() {
        let pair = compression_pair(42, 800, CausalDirection::SourceCausesTarget, false);
        assert_eq!(judge_pair(&pair), CausalDirection::SourceCausesTarget);
    }

    /// **Control.** Same generated data, source/target labels
    /// swapped. The substrate must invert its judgment — failure
    /// here would mean we always report `SourceCausesTarget`
    /// regardless of evidence.
    #[test]
    fn swapped_labels_invert_the_judgment() {
        let pair = compression_pair(42, 800, CausalDirection::TargetCausesSource, true);
        assert_eq!(judge_pair(&pair), CausalDirection::TargetCausesSource);
    }

    /// Two independent uniform samples: no causal structure.
    /// Directionality magnitude should stay near zero.
    #[test]
    fn independent_variables_give_near_zero_direction() {
        let mut rng = Rng::new(7);
        let n = 800;
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);
        for _ in 0..n {
            xs.push(rng.uniform());
            ys.push(rng.uniform());
        }
        let pair = CausalPair {
            source_label: "X".into(),
            target_label: "Y".into(),
            source_series: xs,
            target_series: ys,
            ground_truth: CausalDirection::SourceCausesTarget,
        };
        let ep = pair_to_episode(&pair, "P");
        let dir = iid_directionality(&ep, &ep.nodes[0], &ep.nodes[1]);
        assert!(
            dir.abs() < 0.3,
            "expected near-zero direction, got {dir}"
        );
    }

    /// `evaluate` reports accuracy across a batch. Sanity check on
    /// the eval glue: a batch where 2 of 3 pairs are correct should
    /// report 2/3.
    #[test]
    fn evaluate_reports_batch_accuracy() {
        let pairs = vec![
            compression_pair(1, 600, CausalDirection::SourceCausesTarget, false),
            compression_pair(2, 600, CausalDirection::SourceCausesTarget, false),
            // Deliberately mislabelled — true direction is X → Y but
            // we claim Y → X. `judge_pair` will say SourceCausesTarget;
            // ground truth is TargetCausesSource → counted as wrong.
            CausalPair {
                ground_truth: CausalDirection::TargetCausesSource,
                ..compression_pair(3, 600, CausalDirection::SourceCausesTarget, false)
            },
        ];
        let summary = evaluate(&pairs);
        assert_eq!(summary.n, 3);
        assert_eq!(summary.correct, 2);
        assert!((summary.accuracy - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn parse_pairmeta_line_bivariate() {
        let parsed = parse_pairmeta_line("0001 1 1 2 2 1.0").unwrap();
        assert_eq!(
            parsed,
            PairMeta {
                id: "0001".to_string(),
                cause_cols: (1, 1),
                effect_cols: (2, 2),
                weight: 1.0,
            }
        );
    }

    #[test]
    fn parse_pairmeta_line_handles_blank_and_comments() {
        assert!(parse_pairmeta_line("").is_none());
        assert!(parse_pairmeta_line("   ").is_none());
        assert!(parse_pairmeta_line("# header").is_none());
        assert!(parse_pairmeta_line("0042 not numbers").is_none());
    }

    #[test]
    fn parse_pairmeta_line_multivariate() {
        let parsed = parse_pairmeta_line("0052 1 3 4 5 0.5").unwrap();
        assert_eq!(parsed.cause_cols, (1, 3));
        assert_eq!(parsed.effect_cols, (4, 5));
        assert_eq!(parsed.weight, 0.5);
    }

    #[test]
    fn parse_pair_data_skips_unparsable_lines() {
        let content = "1.0 2.0\n3.0 4.0\n\n5.0 6.0\nheader line\n7.0 8.0";
        let rows = parse_pair_data(content);
        assert_eq!(
            rows,
            vec![
                vec![1.0, 2.0],
                vec![3.0, 4.0],
                vec![5.0, 6.0],
                vec![7.0, 8.0]
            ]
        );
    }

    #[test]
    fn pair_from_meta_rejects_multivariate() {
        let meta = PairMeta {
            id: "0052".into(),
            cause_cols: (1, 3),
            effect_cols: (4, 5),
            weight: 0.5,
        };
        let rows = vec![vec![1.0; 5]; 100];
        assert!(pair_from_meta(&meta, &rows).is_none());
    }

    #[test]
    fn pair_from_meta_rejects_too_few_samples() {
        let meta = PairMeta {
            id: "0099".into(),
            cause_cols: (1, 1),
            effect_cols: (2, 2),
            weight: 1.0,
        };
        let rows = vec![vec![0.1, 0.2]; 3];
        assert!(pair_from_meta(&meta, &rows).is_none());
    }

    /// End-to-end: serialise a compression pair into Tübingen-format
    /// strings, parse them back through the loader, and verify that
    /// `judge_pair` still recovers `SourceCausesTarget`. This is the
    /// shape the real loader will operate in once the dataset is
    /// downloaded into a local `pair0001.txt` + `pairmeta.txt`.
    #[test]
    fn end_to_end_tubingen_format_round_trip() {
        let meta_line = "0001 1 1 2 2 1.0";

        let mut rng = Rng::new(123);
        let mut data_lines = Vec::new();
        for _ in 0..600 {
            let x = rng.uniform();
            let y = if x > 0.5 {
                (0.5 + rng.normal(0.0, 0.05)).clamp(0.0, 1.0)
            } else {
                rng.uniform()
            };
            data_lines.push(format!("{x:.6} {y:.6}"));
        }
        let data_content = data_lines.join("\n");

        let meta = parse_pairmeta_line(meta_line).unwrap();
        let rows = parse_pair_data(&data_content);
        let pair = pair_from_meta(&meta, &rows).unwrap();

        assert_eq!(pair.n_samples(), 600);
        assert_eq!(judge_pair(&pair), CausalDirection::SourceCausesTarget);
    }

    fn compression_pair(
        seed: u64,
        n: usize,
        ground_truth: CausalDirection,
        swap_labels: bool,
    ) -> CausalPair {
        let mut rng = Rng::new(seed);
        let mut xs = Vec::with_capacity(n);
        let mut ys = Vec::with_capacity(n);
        for _ in 0..n {
            let x = rng.uniform();
            let y = if x > 0.5 {
                (0.5 + rng.normal(0.0, 0.05)).clamp(0.0, 1.0)
            } else {
                rng.uniform()
            };
            xs.push(x);
            ys.push(y);
        }
        if swap_labels {
            CausalPair {
                source_label: "Y_in_source_slot".into(),
                target_label: "X_in_target_slot".into(),
                source_series: ys,
                target_series: xs,
                ground_truth,
            }
        } else {
            CausalPair {
                source_label: "X".into(),
                target_label: "Y".into(),
                source_series: xs,
                target_series: ys,
                ground_truth,
            }
        }
    }
}
