//! ADR 0076 — Micro-agent reframing query helpers.
//!
//! Read-only views over `Memory::episodes` that re-group the
//! existing dispatch log into "agent classes" (each class being
//! an `(ActionKind, target-type)` pair). No new state is stored;
//! these are query results derived from data the runtime already
//! records. Constitution-compliant under the heavy reading: no
//! agent token is registered, no agent attribute is named — the
//! reframing happens entirely at interpretation.

use std::collections::HashMap;

use super::action::{ActionKind, FrontierTarget};
use super::memory::Episode;

/// Stable string key for a target's variant kind. Strips
/// per-instance ids (e.g., `Pattern("p_3") → "Pattern"`) so
/// dispatches against different specific patterns still cluster
/// into the same agent class. Preserves `PatternSize(N)` because
/// size is meaningful for differentiating pattern-mining agents.
pub fn target_kind_label(t: &FrontierTarget) -> String {
    match t {
        FrontierTarget::WholeRSet => "WholeRSet".to_string(),
        FrontierTarget::PatternSize(n) => format!("PatternSize({})", n),
        FrontierTarget::Pattern(_) => "Pattern".to_string(),
        FrontierTarget::Theory(_) => "Theory".to_string(),
        FrontierTarget::Axiom(_) => "Axiom".to_string(),
        FrontierTarget::ActionSequence(_) => "ActionSequence".to_string(),
        FrontierTarget::ShapeFamily(_) => "ShapeFamily".to_string(),
    }
}

/// Summary of one agent class's behaviour over a window of
/// episodes. ADR 0076 — no persistent state; this is computed
/// on demand from the episode log.
#[derive(Debug, Clone)]
pub struct AgentClassSummary {
    pub action_kind: ActionKind,
    pub target_label: String,
    pub episode_count: usize,
    pub success_count: usize,
    pub success_rate: f64,
    pub first_tick: u64,
    pub last_tick: u64,
    pub mean_delta: f64,
}

/// Group episode log by `(action_kind, target-kind-label)` into
/// agent classes; return summaries sorted by descending episode
/// count. Generic over any iterable of `&Episode` so callers can
/// pass `&Vec<Episode>`, `&VecDeque<Episode>`, or `&[Episode]`
/// equivalently.
pub fn agent_classes<'a, I>(episodes: I) -> Vec<AgentClassSummary>
where
    I: IntoIterator<Item = &'a Episode>,
{
    let mut groups: HashMap<(ActionKind, String), Vec<&Episode>> = HashMap::new();
    for ep in episodes {
        let key = (ep.action_kind, target_kind_label(&ep.target));
        groups.entry(key).or_default().push(ep);
    }

    let mut out: Vec<AgentClassSummary> = groups
        .into_iter()
        .map(|((kind, label), eps)| {
            let count = eps.len();
            let success = eps.iter().filter(|e| e.delta > 0.0).count();
            let total_delta: f64 = eps.iter().map(|e| e.delta).sum();
            let first = eps.iter().map(|e| e.tick).min().unwrap_or(0);
            let last = eps.iter().map(|e| e.tick).max().unwrap_or(0);
            let mean_delta = if count > 0 {
                total_delta / count as f64
            } else {
                0.0
            };
            let success_rate = if count > 0 {
                success as f64 / count as f64
            } else {
                0.0
            };
            AgentClassSummary {
                action_kind: kind,
                target_label: label,
                episode_count: count,
                success_count: success,
                success_rate,
                first_tick: first,
                last_tick: last,
                mean_delta,
            }
        })
        .collect();

    out.sort_by(|a, b| b.episode_count.cmp(&a.episode_count));
    out
}

/// Episodes belonging to a single agent class. ADR 0076 —
/// the agent class's "memory" reconstructed from the log.
/// Generic over any iterable of `&Episode`.
pub fn agent_episodes<'a, I>(
    episodes: I,
    kind: ActionKind,
    target_label: &str,
) -> Vec<&'a Episode>
where
    I: IntoIterator<Item = &'a Episode>,
{
    episodes
        .into_iter()
        .filter(|e| {
            e.action_kind == kind && target_kind_label(&e.target) == target_label
        })
        .collect()
}

/// Recent attention share for an action kind: fraction of the
/// last `n_recent` episodes that were dispatches of this kind.
/// Ignores target-type. Returns 0.0 when episode log is empty.
/// Generic over any iterable of `&Episode`.
pub fn agent_attention_share_recent<'a, I>(
    episodes: I,
    kind: ActionKind,
    n_recent: usize,
) -> f64
where
    I: IntoIterator<Item = &'a Episode>,
{
    if n_recent == 0 {
        return 0.0;
    }
    let all: Vec<&Episode> = episodes.into_iter().collect();
    if all.is_empty() {
        return 0.0;
    }
    let start = all.len().saturating_sub(n_recent);
    let window = &all[start..];
    let count = window.iter().filter(|e| e.action_kind == kind).count();
    count as f64 / window.len() as f64
}

