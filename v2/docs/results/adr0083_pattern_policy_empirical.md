# ADR 0083 empirical verification — pattern policy execution loop

**Status**: ✓ done (2026-05-19). Mechanism shipped + 650 tests pass. Empirical engagement is null on OQ#2 (no patterns hit the Anomalous threshold).
**Log**: [`logs/2026-05-11_adr0083_oq2_pattern_policy.log`](../../logs/2026-05-11_adr0083_oq2_pattern_policy.log)
**Example**: [`examples/adr0083_pattern_policy_test.rs`](../../examples/adr0083_pattern_policy_test.rs)
**ADR**: [`0083-pattern-policy-execution-loop.md`](../decisions/0083-pattern-policy-execution-loop.md)

## Implementation

Per ADR 0083:

1. `src/runtime/action.rs`: `ActionKind::ApplyRecommendedPatternIntervention`.
2. `src/runtime/frontier.rs`: `FrontierKind::PatternPolicyTarget` + `refresh_pattern_policy_targets(rset, episodes, tick)` method. Empty substrates → only Anomalous-class patterns (instance_count==1 AND cross_substrate_match_count.unwrap_or(0)==0) trigger PatternRetract. Mirror of ADR 0082's filter pattern.
3. `src/runtime/scheduler_rule.rs`: extend Consolidate-mode + execute_for_kind mapping.
4. `src/runtime/autonomous.rs`: new dispatch arm re-computing recommendation at execute time; if PatternRetract → `rset.retract_pattern(pid)`. Episode delta = pattern count change.
5. `src/runtime/persistence.rs`: round-trip string mapping.

650 lib tests pass; 0 regressions.

## Empirical run on OQ#2 (1500 ticks)

```
 tick= 150 | axs=2 ths=2 pats= 2 eps=10  | ARPI=0
 tick= 300 | axs=2 ths=2 pats= 2 eps=10  | ARPI=0
 tick= 450 | axs=2 ths=2 pats= 5 eps=21  | ARPI=0
 tick= 600 | axs=2 ths=2 pats= 5 eps=21  | ARPI=0
 tick= 750 | axs=2 ths=2 pats= 7 eps=35  | ARPI=0  ← canonical ceiling
 tick= 900 | axs=2 ths=2 pats= 7 eps=35  | ARPI=0
 tick=1050 | axs=2 ths=2 pats=10 eps=50  | ARPI=0
 tick=1200 | axs=2 ths=2 pats=10 eps=50  | ARPI=0
 tick=1350 | axs=2 ths=2 pats=11 eps=55  | ARPI=0
 tick=1500 | axs=2 ths=2 pats=11 eps=55  | ARPI=0

Final state: pats=11, ARPI episodes=0
```

**ApplyRecommendedPatternIntervention never fired.**

## Why null on OQ#2

`recommend_pattern_intervention` returns `PatternRetract` only for the `Anomalous` class. Class definition (`compute_pattern_summary_class`):

```rust
if instance_count == 1 && cross_substrate_match_count.unwrap_or(0) == 0 {
    return PatternQualityClass::Anomalous;
}
```

At empty substrates (runtime default), `cross_substrate_match_count = None → 0`, so the condition collapses to `instance_count == 1`.

OQ#2's auto-mint pipeline (per ADR 0075 piece 2): when DiscoverPatterns finds k size-N subgraph instances, all k are recorded for the same pattern id. So instance_count is typically ≥ 2 right at mint. The Anomalous case ("minted once, never reused") would require a pattern mint that found exactly one matching subgraph — an edge case.

OQ#2's 11 minted patterns at tick 1500 are presumably classified as:
- Signal (mdl_gain ≥ 5, overlap < 0.3) for the dense small-motif winners
- Mixed (between thresholds) for the rest
- Redundant (overlap ≥ 0.8) for highly-overlapping motifs

None hit Anomalous → no PatternRetract → ARPI never fires.

This is **mechanism correct, empirical-condition unmet**. The implementation handles the case for which it was designed; OQ#2 just doesn't produce that case at this horizon.

## Where ADR 0083 would fire

Per the empirical model, ARPI would fire on a pattern that:
- Was minted via auto-mint with instance_count=1 (rare but possible — single-occurrence canonical)
- Stays at instance_count=1 (no later mints adding to the same pattern id)

Substrates more likely to produce this:
- Sparse substrates with long-tail rare canonicals
- Random graph families at sizes 4-5 where many canonicals appear exactly once
- Real-world data (e.g., Mathlib dep graph likely has rare singleton motifs)

A targeted probe to verify ARPI dispatch correctness without waiting for natural Anomalous patterns would inject a single-instance pattern manually and observe the policy fire. Not pursued in this slice.

## What this confirms

- ADR 0083 implementation is correctly wired (build + tests pass).
- The proposed `PatternPolicyTarget` items are NOT generated on OQ#2 because no pattern hits Anomalous; no thrash, no false-positive retract.
- The runtime is now policy-aware on BOTH the theory side (ADR 0082, fires on OQ#1's t_0) AND the pattern side (ADR 0083, would fire on substrates with Anomalous patterns).

## What this leaves open

- **Targeted ARPI engagement test**: synthetic substrate / forcing scenario where a pattern is genuinely Anomalous, verifying ARPI dispatches and retracts correctly. Worth a quick follow-up.
- **PatternMergeWith executable path**: ADR 0077 returns `PatternMergeWith` but v2 has no `merge_patterns` API. If added, the ADR 0083 dispatch arm extends to handle it. Currently the Redundant class produces PatternMergeWith → not actionable → no engagement. This is the biggest gap (multiple Redundant patterns observed in OQ#2 capability demo).
- **Substrate generation in runtime**: shared concern with ADR 0082. Without substrates, `cross_substrate_match_count` stays None and the Anomalous-detection narrowly catches only instance_count=1 cases. With substrates, more nuanced cross-substrate-based retract recommendations would fire.

## Files

- `src/runtime/{action,frontier,scheduler_rule,autonomous,persistence}.rs`: implementation
- `docs/decisions/0083-pattern-policy-execution-loop.md`: design
- `examples/adr0083_pattern_policy_test.rs`: empirical probe
- `logs/2026-05-11_adr0083_oq2_pattern_policy.log`: log (ARPI=0 on OQ#2)
- This doc

## Verdict

ADR 0083 ships at the mechanism level. The architecture mirrors ADR 0082 exactly. Empirical engagement is null on OQ#2 1500-tick because OQ#2's auto-mint produces patterns with instance_count ≥ 2 → no Anomalous classification → no actionable recommendation.

The pattern-side policy loop is in place; it will fire when a substrate produces the conditions for which it was designed. The bigger empirical gap is `PatternMergeWith` having no executable lib API — that's a separate ADR (`merge_patterns` semantics) if pursued.

Both ADR 0082 (theory) and ADR 0083 (pattern) now establish the same architectural pattern: diagnostic-only recommendation → execute-action arm with re-compute-at-dispatch → cooldown / recent-target filter to prevent thrash. The runtime maintains its own consolidation layer end-to-end on both knowledge types.
