//! A3 serialization helpers — checkpoint parse/write primitives
//! and free functions for round-tripping runtime state. ADR 0052 / A3.

use std::collections::HashMap;

use super::action::{ActionKind, FrontierTarget};
use super::lifecycle::{LifecycleState, RuntimeMode};
use super::memory::ObjectHistory;

// ─── A3: serialization helpers ─────────────────────────────────────

pub(crate) fn mode_to_str(m: RuntimeMode) -> &'static str {
    match m {
        RuntimeMode::Expand => "Expand",
        RuntimeMode::Consolidate => "Consolidate",
        RuntimeMode::Reflect => "Reflect",
    }
}

pub(crate) fn parse_mode(s: &str) -> Result<RuntimeMode, String> {
    match s {
        "Expand" => Ok(RuntimeMode::Expand),
        "Consolidate" => Ok(RuntimeMode::Consolidate),
        "Reflect" => Ok(RuntimeMode::Reflect),
        other => Err(format!("unknown RuntimeMode '{}'", other)),
    }
}

pub(crate) fn lifecycle_to_str(l: LifecycleState) -> &'static str {
    match l {
        LifecycleState::Booting => "Booting",
        LifecycleState::Running => "Running",
        LifecycleState::Sleeping => "Sleeping",
        LifecycleState::Stopped => "Stopped",
    }
}

pub(crate) fn parse_lifecycle(s: &str) -> Result<LifecycleState, String> {
    match s {
        "Booting" => Ok(LifecycleState::Booting),
        "Running" => Ok(LifecycleState::Running),
        "Sleeping" => Ok(LifecycleState::Sleeping),
        "Stopped" => Ok(LifecycleState::Stopped),
        other => Err(format!("unknown LifecycleState '{}'", other)),
    }
}

pub(crate) fn action_kind_to_str(a: ActionKind) -> &'static str {
    match a {
        ActionKind::DiscoverPatterns => "DiscoverPatterns",
        ActionKind::DiscoverTheory => "DiscoverTheory",
        ActionKind::PruneLowValueObjects => "PruneLowValueObjects",
        ActionKind::UpdateTheoryRelations => "UpdateTheoryRelations",
        ActionKind::Declarativize => "Declarativize",
        ActionKind::DiscoverMetaMetaPatterns => "DiscoverMetaMetaPatterns",
        ActionKind::EvaluatePredictions => "EvaluatePredictions",
        ActionKind::ExecuteComposite => "ExecuteComposite",
        ActionKind::DiscoverAxiomShapeFamilies => "DiscoverAxiomShapeFamilies",
        ActionKind::RetractShapeFamily => "RetractShapeFamily",
        ActionKind::ApplyRecommendedIntervention => "ApplyRecommendedIntervention",
        ActionKind::ApplyRecommendedPatternIntervention => "ApplyRecommendedPatternIntervention",
    }
}

pub(crate) fn parse_action_kind(s: &str) -> Result<ActionKind, String> {
    match s {
        "DiscoverPatterns" => Ok(ActionKind::DiscoverPatterns),
        "DiscoverTheory" => Ok(ActionKind::DiscoverTheory),
        "PruneLowValueObjects" => Ok(ActionKind::PruneLowValueObjects),
        "UpdateTheoryRelations" => Ok(ActionKind::UpdateTheoryRelations),
        "Declarativize" => Ok(ActionKind::Declarativize),
        "DiscoverMetaMetaPatterns" => {
            Ok(ActionKind::DiscoverMetaMetaPatterns)
        }
        "EvaluatePredictions" => Ok(ActionKind::EvaluatePredictions),
        "ExecuteComposite" => Ok(ActionKind::ExecuteComposite),
        "DiscoverAxiomShapeFamilies" => Ok(ActionKind::DiscoverAxiomShapeFamilies),
        "RetractShapeFamily" => Ok(ActionKind::RetractShapeFamily),
        "ApplyRecommendedIntervention" => {
            Ok(ActionKind::ApplyRecommendedIntervention)
        }
        "ApplyRecommendedPatternIntervention" => {
            Ok(ActionKind::ApplyRecommendedPatternIntervention)
        }
        other => Err(format!("unknown ActionKind '{}'", other)),
    }
}

