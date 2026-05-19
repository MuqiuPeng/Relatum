# 0082: Recommendation execution loop

Status: Proposed (design only; implementation deferred)
Date: 2026-05-19

Parents:
- [0070 — Shape-family abstraction layer](0070-shape-family-abstraction-layer.md)
- [0071 — Unified theory-quality report](0071-unified-theory-quality-report.md)
- [0072 — Intervention policy classifier](0072-intervention-policy-classifier.md)

Reference:
- [forward-directions-2026-05-01.md §O1 — Recommendation execution loop](../forward-directions-2026-05-01.md) (this ADR is the named O1)

## Context

The consolidation triad (ADR 0070 + 0071 + 0072) produced a complete theory-maintenance vocabulary:

- **0070** named shape families as Layer-2 abstractions.
- **0071** built `TheoryQualityReport` aggregating primary-rate, cross-precision, family signatures, neighborhood.
- **0072** classified the report into a `RecommendedIntervention` enum (7 variants: None / ShadowMonitor / FamilyDemote / AxiomRepair / TheoryDemote / DemoteSuperset / Merge / Manual).

But — the runtime doesn't speak this vocabulary. As of 2026-05-19, `recommend_intervention` is only consumed by:
- diagnostics (`phase_consolidation_multi_substrate_diagnostic.rs`)
- the migration atlas
- example scripts

The forward-directions-2026-05-01 observation phrased it directly:

> The runtime has a complete theory-maintenance LANGUAGE but doesn't yet speak it to itself.

This ADR closes the loop: the scheduler periodically calls `theory_quality_report_all` + `recommend_intervention` for each theory, and dispatches the recommended action through the existing autonomous-runtime action-dispatch path.

## Decision

Add a runtime mechanism that **periodically reads recommendations and executes them**.

### Mechanism (concrete)

1. **New `ActionKind::ApplyRecommendedIntervention`** representing "consult the policy layer for theory T and execute the chosen intervention."
2. **New `FrontierKind::PolicyTarget`** wrapping a theory id whose `recommend_intervention` returned a non-trivial action (i.e., not `None` / `ShadowMonitor` / `Manual`).
3. **Frontier refresh proposes one `PolicyTarget` item per actionable theory**:
   - Call `theory_quality_report_all(rset, substrates)` to get per-theory reports.
   - For each theory T: call `recommend_intervention(report)`.
   - If result is `FamilyDemote`, `AxiomRepair`, `TheoryDemote`, `DemoteSuperset`, or `Merge`: propose `FrontierItem { kind: PolicyTarget, target: Theory(T) }`.
   - Skip `None`, `ShadowMonitor`, `Manual` (no actionable change).
4. **Scheduler routes `PolicyTarget` items via Consolidate mode** (alongside existing `LowValueObjectForPrune` / `TheoryNeedsRelations` / `EstablishedPromotion`).
5. **`execute_action` dispatches `ApplyRecommendedIntervention`**: re-computes the recommendation at execute time (state may have changed since proposal), then routes to the appropriate lib API:
   - `FamilyDemote { family_id, .. }` → `rset.retract_shape_family(&family_id)`
   - `AxiomRepair { axiom_ids }` → for each axiom: `rset.detach_axiom_from_theory(theory, axiom)`
   - `TheoryDemote { .. }` → `rset.retract_theory(theory)`
   - `DemoteSuperset { .. }` → `rset.retract_theory(theory)` (same operationally; reason differs in the recommendation log)
   - `Merge { partner_theory, .. }` → `rset.merge_theories(theory, &partner_theory)`
   - Otherwise: no-op (recommendation flipped at execute time).
6. **Cooldown to prevent thrash**: a `policy_target_cooldown_active` gate on `ActionKind::ApplyRecommendedIntervention` mirroring `pattern_cooldown_active`. If recent dispatches have ≥ `policy_min_attempts_before_cooldown` attempts and < `policy_min_success_rate` success (delta > 0), block further proposals at this kind.
7. **Recent-target filter (mirror prune-loop fix)**: `recent_policy_targets` set computed during frontier refresh; skip ids that were targeted in the recent window (e.g., last 30 episodes).

### Why a single `ApplyRecommendedIntervention` ActionKind vs N specific ActionKinds

