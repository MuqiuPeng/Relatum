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

---

## Strict reading: differentiation requires registration

(Added 2026-05-06; confirmed via [reflection 0001](reflections/0001-meaning-emerges-with-concept.md).)

Commitments 1, 3, 4, 5 read together imply a stricter rule than any of them
states alone. Making it explicit:

> **Two tokens are distinguishable in the system iff some explicitly-registered
> concept names the distinction.**

Before any concept is registered, all tokens are indistinguishable. Token `a`
and token `b` are simply two strings that the rset has seen — no derived
structural attribute (degree, neighbor profile, role in subgraphs, locality
profile) is *part of* the token's identity. Such derived quantities are
*computations performed by mechanisms*, not *properties carried by tokens*.

A mechanism may compute and use derived structural quantities, but only when
the use is **bound to an act of concept registration**. Specifically:

- ✓ **Legal**: a derived signature is computed *during* an atomic act that
  mints a concept token P and registers participating tokens as instances of P
  (via meta-R chains rooted in P or in a marker that names what P is). The
  signature is internal scaffolding inside one atomic operation.
- ✓ **Legal**: a derived signature is computed *to decide whether* a token
  fits an already-registered concept C, when registering it as an instance of C.
  C's existence is what licenses the comparison.
- ✗ **Illegal**: a derived signature is treated as a stand-alone token
  property — a "label" that lets downstream code branch on which tokens
  differ — without any explicitly registered concept naming the distinction.
  Bucketing R by `EdgeFingerprint` as a free-standing diagnostic is illegal
  by this reading. So is any classification, attention pointer, or selection
  mechanism that uses signature-derived differences as *visible behavior*
  without an accompanying concept registration.

The illegal case is what reflection 0001 calls **implicit conceptualization**:
the system computes a difference, lets the difference shape its behavior, but
never registers what kind of object each side of the difference is. The
phantom type lives in the code, not in the rset. This violates the spirit of
commitment 3 (types are meta-R) even when no explicit type is named, because
the system is *acting on a typing distinction* without the type existing as a
queryable rset object.

### Implication: concept creation, object emergence, R-meaning are one act

Under this strict reading, "concept creation" decomposes into three
simultaneous facets that cannot be staged separately:

1. A new token P is added to the rset (concept exists as a node)
2. For every token `t` participating in an instance of P's pattern, a meta-R
   chain registers `t` as an instance of P (object emergence — `t` now has an
   explicit type-like property)
3. Each `R(a, b)` where `a` and `b` are P-instances acquires meaning at the
   type level (R-meaning emerges)

The three facets are inseparable. Without (2), nothing changes about token
properties — the act is just naming, not creation. Without (1), there's
nothing to register tokens as. Without (3) being possible, the concept is
inert.

ADR 0073's earlier framing of E1 (shape mining), E2 (object lifting), E3
(intrinsic drive) as three parallel entry points is therefore re-read: E1 + E2
must happen in the same atomic act, and E3 is the trigger signal that motivates
that act.

### Implication: derived computation as scaffolding, not as registry

The five existing structural-derivation ADRs (0004, 0005, 0006, 0007, 0009)
remain valid as **computational tools** for use *inside* concept-registration
acts. They are not invalid; they are demoted from "free-standing observational
layers that can be used anywhere" to "internal scaffolding for canonicalization
during emergence". Their outputs must not be persisted as token attributes
or used as visible classification labels outside a concept-creation act.

The implementation audit consequence: any code that bucket-classifies tokens
by `Signature` / `LocalityProfile` / `EdgeFingerprint` for downstream behavior
**without** simultaneously registering a concept that names the bucket is
out-of-bounds and must be either:

- removed, or
- rewritten so the bucket key participates in a concept-mint act, or
- explicitly marked as internal-scaffolding inside a single atomic operation
  whose output is a concept registration

### Open implementation question

How to identify a recurring substructure for concept creation **without**
presupposing token differentiation? Candidate: subgraph-isomorphism over R
substructures (ADR 0008 / 0009 in their canonicalization role), where two
R-substructures are "the same" iff a bijection between their token-slots
preserves all R adjacencies. Token identity does not enter; only the
role-share pattern does. This is the design space the next emergence ADR
must navigate.