pub(crate) fn target_to_pair(t: &FrontierTarget) -> (&'static str, String) {
    match t {
        FrontierTarget::WholeRSet => ("WholeRSet", String::new()),
        FrontierTarget::PatternSize(s) => ("PatternSize", s.to_string()),
        FrontierTarget::Pattern(id) => ("Pattern", id.clone()),
        FrontierTarget::Theory(id) => ("Theory", id.clone()),
        FrontierTarget::Axiom(id) => ("Axiom", id.clone()),
        FrontierTarget::ActionSequence(id) => ("ActionSequence", id.clone()),
        FrontierTarget::ShapeFamily(id) => ("ShapeFamily", id.clone()),
    }
}

pub(crate) fn pair_to_target(kind: &str, value: &str) -> Result<FrontierTarget, String> {
    match kind {
        "WholeRSet" => Ok(FrontierTarget::WholeRSet),
        "PatternSize" => Ok(FrontierTarget::PatternSize(
            parse_usize(value, "PatternSize.value")?,
        )),
        "Pattern" => Ok(FrontierTarget::Pattern(value.to_string())),
        "Theory" => Ok(FrontierTarget::Theory(value.to_string())),
        "Axiom" => Ok(FrontierTarget::Axiom(value.to_string())),
        "ActionSequence" => {
            Ok(FrontierTarget::ActionSequence(value.to_string()))
        }
        "ShapeFamily" => Ok(FrontierTarget::ShapeFamily(value.to_string())),
        other => Err(format!("unknown FrontierTarget kind '{}'", other)),
    }
}

pub(crate) fn parse_u64(s: &str, ctx: &str) -> Result<u64, String> {
    s.parse::<u64>()
        .map_err(|e| format!("{}: parse u64 '{}' failed: {}", ctx, s, e))
}

pub(crate) fn parse_u32(s: &str, ctx: &str) -> Result<u32, String> {
    s.parse::<u32>()
        .map_err(|e| format!("{}: parse u32 '{}' failed: {}", ctx, s, e))
}

pub(crate) fn parse_usize(s: &str, ctx: &str) -> Result<usize, String> {
    s.parse::<usize>()
        .map_err(|e| format!("{}: parse usize '{}' failed: {}", ctx, s, e))
}

pub(crate) fn parse_f64(s: &str, ctx: &str) -> Result<f64, String> {
    s.parse::<f64>()
        .map_err(|e| format!("{}: parse f64 '{}' failed: {}", ctx, s, e))
}

pub(crate) fn check_reason(reason: &str, ctx: &str) -> Result<(), String> {
    if reason.contains('\t') || reason.contains('\n') {
        return Err(format!(
            "{} reason '{}' contains tab or newline",
            ctx, reason
        ));
    }
    Ok(())
}

/// Sentinel for missing optional values in the checkpoint format.
/// `parse_opt_*` accept this and return `None`; `format_opt_*` write
/// it when the value is `None`. Chosen because `-` is not a legal
/// prefix for the unsigned and float values we serialize, so it
/// can't ambiguously parse as data.
const OPT_NONE: &str = "-";

pub(crate) fn format_opt_u64(v: Option<u64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => OPT_NONE.to_string(),
    }
}

pub(crate) fn parse_opt_u64(s: &str, ctx: &str) -> Result<Option<u64>, String> {
    if s == OPT_NONE {
        Ok(None)
    } else {
        Ok(Some(parse_u64(s, ctx)?))
    }
}

pub(crate) fn format_opt_f64(v: Option<f64>) -> String {
    match v {
        Some(n) => format!("{:?}", n),
        None => OPT_NONE.to_string(),
    }
}

