# Phase 0072-B — MERGE_QUALITY_FLOOR threshold sensitivity scan

**Status**: ✓ done (2026-05-01); validates Addendum 3's hand-picked 0.70
**Log**: [`logs/2026-05-01_phase_0072_b_threshold_scan.log`](../../logs/2026-05-01_phase_0072_b_threshold_scan.log)
**Example**: [`examples/phase_0072_b_threshold_scan.rs`](../../examples/phase_0072_b_threshold_scan.rs)
**Predecessors**: [`phase_0072_a_intervention_ablation.md`](phase_0072_a_intervention_ablation.md), [`phase_0072_a_long5k_ablation.md`](phase_0072_a_long5k_ablation.md)
**ADRs touched**: [0072](../decisions/0072-intervention-policy-classifier.md) Addendum 3

## Goal

Addendum 3 sets `MERGE_QUALITY_FLOOR = 0.70` by hand. The
ablations on OQ#1 and long5k confirmed the rule's direction
(Complementary merges of Mixed theories pollute Signal partners),
but neither tested whether **0.70 is the right cutoff** — only
that it BLOCKs the one known-harmful case (t_1, OQ#1).

This slice scans candidate floors ∈ {0.55, 0.60, 0.65, 0.70,
0.75, 0.80, 0.85, 0.90} statically: for each Mixed theory, does
it pass the floor gate? Cross-references against Phase 0072-A's
empirical oracle.

## Method (static)

1. Run OQ#1 to Phase 0 maturity (1000 ticks).
2. Build `TheoryQualityReport` for every theory.
3. Identify Mixed-class focal theories (the ones whose merge
   recommendations pass through Step 5's quality gate).
4. For each (focal, floor) pair, evaluate
   `focal_primary >= floor AND focal_cross >= floor` →
   BLOCK / ALLOW.
5. Cross-reference against Phase 0072-A oracle: t_1 ALLOW =
   -0.0907 cross_min regression.

## Result

### Per-theory quality summary (OQ#1, 1000 ticks)

```
theory    class     p_mean     c_mean
t_0       Mixed     0.3759     0.6835
t_1       Mixed     0.5863     0.8354
t_2      Signal     1.0000     1.0000
t_3      Signal     0.9144     1.0000
```

Only t_0 and t_1 are Mixed and therefore subject to Step 5's
quality gate.

### Active (focal, partner) pairs reaching Step 5

```
(t_0, t_2) jaccard=0.3333
(t_0, t_3) jaccard=0.5000
(t_1, t_2) jaccard=0.4000
```

(t_1, t_3) had jaccard=0.6000 > 0.50 so does not reach Step 5
under Addendum 2.

### BLOCK/ALLOW table

```
focal      p_mean     c_mean  ≥0.55  ≥0.60  ≥0.65  ≥0.70  ≥0.75  ≥0.80  ≥0.85  ≥0.90
t_0        0.3759     0.6835    B      B      B      B      B      B      B      B
t_1        0.5863     0.8354    A      B      B      B      B      B      B      B
```

t_1 is the only theory whose ALLOW/BLOCK status changes within
the candidate range. Switch point: **floor > 0.5863**.

### Decision tree caveat

t_0 appears in this scan because it satisfies "Mixed +
near-disjoint Signal partner" structurally. But on the live
decision tree, t_0 hits Step 4 (`FamilyDemote` on its noise
family `shape_premise_p0-0_p1-2`) **before** Step 5 is reached.
So in practice t_0 never visits the floor gate. The scan reports
its hypothetical Step-5 behaviour for completeness; the operative
constraint is t_1's switch point.

## Verdict

**0.70 is within the empirically safe band.**

- **Lower bound**: 0.5863 (the floor MUST exceed this to BLOCK
  the empirically validated pollution case from t_1)
- **Upper bound**: not constrained by current data — no Mixed
  theory has been empirically demonstrated as "safe-to-merge"
  on either OQ#1 or long5k
- **Buffer**: 0.70 sits 0.114 above the lower bound, leaving
  headroom for slightly worse Mixed theories (e.g. p_mean ∈
  [0.59, 0.69]) without re-tuning

A more aggressive floor like 0.60 would still BLOCK t_1, but
gives only 0.014 buffer. A floor of 0.55 would ALLOW t_1 and
re-introduce the bug. A floor of 0.85+ would over-tighten if any
Mixed theory in the (0.70, 0.85) range turned out to be merge-
safe.

## What this slice produced

1. **Empirical lower bound: 0.5863** — derived from t_1's quality
   profile and Phase 0072-A's ablation oracle.
2. **Confirmation that 0.70 is not arbitrary** — it sits in the
   safe band with reasonable headroom.
3. **A new methodological tool** — `phase_0072_b_threshold_scan`
   reusable for any future substrate where Mixed theories appear
   with quality profiles in the (0.55, 0.85) zone.
4. **Identification of the missing data**: the *upper* bound. We
   don't yet have an empirical "safe Mixed merge" example. Any
   future substrate that produces a Mixed theory with both
   primary and cross ≥ 0.75 AND empirically-safe merge would
   tighten the band from above.

## Next slices unlocked

- **Find a Mixed-but-safe-to-merge case** — explicitly construct
  or discover a substrate where the focal theory has p_mean
  ≈ 0.75, c_mean ≈ 0.80, and the merge with a near-disjoint
  Signal partner does NOT regress cross_min. This would set the
  upper bound on the floor.
- **Cross-substrate threshold scan** on OQ#2 / narrow_a once
  they mature into Mixed-bearing states.
- **Sensitivity analysis on Jaccard cutoff (0.50)** — analogous
  scan over near-disjoint thresholds; 0.50 was also chosen by
  hand.
- **(O1) Recommendation execution loop** — with the floor
  empirically validated, runtime auto-execution of
  recommendations is one threshold safer.

## Verdict

**Phase 0072-B confirms Addendum 3's hand-picked 0.70 is
defensible.** The empirical lower bound is 0.5863; 0.70 leaves
0.114 of headroom. Without an empirical "safe Mixed merge"
example to set the upper bound, any floor in (0.5863, 1.0] would
be technically safe; 0.70 is a reasonable conservative choice.
The scan is now reusable infrastructure for future substrates
with different Mixed-theory quality profiles.
