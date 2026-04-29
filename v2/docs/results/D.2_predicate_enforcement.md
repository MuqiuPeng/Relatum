# D.2 — Predicate-axiom enforcement during substrate generation

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_d2_predicate_enforcement.log`](../../logs/2026-04-29_phase_d2_predicate_enforcement.log)

## Goal

Close the soundness gap noted in ADR 0066 Addendum 13: `RSet::generate_substrate_from_theory` was constructively applying template axioms (forward-applicable rules) but ignoring **predicate axioms** (`AX_ANTISYMMETRY`, `AX_TOTALITY`). Random seeds could violate them.

## Implementation

Two predicate-axiom enforcement steps added to `generate_substrate_from_theory`:

1. **Antisymmetry**: when theory contains `AX_ANTISYMMETRY`, restrict seeds to `i < j` only (DAG construction). Saturation under template axioms (e.g., transitivity) preserves DAG-ness, so antisymmetry holds at the post-saturation rset.

2. **Totality**: after seeding + saturation, sweep unordered pairs (i, j) and add forward edge if neither direction is present. Deterministic direction (lower-index source).

## Why DAG-restriction over post-saturation sweep

Naive approach (post-saturation removal of violations): unstable because the choice of which direction to remove can depend on visit order. Worse, removing edges from a transitively-saturated rset doesn't preserve transitivity (you'd need to re-saturate, which could re-introduce violations).

DAG restriction during seeding: structurally guarantees antisymmetry pre AND post saturation. Transitive closure of a DAG is still a DAG.

## Tests

2 new unit tests pass (547 total):
- `adr0068_d2_generate_substrate_respects_antisymmetry`: build theory with antisymmetry + transitivity; assert no (a,b) ∧ (b,a) coexist on data ids
- `adr0068_d2_generate_substrate_respects_totality`: build theory with totality + transitivity; assert every unordered pair has at least one direction

First version of the antisymmetry test FAILED on first run because the saturation step re-introduced reverse edges (3-cycle seed + transitive closure → complete digraph 3-cycle). The DAG restriction fixed it.

## Verdict

**POSITIVE**. Soundness gap closed for the two predicate axioms v2 currently uses. Generated substrates from theories containing antisymmetry are now guaranteed antisymmetric; substrates from theories containing totality are guaranteed total.

## What this slice produced

1. Two predicate-axiom enforcement steps in `generate_substrate_from_theory`
2. 2 new unit tests; 547 lib tests pass
3. Methodological note: DAG-restriction is the right shape for antisymmetry, not post-saturation sweep
4. Side benefit: total-order substrates can now be generated from theories that include totality, opening a new substrate kind for future experiments

## Future implications

- Substrates generated from total-order theories (poset + totality) are now structurally valid
- Future Beta-X may discover total-order families and use them for cross-validation
- The DAG-restriction technique generalizes: any "negative" axiom (forbidding a pattern) can be enforced by restricting the seed space to never permit the forbidden pattern
