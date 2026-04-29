# C.2 — Cross-precision validation on long5k

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_c2_long5k_cross_precision.log`](../../logs/2026-04-29_phase_c2_long5k_cross_precision.log)
**Example**: [`examples/phase_c2_long5k_cross_precision.rs`](../../examples/phase_c2_long5k_cross_precision.rs)

## Goal

Phase Alpha-7..9 + Beta-1..5 all validated on OQ#1. Test whether the cross-precision signal and shape-family discovery generalize to a structurally distinct substrate (long5k: 5 regimes × 10 phases each, 5000-tick HORIZON).

## Method

1. Run Phase 0 (1500 ticks) on long5k stream
2. Discover theories + axioms via the autonomous loop
3. Generate substrates per theory (sorted theory ordering for determinism)
4. Compute cross-precision matrix (rows = substrate, cols = forward-applying theory)
5. Compute per-theory column means (off-diagonal)
6. Run `discover_axiom_shape_families(2)` and report family list

## Result on long5k @ 1500 ticks

State:
- 4 theories (t_0, t_1, t_2, t_3) — same count as OQ#1
- 13 axioms — same count
- 175 episodes (vs 110 on OQ#1 @ 1000 ticks; longer Phase 0 → more episodes)

Axiom counts per theory match OQ#1 byte-identically:
- t_0: 10 axioms (the noise theory)
- t_1: 6 axioms
- t_2: 3 axioms
- t_3: 4 axioms

Cross-precision matrix:
```
              t_0       t_1       t_2       t_3
   t_0    1.0000    1.0000    1.0000    1.0000
   t_1    0.6622    1.0000    1.0000    1.0000
   t_2    0.0889    0.6897    1.0000    1.0000
   t_3    0.2231    0.3600    1.0000    1.0000
```

Column means (off-diagonal):
| theory | mean | OQ#1 reference | match? |
|---|---|---|---|
| **t_0** | **0.3248** | 0.3756 | ✓ similar magnitude |
| t_1 | 0.6832 | 0.65 | ✓ similar |
| t_2 | 1.0000 | 1.0000 | ✓ exact |
| t_3 | 1.0000 | 1.0000 | ✓ exact |

Shape families (Beta-1 discovery on long5k):

| family | n members | OQ#1 match? |
|---|---|---|
| shape_premise_p0-0_p1-2 | 4 | ✓ |
| shape_premise_p0-1 | 3 | ✓ |
| shape_premise_p0-1_p1-2 | 2 | ✓ |
| shape_conclusion_c0-2 | 3 | ✓ |
| shape_conclusion_c1-0 | 2 | ✓ |
| shape_conclusion_c2-0 | 2 | ✓ |

**6 families minted. Identical set to OQ#1.**

## Verdict

**STRONGLY POSITIVE**. Three signals all generalize from OQ#1 to long5k:

1. **Theory discovery**: same 4 theories with same axiom counts
2. **Cross-precision ranking**: t_0 still the bottom by a wide margin (column mean 0.3248)
3. **Shape family discovery**: identical 6 families, same member counts

## Why the substrate-invariance

OQ#1 and long5k share regime types (diamond posets, bipartite, equivalence classes) — the regimes are scaled-up versions of each other. The axiom catalogue produced by `discover_theory` depends on what structures are present, not on stream length.

## What this slice does NOT show

- Does dream-phase signal generalize to a STRUCTURALLY DIFFERENT substrate (one with regimes OQ#1 doesn't have)? Future deferred slice (C.2.1?): construct OQ#2 with non-overlapping regimes (e.g., trees, lattices, non-equivalence partitions).
- Does the cross-precision signal break in any specific regime?

## What this slice produced

1. Cross-precision validation example for long5k
2. Empirical confirmation: OQ#1's signals generalize to long5k
3. Methodological observation: regime-similarity drives axiom-set similarity → drives signal similarity
4. 51+1 examples build, 547 lib tests pass (no API changes)
