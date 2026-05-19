# 0083: Pattern policy execution loop

Status: Proposed (design + impl in one slice)
Date: 2026-05-19

Parents:
- [0077 — Pattern quality framework + intervention recommendations](0077-pattern-quality-and-intervention.md)
- [0082 — Recommendation execution loop](0082-recommendation-execution-loop.md) (theory-side analog)

## Context

ADR 0082 shipped runtime-level execution of `RSet::recommend_intervention` for **theories** (closing the consolidation triad's diagnostic-only gap). The pattern-side analog (`RSet::recommend_pattern_intervention`, ADR 0077) was always meant to mirror — but no runtime loop existed.

This ADR specifies the pattern mirror.

## Decision

Add `ActionKind::ApplyRecommendedPatternIntervention` and `FrontierKind::PatternPolicyTarget`. Frontier refresh proposes one item per pattern whose `recommend_pattern_intervention` returns an actionable variant. Scheduler routes via Consolidate mode. Execute_action arm re-computes the recommendation at execute time and dispatches.

### Actionable variants

Looking at `RecommendedPatternIntervention`:

| Variant | Actionable? | Lib API |
|---------|-------------|---------|
| None | no | — |
| ShadowMonitor | no | — |
| **PatternRetract** | **yes** | `rset.retract_pattern(pid)` |
| PatternMergeWith | **no — no merge_patterns API yet** | (advisory only per ADR 0077) |
| Manual | no | — |

Only `PatternRetract` is executable. `PatternMergeWith` is skipped (would need a future `merge_patterns` API; out of scope for this ADR).

### Difference from ADR 0082

ADR 0082 supports 5 actionable theory-side variants (FamilyDemote, AxiomRepair, TheoryDemote, DemoteSuperset, Merge). ADR 0083 supports only 1 (PatternRetract). The architecture is identical; the dispatch surface is narrower.

## Mechanism

1. `Frontier::refresh_pattern_policy_targets(rset, episodes, tick)`:
   - Compute `recent_pattern_policy_targets` from last 30 episodes targeting `ApplyRecommendedPatternIntervention`.
   - Call `rset.pattern_quality_report_all(&[], None)` (empty substrates).
   - For each report: call `RSet::recommend_pattern_intervention(report, others)`.
   - If result is `PatternRetract`: propose `FrontierItem { kind: PatternPolicyTarget, target: Pattern(pid), priority: 1.1 }` (slightly below theory PolicyTarget priority 1.2, so theory consolidation precedes pattern consolidation when both pending).
   - Skip `None`, `ShadowMonitor`, `PatternMergeWith`, `Manual`.
   - Skip ids in `recent_pattern_policy_targets`.

2. Scheduler: extend `has_consolidate_work` + `pick_top` filter + `execute_for_kind`.

3. `execute_action::ApplyRecommendedPatternIntervention`:
   - Re-compute the recommendation at execute time.
   - If `PatternRetract`: call `rset.retract_pattern(pid)`.
   - Else: no-op.
   - Episode delta = abs(pattern_count_before - pattern_count_after).

4. Persistence round-trip for the new ActionKind.

## Constitution check

Identical to ADR 0082 — no R changes, no token differentiation, no new meta-R registrations. Runtime metadata only.

## Risk and mitigation

| Risk | Mitigation |
|------|------------|
| Retract a pattern that's actually still being mined for instances | `recommend_pattern_intervention` only returns `PatternRetract` for `Anomalous` class (instance_count=1 AND cross_substrate_match_count=0). At empty-substrate, this means "minted once, never reused" — genuinely unproductive. |
| Auto-mint immediately re-mints the retracted pattern | If the same canonical fires `DiscoverPatterns` post-retract, it gets a fresh pattern id (per ADR 0075 piece 2). Recent_pattern_policy_targets blocks re-proposal on the OLD id; new id with new evidence has its own track. |
| Thrash on a pattern that's borderline | Single ActionKind cooldown + recent-target filter (mirror ADR 0082). |

## Empirical predictions

On OQ#1 (after t_0 demote via ADR 0082):
- Patterns currently have varied quality. After ADR 0083, any Anomalous-class pattern (instance=1, no cross-substrate evidence at empty substrates) gets retracted.
- On OQ#2 capability demo: 16 patterns minted; per Round 8 capability demo, many are classified Mixed or Redundant (per ADR 0077 `recommend_pattern_intervention`). Anomalous patterns specifically would be candidates.

This ADR's purpose isn't to dramatically reshape v2; it's to **mirror the architecture** so pattern-side policy is consistent with theory-side. Runtime should be policy-driven across BOTH knowledge types.

## Implementation note

Implementation included in this slice (not deferred), following the ADR 0082 precedent.
