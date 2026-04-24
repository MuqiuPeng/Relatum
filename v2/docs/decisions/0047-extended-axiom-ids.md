# 0047: Extended axiom ids — equality and disjunctive codec

Status: Accepted
Date: 2026-04-24

## Context

ADR 0044 introduced `EqualityAxiomTemplate` and
`DisjunctiveAxiomTemplate` as sibling types to `AxiomTemplate`.
They had evaluation and discovery but no deterministic id codec —
meaning `name_theory` could not accept them, and they couldn't be
registered in meta-R theories the way edge-family axioms could.

Task 1 of the 1'''→5''' round closes that gap: give both families
a deterministic id and teach `verify_axiom_holds` to dispatch them.

## Decision

### Id formats

```text
Edge (ADR 0030):          ax_tpl_v{n}_p{x}-{y}_..._c{x}-{y}
Equality (new):           ax_eq_v{n}_p{x}-{y}_..._eq{a}-{b}
Disjunctive (new):        ax_disj_v{n}_p{x}-{y}_..._d{x}-{y}_...
```

- All three share `num_vars` and sorted `p{x}-{y}` premise edges.
- Edge has one `c{x}-{y}` conclusion.
- Equality has one `eq{a}-{b}` equality pair, normalized to `(min, max)`.
- Disjunctive has ≥ 1 `d{x}-{y}` conclusion edge, sorted.

Prefix disambiguates the family: `ax_tpl_` vs `ax_eq_` vs `ax_disj_`.
Never collides.

### Codec API

```rust
pub fn equality_axiom_id(template: &EqualityAxiomTemplate) -> String;
pub fn equality_id_to_template(id: &str) -> Option<EqualityAxiomTemplate>;
pub fn disjunctive_axiom_id(template: &DisjunctiveAxiomTemplate) -> String;
pub fn disjunctive_id_to_template(id: &str) -> Option<DisjunctiveAxiomTemplate>;
```

Roundtrip-exact for canonical templates (antisymmetry, totality).

### `verify_axiom_holds` dispatch

Now tries each family in order: edge → equality → disjunctive. The
first parser that recognizes the id wins and evaluates against the
RSet. Unrecognized ids return `UnparseableAxiomId` as before. For
recognized ids, requires rate = 1.0 + at least one binding to
accept (same strict-mode rule as before).

### Consequence: three-family theories are now nameable

A caller can now write

```rust
rs.name_theory(&[
    "ax_tpl_v3_p0-1_p1-2_c0-2",   // transitivity (edge)
    "ax_eq_v2_p0-1_p1-0_eq0-1",   // antisymmetry (equality)
    "ax_disj_v2_d0-1_d1-0",       // totality (disjunctive)
])
```

and get back a single theory id with three axioms of three
different families. On a total order all three hold; the theory
persists as conventional meta-R with `R(t_N, ax_i)` membership.

## Alternatives considered

- **Unify all three id prefixes into `ax_` with a kind byte**.
  Rejected — cleaner to use distinct prefixes so code inspecting an
  id string immediately knows the family.
- **Write meta-R intension for equality / disjunctive**. Deferred
  for a future ADR — the intension encoding would need new markers
  (`__equality_pair__`, `__disjunction__`) and careful chain
  design. `verify_axiom_holds` works fine with parse-on-demand for
  now; intension is for inspection / composition / retraction,
  which are edge-family-only today.
- **Include AX_REFLEXIVITY / ANTISYMMETRY / TOTALITY parsing in
  the same dispatch**. Already handled at the top of
  `verify_axiom_holds` by name equality. No change needed.

## Consequences

### `discover_extended_axioms` is now round-trippable via names

Previously `ExtendedAxiomEvidence::Equality` and `Disjunctive`
held the template value but had no serializable id. Now, if the
caller wants to persist one as a theory member, they can obtain
the id via `equality_axiom_id` / `disjunctive_axiom_id` and call
`name_theory`.

### Still not covered (explicit limits)

- **No meta-R intension** for equality / disjunctive — axiom
  variables, premise-edge chains, and special conclusion nodes
  are not written to R. Inspecting the axiom requires parsing
  the id string rather than reading meta-R.
- **No `register_*_with_intension`** for non-edge families.
  `name_theory` adds only `R(AXIOM_MARKER, id)` registry edge
  for them.
- **No subsumption / composition** on non-edge axioms.
  `discover_axioms_minimal` and `discover_axioms_minimal_
  compositional` still filter only edge-family axioms.

These are consistent with ADR 0044's stated scope; extending them
is a separate future ADR.

## Verification

- 257 → 265 tests pass (8 new: equality roundtrip, disjunctive
  roundtrip, id-dispatch unambiguous, name_theory accept/reject
  equality on poset/equivalence, accept/reject disjunctive on
  total-order/diamond, three-family bundle in one theory).

## Implementation

- `v2/src/lib.rs` — `equality_axiom_id` / `equality_id_to_template`,
  `disjunctive_axiom_id` / `disjunctive_id_to_template`, dispatch
  update in `verify_axiom_holds`.
- `v2/docs/decisions/0047-extended-axiom-ids.md` — this ADR.
