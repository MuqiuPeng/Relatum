# Relatum

A relation-first reasoning system exploring autonomous abstraction from minimal primitives.

## Versions

### [v1/](v1/) — archived (tag `v1.0`)

Relational closure engine with axiom instantiation.

- **Algebraic discovery (inductive).** From exhaustive enumeration of the
  19,683 binary operations on a 3-element carrier, the engine partitions the
  model space by axiom class and identifies the abelian group region through
  dual-signal alignment of rarity and closure richness.
- **Set-theoretic derivation (deductive).** With variable-variable unification,
  ZFC axioms produce universally quantified consequences directly.
- **Phase 1–9** exhaustive verification across 935 models / 10 axiom classes.
- **DSL playground** at [v1/www/](v1/www/) (deployed to GitHub Pages).
- **Essay:** [v1/docs/essay/main.tex](v1/docs/essay/main.tex).

Frozen. No further development except critical fixes.

### [v2/](v2/) — paused (2026-05-29)

Rebuild from a single ontological commitment: `R(x, y)` as the only primitive.
Self-driven abstraction over R instances. No frontend.

See [v2/docs/constitution.md](v2/docs/constitution.md) for the five
non-negotiable commitments that define v2.

Paused, not frozen — the closure / theory runtime is mature through ADR 0083
and stable on the long-horizon OQ#1 (tick 2400). Work resumes when v3 reaches
M5 (the bridge milestone).

### [v3/](v3/) — active

World-model substrate. Primitive shifts from `R(x, y)` to `state(node, t)` /
`transition(node, t → t+1)`; `R(x, y)` becomes a *derived* layer recovered
from anonymous state sequences.

Working proposition: recover de-named relation structure from anonymous
state sequences. n-ary primitives are allowed natively, with mandatory
binary projection + irreducibility test.

See [v3/docs/constitution.md](v3/docs/constitution.md) for the changed
primitive, the changed arity, and the v3-specific augmented commitments
(A1–A4).

## Why the split

v1 proved that relational closure can discover and derive. v2 added
self-driven abstraction over given R. v3 attacks the layer underneath:
recovering R from state dynamics, with anonymization as the test bench.

v1 is the benchmark; v2 is the closure / theory runtime; v3 is the
world-model substrate. The three coexist by design — v3 has no `Cargo`
dependency on v2 and vice versa.