The recommendation is a query result, not a fixed plan. State may change between proposal and dispatch (e.g., a previous tick demoted theory X; now T's recommendation would differ). Re-computing at execute time avoids stale-plan execution. A single ActionKind that internally dispatches keeps the episode log clean (all policy-driven actions show as one class) while preserving the routing fan-out internally.

Alternative considered: emit `FrontierKind::TheoryDemoteCandidate`, `FrontierKind::FamilyDemoteCandidate`, etc., with N parallel ActionKinds. Rejected because:
- Recommendation can change between propose and execute; need re-evaluation at dispatch.
- Episode log churn — N parallel `actionKind`s for a single conceptual loop blurs the picture.
- Adds maintenance burden when intervention types evolve.

The single-ActionKind design treats the recommender as a sub-dispatch decision rather than a fixed schedule.

### Why cooldown + recent-target filter together

Two failure modes were observed in adjacent work (ADR 0080 LP-tuning):
- **Frequency runaway** (LP-tuning's original bug): unbounded dispatch on saturated targets.
- **Stale-target proposal loop** (prune-loop fix): re-proposing the same id every refresh after silent-fail retract.

Both are addressed by hit-rate cooldown + recent-target filter. Mirror them here from the start.

## What this ADR does NOT do

- Does not change the recommendation logic itself (ADR 0072 stays as-is).
- Does not add new intervention types.
- Does not add per-recommendation cooldowns (e.g., specific Merge cooldown). Single global ApplyRecommendedIntervention cooldown is sufficient for v1.
- Does not introduce a confirmation step (e.g., human-in-loop approval). Recommendations execute autonomously; if that's wrong for a use case, add a guard later.

## Constitution check

- C1 (R is singular): no R changes.
- C2 (R is binary): no R changes.
- C3 (types are meta-R): no new types registered. `ApplyRecommendedIntervention` is a new ActionKind (runtime metadata); per ADR 0076 ActionKinds are observation handles, not new tokens in rset.
- C4 (identity is token-based): no token changes. The interventions (retract_*, merge_theories) are already constitution-compliant per their own ADRs.
- C5 (similarity is structural): the policy heuristics are structural (per ADR 0072); no per-token signature classification.
- Heavy reading: the recommendation is a query result computed from existing data, not a registered fact. ApplyRecommendedIntervention dispatch consumes the query.

All clean.

## Risk and mitigation

| Risk | Mitigation |
|------|------------|
| Auto-merge of wrong theories (e.g., merging signal+noise) | ADR 0072's classifier rejects `Merge` when either theory has `QualityClass::Noise`. Cooldown + recent-target filter prevent re-merging. |
| Demote cascade (T1 demoted → T2 inherits → also demoted → ...) | Frontier re-refreshes between dispatches; if cascade is genuine, that's correct behavior. If cascade is wrong, cooldown stops it after ~10 attempts. |
| Thrash between Merge and Demote on the same theory | Single ActionKind cooldown counts both as one. If thrash occurs, cooldown blocks all policy interventions on that target. |
| Policy layer returns Manual indefinitely | Manual is intentionally non-actionable (skipped at proposal). No dispatch fired; no thrash possible. |
| Performance cost of recommendation computation per refresh | Recommendation is O(theories × axioms). Currently theories ≤ 5 on OQ-style substrates; axioms ≤ 30. Cheap. Re-evaluated only when frontier.dirty. |

## Empirical predictions (testable post-implementation)

- On OQ#1: t_0 has noise family `shape_premise_p0-0_p1-2` (Round 2 finding). After O1 ships, scheduler should fire ApplyRecommendedIntervention(t_0) → FamilyDemote on that family. Subsequent run: t_0's quality improves OR t_0 is removed.
- On long5k: same expected (per C.2 cross-substrate parity).
- On OQ#2: `t_2` and `t_3` have identical cross-precision profiles (F.3 finding). After O1: ApplyRecommendedIntervention(t_2) → Merge(t_3). Theories count goes 4 → 3. F.5 already validated this is lossless.
- 800-tick OQ#1 after O1: theory count should converge to 3-4 (post-merge), with mean quality strictly improving from baseline.

If predictions fail, the policy heuristics need refinement (separate ADR), not the execution mechanism.

## Implementation plan

Estimated scope: M (300-500 lines across 4 files).

1. `src/runtime/action.rs`: add `ActionKind::ApplyRecommendedIntervention`. ~5 lines.
2. `src/runtime/frontier.rs`: add `FrontierKind::PolicyTarget` + refresh logic that proposes one PolicyTarget per actionable-recommendation theory. ~80 lines. Include `recent_policy_targets` filter (mirror prune-loop fix).
3. `src/runtime/scheduler_rule.rs`: extend `has_consolidate_work` to include `PolicyTarget`. Add `policy_target_cooldown_active` gate. Add execute_for_kind mapping. ~30 lines.
4. `src/runtime/autonomous.rs`: add execute_action arm dispatching ApplyRecommendedIntervention to recommend → match → call lib API. ~60 lines.
5. `src/runtime/persistence.rs`: round-trip serialization of new ActionKind. ~10 lines.
6. Tests:
   - Unit: each intervention dispatch path (5 paths).
   - Integration: 800-tick OQ#1 should auto-execute t_0 family demote.
   - Cooldown engagement test.
   - Stale-recommendation handling (recommendation changes between propose and execute).

Total ~300-500 LOC including tests.

## What follows once shipped

- **Long-horizon stability observation** with policy execution active. Does the system converge to a steady-state theory set, or oscillate? Currently OQ#1 settles at 4 theories diagnostic-only; with O1 active it should settle at 3 (after t_0 demote + t_2/t_3 merge).
- **Pattern-side analog**. ADR 0077 already produced `RecommendedPatternIntervention`. After O1 ships, an O1' for patterns is a natural mirror (retract / merge pattern recommendations).
- **The G-series autonomy bridge (O2)** becomes more straightforward — drive-for-creation + execute-recommendation share the same proposal/dispatch architecture.

## Verdict

O1 is the highest-leverage operationalization remaining in the consolidation triad. The mechanisms it requires already exist:
- recommend_intervention (ADR 0072)
- retract_shape_family / detach_axiom_from_theory / retract_theory / merge_theories (ADRs 0070, 0030, 0034)
- frontier proposal + scheduler dispatch + episode log (ADR 0052)
- cooldown gates (ADR 0052 / B1)
- recent-target filter pattern (2026-05-19 prune-loop fix)

What's missing is the wiring. ADR 0082 specifies the wiring without changing any subsystem semantics. Once shipped, the runtime would maintain its own theory layer end-to-end without human or example-driven intervention.

Implementation deferred to a focused session: the 4-file edit + tests + integration verification is ~half a day's work + needs careful empirical observation post-ship. Not a one-iteration autonomous slice.
