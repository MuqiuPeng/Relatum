# v2 Ontological Commitments

Five non-negotiable commitments. Every design decision must respect these.
If a commitment blocks implementation, question the design — not the commitment.

---

## 1. R is singular

The entire system has exactly one R.

All relations are patterns over this single R. No `R_causal`, no `R_contains`,
no `R_temporal`. Meaning differences live in the distribution of instances,
not in the multiplicity of Rs.

**Enforcement:** no named relation types at the primitive layer.
Type names appear only as emergent labels over R instance clusters.

## 2. R is binary

`R(x, y)`. Two slots, one direction.

This is an explicit choice, not a necessity. Binary is the smallest non-trivial
arity — unary is just attribute; binary is the first true connection.

**Enforcement:** no n-ary relation primitive. Higher arity must be encoded
as structures over binary R (e.g., a ternary `R(a, b, c)` is decomposed into
R instances linked by shared identifiers).

## 3. Types are meta-R instances

When the system names a pattern as a type T, T is another node in the R graph.
A type's "typeness" is a structural property of how T appears in R.

Single-layer ontology. No separate type system above R.

**Enforcement:** type-of / membership / schema relations are all expressed
as R instances. No separate data structure for "the type registry."

**Clarification (ADR 0029):** commitment 3 is about the type's *intension*
— its structural definition — being expressible in meta-R. It is not a
claim about the type's *extension* (the set of observed instances and their
token bindings). Extensional records are a matter of instrumentation policy:
they may be written, partially written, or not written, without violating
this commitment. What commitment 3 does require is that the intension —
what makes T the type it is — be present as meta-R whenever T is named.

## 4. Identity is token-based

Two appearances of the same identifier denote the same object.
No implicit deduplication via structural equivalence.

**Enforcement:** identity is string equality at the R level.
Structural identity (two structurally equivalent nodes being "the same")
can be added as a named relation, but not as implicit collapse.

## 5. Similarity is structural

Two R instances are similar iff they are structurally equivalent in the R graph
(role / position / neighbor-pattern equivalence).

No external labels, no semantic judgment, no hand-coded similarity metric.

**Enforcement:** any similarity function must be derivable from the graph
structure alone. If a proposed mechanism requires a non-structural input,
the mechanism is out of bounds for v2.

---

## When a commitment conflicts with implementation

The right response is to re-examine the mechanism, not to bend the commitment.
If a commitment genuinely cannot be implemented, that's a discovery — record
it and raise the question before relaxing. Silent drift is the failure mode.