// ─────────────────────────────────────────────────────────────────
// ADR 0076 phase 2 — episode-log enrichments. Three query helpers
// surfacing finer-grained behaviour patterns over the same log:
//  - outcome distribution (delta histogram per agent class)
//  - temporal density (when did the class fire?)
//  - target overlap (which specific instances did the class act on?)
// All read-only; no new state.
// ─────────────────────────────────────────────────────────────────

/// Outcome distribution for one agent class. Buckets the
/// per-episode deltas into negative / zero / positive,
/// reports min / max / mean / median. ADR 0076 phase 2.
#[derive(Debug, Clone)]
pub struct AgentOutcomeDistribution {
    pub episode_count: usize,
    pub negative_count: usize,
    pub zero_count: usize,
    pub positive_count: usize,
    pub min_delta: f64,
    pub max_delta: f64,
    pub mean_delta: f64,
    pub median_delta: f64,
}

impl AgentOutcomeDistribution {
    /// Empty distribution — used when no episodes match.
    pub fn empty() -> Self {
        Self {
            episode_count: 0,
            negative_count: 0,
            zero_count: 0,
            positive_count: 0,
            min_delta: 0.0,
            max_delta: 0.0,
            mean_delta: 0.0,
            median_delta: 0.0,
        }
    }
}

/// Compute the outcome-distribution profile for an agent class.
pub fn agent_outcome_distribution<'a, I>(
    episodes: I,
    kind: ActionKind,
    target_label: &str,
) -> AgentOutcomeDistribution
where
    I: IntoIterator<Item = &'a Episode>,
{
    let mut deltas: Vec<f64> = episodes
        .into_iter()
        .filter(|e| {
            e.action_kind == kind
                && target_kind_label(&e.target) == target_label
        })
        .map(|e| e.delta)
        .collect();
    if deltas.is_empty() {
        return AgentOutcomeDistribution::empty();
    }
    let count = deltas.len();
    let negative = deltas.iter().filter(|d| **d < 0.0).count();
    let zero = deltas.iter().filter(|d| **d == 0.0).count();
    let positive = deltas.iter().filter(|d| **d > 0.0).count();
    let total: f64 = deltas.iter().sum();
    let mean = total / count as f64;
    let min = deltas.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = deltas.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    deltas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if count % 2 == 1 {
        deltas[count / 2]
    } else {
        (deltas[count / 2 - 1] + deltas[count / 2]) / 2.0
    };
    AgentOutcomeDistribution {
        episode_count: count,
        negative_count: negative,
        zero_count: zero,
        positive_count: positive,
        min_delta: min,
        max_delta: max,
        mean_delta: mean,
        median_delta: median,
    }
}

/// Temporal density of one agent class — how its dispatch
/// frequency varies across `n_windows` equal-width tick
/// windows up to `runtime_horizon`. Returns the
/// (start_tick, end_tick, count) per window plus the index of
/// the densest window. ADR 0076 phase 2.
#[derive(Debug, Clone)]
pub struct AgentTemporalDensity {
    pub windows: Vec<(u64, u64, usize)>,
    pub peak_window_idx: Option<usize>,
    pub total_episodes: usize,
}

/// Compute the temporal density for an agent class.
pub fn agent_temporal_density<'a, I>(
    episodes: I,
    kind: ActionKind,
    target_label: &str,
    n_windows: usize,
    runtime_horizon: u64,
) -> AgentTemporalDensity
where
    I: IntoIterator<Item = &'a Episode>,
{
    if n_windows == 0 || runtime_horizon == 0 {
        return AgentTemporalDensity {
            windows: Vec::new(),
            peak_window_idx: None,
            total_episodes: 0,
        };
    }
    let window_size = runtime_horizon.div_ceil(n_windows as u64).max(1);
    let mut counts = vec![0usize; n_windows];
    let mut total = 0usize;
    for ep in episodes {
        if ep.action_kind != kind
            || target_kind_label(&ep.target) != target_label
        {
            continue;
        }
        total += 1;
        if ep.tick == 0 {
            counts[0] += 1;
            continue;
        }
        let idx = ((ep.tick.saturating_sub(1)) / window_size) as usize;
        if idx < n_windows {
            counts[idx] += 1;
        } else {
            counts[n_windows - 1] += 1;
        }
    }
    let windows: Vec<(u64, u64, usize)> = (0..n_windows)
        .map(|i| {
            let start = (i as u64) * window_size + 1;
            let end = ((i as u64) + 1) * window_size;
            (start, end, counts[i])
        })
        .collect();
    let peak_idx = if total == 0 {
        None
    } else {
        Some(
            counts
                .iter()
                .enumerate()
                .max_by_key(|(_, c)| *c)
                .map(|(i, _)| i)
                .unwrap_or(0),
        )
    };
    AgentTemporalDensity {
        windows,
        peak_window_idx: peak_idx,
        total_episodes: total,
    }
}

