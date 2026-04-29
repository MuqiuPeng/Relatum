# F.2.1 — Quality-aware merge candidate selector

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_f21_quality_aware_merge.log`](../../logs/2026-04-29_phase_f21_quality_aware_merge.log)
**Example**: [`examples/phase_f21_quality_aware_merge.rs`](../../examples/phase_f21_quality_aware_merge.rs)

## Goal

F.2 surfaced family-signature complementarity as a useful merge selection signal but flagged a caveat: its top pick `(t_0, t_2)` would dilute high-quality t_2 with noisy t_0. F.2.1 codifies the gate: combine F.2 with a cross-precision quality floor.

## Method

1. Compute cross-precision quality per theory (column-mean, excluding self-substrate diagonal).
2. Mark theories `PASS` if quality ≥ FLOOR (0.50), `FAIL` otherwise.
3. Among pairs where BOTH sides PASS, pick highest signature complementarity (`1 - Jaccard(family_signatures)`).

## Result on OQ#1 @ 1000 ticks

Quality column (excluding diagonal):

| theory | quality | gate (≥ 0.50) |
|---|---|---|
| t_0 | 0.3248 | FAIL |
| t_1 | 0.6832 | PASS |
| t_2 | 1.0000 | PASS |
| t_3 | 1.0000 | PASS |

Pairwise table:

| pair | complementarity | a_qual | b_qual | status |
|---|---|---|---|---|
| (t_0, t_1) | 0.1667 | 0.3248 | 0.6832 | rejected |
| (t_0, t_2) | 0.6667 | 0.3248 | 1.0000 | rejected (F.2's caveat exactly) |
| (t_0, t_3) | 0.5000 | 0.3248 | 1.0000 | rejected |
| (t_1, t_2) | **0.6000** | 0.6832 | 1.0000 | **ELIGIBLE** ← best |
| (t_1, t_3) | 0.4000 | 0.6832 | 1.0000 | ELIGIBLE |
| (t_2, t_3) | 0.3333 | 1.0000 | 1.0000 | ELIGIBLE |

**F.2.1 pick: (t_1, t_2) at complementarity 0.60.**

## Verdict

**POSITIVE — F.2.1 occupies a unique slot in the merge-selector family.**

| selector | basis | OQ#1 pick |
|---|---|---|
| Alpha-3++++ Jaccard | membership overlap | (t_0, t_1) — subset+noise (wrong) |
| Alpha-5 smart | non-subset Jaccard | (t_2, t_3) |
| F.2 raw | family-signature complementarity | (t_0, t_2) — caveat: t_0 noisy |
| F.3 | cross-precision profile equivalence | (t_2, t_3) |
| **F.2.1 (this)** | **quality floor × complementarity** | **(t_1, t_2)** ← new |

F.2.1 picks distinct from all four prior selectors:
- Resolves F.2's noise-dilution caveat by gating on quality
- Selects t_1+t_2 — a "broad-mid-quality" + "narrow-high-quality" combination — different goal from Alpha-5/F.3's "pair the equivalents"

## Why this is a meaningful selector

The intuition: merge candidates should consolidate **without losing information**. Two consolidating principles work:

1. **Equivalence consolidation** (Alpha-5, F.3): merge theories with overlapping/equivalent extension. Loss = 0 because they cover the same edges.
2. **Complementarity consolidation** (F.2.1): merge theories with disjoint family signatures, both above quality floor. Loss = 0 because what one covers the other doesn't, and both are trustworthy.

F.2.1's (t_1, t_2) pick reflects principle 2: t_1 covers 5 families (broad), t_2 covers 2 (narrow but high-precision). Their structural niches are mostly complementary, AND both are quality-passing. Merging them produces a theory with broader coverage than either, no quality regression.

## What this slice produced

1. F.2.1 selector implemented as inline composition: quality gate × complementarity rank
2. Empirical: F.2.1 produces a unique pick on OQ#1 distinct from 4 prior selectors
3. Methodological: codification of "consolidation goals" — equivalence vs complementarity. Different selectors target different consolidation principles.

## Future implications

- **F.4**: combine multiple selectors (Alpha-5 ∩ F.3 ∪ F.2.1) for a portfolio-of-merges pipeline
- **F.2.1 generalized**: substitute "quality" with any per-theory metric (primary rate, cross-precision, family count). The gate × selector composition pattern is reusable.
- **Risk-aware merge**: F.2.1's 0.50 floor is conservative; a more aggressive 0.30 floor would re-admit (t_0, t_2). Quality floor is a tunable knob.
- **Ablation needed**: are F.2.1's merges actually safe to execute? Real merge testing (do the resulting theories validate post-merge?) is open.
