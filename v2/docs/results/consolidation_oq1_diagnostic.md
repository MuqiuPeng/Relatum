# Consolidation triad end-to-end diagnostic on OQ#1

**Status**: ✓ done (2026-05-01)
**Log**: [`logs/2026-05-01_phase_consolidation_oq1_diagnostic.log`](../../logs/2026-05-01_phase_consolidation_oq1_diagnostic.log)
**Example**: [`examples/phase_consolidation_oq1_diagnostic.rs`](../../examples/phase_consolidation_oq1_diagnostic.rs)
**ADRs validated**: [0070](../decisions/0070-shape-family-abstraction-layer.md), [0071](../decisions/0071-unified-theory-quality-report.md), [0072](../decisions/0072-intervention-policy-classifier.md)

## Goal

Validate that the consolidation triad — shape-family layer
(0070) + unified quality report (0071) + intervention classifier
(0072) — produces sensible recommendations on REAL data, not just
synthetic-input unit tests. Specifically:

- t_0 (noisy) should target the noise family
- t_2, t_3 (universal predictors) should be left alone (None)
- t_1 (mid-quality) should land somewhere defensible
- Classifier should not be Manual on > 1 theory

## Setup

- Train OQ#1 1000 ticks → 13 axioms, 4 theories, 6 L2 families
- Generate 4 per-theory imagined substrates (NUM_GEN_IDS=15, density=0.05)
- Build `primary_rates` HashMap from `prediction_state.hit_rate(ax, 5)`
- 11 of 13 axioms had ≥ 5 predictions (the two predicate axioms
  `ax_reflexivity`/`ax_antisymmetry` don't accumulate primary
  rate via the template path)

## Reports (ADR 0071)

| theory | axs | primary mean | primary min | cross mean | cross min | noise# | signal# | summary |
|---|---|---|---|---|---|---|---|---|
| t_0 | 10 | 0.3759 | 0.1095 | 0.6835 | 0.4936 | **4** | 3 | Mixed |
| t_1 | 6 | 0.5863 | 0.4113 | 0.8354 | 0.7859 | 0 | 3 | Mixed |
| t_2 | 3 | **1.0000** | 1.0000 | **1.0000** | 1.0000 | 0 | 1 | **Signal** |
| t_3 | 4 | **0.9144** | 0.8476 | **1.0000** | 1.0000 | 0 | 3 | **Signal** |

`noise#` = count of theory axioms in Noise- or Uniform-class families.
`signal#` = count of theory axioms in Signal-class families.

### Family memberships (selected highlights)

```
t_0:
  shape_premise_p0-0_p1-2  [kind_premise_shared]  4/4 members  class=Uniform  mean=0.4936 std=0.0000
  shape_premise_p0-1       [kind_premise_shared]  1/3 members  class=Signal   mean=0.9298
  ...
  neighbors: extends=["t_1"]  parallel=["t_2", "t_3"]

t_1:
  (only signal/mixed families; no noise-class)
  neighbors: extended_by=["t_0"]  parallel=["t_2", "t_3"]

t_2: 3 axioms, all in signal/mixed families.  parallel to all others.
t_3: 4 axioms, all in signal/mixed families.  parallel to all others.
```

The classic Beta-1 finding (`shape_premise_p0-0_p1-2` is the
**variance-zero noise family**, 4 members all behave identically
under cross-validation) reproduces cleanly through the unified
report — this time exposed via `family_memberships[i].class =
Uniform` plus `family_memberships[i].quality.std = 0.0000`.

## Recommendations (ADR 0072)

| theory | recommendation |
|---|---|
| **t_0** | `FamilyDemote(shape_premise_p0-0_p1-2, Uniform)` |
| t_1 | `Manual(Mixed theory; no specific intervention pattern matched)` |
| **t_2** | `None` |
| **t_3** | `None` |

## Sanity checks

| # | check | result |
|---|---|---|
| 1 | t_2 → None (Signal-class expected) | ✓ |
| 2 | t_3 → None (Signal-class expected) | ✓ |
| 3 | t_0 → noise-targeting intervention | ✓ |
| 4 | non-Manual ≥ 3 of 4 theories | ✓ (3/4) |

**STRONGLY POSITIVE — every expected behavior holds.**

## Why each recommendation is correct

### t_0 → FamilyDemote(shape_premise_p0-0_p1-2)

The classifier's decision tree (ADR 0072 §3) walks:

- Step 0: t_0 has data → not Indeterminate
- Step 1: t_0 is Mixed → not Signal → continue
- Step 2: t_0.neighborhood.extends = [t_1]; but t_1 is Mixed (not Signal) → no DemoteSuperset
- Step 3: t_0 has a Uniform-class family (`shape_premise_p0-0_p1-2`) with 4 members in t_0 → **FamilyDemote, target this family**

This is the most precise possible intervention: it removes the
4 noise axioms wholesale + cleans up axiom registrations, leaving
t_0's 6 non-noise axioms intact. Compare to:
- TheoryDemote: would discard t_0 entirely (loses 6 good axioms)
- AxiomRepair: would detach noise axioms from t_0 only, leaving them
  registered globally as orphans
- DemoteSuperset: would only fire if t_1 were Signal-class, which it isn't

### t_1 → Manual

t_1 is Mixed (primary 0.59, cross 0.84 — the cross dimension is
strong, primary is weak-to-mid). Decision tree walks:

