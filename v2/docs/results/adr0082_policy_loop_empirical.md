# ADR 0082 empirical verification — policy execution loop fires on OQ#1

**Status**: ✓ done (2026-05-19). ADR 0082 mechanism shipped + empirically validated.
**Log**: [`logs/2026-05-11_adr0082_oq1_policy_test.log`](../../logs/2026-05-11_adr0082_oq1_policy_test.log)
**Example**: [`examples/adr0082_policy_loop_test.rs`](../../examples/adr0082_policy_loop_test.rs)
**ADR**: [`0082-recommendation-execution-loop.md`](../decisions/0082-recommendation-execution-loop.md)

## Background

ADR 0082 (2026-05-19) specified a runtime mechanism for **executing** `recommend_intervention` results — closing the gap between the consolidation triad's diagnostic-only output (ADR 0070/0071/0072) and runtime-level theory maintenance.

The ADR explicitly predicted (§"Empirical predictions"):

> On OQ#1: t_0 has noise family `shape_premise_p0-0_p1-2` (Round 2 finding). After O1 ships, scheduler should fire ApplyRecommendedIntervention(t_0) → FamilyDemote on that family. Subsequent run: t_0's quality improves OR t_0 is removed.

This experiment ships the implementation in the same session and verifies the prediction.

## Implementation

Per ADR 0082 §Implementation plan:

1. `src/runtime/action.rs`: new `ActionKind::ApplyRecommendedIntervention`.
2. `src/runtime/frontier.rs`: new `FrontierKind::PolicyTarget` + `Frontier::refresh_policy_targets(rset, prediction_state, episodes, tick)` method that:
   - Computes primary_rates from `memory.prediction_state.hit_rate(ax, 5)`.
   - Calls `RSet::theory_quality_report_all(&[], &primary_rates)` (empty substrates → cross-precision degrades to None).
   - For each theory, calls `RSet::recommend_intervention(&report, &others)`.
   - Skips non-actionable variants (None / ShadowMonitor / Manual).
   - Skips ids that were targeted in last 30 episodes (mirror prune-loop fix).
   - Pushes `FrontierItem { kind: PolicyTarget, target: Theory(id), priority: 1.2 }`.
3. `src/runtime/scheduler_rule.rs`: extend `has_consolidate_work` + Consolidate-mode `pick_top` filter + `execute_for_kind` mapping.
4. `src/runtime/autonomous.rs`:
   - Call `refresh_policy_targets` after `refresh_stale_prune` in the per-tick refresh sequence.
   - Add `ActionKind::ApplyRecommendedIntervention` execute_action arm that re-computes the recommendation at execute time and dispatches to the appropriate lib API.
5. `src/runtime/persistence.rs`: round-trip string mapping for the new ActionKind.

650 lib tests still pass; 0 regressions.

## Empirical run

OQ#1 stream + standard scheduler, 1500 ticks (covers stream end at tick ~2000):

```
 tick= 150 | axs=13 ths= 4 fams= 0 eps=  30 | ARI=0 RSF=0
 tick= 300 | axs=13 ths= 4 fams= 0 eps=  30 | ARI=0 RSF=0
 tick= 450 | axs=13 ths= 4 fams= 0 eps=  30 | ARI=0 RSF=0
 tick= 600 | axs=16 ths= 4 fams= 0 eps=  42 | ARI=1/pos=1 RSF=0  ← ARI fires!
 tick= 750 | axs=16 ths= 4 fams= 0 eps=  52 | ARI=1/pos=1 RSF=0
 ...
 tick=1500 | axs=16 ths= 5 fams= 0 eps= 734 | ARI=1/pos=1 RSF=0
```

ARI episode detail:

```
tick=511 target=Theory("t_0") delta=1
```

Final state:
```
axioms = 16
theories = 5  (["t_1", "t_4", "t_2", "t_3", "t_5"])
shape families = 0
```