pub(crate) fn parse_opt_f64(s: &str, ctx: &str) -> Result<Option<f64>, String> {
    if s == OPT_NONE {
        Ok(None)
    } else {
        Ok(Some(parse_f64(s, ctx)?))
    }
}

/// Format: `<id>\t<first>\t<last_seen>\t<last_improved>\t<focus>\t<pruned>\t<cv>\t<stability>`
/// where `last_improved`, `cv`, `stability` use `-` for `None`.
pub(crate) fn write_history_section(
    out: &mut String,
    header: &str,
    map: &HashMap<String, ObjectHistory>,
) -> Result<(), String> {
    out.push_str(header);
    out.push('\n');
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    for k in keys {
        if k.contains('\t') || k.contains('\n') {
            return Err(format!(
                "history id '{}' contains tab or newline",
                k
            ));
        }
        let h = &map[k];
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            k,
            h.first_seen_tick,
            h.last_seen_tick,
            format_opt_u64(h.last_improved_tick),
            h.times_selected_as_focus,
            h.times_pruned,
            format_opt_f64(h.last_counterfactual_value),
            format_opt_f64(h.stability_estimate),
            h.times_contributed_positive,
        ));
    }
    Ok(())
}

pub(crate) fn parse_history_lines(
    lines: &[String],
    label: &str,
) -> Result<HashMap<String, ObjectHistory>, String> {
    let mut out: HashMap<String, ObjectHistory> = HashMap::new();
    for (idx, raw) in lines.iter().enumerate() {
        let fields: Vec<&str> = raw.split('\t').collect();
        if fields.len() != 9 {
            return Err(format!(
                "{} line {} has {} fields, expected 9",
                label,
                idx + 1,
                fields.len()
            ));
        }
        let id = fields[0].to_string();
        let h = ObjectHistory {
            first_seen_tick: parse_u64(fields[1], &format!("{}.first", label))?,
            last_seen_tick: parse_u64(fields[2], &format!("{}.last", label))?,
            last_improved_tick: parse_opt_u64(
                fields[3],
                &format!("{}.last_improved", label),
            )?,
            times_selected_as_focus: parse_u32(
                fields[4],
                &format!("{}.focus", label),
            )?,
            times_pruned: parse_u32(fields[5], &format!("{}.pruned", label))?,
            last_counterfactual_value: parse_opt_f64(
                fields[6],
                &format!("{}.cv", label),
            )?,
            stability_estimate: parse_opt_f64(
                fields[7],
                &format!("{}.stability", label),
            )?,
            times_contributed_positive: parse_u32(
                fields[8],
                &format!("{}.contributed", label),
            )?,
        };
        out.insert(id, h);
    }
    Ok(out)
}

pub(crate) fn check_no_tab_or_newline(t: &FrontierTarget, ctx: &str) -> Result<(), String> {
    let id = match t {
        FrontierTarget::WholeRSet | FrontierTarget::PatternSize(_) => return Ok(()),
        FrontierTarget::Pattern(s)
        | FrontierTarget::Theory(s)
        | FrontierTarget::Axiom(s)
        | FrontierTarget::ActionSequence(s)
        | FrontierTarget::ShapeFamily(s) => s,
    };
    if id.contains('\t') || id.contains('\n') {
        return Err(format!(
            "{} target id '{}' contains tab or newline",
            ctx, id
        ));
    }
    Ok(())
}

#[derive(Default)]
pub(crate) struct ParsedCheckpoint {
    pub(crate) meta: HashMap<String, String>,
    pub(crate) rset_lines: Vec<String>,
    pub(crate) episode_lines: Vec<String>,
    pub(crate) mode_transition_lines: Vec<String>,
    pub(crate) lifecycle_transition_lines: Vec<String>,
    // B2 — history + stats sections.
    pub(crate) history_patterns_lines: Vec<String>,
    pub(crate) history_axioms_lines: Vec<String>,
    pub(crate) history_theories_lines: Vec<String>,
    pub(crate) action_count_lines: Vec<String>,
    pub(crate) mode_transition_count_lines: Vec<String>,
    pub(crate) lifecycle_count_lines: Vec<String>,
    // ADR 0059 / G1.3 — prediction-state cumulative counters.
    pub(crate) prediction_state_lines: Vec<String>,
    // ADR 0061 / H1.0 — sequence-stats accounting.
    pub(crate) sequence_stats_lines: Vec<String>,
    // ADR 0063 / H2.0 step 2 — DriveMix A/B state.
    pub(crate) drive_mix_lines: Vec<String>,
}

