# 0037: Compositional subsumption via forward chaining

Status: Accepted
Date: 2026-04-24

## Context

The rigorous-battery log for ADR 0030 noted that the equivalence
relation produced 5 minimal axioms (after ADR 0028 subsumption):
symmetry + four transitivity variants. All four variants are
consequences of {symmetry, transitivity} under composition — but
ADR 0028's subsumption only looks for direct premise-weakening
under variable remapping, not for multi-step derivation. Hence
the "not fully minimal" residue.

Task 4 of the 1→5 extension adds **compositional subsumption**:
a forward-chaining check that determines whether a target axiom's
conclusion follows from the other axioms by repeatedly applying
them as rules. This reduces the equivalence case from 5 to exactly
2 axioms — the canonical generating pair.

## Decision

### Forward-chaining derivability check

```rust
pub fn template_derivable_from(
    target: &AxiomTemplate,
    sources: &[AxiomTemplate],
) -> bool
```

Algorithm:
1. Create a fresh RSet over nodes `v_0, v_1, …, v_{n-1}` where
   `n = max(target.num_vars, any source.num_vars)`.
2. Seed with `target.premise` as R facts.
3. Iterate to fixpoint: for each source axiom, enumerate all
   bindings of its variables to the node set; when premise
   holds, add the conclusion.
4. Check: is `target.conclusion` in the final RSet?

Complexity per iteration: `Σ_source n^source.num_vars × premise_check`.
Fixpoint bounded by domain²; in practice terminates quickly for
β-scale templates.

**Soundness**: only valid if all sources hold at rate 1.0.
Callers ensure this by invoking composition only in strict mode.

### Compositional subsumption

```rust
pub fn subsume_by_composition(
    axioms: Vec<AxiomEvidence>,
) -> Vec<AxiomEvidence>
```

Iteratively: for each axiom i (processed in descending template-key
order for determinism), check if it's derivable from the remaining
(still-kept) axioms. If yes, drop it. Repeat until a full pass
drops nothing.

Order-dependence is real but non-issue: on the equivalence case's
4 transitivity variants, whichever is processed first gets dropped;
the final set is always `{symmetry, any_one_variant}`, size 2. All
4 choices generate the same full theory, so any specific one is a
valid minimal-generator choice — the system converges to a minimal
set, not a canonical set.

### Integration

```rust
pub fn discover_axioms_minimal_compositional(
    &self,
    config: &AxiomDiscoveryConfig,
) -> Vec<AxiomEvidence>
```

Runs `discover_axioms_minimal` (which uses ADR 0028's direct
subsumption) and then applies `subsume_by_composition`. Strict mode
only — defeasible sources would break the soundness argument.

`discover_axioms_minimal` itself is **unchanged**. Callers who want
the stronger reduction opt into the `_compositional` variant.

## Alternatives considered

- **Fold composition into `discover_axioms_minimal`**. Would
  change the default output shape for every caller. Rejected;
  opt-in variant preserves backward compat.
- **Symbolic derivation via resolution / SLD**. Rejected — the
  template space is small enough that forward chaining on a
  finite-domain instance is cheaper and doesn't need a unification
  engine.
- **Bounded-depth check**. Consider only derivations of depth ≤ k.
  Rejected — the fixpoint size is bounded by `n^2` edges and the
  variable domain is small (`n ≤ 3-4`), so fixpoint is cheap to
  reach.
- **Pick a canonical minimal set**. Would require lexicographic
  ordering of minimal sets. Rejected as unnecessary complexity —
  any minimal set has the same generating power; which specific
  members survive has no semantic consequence.

## Consequences

### Equivalence collapses 5 → 2

```
before (ADR 0028): {sym, trans, var_A, var_B, var_C}
after  (ADR 0037): {sym, one_transitivity_variant}
```

`adr0037_equivalence_minimal_compositional_collapses_to_two`
confirms size = 2.

### Strict poset / total order unchanged

Cases where minimal is already 1 axiom (strict poset: {trans},
total order: {trans}) have nothing to subsume. Compositional =
minimal.

### Works with higher-order theory relations

Once an equivalence theory is named with just {sym, trans}
members (plus reflexivity predicate), `name_theory_extension` can
record "strict partial order extends {trans, antisym}" cleanly —
whereas the 5-member equivalence previously would have had 4
transitivity variants obscuring the relation shape.

### Limits

1. **Strict only**. Composition under defeasible semantics needs
   probabilistic reasoning; deferred.
2. **No reflexivity shortcut**. If reflexivity holds universally,
   many conclusions are trivially true (ADR 0028 filters these);
   the composition check re-derives them but wastes cycles. A
   future optimization: pre-filter with 0028 (already done in
   `discover_axioms_minimal_compositional`), then run composition
   only on the survivors.
3. **Only drops; does not add**. If an axiom SHOULD be in the
   set but wasn't found by enumeration (e.g., its template was
   outside `max_premise_edges`), composition can't recover it.
   Composition only prunes; discovery still drives coverage.

## Verification

- 194 → 200 tests pass (6 new covering: variant derivable from
  {sym, trans}, transitivity not derivable from sym alone,
  equivalence collapses to 2, strict poset unchanged, defeasible
  passes through, singleton handling).
- Test `adr0037_equivalence_minimal_compositional_collapses_to_two`
  verifies:
  - `discover_axioms_minimal` returns 5 on equivalence
  - `discover_axioms_minimal_compositional` returns 2
  - symmetry always survives
  - exactly one transitivity-shape axiom survives

## Implementation

- `v2/src/lib.rs` — `template_derivable_from`, `forward_chain_apply`,
  `subsume_by_composition`, `discover_axioms_minimal_compositional`.
- `v2/docs/decisions/0037-compositional-subsumption.md` — this ADR.
