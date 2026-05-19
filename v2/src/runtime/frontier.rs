//! Frontier subsystem (A1): candidate enumeration, dirty tracking,
//! cooldown bookkeeping. ADR 0052 / A1.

use std::collections::{HashSet, VecDeque};

use crate::{
    AxiomDiscoveryConfig, RSet, ESTABLISHED_MARKER, R,
    SHARED_AXIOM_MARKER,
};
use super::action::{ActionKind, FrontierTarget};
use super::agent_view::compute_learning_progress;
use super::memory::Episode;
use super::scheduler_rule::RuleBasedScheduler;
use super::{parse_action_kind, theory_pair_has_relation};
use super::{ObjectHistory, ObjectHistoryStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontierKind {
    TheoryCandidate,
    PatternCandidate,
    LowValueObjectForPrune,
    /// At least two named theories exist with no recorded relation
    /// edge between them. ADR 0052 / A2.
    TheoryNeedsRelations,
    /// A named pattern has met the C0 promotion gate (age + has at
    /// least one positive-delta contribution) and is not yet marked
    /// `R(id, ESTABLISHED_MARKER)`. ADR 0053 / Phase C0.
    EstablishedPromotion,
    /// The rset has accumulated enough M1 marker edges to warrant a
    /// pass of meta-meta discovery. ADR 0054 / Phase D0.
    MetaMetaCandidate,
    /// A promoted `R(ACTION_SEQ_MARKER, seq_N)` chain exists in rset
    /// AND the frontier holds matching items for both prefix and
    /// suffix kinds. The scheduler dispatches `ExecuteComposite`
    /// against the seq_id. ADR 0061 / Phase H1.2.
    CompositeCandidate,
    /// At least 2 registered axioms share a structural sub-component
    /// (premise or conclusion edge set) that has not yet been
    /// captured by an existing shape family. The scheduler dispatches
    /// `DiscoverAxiomShapeFamilies` to mint the family. ADR 0068 /
    /// Phase Beta-1.5.1 (B.5.1). Surfaced when the registered axiom
    /// count > the count at last family discovery (cheap freshness
    /// check; no need to enumerate all premises here).
    ShapeFamilyDiscoveryCandidate,
    /// ADR 0082 — A theory whose current quality report yields an
    /// actionable `recommend_intervention` result (FamilyDemote /
    /// AxiomRepair / TheoryDemote / DemoteSuperset / Merge). The
    /// scheduler dispatches `ApplyRecommendedIntervention` which
    /// re-computes the recommendation at execute time. None /
    /// ShadowMonitor / Manual recommendations are not proposed
    /// (no-op). Cooldown + recent-target filter prevent thrash.
    PolicyTarget,
    /// ADR 0083 — A pattern whose current quality report yields a
    /// `PatternRetract` recommendation. Scheduler dispatches
    /// `ApplyRecommendedPatternIntervention` which re-computes at
    /// execute time and routes to `retract_pattern`. Mirror of
    /// PolicyTarget for the pattern side.
    PatternPolicyTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontierStatus {
    Fresh,
    Active,
    Cooling,
    Saturated,
    Blocked,
}

#[derive(Debug, Clone)]
pub struct FrontierItem {
    pub id: String,
    pub kind: FrontierKind,
    pub target: FrontierTarget,
    pub priority: f64,
    pub estimated_value: f64,
    pub estimated_cost: f64,
    pub novelty_score: f64,
    pub first_seen_tick: u64,
    pub last_visited_tick: Option<u64>,
    pub revisit_count: u32,
    pub cooldown_until_tick: Option<u64>,
    pub status: FrontierStatus,
}

/// Threshold config for staleness-based prune injection.
/// ADR 0052 / B3.
///
/// A named pattern is "stale" if it has been around long enough
/// (`first_seen_tick` ≥ `min_pattern_age_for_staleness` ticks ago)
/// but its `last_improved_tick` has not advanced for at least
/// `max_pattern_staleness_ticks`. Stale patterns become
/// `LowValueObjectForPrune` candidates with a low fixed priority,
/// so the existing Consolidate / Prune lane retires them without
/// the scheduler needing a new dispatch path.
#[derive(Debug, Clone, Copy)]
pub struct StalenessConfig {
    pub max_pattern_staleness_ticks: u64,
    pub min_pattern_age_for_staleness: u64,
}

impl Default for StalenessConfig {
    fn default() -> Self {
        Self {
            max_pattern_staleness_ticks: 30,
            min_pattern_age_for_staleness: 50,
        }
    }
}

/// Config for meta-meta-pattern discovery. ADR 0054 / Phase D0.
///
/// Drives the `MetaMetaCandidate` frontier item: surfaced once the
/// rset has accumulated at least `min_m1_edges_for_meta_meta` edges
/// involving the listed M1 markers. The default markers correspond
/// to ADR 0053's two M1 marker classes.
#[derive(Debug, Clone)]
pub struct MetaMetaConfig {
    pub min_m1_edges_for_meta_meta: usize,
    pub markers: Vec<&'static str>,
    pub target_size: usize,
    pub sample_count: usize,
    pub top_m: usize,
    pub rng_seed: u64,
}

impl Default for MetaMetaConfig {
    fn default() -> Self {
        Self {
            min_m1_edges_for_meta_meta: 5,
            markers: vec![ESTABLISHED_MARKER, SHARED_AXIOM_MARKER],
            target_size: 3,
            sample_count: 200,
            top_m: 10,
            rng_seed: 2026,
        }
    }
}

/// Threshold config for ESTABLISHED promotion. ADR 0053 / Phase C0/C1.
///
/// A named object (pattern or theory) earns the
/// `R(id, ESTABLISHED_MARKER)` edge once it has been alive in the
/// runtime's `ObjectHistory` for at least the relevant age threshold
/// AND has contributed to at least the relevant `min_*_use_for_promotion`
/// number of positive-delta episodes. The contribution count is
/// the `times_contributed_positive` counter on `ObjectHistory`,
/// added in Phase C0+ alongside this knob.
///
/// Theory thresholds are more conservative than pattern (200/3
/// vs. 100/3) per ADR 0053 / Phase C1 — theories are larger
/// investments and the runtime should be slower to declare them
/// stable. The default `min_*_use_for_promotion = 3` reproduces
/// ADR 0053's original sketch ("M = 3"), now that the counter
/// exists to enforce it.
#[derive(Debug, Clone, Copy)]
pub struct PromotionConfig {
    pub min_pattern_age_for_promotion: u64,
    pub min_theory_age_for_promotion: u64,
    pub min_pattern_use_for_promotion: u32,
    pub min_theory_use_for_promotion: u32,
}

impl Default for PromotionConfig {
    fn default() -> Self {
        Self {
            min_pattern_age_for_promotion: 100,
            min_theory_age_for_promotion: 200,
            min_pattern_use_for_promotion: 3,
            min_theory_use_for_promotion: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frontier {
    pub items: Vec<FrontierItem>,
    pub last_full_refresh_tick: u64,
    pub dirty: bool,
    /// ADR 0052 / B3.
    pub staleness: StalenessConfig,
    /// ADR 0053 / Phase C0.
    pub promotion: PromotionConfig,
    /// ADR 0054 / Phase D0.
    pub meta_meta: MetaMetaConfig,
    /// Targets recently dispatched by `PruneLowValueObjects`.
    /// Computed during `refresh_with_episodes`; used by
    /// `refresh_stale_prune` to avoid re-proposing prune for
    /// patterns whose previous prune attempt already happened
    /// in the recent window (rate-limit fix 2026-05-11).
    pub recent_prune_targets: HashSet<String>,
}

impl Default for Frontier {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            last_full_refresh_tick: 0,
            dirty: true,
            staleness: StalenessConfig::default(),
            promotion: PromotionConfig::default(),
            meta_meta: MetaMetaConfig::default(),
            recent_prune_targets: HashSet::new(),
        }
    }
}

impl Frontier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Enumerate candidate actions from the current RSet state and
    /// sort by priority (descending). ADR 0052 / A1.
    ///
    /// Backward-compatible thin wrapper that calls
    /// `refresh_with_episodes` with an empty slice. Used by
    /// existing tests + callers that don't have a memory ref.
    /// Drive-driven candidate priority will skip
    /// learning-progress weighting (ADR 0080) in this code path.
    pub fn refresh(&mut self, rset: &RSet, tick: u64) {
        let empty: VecDeque<Episode> = VecDeque::new();
        self.refresh_with_episodes(rset, tick, &empty);
    }

    /// Like `refresh` but consults episode log for ADR 0080
    /// learning-progress weighting on drive-driven candidates.
    /// Runtime should call this variant; tests and pre-0080
    /// callers can stay on `refresh`.
    pub fn refresh_with_episodes(
        &mut self,
        rset: &RSet,
        tick: u64,
        episodes: &VecDeque<Episode>,
    ) {
        let mut items: Vec<FrontierItem> = Vec::new();

        // 1. TheoryCandidate: propose if discover_theory yields a
        //    nonempty member set AND no existing theory has exactly
        //    that member set.
        let cfg = AxiomDiscoveryConfig::default();
        let th = rset.discover_theory(&cfg);
        if !th.member_axiom_ids.is_empty() {
            let want: HashSet<&str> = th
                .member_axiom_ids
                .iter()
                .map(|s| s.as_str())
                .collect();
            let already_named = rset.theories().iter().any(|t| {
                let members: HashSet<&str> =
                    rset.theory_axioms(t).into_iter().collect();
                members == want
            });
            if !already_named {
                let value = (th.member_axiom_ids.len() * 2) as f64;
                items.push(FrontierItem {
                    id: format!("theory_cand_{}", tick),
                    kind: FrontierKind::TheoryCandidate,
                    target: FrontierTarget::WholeRSet,
                    priority: value / 1.0,
                    estimated_value: value,
                    estimated_cost: 1.0,
                    novelty_score: value,
                    first_seen_tick: tick,
                    last_visited_tick: None,
                    revisit_count: 0,
                    cooldown_until_tick: None,
                    status: FrontierStatus::Fresh,
                });
            }
        }

        // 2. PatternCandidate: for each k in [2, 3, 4, 5], if there
        //    are at least k data edges, propose discovery at that size.
        //
        //    ADR 0075 piece 2 — sizes extended from [2, 3] to
        //    [2, 3, 4, 5]. Smaller subgraphs almost never pass
        //    `is_clean_subgraph` on dense rsets (e.g., OQ#1's
        //    diamond posets) because the participants' neighbourhood
        //    induces more edges than the sample contains. Sizes 4-5
        //    cover whole connected clusters whose induced edge count
        //    matches the canonical, allowing autonomous_pass to mint
        //    successfully. The kernel audit empirically validated
        //    this — 7 patterns minted on OQ#1 across sizes 2-5.
        let meta = rset.collect_meta_ids();
        let data_edge_count = rset
            .iter()
            .filter(|r| !meta.contains(&r.x) && !meta.contains(&r.y))
            .count();
        for &size in &[2usize, 3, 4, 5] {
            if data_edge_count >= size {
                let value = (data_edge_count as f64) / (size as f64);
                items.push(FrontierItem {
                    id: format!("pattern_size_{}_{}", size, tick),
                    kind: FrontierKind::PatternCandidate,
                    target: FrontierTarget::PatternSize(size),
                    priority: value / (size as f64 + 1.0),
                    estimated_value: value,
                    estimated_cost: size as f64,
                    novelty_score: value / 2.0,
                    first_seen_tick: tick,
                    last_visited_tick: None,
                    revisit_count: 0,
                    cooldown_until_tick: None,
                    status: FrontierStatus::Fresh,
                });
            }
        }

        // 2.5. Drive-driven PatternCandidate. ADR 0079.
        //
        // When the unexplained-R drive signal has substance AND
        // the rset is mature enough that pattern minting could
        // succeed, propose one extra PatternCandidate at the
        // modal canonical's edge count (clamped to [2, 5]).
        //
        // The maturity gate (axioms ≥ 1 AND data_edges ≥ 100)
        // matches ADR 0075 piece 2 (revisited)'s gate, ensuring
        // small lifecycle-test fixtures (`diamond_poset` with
        // 9 edges + 0 axioms initially) never trigger drive-
        // driven proposals. Without the gate, drive=100% on
        // empty rsets would make this candidate dominate the
        // first dispatch and starve TheoryCandidate, breaking
        // `a1_rule_based_runs_and_sleeps`-style tests.
        //
        // Priority `modal_count * 5.0` gives drive-driven
        // candidates ~25-100 typical priority — comfortably
        // above organic PatternCandidate (~10) but well below
        // TheoryCandidate (~axiom_count * 2 ≈ 8 in tests, but
        // up to ~200 on real substrates). Theory discovery
        // still wins on fresh rsets; drive wins among pattern
        // proposals once both have matured.
        const MATURE_DATA_EDGE_FLOOR: usize = 100;
        let mature = rset.axioms().len() >= 1
            && rset.iter().count() >= MATURE_DATA_EDGE_FLOOR;
        if mature {
            let drive = rset.unexplained_drive_signal();
            if drive.has_signal() {
                if let Some(canonical) = &drive.modal_canonical {
                    let raw_size = canonical.len();
                    if raw_size >= 1 {
                        let size = raw_size.clamp(2, 5);
                        let modal_count = drive.modal_count() as f64;
                        // ADR 0080 — learning-progress gating.
                        // LP is in [0, 1]. = 1.0 when no recent DP
                        // history at this size; in [0, 1) when
                        // history exists.
                        //
                        // Below LP_THRESHOLD: don't even propose
                        // drive-driven candidate. Above:
                        // priority = modal_count * 5.0 * lp.
                        // Threshold-based skipping prevents
                        // frontier from carrying ~zero-priority
                        // items that still cost scheduler iteration.
                        // Constants centralized in agent_view (ADR 0080
                        // threshold-tuning slice 2026-05-11).
                        use super::agent_view::{LP_WINDOW, LP_DRIVE_THRESHOLD};
                        let lp = compute_learning_progress(
                            episodes, size, LP_WINDOW,
                        );
                        if lp >= LP_DRIVE_THRESHOLD {
                            items.push(FrontierItem {
                                id: format!(
                                    "drive_pattern_size_{}_{}",
                                    size, tick,
                                ),
                                kind: FrontierKind::PatternCandidate,
                                target: FrontierTarget::PatternSize(size),
                                priority: modal_count * 5.0 * lp,
                                estimated_value: modal_count * lp,
                                estimated_cost: size as f64,
                                novelty_score: modal_count,
                                first_seen_tick: tick,
                                last_visited_tick: None,
                                revisit_count: 0,
                                cooldown_until_tick: None,
                                status: FrontierStatus::Fresh,
                            });
                        }
                    }
                }
            }
        }

        // 3. LowValueObjectForPrune: every named object with
        //    counterfactual value < 0.
        //
        // Two corrections (2026-05-11 fix, follow-up to
        // adr0080_lp_threshold_tuning):
        //
        // (a) `rank_by_counterfactual` returns patterns + theories
        //     + extension_edges (per lib.rs:5086). The original
        //     frontier wrapped ALL as `FrontierTarget::Pattern(id)`,
        //     but `PruneLowValueObjects`'s Pattern(id) handler only
        //     calls `retract_pattern`. For theories and extension
        //     edges the retraction silently fails — the rset
        //     doesn't change but the episode still counts.
        //     Result observed in 3k OQ#2 LP-tuned run: 1000 prune
        //     episodes between tick 2000-3000 with pats unchanged
        //     at 9. Theories and extension edges with cv<0 were
        //     being targeted indefinitely.
        //
        //     Fix: route to FrontierTarget::Theory(id) when the id
        //     is a theory. Skip extension edges entirely — they
        //     have no dedicated FrontierTarget variant and the
        //     action handler's WholeRSet branch handles them, but
        //     proposing them individually wastes scheduler cycles.
        //
        // (b) Recently-pruned id filter: even with correct routing,
        //     if retract returns Err (e.g., counterfactual value
        //     re-flips negative between mints), the same id keeps
        //     being proposed. Skip ids that were targeted by Prune
        //     in the last `RECENT_PRUNE_WINDOW` episodes.
        const RECENT_PRUNE_WINDOW: usize = 50;
        let recent_prune_targets: HashSet<String> = {
            let start = episodes.len().saturating_sub(RECENT_PRUNE_WINDOW);
            episodes
                .iter()
                .skip(start)
                .filter(|ep| matches!(
                    ep.action_kind,
                    ActionKind::PruneLowValueObjects,
                ))
                .filter_map(|ep| match &ep.target {
                    FrontierTarget::Pattern(id) => Some(id.clone()),
                    FrontierTarget::Theory(id) => Some(id.clone()),
                    _ => None,
                })
                .collect()
        };
        // Cache recent prune targets on `self` so `refresh_stale_prune`
        // (called immediately after this) can apply the same filter.
        self.recent_prune_targets = recent_prune_targets.clone();
        let pattern_ids: HashSet<&str> =
            rset.patterns().into_iter().collect();
        let theory_ids: HashSet<&str> =
            rset.theories().into_iter().collect();
        for (id, cv) in rset.rank_by_counterfactual() {
            if cv >= 0.0 { continue; }
            if recent_prune_targets.contains(&id) { continue; }
            let target = if pattern_ids.contains(id.as_str()) {
                FrontierTarget::Pattern(id.clone())
            } else if theory_ids.contains(id.as_str()) {
                FrontierTarget::Theory(id.clone())
            } else {
                // Extension edge or other named object — skip.
                // PruneLowValueObjects' single-target dispatch
                // path can't handle these; they're handled by
                // the WholeRSet branch which isn't proposed by
                // this code path.
                continue;
            };
            items.push(FrontierItem {
                id: format!("prune_{}_{}", id, tick),
                kind: FrontierKind::LowValueObjectForPrune,
                target,
                priority: (-cv) * 2.0, // slight preference over
                                        // equal-value discovery
                estimated_value: -cv,
                estimated_cost: 1.0,
                novelty_score: 0.0,
                first_seen_tick: tick,
                last_visited_tick: None,
                revisit_count: 0,
                cooldown_until_tick: None,
                status: FrontierStatus::Fresh,
            });
        }

        // 4. TheoryNeedsRelations: ≥ 2 named theories AND at least one
        //    pair has no extension/independence/parallel edge between
        //    them. ADR 0052 / A2.
        let theories: Vec<String> =
            rset.theories().iter().map(|s| s.to_string()).collect();
        if theories.len() >= 2 {
            let missing_pair = (0..theories.len()).any(|i| {
                ((i + 1)..theories.len()).any(|j| {
                    !theory_pair_has_relation(rset, &theories[i], &theories[j])
                })
            });
            if missing_pair {
                items.push(FrontierItem {
                    id: format!("theory_relations_{}", tick),
                    kind: FrontierKind::TheoryNeedsRelations,
                    target: FrontierTarget::WholeRSet,
                    // Mid priority — slightly below pruning, above
                    // pattern-discovery on small graphs.
                    priority: 1.5,
                    estimated_value: 1.0,
                    estimated_cost: 1.0,
                    novelty_score: 0.5,
                    first_seen_tick: tick,
                    last_visited_tick: None,
                    revisit_count: 0,
                    cooldown_until_tick: None,
                    status: FrontierStatus::Fresh,
                });
            }
        }

        items.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        self.items = items;
        self.last_full_refresh_tick = tick;
        self.dirty = false;
    }

    /// Append `PolicyTarget` items for theories whose
    /// `recommend_intervention` returns an actionable variant. ADR 0082.
    ///
    /// Called by the runtime after `refresh_with_episodes`. Re-uses
    /// the cached `recent_prune_targets` pattern via a new
    /// `recent_policy_targets` set: skip ids that were targeted by
    /// `ApplyRecommendedIntervention` in the recent window.
    ///
    /// `None`, `ShadowMonitor`, and `Manual` recommendations are not
    /// proposed (they're no-op or not actionable). Only `FamilyDemote`,
    /// `AxiomRepair`, `TheoryDemote`, `DemoteSuperset`, and `Merge`
    /// produce frontier items.
    pub fn refresh_policy_targets(
        &mut self,
        rset: &RSet,
        prediction_state: &super::memory::PredictionState,
        episodes: &VecDeque<Episode>,
        tick: u64,
    ) {
        // Compute recent policy targets — skip ids tried in last
        // RECENT_POLICY_WINDOW episodes (mirror prune-loop fix).
        const RECENT_POLICY_WINDOW: usize = 30;
        let recent_policy_targets: HashSet<String> = {
            let start = episodes.len().saturating_sub(RECENT_POLICY_WINDOW);
            episodes
                .iter()
                .skip(start)
                .filter(|ep| matches!(
                    ep.action_kind,
                    ActionKind::ApplyRecommendedIntervention,
                ))
                .filter_map(|ep| match &ep.target {
                    FrontierTarget::Theory(id) => Some(id.clone()),
                    _ => None,
                })
                .collect()
        };

        // Compute primary_rates from prediction state for each axiom.
        // MIN_AXIOM_PREDICTIONS=5 mirrors the diagnostic example.
        const MIN_AXIOM_PREDICTIONS: u64 = 5;
        let mut primary_rates: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();
        for ax in rset.axioms() {
            if let Some(r) = prediction_state.hit_rate(ax, MIN_AXIOM_PREDICTIONS) {
                primary_rates.insert(ax.to_string(), r);
            }
        }

        // Empty substrates → cross_precision metrics in reports degrade
        // to None; recommend_intervention falls back to non-cross-
        // precision-driven paths. The runtime doesn't have generated
        // substrates readily available; this is the conservative
        // operational stance.
        let substrates: Vec<RSet> = Vec::new();
        let reports = rset.theory_quality_report_all(&substrates, &primary_rates);
        let other_reports: Vec<&crate::TheoryQualityReport> =
            reports.iter().collect();

        for report in &reports {
            if recent_policy_targets.contains(&report.theory_id) {
                continue;
            }
            // Borrow reports excluding the current focal.
            let others: Vec<crate::TheoryQualityReport> = reports
                .iter()
                .filter(|r| r.theory_id != report.theory_id)
                .cloned()
                .collect();
            let _ = other_reports; // silence
            let rec = RSet::recommend_intervention(report, &others);
            // Skip non-actionable variants.
            let actionable = matches!(
                &rec,
                crate::RecommendedIntervention::FamilyDemote { .. }
                    | crate::RecommendedIntervention::AxiomRepair { .. }
                    | crate::RecommendedIntervention::TheoryDemote { .. }
                    | crate::RecommendedIntervention::DemoteSuperset { .. }
                    | crate::RecommendedIntervention::Merge { .. },
            );
            if !actionable {
                continue;
            }
            // De-dupe: skip if an existing PolicyTarget already
            // points at this theory (idempotent within a refresh).
            let already = self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::PolicyTarget)
                    && it.target == FrontierTarget::Theory(report.theory_id.clone())
            });
            if already { continue; }
            self.items.push(FrontierItem {
                id: format!("policy_{}_{}", report.theory_id, tick),
                kind: FrontierKind::PolicyTarget,
                target: FrontierTarget::Theory(report.theory_id.clone()),
                // Mid-low priority — sits between TheoryNeedsRelations
                // (1.5) and large-graph PatternCandidate. Policy
                // interventions are corrective, not exploratory; they
                // should fire when no fresh discovery work dominates.
                priority: 1.2,
                estimated_value: 1.0,
                estimated_cost: 1.0,
                novelty_score: 0.0,
                first_seen_tick: tick,
                last_visited_tick: None,
                revisit_count: 0,
                cooldown_until_tick: None,
                status: FrontierStatus::Fresh,
            });
        }
        // Re-sort items so new PolicyTargets settle into priority
        // order alongside everything else proposed above.
        self.items.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    /// Append `PatternPolicyTarget` items for patterns whose
    /// `recommend_pattern_intervention` returns `PatternRetract`.
    /// ADR 0083 — mirror of `refresh_policy_targets` for patterns.
    ///
    /// Empty substrates → cross_substrate_match_count = None →
    /// only the Anomalous-class path (instance_count == 1) triggers
    /// PatternRetract. The other actionable variant (PatternMergeWith)
    /// has no executable lib API and is skipped.
    pub fn refresh_pattern_policy_targets(
        &mut self,
        rset: &RSet,
        episodes: &VecDeque<Episode>,
        tick: u64,
    ) {
        const RECENT_PATTERN_POLICY_WINDOW: usize = 30;
        let recent_targets: HashSet<String> = {
            let start = episodes.len()
                .saturating_sub(RECENT_PATTERN_POLICY_WINDOW);
            episodes.iter().skip(start)
                .filter(|ep| matches!(
                    ep.action_kind,
                    ActionKind::ApplyRecommendedPatternIntervention,
                ))
                .filter_map(|ep| match &ep.target {
                    FrontierTarget::Pattern(id) => Some(id.clone()),
                    _ => None,
                })
                .collect()
        };
        let substrates: Vec<RSet> = Vec::new();
        let reports = rset.pattern_quality_report_all(&substrates, None);
        for report in &reports {
            if recent_targets.contains(&report.pattern_id) {
                continue;
            }
            let others: Vec<crate::PatternQualityReport> = reports
                .iter()
                .filter(|r| r.pattern_id != report.pattern_id)
                .cloned()
                .collect();
            let rec = RSet::recommend_pattern_intervention(report, &others);
            // Only PatternRetract is actionable; PatternMergeWith has
            // no merge_patterns lib API and stays advisory.
            let actionable = matches!(
                rec,
                crate::RecommendedPatternIntervention::PatternRetract { .. },
            );
            if !actionable { continue; }
            let already = self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::PatternPolicyTarget)
                    && it.target == FrontierTarget::Pattern(report.pattern_id.clone())
            });
            if already { continue; }
            self.items.push(FrontierItem {
                id: format!("pattern_policy_{}_{}",
                            report.pattern_id, tick),
                kind: FrontierKind::PatternPolicyTarget,
                target: FrontierTarget::Pattern(report.pattern_id.clone()),
                // Priority 1.1: slightly below theory PolicyTarget
                // (1.2) so theory consolidation precedes pattern
                // consolidation when both pending.
                priority: 1.1,
                estimated_value: 1.0,
                estimated_cost: 1.0,
                novelty_score: 0.0,
                first_seen_tick: tick,
                last_visited_tick: None,
                revisit_count: 0,
                cooldown_until_tick: None,
                status: FrontierStatus::Fresh,
            });
        }
        self.items.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    /// Append `LowValueObjectForPrune` items for named patterns whose
    /// `last_improved_tick` is too stale relative to `tick`. Idempotent
    /// against repeat calls in the same tick (skips targets that
    /// already have a Prune item) and re-sorts items by priority on
    /// exit. ADR 0052 / B3.
    pub fn refresh_stale_prune(
        &mut self,
        history: &ObjectHistoryStore,
        tick: u64,
    ) {
        let cfg = self.staleness;
        let mut added = false;
        for (id, h) in &history.patterns {
            let age = tick.saturating_sub(h.first_seen_tick);
            if age < cfg.min_pattern_age_for_staleness {
                continue;
            }
            let stale_since = match h.last_improved_tick {
                Some(t) => tick.saturating_sub(t),
                None => age,
            };
            if stale_since < cfg.max_pattern_staleness_ticks {
                continue;
            }
            let target = FrontierTarget::Pattern(id.clone());
            let already = self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::LowValueObjectForPrune)
                    && it.target == target
            });
            if already {
                continue;
            }
            // Skip if a prune of this pattern already happened in
            // the recent window (2026-05-11 fix to stop the
            // per-tick prune loop observed in long-horizon runs
            // after ADR 0080 LP gate closes).
            if self.recent_prune_targets.contains(id) {
                continue;
            }
            self.items.push(FrontierItem {
                id: format!("prune_stale_{}_{}", id, tick),
                kind: FrontierKind::LowValueObjectForPrune,
                target,
                // Below the typical negative-cv prune priority
                // (≈ -cv * 2.0, normally ≥ 1.0). Staleness is a
                // softer signal so it should not preempt a
                // counterfactually-bad object.
                priority: 0.5,
                estimated_value: 0.5,
                estimated_cost: 1.0,
                novelty_score: 0.0,
                first_seen_tick: tick,
                last_visited_tick: None,
                revisit_count: 0,
                cooldown_until_tick: None,
                status: FrontierStatus::Fresh,
            });
            added = true;
        }
        if added {
            self.items.sort_by(|a, b| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
    }

    /// Append `EstablishedPromotion` items for named patterns and
    /// theories that meet the C0/C1 gate: alive for ≥ the relevant
    /// age threshold AND `last_improved_tick.is_some()` (M ≥ 1) AND
    /// not yet promoted. Skips ids that already have a pending
    /// promotion item. Re-sorts on exit.
    /// ADR 0053 / Phase C0 (patterns) + C1 (theories).
    pub fn refresh_established_promotions(
        &mut self,
        rset: &RSet,
        history: &ObjectHistoryStore,
        tick: u64,
    ) {
        let cfg = self.promotion;
        let mut added = false;

        // Patterns (C0).
        let named_patterns: HashSet<&str> =
            rset.patterns().into_iter().collect();
        for (id, h) in &history.patterns {
            if !named_patterns.contains(id.as_str()) {
                continue;
            }
            if !Self::passes_promotion_gate(
                h,
                tick,
                cfg.min_pattern_age_for_promotion,
                cfg.min_pattern_use_for_promotion,
            ) {
                continue;
            }
            if rset.contains(&R::new(id.clone(), ESTABLISHED_MARKER)) {
                continue;
            }
            let target = FrontierTarget::Pattern(id.clone());
            if self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::EstablishedPromotion)
                    && it.target == target
            }) {
                continue;
            }
            self.items.push(Self::make_promotion_item(
                id, target, tick,
            ));
            added = true;
        }

        // Theories (C1).
        let named_theories: HashSet<&str> =
            rset.theories().into_iter().collect();
        for (id, h) in &history.theories {
            if !named_theories.contains(id.as_str()) {
                continue;
            }
            if !Self::passes_promotion_gate(
                h,
                tick,
                cfg.min_theory_age_for_promotion,
                cfg.min_theory_use_for_promotion,
            ) {
                continue;
            }
            if rset.contains(&R::new(id.clone(), ESTABLISHED_MARKER)) {
                continue;
            }
            let target = FrontierTarget::Theory(id.clone());
            if self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::EstablishedPromotion)
                    && it.target == target
            }) {
                continue;
            }
            self.items.push(Self::make_promotion_item(
                id, target, tick,
            ));
            added = true;
        }

        if added {
            self.items.sort_by(|a, b| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
    }

    fn passes_promotion_gate(
        h: &ObjectHistory,
        tick: u64,
        min_age: u64,
        min_use: u32,
    ) -> bool {
        let age = tick.saturating_sub(h.first_seen_tick);
        age >= min_age && h.times_contributed_positive >= min_use
    }

    fn make_promotion_item(
        id: &str,
        target: FrontierTarget,
        tick: u64,
    ) -> FrontierItem {
        FrontierItem {
            id: format!("promote_{}_{}", id, tick),
            kind: FrontierKind::EstablishedPromotion,
            target,
            // Mid-tier consolidate priority: above stale-prune
            // (0.5) so a freshly-mature object is acknowledged
            // before stale ones are trimmed, but below normal
            // negative-cv prune so a known-bad object still wins.
            priority: 1.5,
            estimated_value: 1.0,
            estimated_cost: 1.0,
            novelty_score: 0.5,
            first_seen_tick: tick,
            last_visited_tick: None,
            revisit_count: 0,
            cooldown_until_tick: None,
            status: FrontierStatus::Fresh,
        }
    }

    /// Append a single `MetaMetaCandidate` item if the rset carries
    /// at least `meta_meta.min_m1_edges_for_meta_meta` M1-marker
    /// edges and no MetaMetaCandidate is already pending. The
    /// runtime executes this through `DiscoverMetaMetaPatterns`,
    /// which calls `RSet::discover_motifs_with_meta_subset` over a
    /// view that contains data + edges anchored to the markers.
    /// ADR 0054 / Phase D0.
    pub fn refresh_meta_meta_candidates(
        &mut self,
        rset: &RSet,
        tick: u64,
    ) {
        if self.items.iter().any(|it| {
            matches!(it.kind, FrontierKind::MetaMetaCandidate)
        }) {
            return;
        }
        let cfg = &self.meta_meta;
        let m1_edge_count: usize = cfg
            .markers
            .iter()
            .map(|m| rset.right_of(*m).len())
            .sum();
        if m1_edge_count < cfg.min_m1_edges_for_meta_meta {
            return;
        }
        // Conservative priority: above pattern-discovery floor but
        // below TheoryCandidate when a useful theory is in play.
        // Meta-meta is exploratory; let it lose ties.
        self.items.push(FrontierItem {
            id: format!("meta_meta_{}", tick),
            kind: FrontierKind::MetaMetaCandidate,
            target: FrontierTarget::WholeRSet,
            priority: 1.0,
            estimated_value: m1_edge_count as f64,
            estimated_cost: cfg.target_size as f64,
            novelty_score: 1.0,
            first_seen_tick: tick,
            last_visited_tick: None,
            revisit_count: 0,
            cooldown_until_tick: None,
            status: FrontierStatus::Fresh,
        });
        self.items.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    /// Append a `CompositeCandidate` item for each promoted
    /// action-sequence pair where BOTH the prefix and suffix
    /// `ActionKind`s have at least one matching frontier item to
    /// dispatch through. ADR 0061 / Phase H1.2.
    ///
    /// Idempotent: skips seq_ids already represented in the
    /// frontier. Re-sorts on exit. Priority is conservatively set
    /// to mid-tier (1.5) — above stale-prune (0.5), below typical
    /// negative-cv prune. The H1.1 priority bias still applies on
    /// top via `pick_top_biased`.
    pub fn refresh_composite_candidates(
        &mut self,
        rset: &RSet,
        tick: u64,
    ) {
        let pairs = rset.action_sequence_pairs();
        let triples = rset.action_sequence_triples();
        if pairs.is_empty() && triples.is_empty() {
            return;
        }
        // ADR 0062 retrospective #3 — EP is dispatched outside the
        // frontier (zero-streak anti-stagnation path), so no
        // FrontierKind maps to it. Treat it as always-present here
        // to permit composite eligibility for EP-containing pairs;
        // the scheduler's own EP gating still controls when the
        // synthetic step actually fires.
        let mut kinds_present: HashSet<ActionKind> = self
            .items
            .iter()
            .map(|it| RuleBasedScheduler::execute_for_kind(it.kind))
            .collect();
        kinds_present.insert(ActionKind::EvaluatePredictions);
        let mut added = false;
        let try_add = |seq_id: &str,
                           kinds: Vec<ActionKind>,
                           items: &mut Vec<FrontierItem>,
                           added: &mut bool| {
            if kinds.iter().any(|k| !kinds_present.contains(k)) {
                return;
            }
            let target =
                FrontierTarget::ActionSequence(seq_id.to_string());
            if items.iter().any(|it| {
                matches!(it.kind, FrontierKind::CompositeCandidate)
                    && it.target == target
            }) {
                return;
            }
            items.push(FrontierItem {
                id: format!("composite_{}_{}", seq_id, tick),
                kind: FrontierKind::CompositeCandidate,
                target,
                priority: 1.5,
                estimated_value: 1.0,
                estimated_cost: kinds.len() as f64,
                novelty_score: 0.5,
                first_seen_tick: tick,
                last_visited_tick: None,
                revisit_count: 0,
                cooldown_until_tick: None,
                status: FrontierStatus::Fresh,
            });
            *added = true;
        };
        for (seq_id, prefix_name, suffix_name) in pairs {
            let kinds_opt: Option<Vec<ActionKind>> = (|| {
                Some(vec![
                    parse_action_kind(&prefix_name).ok()?,
                    parse_action_kind(&suffix_name).ok()?,
                ])
            })();
            if let Some(ks) = kinds_opt {
                try_add(&seq_id, ks, &mut self.items, &mut added);
            }
        }
        for (seq_id, a_name, b_name, c_name) in triples {
            let kinds_opt: Option<Vec<ActionKind>> = (|| {
                Some(vec![
                    parse_action_kind(&a_name).ok()?,
                    parse_action_kind(&b_name).ok()?,
                    parse_action_kind(&c_name).ok()?,
                ])
            })();
            if let Some(ks) = kinds_opt {
                try_add(&seq_id, ks, &mut self.items, &mut added);
            }
        }
        if added {
            self.items.sort_by(|a, b| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
    }

    /// Append a `ShapeFamilyDiscoveryCandidate` item if the rset has
    /// at least 2 registered template axioms whose canonicalized
    /// premise key is shared but no `shape_premise_*` family
    /// containing them yet exists. ADR 0068 / Phase Beta-1.5.1
    /// (B.5.1).
    ///
    /// Cheap freshness check: bucket axioms by premise key, surface
    /// the candidate iff any bucket of size ≥ 2 has no corresponding
    /// shape_premise_<...> family registered. Avoids
    /// re-discovering already-named families. Item priority kept
    /// low (1.0) — discovery is cheap, but should not steal from
    /// theory/pattern work.
    pub fn refresh_shape_family_candidates(
        &mut self,
        rset: &RSet,
        tick: u64,
    ) {
        // Already an item? Skip.
        if self.items.iter().any(|it| {
            it.kind == FrontierKind::ShapeFamilyDiscoveryCandidate
        }) {
            return;
        }
        // Bucket template axioms by canonicalized premise key.
        use std::collections::BTreeMap;
        let mut buckets: BTreeMap<Vec<(usize, usize)>, usize> =
            BTreeMap::new();
        for ax_id in rset.axioms() {
            if let Some(template) =
                crate::axiom_id_to_template(ax_id)
            {
                let canon =
                    crate::canonicalize_template(template);
                let mut key: Vec<(usize, usize)> = canon
                    .premise
                    .iter()
                    .map(|e| (e.x_var, e.y_var))
                    .collect();
                key.sort();
                if key.is_empty() {
                    continue;
                }
                *buckets.entry(key).or_insert(0) += 1;
            }
        }
        // Find any bucket with ≥ 2 axioms whose canonical
        // shape_premise_<...> id is NOT yet registered.
        let mut needs_discovery = false;
        for (key, count) in &buckets {
            if *count < 2 {
                continue;
            }
            let key_str: Vec<String> = key
                .iter()
                .map(|(x, y)| format!("p{}-{}", x, y))
                .collect();
            let shape_id = format!("shape_premise_{}", key_str.join("_"));
            if !rset.is_axiom_shape_family(&shape_id) {
                needs_discovery = true;
                break;
            }
        }
        if !needs_discovery {
            return;
        }
        self.items.push(FrontierItem {
            id: format!("shape_family_{}", tick),
            kind: FrontierKind::ShapeFamilyDiscoveryCandidate,
            target: FrontierTarget::WholeRSet,
            priority: 1.0,
            estimated_value: 1.0,
            estimated_cost: 0.5,
            novelty_score: 0.5,
            first_seen_tick: tick,
            last_visited_tick: None,
            revisit_count: 0,
            cooldown_until_tick: None,
            status: FrontierStatus::Fresh,
        });
        self.items.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
    }

    /// Append `EstablishedPromotion` items for axioms that are
    /// referenced by ≥ 2 named theories AND don't yet carry the
    /// `SHARED_AXIOM_MARKER` edge. Demotion is handled by
    /// `RSet::retract_theory`'s cascade — no history is consulted
    /// because C2's gate is purely structural. Re-sorts on exit.
    /// ADR 0053 / Phase C2.
    pub fn refresh_shared_axiom_promotions(
        &mut self,
        rset: &RSet,
        tick: u64,
    ) {
        let mut added = false;
        for axiom_id in rset.axioms() {
            if rset.theories_containing(axiom_id).len() < 2 {
                continue;
            }
            if rset.contains(&R::new(axiom_id, SHARED_AXIOM_MARKER)) {
                continue;
            }
            let target = FrontierTarget::Axiom(axiom_id.to_string());
            if self.items.iter().any(|it| {
                matches!(it.kind, FrontierKind::EstablishedPromotion)
                    && it.target == target
            }) {
                continue;
            }
            self.items.push(Self::make_promotion_item(
                axiom_id, target, tick,
            ));
            added = true;
        }
        if added {
            self.items.sort_by(|a, b| {
                b.priority
                    .partial_cmp(&a.priority)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
    }
}
