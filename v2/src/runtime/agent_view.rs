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
