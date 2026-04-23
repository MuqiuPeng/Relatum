# 0027: Axiom discovery probe (extensional → intensional)

Status: Accepted
Date: 2026-04-24

## Context

v2's abstraction unit is a finite canonical subgraph — a pattern
is a fixed shape with k edges. Concepts like "partial order,"
"equivalence relation," or "group" are not subgraphs: they are
**intensional** — universally quantified axioms that the whole
relation must satisfy.

User asked whether v2 can construct concepts like a poset. The
honest answer (from the previous turn) was: not with the current
extensional machinery. Three paths were named — property check (A),
rule layer (B), and observation→axiom inference (C). User chose C.

This ADR is the **minimum viable axiom-discovery probe**: enumerate
a bounded space of axiom templates (up to 2-edge premise, 1-edge
conclusion, up to 3 variables), evaluate each against the RSet,
report templates that hold with 100% rate and ≥ 1 binding. Antisym-
metry and reflexivity are checked separately because they do not
fit the positive-implication template form.

This probe does **not** encode axioms as meta-R instances. Commit-
ment 3's "types are meta-R" does not obviously extend to rules-
with-variables. The encoding question is deferred; axioms live as
Rust data structures for now. That's why this is a probe, not a
mechanism.

## Decision

### Types

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeTemplate { pub x_var: usize, pub y_var: usize }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AxiomTemplate {
    pub num_vars: usize,
    pub premise: Vec<EdgeTemplate>,      // conjunction
    pub conclusion: EdgeTemplate,
}

#[derive(Debug, Clone)]
pub struct AxiomEvidence {
    pub template: AxiomTemplate,
    pub premise_bindings: usize,
    pub conclusion_satisfied: usize,
    pub rate: f64,
}

#[derive(Debug, Clone)]
pub struct AxiomDiscoveryConfig {
    pub max_premise_edges: usize,     // default 2
    pub max_vars: usize,              // default 3
    pub min_evidence: usize,          // default 1 (at least one premise binding)
}

/// Separate check for reflexivity (does not fit template form —
/// premise is "x is an identifier," not an edge).
pub struct ReflexivityEvidence {
    pub identifiers_total: usize,
    pub self_loops_present: usize,
    pub rate: f64,
}

/// Separate check for antisymmetry (conclusion is `x = y`, not an
/// edge). Reports count of violating pairs.
pub struct AntisymmetryEvidence {
    pub directed_pairs_checked: usize,
    pub violations: usize,            // R(x,y) ∧ R(y,x) with x ≠ y
    pub holds: bool,
}

#[derive(Debug, Clone)]
pub struct PosetCheck {
    pub reflexive: ReflexivityEvidence,
    pub antisymmetric: AntisymmetryEvidence,
    pub transitive: Option<AxiomEvidence>,  // the template evidence
    pub is_poset: bool,
}
```

### API

```rust
impl RSet {
    pub fn discover_axioms(&self, config: &AxiomDiscoveryConfig)
        -> Vec<AxiomEvidence>;

    pub fn check_reflexivity(&self) -> ReflexivityEvidence;
    pub fn check_antisymmetry(&self) -> AntisymmetryEvidence;
    pub fn check_poset(&self) -> PosetCheck;
}
```

### Template enumeration

For `max_premise_edges = m` and `max_vars = v`:

- Single-edge templates: `v²` (each endpoint picks a var).
- Multi-edge premises: unordered sets of 1..=m edge templates.
  Deduped by sorting.
- Conclusion: any single-edge template.

Filter:
- Conclusion's variables must all appear in the premise.
- Conclusion must not be literally equal to any premise edge.
- Canonicalize variable numbering (first-use order) to avoid
  duplicates that differ only by variable renaming.

Space bound at defaults (m=2, v=3): ~hundred templates.

### Template evaluation

For each template:
1. Enumerate every variable binding — each of the `num_vars`
   variables picks an identifier from the RSet. Bindings are
   `|identifiers|^num_vars`; for β-scale RSets this is tractable.
2. For each binding, instantiate all premise edges; if every one
   is in the RSet, premise is satisfied (count it).
3. Of those, check if the instantiated conclusion edge is in the
   RSet (count it).
4. `rate = satisfied / bindings`, or 1.0 if bindings == 0 (vacuous).

Exclude meta-R edges from both the identifier set and the "in RSet"
check so axioms are evaluated on *data only*.

### Reporting rule

Return templates with `rate == 1.0` AND
`premise_bindings >= config.min_evidence`. Lower-rate templates
are omitted from the default output (they would be "defeasible"
rules; out of probe scope).

## Alternatives considered

- **Hardcode three axioms** (reflexivity, symmetry, transitivity)
  and call it option A under another name. Rejected — user's
  request was C, "system discovers axioms." Template enumeration
  is the minimum genuine realization.
- **Store axioms as meta-R instances.** Rejected as premature.
  Encoding rules-with-variables in binary R requires several new
  conventions (variable markers, slot positions, premise/conclusion
  separation). That's a follow-up ADR if/when axiom discovery
  proves useful.
- **Extend to arbitrary premise size.** Rejected — combinatorial
  explosion. `m=2, v=3` covers classical binary-relation axioms;
  larger spaces are a separate design choice.
- **Include defeasible rules** (rate close to but below 1.0).
  Deferred. Adding fuzzy / high-support rules is a separate axis
  that interacts with scoring. Probe stays strict.

## Consequences

- v2 gains a first **intensional** inference primitive. No
  extensional-pattern mechanism is affected.
- Axioms do not enter the R space; they are Rust values returned
  from a method. Commitment 3 not yet extended to rules.
- Complexity: enumeration is bounded; evaluation is
  `O(n_templates · |ids|^max_vars)`. For ~ 17 identifiers, 3 vars,
  100 templates: ~0.5M lookups, sub-second. Scales poorly beyond
  this without pruning.
- The probe's success criterion is: on a hand-constructed poset-
  shaped RSet, discover transitivity (as template) and pass the
  `check_poset` boolean.

## Implementation

- Source: `v2/src/lib.rs` — new types, enumeration helper,
  evaluation helper, `discover_axioms`, `check_reflexivity`,
  `check_antisymmetry`, `check_poset`.
- Tests: 6 new — poset: all three axioms hold; chain: transitivity
  fails; symmetric graph: symmetry holds; empty RSet: vacuously;
  fix-point of RNG determinism (enumeration order stable).
- Example: `v2/examples/axiom_discovery.rs` — constructs a
  diamond poset and reports.
- Experiment log: `v2/logs/2026-04-24_axiom_discovery.log`.
