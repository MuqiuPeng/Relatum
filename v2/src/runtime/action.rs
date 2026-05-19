//! Action types: what the scheduler can decide to do, where it
//! should apply, and the result it returns. ADR 0052 / A1, A2; ADR
//! 0053 / Phase C0/C2; ADR 0054 / Phase D0; ADR 0059 / Phase G1.5;
//! ADR 0061 / Phase H1.2.

use super::lifecycle::RuntimeMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionKind {
    DiscoverPatterns,
    DiscoverTheory,
    PruneLowValueObjects,
    /// Scan named theories pairwise, persist any missing
    /// extension / independence / parallel edges. ADR 0052 / A2.
    UpdateTheoryRelations,
    /// Promote a named pattern (or other knowledge object) to the
    /// experience-with meta-R class by emitting the edge
    /// `R(<id>, ESTABLISHED_MARKER)`. ADR 0053 / Phase C0.
    Declarativize,
    /// Run `discover_motifs_with_meta_subset` over the rset's M1
    /// markers and the named objects they anchor. ADR 0054 / Phase D0.
    /// Reports candidates as an Episode without naming new patterns
    /// — the loop-closure naming pipeline is deferred to a follow-on
    /// slice.
    DiscoverMetaMetaPatterns,
    /// Re-run `forward_apply_all`, compare the per-axiom hit rate
    /// against the previous Reflect-time snapshot, and record an
    /// Episode whose delta = sum of per-axiom hit-rate
    /// improvements. ADR 0059 / Phase G1.5. The action does NOT
    /// mutate the rset; it produces a positive-delta-without-mutation
    /// signal that feeds `recent_positive_discovers` and resets
    /// `steps_since_last_gain`, decoupling sustained activity from
    /// mode-transition counters that the B1 thrash gate watches.
    EvaluatePredictions,
    /// Run a promoted action-sequence pair as a single dispatched
    /// unit. ADR 0061 / Phase H1.2. The dispatch reads the
    /// `(prefix, suffix)` from rset's `R(ACTION_SEQ_MARKER, seq_N)`
    /// chain (seq_id carried via `FrontierTarget::ActionSequence`),
    /// looks up matching frontier items for each step kind, and
    /// runs them in order within one episode. Episode delta = sum
    /// of abstraction-score deltas across both steps. This is the
    /// move that makes `ActionKind` no longer a compile-time
    /// constant — sequences are minted at runtime via H1.1's
    /// promotion sweep, then dispatched here.
    ExecuteComposite,
    /// Run `RSet::discover_axiom_shape_families(min_members)`. ADR
    /// 0068 / Phase Beta-1.5 (Direction B.5). Mints new
    /// `R(SHAPE_FAMILY_MARKER, shape_id)` plus member edges when ≥ N
    /// registered axioms share a structural sub-component. Episode
    /// delta is the count of newly-minted families (0 when nothing
    /// new). Pure structural derivation; no rset mutation outside
    /// the meta-R additions for the family registrations.
    DiscoverAxiomShapeFamilies,
    /// ADR 0070 Step 2 — Retract a named shape family wholesale.
    ///
    /// The target family id is carried on the `ActionPlan` via
    /// `FrontierTarget::ShapeFamily(id)`. ActionKind itself stays
    /// `Copy + Hash` so it can keep participating in
    /// `SequenceStats` HashMaps and `MetaScheduler` mutation
    /// accounting without per-key heap data.
    ///
    /// Layer-dispatched: L2 cascades to axiom-level cleanup
    /// (detach from all theories + global axiom retraction);
    /// L3+ removes the family's own meta-R without recursing into
    /// its members.
    ///
    /// Episode delta is the count of axioms globally retracted
    /// (L2) or the count of member links removed (L3+); 0 if the
    /// family is not registered.
    RetractShapeFamily,
    /// ADR 0082 — Apply the recommendation returned by
    /// `RSet::recommend_intervention` for the target theory.
    ///
    /// The target theory id is carried on `ActionPlan` via
    /// `FrontierTarget::Theory(id)`. The dispatcher re-computes
    /// the recommendation at execute time (state may have shifted
    /// since proposal), then routes to the appropriate lib API:
    ///
    /// - FamilyDemote      → rset.retract_shape_family(family_id)
    /// - AxiomRepair       → rset.retract_theory_member(theory, ax) ×N
    /// - TheoryDemote      → rset.retract_theory(theory_id)
    /// - DemoteSuperset    → rset.retract_theory(theory_id)
    /// - Merge             → rset.merge_theories(theory, partner)
    /// - None/Shadow/Manual → no-op
    ///
    /// Episode delta is the abstraction-score change from before
    /// to after; if no mutation happened (no-op variants),
    /// delta = 0.0. Cooldown + recent-target filter prevent
    /// re-targeting the same theory in the recent window.
    ApplyRecommendedIntervention,
    /// ADR 0083 — Pattern-side mirror of ADR 0082.
    ///
    /// Target carries a pattern id via `FrontierTarget::Pattern(id)`.
    /// Dispatcher re-computes `RSet::recommend_pattern_intervention`
    /// at execute time and routes to `rset.retract_pattern(pid)` for
    /// the `PatternRetract` variant; other variants (None / Shadow /
    /// PatternMergeWith / Manual) no-op.
    ///
    /// Episode delta = abs(pattern_count_before - pattern_count_after).
    ApplyRecommendedPatternIntervention,
}

/// Where (in the RSet) the action should apply. ADR 0052 / A1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontierTarget {
    WholeRSet,
    PatternSize(usize),
    Pattern(String),
    Theory(String),
    /// ADR 0053 / Phase C2. Used by `Declarativize` when the target
    /// is a named axiom (e.g., for `SHARED_AXIOM_MARKER` promotion).
    Axiom(String),
    /// ADR 0061 / Phase H1.2. Used by `ExecuteComposite` to carry
    /// the `seq_N` id of a promoted action-sequence pair.
    ActionSequence(String),
    /// ADR 0070 Step 2. Used by `RetractShapeFamily` to carry the
    /// family id (L2/L3/L4) being retracted.
    ShapeFamily(String),
}

#[derive(Debug, Clone)]
pub struct ActionPlan {
    pub action_kind: ActionKind,
    pub target: FrontierTarget,
}

#[derive(Debug, Clone)]
pub enum SchedulerDecision {
    Execute(ActionPlan),
    SwitchMode(RuntimeMode),
    Sleep,
    Stop,
}
