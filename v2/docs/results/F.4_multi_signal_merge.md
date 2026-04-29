# F.4 — Multi-signal composite merge picker

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_f4_multi_signal_merge.log`](../../logs/2026-04-29_phase_f4_multi_signal_merge.log)
**Example**: [`examples/phase_f4_multi_signal_merge.rs`](../../examples/phase_f4_multi_signal_merge.rs)

## Goal

Three independent merge selectors built up over Rounds 1–2 (Alpha-5, F.3, F.2.1) each pick a top candidate from a different angle. F.4 aggregates: which pair gets agreement?

Method: rank pairs under each selector, score by Borda-style aggregation:
- +2 for top-1
- +1 for top-2
- 0 otherwise

Pair with highest aggregate = highest-confidence merge target.

## Result on OQ#1 @ 1000 ticks

### Per-selector top-1 / top-2 ranking

| selector | top-1 | top-2 |
|---|---|---|
| Alpha-5 (non-subset Jaccard, descending) | **(t_2, t_3)** at 0.40 | (t_1, t_2) at 0.286 |
| F.3 (cross-prec profile equiv, ascending max_diff) | **(t_2, t_3)** at 0.00 | (t_1, t_3) at 0.310 |
| F.2.1 (quality-gated complementarity) | **(t_1, t_2)** at 0.60 | (t_1, t_3) at 0.40 |

### Borda leaderboard

| pair | points | sources |
|---|---|---|
| **(t_2, t_3)** | **4** | Alpha-5 top-1 (+2), F.3 top-1 (+2) |
| (t_1, t_2) | 3 | F.2.1 top-1 (+2), Alpha-5 top-2 (+1) |
| (t_1, t_3) | 2 | F.3 top-2 (+1), F.2.1 top-2 (+1) |

**F.4 pick: (t_2, t_3) at 4/6 = 66.7% confidence.**

## Verdict

**STRONGLY POSITIVE — multiple independent signals concentrate on the same pair.**

Two of three selectors place `(t_2, t_3)` at top-1. The third (F.2.1) ranks it last among eligible — but t_2 and t_3 both pass the quality floor, so F.2.1 still considers them; it just prefers complementarity. This means the disagreement isn't pathological — F.2.1 is targeting a *different consolidation goal* (complementarity rather than equivalence).

So F.4 reveals two coherent merge targets:
- **(t_2, t_3) = equivalence merge** — Alpha-5 + F.3 agree. Two universal predictors with identical cross-precision profiles. Lossless consolidation.
- **(t_1, t_2) = complementarity merge** — F.2.1's pick + Alpha-5 rank 2. Distinct family signatures, both above quality floor. Coverage-broadening consolidation.

Both are defensible, but (t_2, t_3) has the strongest multi-signal evidence.

## Why this composition matters

Each individual selector is fallible:
- Alpha-3++++ Jaccard alone → (t_0, t_1), wrong (subset+noise)
- F.2 raw alone → (t_0, t_2), caveat (dilution)
- Even F.3 alone (equivalence-by-profile) might miss a useful complementarity pair

F.4 doesn't replace any selector; it weights their agreement. If 3/3 top-1's agree → very high confidence. If only 1/3 → cautious — investigate further before merging.

This is a **method-of-method**: the same convergence-of-signals reasoning F.3's result documented (Alpha-5 + F.3 → (t_2, t_3) by independent paths) is now systematized into a numerical confidence score.

## What this slice produced

1. F.4 selector — Borda aggregation of three independent selectors
2. Empirical: (t_2, t_3) wins 4/6 = 66.7% confidence on OQ#1
3. Methodological insight: disagreement among selectors can reveal *distinct consolidation goals* (equivalence vs complementarity), not necessarily contradictions
4. Re-confirmation that F.3 + Alpha-5 cleanly converge — F.4 numerically formalizes that prior observation

## Future implications

- **Tunable Borda weights**: instead of +2/+1, weight selectors by historical accuracy (which selector's picks have most often produced safe merges?)
- **Top-K instead of top-2**: with more selectors, expand to top-3/4 to capture mid-rank signals
- **Add F.4 to the runtime**: like B.5.1 wired Beta-1 into the scheduler, F.4 could be a `MergeCandidatePropose` action that fires when ≥ 2 selectors agree at top-1
- **Empirical merge testing**: F.4 picks the *candidate*; whether the merge is *safe* (post-merge cross-precision doesn't degrade) is still untested. The whole F-family selectors propose; nothing yet executes.