/// Learning-progress weight for a target size, computed from
/// the recent episode log. ADR 0080.
///
/// Returns a scalar in `[0.0, 1.0]`:
/// - `1.0` when no recent DiscoverPatterns dispatch matches the
///   target_size (no history → no penalty; new canonical gets
///   full priority)
/// - `dp_positive_delta_count / dp_attempt_count` over the
///   most recent `window` episodes that match
///   (ActionKind::DiscoverPatterns, FrontierTarget::PatternSize(size))
///
/// Used by `Frontier::refresh`'s drive-driven candidate
/// branch to downweight priorities of canonicals whose
/// recent dispatches did not produce new mints, naturally
/// throttling sustained-mode dispatch on already-mined
/// canonicals.
///
/// `delta > 0.0` is the success criterion — ADR 0075 piece 2
/// revisited returns `Some(new_patterns as f64)` from DP
/// dispatch when patterns minted, so positive delta == mint
/// happened.
pub fn compute_learning_progress<'a, I>(
    episodes: I,
    target_size: usize,
    window: usize,
) -> f64
where
    I: IntoIterator<Item = &'a Episode>,
{
    let all: Vec<&Episode> = episodes.into_iter().collect();
    let start = all.len().saturating_sub(window);
    let window_slice = &all[start..];
    let mut attempts = 0usize;
    let mut positive = 0usize;
    for ep in window_slice {
        if ep.action_kind != ActionKind::DiscoverPatterns {
            continue;
        }
        let matches_size = matches!(
            ep.target,
            super::action::FrontierTarget::PatternSize(sz) if sz == target_size
        );
        if !matches_size {
            continue;
        }
        attempts += 1;
        if ep.delta > 0.0 {
            positive += 1;
        }
    }
    if attempts == 0 {
        // No history at this size — give full attention to drive
        // candidate. ADR 0080.
        return 1.0;
    }
    positive as f64 / attempts as f64
}

/// Decide whether drive should currently engage runtime
/// (wake-on-drive, bypasses, drive-driven candidates). ADR 0080.
///
/// Returns `true` iff:
/// - drive has signal (some unexplained R), AND
/// - learning progress at the modal canonical's clamped size
///   exceeds `lp_threshold`.
///
/// `lp_threshold = 0.05` (5%) is the suggested floor: lets a
/// bucket with at least *some* mint success keep engaging,
/// blocks buckets with 30+ consecutive zero-mint dispatches.
///
/// `episodes` is the runtime's episode log.
pub fn drive_should_engage<'a, I>(
    drive: &crate::UnexplainedDriveSignal,
    episodes: I,
    lp_threshold: f64,
) -> bool
where
    I: IntoIterator<Item = &'a Episode>,
{
    if !drive.has_signal() {
        return false;
    }
    let canonical = match &drive.modal_canonical {
        Some(c) => c,
        None => return false,
    };
    let size = canonical.len().clamp(2, 5);
    let all: Vec<&Episode> = episodes.into_iter().collect();
    let lp = compute_learning_progress(all.iter().copied(), size, 30);
    lp > lp_threshold
}

/// Target-overlap profile for an agent class: which specific
/// target ids did this class act on, and how often. Useful for
/// agent classes whose target type carries an id (Pattern,
/// Theory, Axiom, ActionSequence, ShapeFamily). For
/// id-less target types (WholeRSet, PatternSize) the profile
/// will collapse to a single bucket. ADR 0076 phase 2.
#[derive(Debug, Clone)]
pub struct AgentTargetOverlap {
    pub target_counts: Vec<(String, usize)>,
    pub distinct_targets: usize,
    pub modal_target: Option<String>,
    pub total_episodes: usize,
}

/// Compute target-overlap for an agent class. The string keys
/// are the targets' `Debug`-formatted strings so id-bearing
/// variants (`Pattern("p_3")`) become distinct entries while
/// id-less variants share a single key.
pub fn agent_target_overlap<'a, I>(
    episodes: I,
    kind: ActionKind,
) -> AgentTargetOverlap
where
    I: IntoIterator<Item = &'a Episode>,
{
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut total = 0usize;
    for ep in episodes {
        if ep.action_kind != kind {
            continue;
        }
        total += 1;
        let key = format!("{:?}", ep.target);
        *counts.entry(key).or_insert(0) += 1;
    }
    let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let distinct = sorted.len();
    let modal = sorted.first().map(|(k, _)| k.clone());
    AgentTargetOverlap {
        target_counts: sorted,
        distinct_targets: distinct,
        modal_target: modal,
        total_episodes: total,
    }
}
