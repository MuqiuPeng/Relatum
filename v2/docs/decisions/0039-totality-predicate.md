# 0039: Totality as predicate axiom

Status: Accepted
Date: 2026-04-24

## Context

Template-layer axioms (ADRs 0027, 0028, 0036, 0037) cover
positive-implication rules with a single-edge conclusion.
Reflexivity was initially a predicate in ADR 0027 and later got
a template form via ADR 0036's empty-premise extension. Two
properties remain inexpressible in the template form:

- **Antisymmetry**: conclusion "x = y" is not an R edge.
- **Totality**: conclusion "R(x, y) ∨ R(y, x)" is a disjunction.

Antisymmetry has lived as a predicate with a reserved id
`ax_antisymmetry` since ADR 0027 / 0030. Totality has never had
an id — `discover_theory` simply never picked it up. That was an
oversight given the 8-case rigorous battery already has a total
order instance. Task 1 of the second five-step extension closes
it by adding totality as a third predicate axiom on par with
reflexivity and antisymmetry.

## Decision

### Constant and check

```rust
pub const AX_TOTALITY: &str = "ax_totality";

pub struct TotalityEvidence {
    pub unordered_pairs_checked: usize,
    pub violations: usize,
    pub holds: bool,
}

impl RSet {
    pub fn check_totality(&self) -> TotalityEvidence;
}
```

`check_totality` enumerates every unordered pair of distinct data
identifiers and verifies `R(x, y) ∨ R(y, x)`. `holds` is true iff
violations == 0 *and* at least one pair was checked (so the empty
RSet does not vacuously satisfy totality — consistent with
antisymmetry's "needs directed pairs").

### Integration

`discover_theory`: after reflexivity and antisymmetry, calls
`check_totality` and appends `AX_TOTALITY` when it holds.

`verify_axiom_holds` (used by `name_theory`): dispatches
`AX_TOTALITY` to `check_totality`.

`register_axiom_with_intension`: treats `AX_TOTALITY` as a
predicate axiom — registry-only, no intension in meta-R (same as
`AX_REFLEXIVITY` and `AX_ANTISYMMETRY`; the template language
still cannot express disjunctive conclusions).

`reconstruct_axiom_template`: returns `None` for `AX_TOTALITY`.

## Alternatives considered

- **Extend `AxiomTemplate` to admit disjunctive conclusions**.
  Rejected for this ADR — would require changes to enumeration,
  canonicalization, id codec, evaluation, reconstruction, and
  forward-chain derivation. Keeping totality as a predicate is
  the minimal change; full disjunctive-template support is a
  larger future ADR.
- **Bolt totality into existing axioms instead of a new id**.
  Rejected — violates the naming model where every predicate
  gets its own id for theory-membership encoding.

## Consequences

- `discover_theory` now recognizes total orders distinctly:
  `{trans, refl, antisym, totality}` vs. a non-total poset's
  `{trans, refl, antisym}`. Theory fingerprints become finer.
- `name_theory` can persist totality; any theory requiring
  totality now round-trips correctly through meta-R.
- The predicate-vs-template split is unchanged: three predicate
  axioms (reflexivity, antisymmetry, totality), and the template
  family. Reflexivity continues to double as a template via
  ADR 0036 opt-in; antisymmetry and totality remain predicate-only
  until the template language is extended.

## Verification

- 209 → 217 tests pass (8 new: totality holds on total order,
  fails on diamond, empty RSet not vacuous, discover_theory
  includes/omits correctly, name_theory accept/reject, predicate-
  only intension).

## Implementation

- `v2/src/lib.rs` — `AX_TOTALITY`, `TotalityEvidence`,
  `check_totality`, extensions to `discover_theory`,
  `verify_axiom_holds`, `register_axiom_with_intension`,
  `reconstruct_axiom_template`.
- `v2/docs/decisions/0039-totality-predicate.md` — this ADR.
