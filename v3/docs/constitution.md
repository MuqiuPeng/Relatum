# v3 Ontological Commitments

v3 working proposition:

> Recover de-named relation structure from anonymous state sequences.
> 从匿名状态序列中恢复去名后的关系结构。

This document records what v3 inherits from v2, where v3 diverges, and what
new commitments v3 adds. Reading this should let any future contributor
judge edge cases without consulting the original conversation.

v3 is a **parallel engine to v2, not a replacement**. v2 (`v2/`) remains
the R-closure / theory runtime. v3 is a world-model substrate beneath the
same Relatum philosophy. The two crates have no `Cargo` dependency on each
other; cross-pollination is by documents like this one.

---

## Inherited from v2

The five v2 commitments (`v2/docs/constitution.md`) are the philosophical
parents. v3 keeps three of them as-stated and changes two with explicit
intent.

### Kept

- **#3 Types are meta-R instances.** Types are nodes in the relation graph,
  not a separate layer. v3 honors this in its *derived* R layer.
- **#4 Identity is token-based, within an episode.** v3 narrows the scope:
  inside one episode, token identity is string equality (no implicit
  dedup). Across episodes, identity is structural and is resolved by the
  alignment procedure described in `design-notes.md`.
- **#5 Similarity is structural.** Any similarity / fingerprint function
  must be computable from the observation graph alone, without external
  labels. Strengthened in v3 to a training objective (see A1 below).

### Changed: the primitive

v2: the single primitive is `R(x, y)`.

v3: the primitives are `state(node, t)` and `transition(node, t → t+1)`.
`R(x, y)` exists in v3 as a **derived** layer over observed state dynamics,
not as a built-in.

Justification: a world-model substrate must look at state differences and
their propagation before any relation can be named. v2's `R(x, y)` is the
right primitive for a closure / theory runtime, where relations are given.
v3 is the layer where relations are recovered.

### Changed: arity

v2 #2: `R` is binary; n-ary structures are encoded as binary R linked by
shared identifiers.

v3: **n-ary primitives are allowed natively**, with two required obligations:

1. **Binary projection.** Every n-ary primitive must publish its binary
   projection — the set of pairwise marginal effects it induces.
2. **Irreducibility test.** Every n-ary primitive must support a procedure
   that decides whether it is reducible to a composition of binary
   primitives or genuinely irreducible (XOR-like, joint-constraint,
   conditional-trigger).

Pure binarization silently loses XOR and joint constraints. Pure n-ary
explodes the space. v3 accepts the representational cost and pays it with
the projection + irreducibility obligations.

---

## v3 augmented commitments

These are v3-specific. v2 does not enforce them.

### A1. Targets are structural invariants under anonymization.

Any objective the system learns or recovers must be a function only of
*anonymized* observation. Concretely: for any bijection `π` over node
identifiers, `f(π(episode)) ≡ π(f(episode))` (the output transforms by the
same bijection, or is bijection-invariant when it does not name nodes).

A target that requires node names to be stable across the training set is
out of bounds.

### A2. No "relation name recognition" objectives.

No task may take the form `(observations) → label ∈ {control, cause,
contain, ...}`. Targets must instead be **operational vectors** —
directionality, constraint_effect, reversibility, latency, stability,
effect_size. Names, if they appear later, are emergent labels over
fingerprint clusters; never training labels.

### A3. Fingerprints are derived, cached, never intrinsic.

The fingerprint vector is a **cached derivation** from observation. It is
not a typed attribute carried by R. Caches may be invalidated; the
underlying R has no intrinsic "directionality field" or "stability field".
The same observations recomputed give back the fingerprint; storage is
optimization, not ontology.

### A4. n-ary primitives must publish projection + irreducibility flag.

A consequence of the arity decision restated as a build-time contract.
Any n-ary mechanism added to the simulator or the recovery layer must
expose both procedures or it is rejected at the type level. No "we will
add the projection later".

---

## Architectural posture

- v2 is the R-closure / theory runtime, operating on given R.
- v3 is the WM substrate, recovering R from state dynamics.
- The two crates are independent. No `Cargo` dependency in either
  direction. If a future bridge is built (v3 recovers R, feeds v2 closure),
  it lives in a third crate.
- Silent merging of the two engines is itself the failure mode this
  posture is designed to prevent.

---

## Intrinsic drive — phased

v2's intrinsic drive (self-driven triggering, MDL-style evaluation) does
**not** apply to v3 in M1.

- **M1 (now):** intrinsic drive runs in *shadow mode* only — it may
  produce candidate signals, but scheduling is fixed and externally
  supplied. Goal: get relation extraction stable on synthetic data first.
- **M2:** relation extraction stable, fingerprint clusters reproducible.
  Still no intrinsic-drive scheduling.
- **M3+:** intrinsic drive enters the scheduling loop. Episode selection,
  intervention targeting, and curriculum may be self-driven.

Approaching intrinsic drive earlier risks the system chasing noise before
the extraction substrate is reliable.

---

## Failure mode discipline

If a v3 commitment blocks a mechanism, the right response is to
re-examine the mechanism, not to bend the commitment. This mirrors v2's
discipline. Specific drifts to watch:

- Any objective that branches on `node_id`-equality across episodes.
- Any n-ary primitive added without its projection + irreducibility pair.
- Any fingerprint field that grows a setter (becomes intrinsic, not
  derived).
- Any code path that reads from v2 or that v2 reads from.

Silent drift on any of these is the failure mode v3 is designed to surface.
