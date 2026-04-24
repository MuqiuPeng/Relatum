# 0044: Template language — equality and disjunctive conclusions

Status: Accepted
Date: 2026-04-24

## Context

The positive-implication template language from ADR 0027 covered
exactly one conclusion shape: `R(x, y)` (a single edge). That
made antisymmetry (equality conclusion) and totality (disjunctive
conclusion) inexpressible as templates — ADR 0036 closed the
*empty premise* case for reflexivity, but antisymmetry and
totality lived as predicate-only checks with hardcoded ids.

Task 2 of the 1''→5'' round: admit both as first-class templates.
Without breaking existing AxiomTemplate-based code.

## Decision

### Two new sibling template types

```rust
pub struct EqualityAxiomTemplate {
    pub num_vars: usize,
    pub premise: Vec<EdgeTemplate>,
    pub equal_vars: (usize, usize),
}

pub struct DisjunctiveAxiomTemplate {
    pub num_vars: usize,
    pub premise: Vec<EdgeTemplate>,
    pub conclusions: Vec<EdgeTemplate>,
}
```

Kept as **separate types** rather than a conclusion-enum on
`AxiomTemplate`. Rationale: AxiomTemplate is used in many places
(enumeration, canonicalization, id codec, forward-chain,
subsumption, intension storage). Making its conclusion an enum
would require a large case-split refactor with broad test impact.
The sibling-type approach is additive and non-breaking.

### Unified evidence type

```rust
pub enum ExtendedAxiomEvidence {
    Edge(AxiomEvidence),
    Equality {
        template: EqualityAxiomTemplate,
        premise_bindings: usize,
        conclusion_satisfied: usize,
        rate: f64,
    },
    Disjunctive {
        template: DisjunctiveAxiomTemplate,
        premise_bindings: usize,
        conclusion_satisfied: usize,
        rate: f64,
    },
}

impl ExtendedAxiomEvidence {
    pub fn rate(&self) -> f64;
    pub fn premise_bindings(&self) -> usize;
}
```

### Discovery API

```rust
impl RSet {
    pub fn discover_antisymmetry_template(&self) -> Option<ExtendedAxiomEvidence>;
    pub fn discover_totality_template(&self) -> Option<ExtendedAxiomEvidence>;
    pub fn discover_extended_axioms(&self, config: &AxiomDiscoveryConfig)
        -> Vec<ExtendedAxiomEvidence>;
}
```

Antisymmetry is the canonical equality template:
`R(0,1) ∧ R(1,0) ⇒ v_0 = v_1`.

Totality is the canonical disjunctive template:
`(empty) ⇒ R(0,1) ∨ R(1,0)`.

`discover_extended_axioms` merges edge-family axioms
(`discover_axioms`) with antisymmetry and totality evidence,
applying `min_rate` / `min_evidence` filters uniformly.

### Evaluation

- Equality template: for each binding, if premise holds, count;
  conclusion holds iff `ids[binding[a]] == ids[binding[b]]`.
  Captures that the only bindings satisfying antisymmetry's premise
  (both directions of an R edge) MUST have `x = y`.
- Disjunctive template: for each binding, if premise holds, count;
  conclusion holds iff any disjunct's edge is in the RSet.

## Alternatives considered

- **Extend `AxiomTemplate.conclusion` to `AxiomConclusion` enum**.
  Would give one uniform type but requires touching every
  canonicalization / id-codec / evaluation / intension /
  subsumption / composition function. Rejected for this ADR — the
  blast radius is too large for the value; may revisit if a
  future ADR needs universal handling.
- **Generate all disjunctive-conclusion templates by enumeration**
  (every subset of 2-edge disjunctions). Rejected — combinatorial
  explosion; the two named templates (antisym, totality) are the
  ones with known semantic weight.
- **Reuse the predicate ids `AX_ANTISYMMETRY` / `AX_TOTALITY`**
  for extended templates. Rejected — keeps confusion between
  predicate form (one hardcoded check) and template form (one
  evaluation over bindings). The two are semantically identical
  but structurally different; separate ids would make the
  distinction observable if needed.

## Consequences

### The predicate / template gap narrows further

After ADR 0039, totality joined reflexivity and antisymmetry as
predicate axioms. After ADR 0036, reflexivity could also appear
as a template (via empty-premise enumeration). After ADR 0044,
all three have template representations:

| Axiom | Predicate form | Template form |
|---|---|---|
| reflexivity | `check_reflexivity` / `AX_REFLEXIVITY` | empty-premise `R(v,v)` (ADR 0036) |
| antisymmetry | `check_antisymmetry` / `AX_ANTISYMMETRY` | `EqualityAxiomTemplate` (ADR 0044) |
| totality | `check_totality` / `AX_TOTALITY` | `DisjunctiveAxiomTemplate` (ADR 0044) |

Callers can use either form. Discovery through the edge-family
pipeline uses predicate form (via `discover_theory`). Callers
wanting uniform evidence shape (rate, support, confidence) use
`discover_extended_axioms`.

### Not yet: meta-R intension, subsumption, composition

Equality and disjunctive templates do not (yet) participate in:
- `register_axiom_with_intension` — no meta-R intension encoding
- `subsume_by_premise_weakening` — edge-conclusion subsumption
- `subsume_by_composition` — forward chaining
- `discover_axioms_minimal` — filters only edge family
- `discover_theory` — still uses predicate forms

These are left as followups. Every mechanism that works on
`AxiomTemplate` is unchanged; the new templates sit alongside
without interfering.

### On the 8-case rigorous battery

- Total order case: `discover_extended_axioms` returns edge
  transitivity + **template-form totality**. Rate=1.0 on both.
- Equivalence case: edge symmetry / transitivity + equivalence's
  **template-form antisymmetry fails** (rate < 1.0 because R(a,b)
  and R(b,a) hold for a ≠ b).
- Diamond poset: transitivity + **antisymmetry at rate 1.0**
  (premise only met when x = y, so equality is trivially true).

### Limits carried forward

- Can't enumerate arbitrary disjunctive templates (combinatorial).
- Can't compose antisym/totality into `discover_theory` fingerprint
  without further integration.
- Can't encode antisym/totality intension in meta-R yet — blocked
  on future encoding design.

## Verification

- 236 → 243 tests pass (7 new: antisym holds on poset, fails on
  equivalence; totality holds on total order, fails on diamond;
  merge returns all three families; equality rate is binding-based;
  defeasible threshold respected).

## Implementation

- `v2/src/lib.rs` — `EqualityAxiomTemplate`,
  `DisjunctiveAxiomTemplate`, `ExtendedAxiomEvidence`,
  `evaluate_equality_template`, `evaluate_disjunctive_template`,
  `discover_antisymmetry_template`,
  `discover_totality_template`, `discover_extended_axioms`.
- `v2/docs/decisions/0044-extended-template-language.md` — this ADR.