- Step 0-1: not Indeterminate, not Signal
- Step 2: t_1.neighborhood.extends = [] (it's the SUBSET; t_0 extends t_1) → no DemoteSuperset
- Step 3: no noise family memberships → no FamilyDemote
- Step 4: primary_mean (0.59) < 0.60 threshold → not eligible for AxiomRepair
- Step 5: t_1 has a Mixed family (`shape_conclusion_c0-2`) overlapping with t_2/t_3's family signature → no DISJOINT signal partner → no Merge
- Step 6: Mixed (not Noise) → no TheoryDemote
- Step 7: **Manual**

This is the **honest** outcome: t_1 doesn't fit any of the
specific patterns. The classifier flagging it as Manual is
correct; an aggressive heuristic would risk a wrong call. A
human reviewer can decide whether to:
- Wait for more data (primary may rise; t_1 is borderline)
- Manually merge with t_2 or t_3 (overlap in `shape_premise_p0-1_p1-2` and `shape_conclusion_c0-2`)
- Adjust the primary repair threshold from 0.60 to 0.55

This is what "Level 1.5" (per user's earlier framing) looks like
in practice: when rules don't match cleanly, surface the case for
manual review rather than guess.

### t_2 → None

t_2 is **Signal-class**: primary 1.0, cross 1.0, all axioms in
non-noise families. Decision tree halts at Step 1 immediately.
No intervention recommended.

### t_3 → None

Same path as t_2 — Signal-class, halts at Step 1.

## What this validates

1. **The full pipeline runs end-to-end**: rset → ADR 0071 reports
   → ADR 0072 recommendations. No glue code beyond what the ADRs
   specify.
2. **The classifier's priority order matters**: t_0 has BOTH
   `extends=[t_1]` AND a noise family. Step 2 (DemoteSuperset)
   correctly defers to Step 3 because t_1 isn't Signal-class.
   Without the priority guard, the classifier might recommend
   "demote t_0 because it extends t_1" — wrong, since t_1 itself
   isn't healthy enough to be the "cleaner" alternative.
3. **Per-axiom stats are correctly populated**: The ADR 0072
   schema extension (`per_axiom_stats: Vec<AxiomQualityStats>`)
   carries through `theory_quality_report` and is consumed by
   the AxiomRepair check (Step 4). No regression on the existing
   ADR 0071 aggregate fields.
4. **Manual is informational, not a failure**: t_1's Manual
   recommendation contains the diagnostic message ("Mixed theory;
   no specific intervention pattern matched") — actionable
   metadata for the operator, not just a `None`-equivalent.
5. **The variance-zero noise family is now actionable**: Beta-1
   discovered it; B.2 demoted it inline; F.1.1 named it; ADR
   0070 layered it; ADR 0071 reported it; ADR 0072 *recommends*
   targeting it. The full chain works.

## What this does NOT validate

- **Multi-substrate behavior**: only OQ#1 tested here. long5k
  and OQ#2 should produce comparable triads but aren't checked.
- **Recommendation execution**: the example reads recommendations
  but doesn't ACT on them. Verifying that executing
  `FamilyDemote(shape_premise_p0-0_p1-2)` produces the same
  outcome as B.2's manual demote is the natural next step.
- **Recommendation stability**: a single 1000-tick run. Whether
  recommendations are stable across re-runs (or across slight
  parameter perturbations) isn't tested.

## What this slice produced

1. End-to-end demonstration that the consolidation triad (0070 +
   0071 + 0072) works on OQ#1 with 0 manual integration code
2. Empirical reproduction of every prior empirical finding
   through the unified API:
   - Beta-1's variance-zero family → reproduced as `class = Uniform`
   - F.1.1's signal/noise/uniform classification → reproduced
   - Alpha-3+ / Beta-2's "demote noise then repair" intent →
     reproduced as automated FamilyDemote recommendation
3. The 9 prior examples (Alpha-3 through F.5) can now migrate
   to a 3-line snippet. (Migration deferred per ADR 0072 §6.)
4. Validation that priority order in the decision tree is
   correctly load-bearing (t_0's case shows Step 2 deferring
   to Step 3 when subset isn't Signal-class).

## Future implications

- **One example replaces nine**: a future cleanup PR can migrate
  Alpha-3, Alpha-3+, Alpha-3++++, Alpha-5, Beta-2, F.2, F.2.1,
  F.4, F.5 to this idiom. ~300 lines of inline classification
  logic become ~30 lines of report+recommendation calls.
- **Runtime maintenance loop (potential ADR 0073)**: the
  `report → recommend → execute → verify` cycle can become a
  scheduler-level meta-action. Currently the example reads but
  doesn't execute; a real runtime would add the dispatch.
- **Threshold tuning becomes ADR-gated**: the empirics here
  (primary_mean = 0.59 falls just below the 0.60 repair eligibility
  threshold) could motivate an empirical-tuning ADR that lowers
  it to 0.55. Or shows that 0.60 is correct and t_1's Manual is
  the right answer.
- **Per-substrate diagnostic battery**: this example is OQ#1.
  Running it on long5k and OQ#2 in a single battery would map
  the classifier's behavior across substrate types.

## Observation: t_1 is the interesting case

t_1 is the theory the classifier punts on as Manual. Looking at
t_1 closely:

- 6 axioms, primary mean 0.59 (just below repair threshold 0.60)
- cross mean 0.84 (Signal-grade on imagined substrates)
- no noise family memberships
- shares 2 family memberships with t_2 (`shape_conclusion_c0-2`,
  `shape_premise_p0-1_p1-2`) — not disjoint signature, so no
  complementarity merge

The interesting question: is t_1 a "good theory observation
struggles to confirm" or a "marginal theory that deserves
demote"? The classifier doesn't decide; it surfaces the case.
This is the right behavior at Level 1.5 — a future ADR could
add a "borderline" recommendation type if Manual proves
under-informative.

## Verdict

**STRONGLY POSITIVE** — consolidation triad validated end-to-end
on OQ#1. The user's strategic critique (2026-04-30) is now
empirically answered: the project IS structurally, not just
mechanically, consolidated.
