# B.8.1 — New L3 kind lifts L5 ceiling

**Status**: ✓ done (2026-04-30)
**Log**: [`logs/2026-04-30_phase_b81_new_l3_kind.log`](../../logs/2026-04-30_phase_b81_new_l3_kind.log)
**Example**: [`examples/phase_b81_new_l3_kind.rs`](../../examples/phase_b81_new_l3_kind.rs)

## Goal

B.8 found L5 = 0 on OQ#1 because only 1 L4 super-meta exists (since only 2 L3 nested families exist with the existing premise-edge-shared kind). B.8.1 tests B.8's prediction that adding a new L3 discovery kind would lift the ceiling.

## New L3 kind: family-overlap-by-shared-member

Existing L3: "L2 families that share an individual premise edge" (e.g., both contain `p0-1`).

New L3: "L2 families that share a member axiom" (different relation — when the SAME axiom appears in two L2 families due to overlapping definitions of structural similarity).

For example, `ax_tpl_v3_p0-1_p1-2_c0-2` appears in BOTH `shape_conclusion_c0-2` (it has conclusion edge `c0-2`) AND `shape_premise_p0-1_p1-2` (it has premise edges `p0-1, p1-2`). The new L3 captures this co-membership.

## Result on OQ#1

### Layer counts

| layer | existing kind | + new kind | total |
|---|---|---|---|
| L2 | 6 | 6 | **6** |
| L3 | 2 | 6 | **8** |
| L4 | 1 | 5 | **6** |
| L5 | 0 | 8 | **8** |

### L3 produced by new kind (6 families)

```
meta_via_ax_tpl_v2_p0-1_c1-0           → {shape_premise_p0-1, shape_conclusion_c1-0}
meta_via_ax_tpl_v3_p0-0_p1-2_c0-2      → {shape_premise_p0-0_p1-2, shape_conclusion_c0-2}
meta_via_ax_tpl_v3_p0-0_p1-2_c1-0      → {shape_premise_p0-0_p1-2, shape_conclusion_c1-0}
meta_via_ax_tpl_v3_p0-0_p1-2_c2-0      → {shape_premise_p0-0_p1-2, shape_conclusion_c2-0}
meta_via_ax_tpl_v3_p0-1_p1-2_c0-2      → {shape_conclusion_c0-2, shape_premise_p0-1_p1-2}
meta_via_ax_tpl_v3_p0-1_p1-2_c2-0      → {shape_conclusion_c2-0, shape_premise_p0-1_p1-2}
```

### L5 candidates (8 of them)

```
meta_premise_p0-1                       in 2 L4s
meta_premise_p1-2                       in 2 L4s
meta_via_ax_tpl_v2_p0-1_c1-0            in 2 L4s
meta_via_ax_tpl_v3_p0-0_p1-2_c0-2       in 2 L4s
meta_via_ax_tpl_v3_p0-0_p1-2_c1-0       in 2 L4s
meta_via_ax_tpl_v3_p0-0_p1-2_c2-0       in 2 L4s
meta_via_ax_tpl_v3_p0-1_p1-2_c0-2       in 2 L4s
meta_via_ax_tpl_v3_p0-1_p1-2_c2-0       in 2 L4s
```

## Verdict

**POSITIVE — adding ONE new L3 kind lifts the L5 ceiling from 0 to 8 candidates.**

B.8's diagnosis was correct: L5 saturation on OQ#1 is a property of the L3 *vocabulary*, not of the substrate. With premise-edge-shared L3 alone, OQ#1 produces only 2 L3s; expanding to family-overlap-by-shared-member adds 6 more L3s, propagates through L4, and creates 8 L5 candidates.

This confirms B.8's option (b) — "additional L2 / L3 discovery kinds" — as the productive path.

## Why the cascade is so dramatic (0 → 8)

The new L3 kind exploits a different kind of structural overlap. Conclusion-shaped families and premise-shaped families NATURALLY overlap on individual axioms — a single axiom has both a premise structure and a conclusion structure. The existing L3 kind (premise-edge-shared) doesn't capture this cross-cutting axis.

Once cross-cutting groupings exist, the higher layers compound:
- More L3 → more chances for L4 super-metas
- More L4 → more chances for L5 super-super-metas

This is a **structural amplification**: a single new axis at L3 produces 8x growth at L5.

## What this slice produced

1. Empirical confirmation of B.8's hypothesis: L5 ceiling lifts via new discovery kinds, not just bigger substrates
2. New L3 kind specification: "L2 families that share a member axiom"
3. Quantitative cascade: 0 → 8 L5 candidates with one new axis
4. Methodological insight: lower-layer discovery vocabulary determines upper-layer ceilings; widening at the bottom is more efficient than deepening at the top

## Future implications

- **Lib implementation**: B.8.1 demonstrated the kind inline. A future slice should ship a `discover_nested_shape_families_by_shared_member` lib API to make this part of the runtime
- **Layer 6+**: with 8 L5 candidates, the recursive abstraction could potentially extend further. Audit needed on L6 / L7 prerequisites
- **More discovery axes**: shared variable arity, shared symmetry, shared validation behavior — each adds one axis. Stacking 3-4 axes might saturate the abstraction lattice on OQ#1
- **Diminishing returns?**: at some point new axes don't add information (they redundantly classify the same axioms). When does this happen? Future B.8.2.

## Methodological note

B.8 was a structural-limit finding ("L5 saturated on OQ#1"); B.8.1 is its reversal ("with one more axis, ceiling lifted"). The pair shows: structural-limit findings should be re-tested with vocabulary expansion before being treated as fundamental. v2's mechanisms have more headroom than B.8 alone suggested.
