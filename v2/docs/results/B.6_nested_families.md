# B.6 — Family of families (nested abstraction)

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_beta_6_nested_families.log`](../../logs/2026-04-29_phase_beta_6_nested_families.log)
**Example**: [`examples/phase_beta_6_nested_families.rs`](../../examples/phase_beta_6_nested_families.rs)

## Goal

Beta-1 grouped axioms into shape families (Layer 1). B.6 takes the next step: group families themselves into meta-families when they share a structural sub-component (shared individual premise edge across `shape_premise_*` families). This is **recursive structural abstraction** — Layer 2 derived from Layer 1, not from raw axioms.

## Implementation

New marker `META_SHAPE_FAMILY_MARKER` and three RSet methods:
- `discover_nested_shape_families(min_member_families)` — for each premise edge that appears in ≥ N premise families, mint `meta_premise_p<x>-<y>` and link members
- `is_nested_shape_family(id) -> bool`
- `nested_shape_families() -> Vec<&str>`
- `nested_shape_family_members(meta_id) -> Vec<&str>`

3 unit tests (550 lib tests pass).

## Result on OQ#1

After Phase 0 (1000 ticks) → 6 shape families discovered (3 premise + 3 conclusion).

**Layer 2 discovers 2 nested families:**

| meta-family | members | shared edge |
|---|---|---|
| `meta_premise_p0-1` | shape_premise_p0-1, shape_premise_p0-1_p1-2 | p0-1 (R(0,1) appears in both) |
| `meta_premise_p1-2` | shape_premise_p0-0_p1-2, shape_premise_p0-1_p1-2 | p1-2 (R(1,2) appears in both) |

The third premise edge `p0-0` only appears in one family (shape_premise_p0-0_p1-2), so no meta-family for it.

## Verdict

**POSITIVE**. Recursive structural abstraction works.

Layer 1: 6 instances of `SHAPE_FAMILY_MARKER`
Layer 2: 2 instances of `META_SHAPE_FAMILY_MARKER`

Both layers discovered structurally from data, not declared.

## Significance

This is the deepest structural-vocabulary extension since H1:

| layer | what it abstracts | source |
|---|---|---|
| L0 | data edges (R) | external input |
| L1 (axioms) | rules over data | `discover_theory` |
| L1.5 (theories) | conjunctions of axioms | `name_theory` |
| **Beta-1** | axioms by shared structure (premise/conclusion) | `discover_axiom_shape_families` |
| **B.6 (this slice)** | shape families by shared structure (shared edge across keys) | `discover_nested_shape_families` |

Each layer is a structural derivation from the layer(s) below. Type instances at Layer 2 are derived from existing meta-R structure at Layer 1, not from raw axioms.

## Future implications

- Layer 3? In principle, if multiple meta-families share structure, an even higher-order abstraction is possible. Currently no meta-family pair on OQ#1 has obvious overlap, so no Layer 3 here.
- Cross-precision over meta-families: if all members of `meta_premise_p1-2` have low cross-precision (the 4 noise axioms in shape_premise_p0-0_p1-2 are uniform-low at 0.41, and the 2 axioms in shape_premise_p0-1_p1-2 are mixed)... actually their union has spread 0.41 to 1.0. The meta-family doesn't capture quality, just structure.
- B.6 demonstrates that the structural-vocabulary layer can recursively extend from data → no compile-time ceiling on abstraction depth.

## What this slice produced

1. `META_SHAPE_FAMILY_MARKER` + 4 new RSet methods
2. 3 unit tests; 550 lib tests pass
3. 2 nested families minted on OQ#1; Layer 2 ⊃ Layer 1 ⊃ axioms
4. Constitutional commitment 3 reaches deepest realization yet:
   declared types (markers) + discovered type instances at multiple
   layers, recursively
