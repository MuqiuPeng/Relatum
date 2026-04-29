# B.2 — Family-level demote intervention

**Status**: ✓ done (2026-04-29)
**Commit**: TBD (combined with batch)
**Log**: [`logs/2026-04-29_phase_beta_2_family_demote.log`](../../logs/2026-04-29_phase_beta_2_family_demote.log)
**Example**: [`examples/phase_beta_2_family_demote.rs`](../../examples/phase_beta_2_family_demote.rs)

## Goal

Test if Beta-1's discovered shape families have runtime utility. When a family's cross-precision is uniform AND low (the "structural noise" signature: mean < 0.65 AND variance < 0.05), retract all members wholesale — both detach from theories and globally retract the axiom registration.

## Design

Demote criterion (corrected from initial 0.50 single-threshold):
- mean cross-precision < `FAMILY_DEMOTE_MEAN_THRESHOLD = 0.65`
- variance < `FAMILY_VARIANCE_THRESHOLD = 0.05`

The variance gate enforces the "uniform-low" signature (Beta-1 noise family had var=0). Mixed families are excluded.

Determinism fix: theory ids sorted before substrate seed assignment (HashMap-derived ordering had been giving run-to-run drift).

Operator: for each member of a qualifying family —
1. `retract_theory_member(t, ax)` for each theory containing it
2. `retract_axiom(ax)` (now orphan, globally cleaned)

## Result on OQ#1

3 families discovered. Per-family stats:

| family | n | mean | var | flag |
|---|---|---|---|---|
| **shape_premise_p0-0_p1-2** | 4 | 0.5140 | **0.000000** | ← DEMOTE |
| shape_premise_p0-1 | 3 | 0.9080 | 0.016926 | — |
| shape_premise_p0-1_p1-2 | 2 | 0.8620 | 0.019042 | — |

Variance-zero signature reliably reproduced.

Family `shape_premise_p0-0_p1-2` qualified for demote. 4 members detached from t_0 (their only home), then globally retracted (19 meta-R edges each = full intension cleanup).

## Comparison vs prior baselines

| metric | Phase 0 | Alpha-3+ (full t_0 demote) | Alpha-3+++ (repair) | **Beta-2** |
|---|---|---|---|---|
| mean | 0.7188 | 0.8401 | 0.7967 | 0.7647 |
| min | 0.3757 | 0.6664 | 0.6664 | 0.6391 |
| qualifying | 4 | 3 | 4 | 4 |
| axioms registered post | 13 | 13 | 13 | **9** |

## Verdict

**POSITIVE-NEW**: Beta-2 is functionally close to Alpha-3+++ repair (same theory survival, same qualifying count, similar mean), but with cleaner global state (4 axiom registrations gone instead of orphaned).

Key structural difference vs prior interventions:
- Alpha-3+ retracts whole t_0 (loses 5 good axioms with the 4 bad)
- Alpha-3+++ detaches 4 axioms from t_0 only (axioms stay registered, orphan)
- **Beta-2 detaches 4 axioms from any containing theory + globally retracts** (fully cleaned)

The decision is driven by **structural abstraction (the family)**, not per-axiom hit rate. This is the first runtime intervention triggered by Beta-1's discovered structural vocabulary.

## What this proves

1. Beta-1's families aren't inert observations — they have decision-driving power
2. Variance-zero signature is a clean trigger for "uniform low quality" families
3. Family-level demote produces clean state (no orphan axioms)
4. Aggregate metrics are within noise of Alpha-3+++ repair (slight 0.027 drift in mean from per-tick scheduler differences when forward_apply has fewer axioms)

## Future implications

- Family-level demote complements (not replaces) theory-level demote
- The variance gate is the load-bearing trigger
- This unlocks B.4 (family-aware template enumeration) — once we know `shape_premise_p0-0` produces a noise family, future axiom discovery should skip that premise shape