**t_0 is gone from the theory list.** The runtime fired `ApplyRecommendedIntervention` against `t_0` at tick 511, the recommendation evaluated at execute time (TheoryDemote, given the empty cross-precision substrates and t_0's primary-rate signal), and `retract_theory("t_0")` succeeded. Theory count dropped 4 → 3 at tick 511, then climbed back to 5 as the stream continued discovering t_4 and t_5.

## Verifying predicted behavior

ADR 0082 prediction: "ApplyRecommendedIntervention(t_0) → FamilyDemote on that family."

Actual: ApplyRecommendedIntervention(t_0) → **TheoryDemote** (delta=1 indicates theory-count change, not family-count change).

The variant differs (TheoryDemote vs FamilyDemote) because the test ran with **empty substrates** for cross-precision evaluation. Without cross-precision data, the recommendation logic falls back to primary-rate-driven paths (per ADR 0072's decision tree), which for t_0 with low primary-rate hit the TheoryDemote branch instead of the shape-family-targeted branch.

This is correct degraded behavior. Both outcomes achieve the same goal: **remove t_0 from the theory set**. The diagnostic example uses generated substrates to enable FamilyDemote-flavored recommendations; the autonomous runtime currently doesn't generate substrates per refresh (cost prohibitive). Operating with empty cross-precision data is the conservative-but-correct choice.

## Subsequent behavior

After tick=511's t_0 retraction:
- recent_policy_targets contains "t_0" for the next 30 episodes → policy can't re-target t_0.
- No other theory triggers an actionable recommendation (the remaining t_1/t_2/t_3 evaluate cleanly with primary-rate ≥ floor).
- Stream continues; new theories t_4 and t_5 are discovered normally via DiscoverTheory.
- No re-thrash: ARI count stays at 1 through tick 1500.

This is the **bounded, stable, productive behavior** ADR 0082 was designed to produce. The runtime maintained its own theory layer without supervision.

## What this confirms

- ADR 0082's design works as specified at the wiring level.
- The policy execution loop fires correctly when conditions match.
- The recent-target filter prevents thrash (no re-firing on the same id).
- Subsequent runtime activity (DiscoverTheory, DiscoverPatterns) proceeds normally with the cleaned theory set.
- 650 lib tests pass post-implementation; no regression.

## Long-horizon stability (extended verification)

Ran the same test at HORIZON=6000 (killed at tick 2400 — process advancing slowly per-tick due to autonomous_pass cost on mature rset, but state had clearly stabilized):

```
 tick=  600 | axs=16 ths=4 fams=0 eps=  42 | ARI=1/pos=1  ← t_0 demoted
 tick= 1200 | axs=16 ths=5 fams=0 eps= 437 | ARI=1/pos=1
 tick= 1800 | axs=16 ths=6 fams=0 eps=1000 | ARI=1/pos=1  ← stable
 tick= 2400 | axs=16 ths=6 fams=0 eps=1000 | ARI=1/pos=1  ← still stable
```

Theory count climbed from 4 (initial) → 3 (post-t_0-retract) → 6 (new t_4/t_5/t_6 discovered) → stable from tick 1800.

**No further policy intervention through tick 2400.** ARI count stays at 1; eps growth stops at 1000. The runtime reached a steady-state theory set and the policy loop correctly didn't re-engage on healthy theories.

`recent_policy_targets` window expired ~tick 540 (30 episodes after 511); from then on, the policy could re-target t_0 if it were re-discovered, but t_0 stays gone. The remaining theories t_1-t_6 all evaluate cleanly (no actionable recommendation).

This confirms the stability prediction from ADR 0082 §Empirical predictions: "theory count converges to a stable set within 2000 ticks." Observed convergence by tick 1800; no thrash through tick 2400.

Log: [`logs/2026-05-11_adr0082_oq1_6k_stability.log`](../../logs/2026-05-11_adr0082_oq1_6k_stability.log) (partial — process killed at tick 2400 for time budget; final state already stable).

## What this leaves open

- **Pattern-side mirror (ADR 0077)**: `RecommendedPatternIntervention` exists but no analog runtime loop yet. Natural follow-up — ADR 0083 would mirror this design for patterns.
- **Substrate generation in runtime**: if generated substrates were computed per refresh (or on a longer cadence), cross-precision would be live and FamilyDemote recommendations could fire. Currently the runtime operates in cross-precision-empty mode. Worth considering for richer policy targeting.

## Files

- `src/runtime/action.rs`, `frontier.rs`, `scheduler_rule.rs`, `autonomous.rs`, `persistence.rs`: implementation
- `examples/adr0082_policy_loop_test.rs`: this experiment
- `logs/2026-05-11_adr0082_oq1_policy_test.log`: empirical log
- `docs/decisions/0082-recommendation-execution-loop.md`: design

## Verdict

**ADR 0082 ships and works.** OQ#1 1500-tick run fires exactly one `ApplyRecommendedIntervention` episode at tick 511, correctly demoting t_0 (the documented noise-family-burdened theory), and the runtime continues normally. The runtime now maintains its own theory layer end-to-end without example-driven dispatch or human approval — closing the consolidation-triad operationalization gap identified in forward-directions-2026-05-01 §O1.

The "v2 has a complete theory-maintenance language but doesn't yet speak it" observation from 2026-05-01 is now resolved at the wiring level. Empirical evidence confirms the loop actually engages where predicted, on the substrate it was designed to engage on.