pub(crate) fn parse_checkpoint(text: &str) -> Result<ParsedCheckpoint, String> {
    let mut out = ParsedCheckpoint::default();
    let mut section: Option<&str> = None;

    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            match name {
                "meta" | "rset" | "episodes" | "mode_transitions"
                | "lifecycle_transitions"
                | "object_history_patterns"
                | "object_history_axioms"
                | "object_history_theories"
                | "policy_stats_action_counts"
                | "policy_stats_mode_transition_counts"
                | "policy_stats_lifecycle_counts"
                | "prediction_state"
                | "sequence_stats"
                | "drive_mix" => {
                    section = Some(match name {
                        "meta" => "meta",
                        "rset" => "rset",
                        "episodes" => "episodes",
                        "mode_transitions" => "mode_transitions",
                        "lifecycle_transitions" => "lifecycle_transitions",
                        "object_history_patterns" => "object_history_patterns",
                        "object_history_axioms" => "object_history_axioms",
                        "object_history_theories" => "object_history_theories",
                        "policy_stats_action_counts" => "policy_stats_action_counts",
                        "policy_stats_mode_transition_counts" => {
                            "policy_stats_mode_transition_counts"
                        }
                        "policy_stats_lifecycle_counts" => "policy_stats_lifecycle_counts",
                        "prediction_state" => "prediction_state",
                        "sequence_stats" => "sequence_stats",
                        _ => "drive_mix",
                    });
                }
                other => {
                    return Err(format!(
                        "unknown section '[{}]' at line {}",
                        other,
                        i + 1
                    ))
                }
            }
            continue;
        }
        match section {
            Some("meta") => {
                let (k, v) = line.split_once('\t').ok_or_else(|| {
                    format!(
                        "meta line {} not key<TAB>value: '{}'",
                        i + 1,
                        line
                    )
                })?;
                out.meta.insert(k.to_string(), v.to_string());
            }
            Some("rset") => out.rset_lines.push(line.to_string()),
            Some("episodes") => out.episode_lines.push(line.to_string()),
            Some("mode_transitions") => {
                out.mode_transition_lines.push(line.to_string())
            }
            Some("lifecycle_transitions") => {
                out.lifecycle_transition_lines.push(line.to_string())
            }
            Some("object_history_patterns") => {
                out.history_patterns_lines.push(line.to_string())
            }
            Some("object_history_axioms") => {
                out.history_axioms_lines.push(line.to_string())
            }
            Some("object_history_theories") => {
                out.history_theories_lines.push(line.to_string())
            }
            Some("policy_stats_action_counts") => {
                out.action_count_lines.push(line.to_string())
            }
            Some("policy_stats_mode_transition_counts") => {
                out.mode_transition_count_lines.push(line.to_string())
            }
            Some("policy_stats_lifecycle_counts") => {
                out.lifecycle_count_lines.push(line.to_string())
            }
            Some("prediction_state") => {
                out.prediction_state_lines.push(line.to_string())
            }
            Some("sequence_stats") => {
                out.sequence_stats_lines.push(line.to_string())
            }
            Some("drive_mix") => {
                out.drive_mix_lines.push(line.to_string())
            }
            None => {
                return Err(format!(
                    "data line {} has no enclosing section: '{}'",
                    i + 1,
                    line
                ))
            }
            _ => unreachable!(),
        }
    }
    Ok(out)
}
