# 0036: Empty-premise axiom templates (reflexivity in template form)

Status: Accepted
Date: 2026-04-24

## Context

ADR 0027's template language required every axiom to have at least
one premise edge. That made reflexivity inexpressible as a template
— "∀x. R(x, x)" has an empty premise — so it had to live as a
separate predicate check (`check_reflexivity`) outside the template
machinery. ADRs 0030, 0031, 0032 all noted this as an explicit gap.

Task 3 of the 1→5 extension closes part of it: admit empty-premise
templates with a single-variable self-loop conclusion `[] ⇒ R(0,0)`.
That's reflexivity in template form. Disjunctive conclusions
(totality) and equality conclusions (antisymmetry) remain outside
the template form and continue to live as predicates.

## Decision

### Config addition

```rust
pub struct AxiomDiscoveryConfig {
    // ... existing fields
    pub include_empty_premise: bool,   // new; default false
}
```

Default `false` preserves every ADR 0027/0028/0030/0033 behavior.
Opt-in extends the enumeration.

### Enumeration change

`enumerate_axiom_templates` now prepends a single empty-premise
template when `include_empty_premise` is true. The only admitted
conclusion shape is `R(v, v)` with `v = 0` (self-loop on one
universally-quantified variable). Multi-variable empty-premise
templates (e.g. `R(0, 1)` with no premise — "everything is related
to everything") are deliberately not generated — they would require
a meaningful prior on which variable pairs to enumerate; out of
scope here.

The id codec `axiom_template_id` / `axiom_id_to_template` already
handles empty premise — the id is `ax_tpl_v1_c0-0`, which
round-trips cleanly.

### Evaluation change

`evaluate_template_recursive` already handles empty premise: the
for-loop over premise is a no-op, every variable binding is counted
as a satisfied premise binding, and the conclusion check proceeds.
For `num_vars=1` over N identifiers this produces exactly N
bindings; conclusion-satisfied counts self-loops; rate is
`|self-loops| / |identifiers|` — matching `check_reflexivity`.

### `discover_theory` intentionally unchanged

`discover_theory` still uses `check_reflexivity` and names
reflexivity as `ax_reflexivity`. This is deliberate: preserves
backward compatibility for every existing theory-layer test,
and avoids two-ids-one-meaning duplication. Callers who want the
template-form id use `discover_axioms` with
`include_empty_premise=true` directly.

## Alternatives considered

- **Default on.** Changes the output shape of `discover_axioms`
  on every reflexive input. Too breaking. Rejected.
- **Add a separate enumerator for empty-premise.** Two-function
  API. Rejected — one config field is cleaner.
- **Also enumerate `R(0,1)` / `R(1,0)` empty-premise**. Would
  model "every ordered pair is related." Only true on complete
  graphs; rare and trivially enumerable. Rejected — adds
  templates with almost-zero value; worth a future ADR only if
  a real use case appears.
- **Retire `check_reflexivity` / `ax_reflexivity` in favor of the
  template form**. Rejected — breaks existing theory-layer API
  and test expectations. Both forms coexist.

## Consequences

### The predicate gap narrows

Reflexivity now has TWO ways to surface:
- Predicate-form: `check_reflexivity` → `ax_reflexivity` (from
  ADR 0027). Used by `discover_theory`.
- Template-form: enumeration with `include_empty_premise=true` →
  `ax_tpl_v1_c0-0`. Used by `discover_axioms` when opted in.

Antisymmetry and totality still can't be expressed in the current
template form. Closing them requires:
- Antisymmetry: equality conclusion (not an R edge)
- Totality: disjunction (not a single conclusion)

These are language extensions beyond a simple "allow empty
premise," left for future ADRs if needed.

### Defeasible reflexivity

With `include_empty_premise=true` and `min_rate < 1.0`, partial
reflexivity surfaces. Tested:

```
graph = {R(a,a), R(b,b), R(c,d), R(d,c)}
→ ax_tpl_v1_c0-0 at rate 0.5, support 2/4
```

Reads as "reflexivity holds on 2 of 4 identifiers" — a legitimate
observation the system previously couldn't make.

## Verification

- 188 → 194 tests pass (6 new: default excludes empty premise,
  opt-in surfaces template reflexivity, id roundtrip,
  non-reflexive input correctly excludes, defeasible surfaces
  partial reflexivity, opt-in only adds never removes).

## Implementation

- `v2/src/lib.rs` — new config field, extended
  `enumerate_axiom_templates`.
- `v2/docs/decisions/0036-empty-premise-templates.md` — this ADR.
