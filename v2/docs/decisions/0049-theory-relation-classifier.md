# 0049: Theory relation classifier + neighborhood

Status: Accepted
Date: 2026-04-24

## Context

After ADRs 0034 (extends), 0042 (independent), and 0046 (parallel),
theory-to-theory relations are exhaustive: every distinct pair of
named theories falls into exactly one category. But no single
method expresses that structure. A caller wanting to classify a
pair had to call three discover functions or manually compare
member sets.

Task 3 of the 1'''→5''' round adds the meta-view:

1. **`classify_theory_pair(a, b)`** — one call returns which of
   five kinds applies.
2. **`theory_neighborhood(t)`** — groups every other named theory
   by its relation to `t`.

## Decision

### The five kinds

```rust
pub enum TheoryRelationKind {
    Equal,        // same member set
    Extends,      // a's members ⊋ b's
    ExtendedBy,   // b's members ⊋ a's
    Independent,  // empty intersection
    Parallel,     // non-empty intersection, neither is subset
}
```

`Equal` is kept even though `name_theory` reuses ids on
member-set match — hand-constructed edge cases could produce two
theories with the same member set, and the classifier should
report them consistently.

### `classify_theory_pair(&self, a, b) -> Option<TheoryRelationKind>`

Reads `theory_axioms(a)` and `theory_axioms(b)`, compares as
sets, returns the appropriate kind. `None` only if either id is
not a registered theory.

### `theory_neighborhood(&self, t) -> Option<TheoryNeighborhood>`

```rust
pub struct TheoryNeighborhood {
    pub equal: Vec<String>,
    pub extends: Vec<String>,
    pub extended_by: Vec<String>,
    pub independent: Vec<String>,
    pub parallel: Vec<String>,
}
```

For every other named theory, classifies and appends to the
appropriate list. Deterministic (lists sorted).

### Read-only; no meta-R side effects

The classifier and neighborhood are pure queries over existing
meta-R state. They do not create extension / independence /
parallel edges. Callers who want to persist the relation still
use `name_theory_extension` / `name_theory_independence` /
`name_theory_parallel` — the discovery-write split from earlier
ADRs is preserved.

## Alternatives considered

- **Auto-write all discovered relations**. Rejected — would
  create many meta-R edges for classifications that are
  re-derivable cheaply. Keep classification read-only, writing
  opt-in.
- **Return the relation edges' ids (if any) along with the kind**.
  Considered; not added — callers can look up via the existing
  `extension_edges` / `independence_edges` / `parallel_edges`
  APIs. Keeps classifier simple.
- **Make neighborhood lazy / streaming**. Premature for the
  current scale (β). Simple `Vec<String>` is fine.

## Consequences

### The theory-space gets a map

A single `theory_neighborhood(t)` call now returns the full
structural context of `t`: what it extends, what extends it, what
it's independent from, what it's parallel to. This is the first
v2 view that lets a caller see a named theory *in context*
rather than in isolation.

### Classification is pair-atomic, O(|ax|) per call

Internally HashSet operations on axiom-id strings. For β-scale
(≤ 10 theories, ≤ 10 axioms each), neighborhood is sub-millisecond.

### Discover / name symmetry preserved

Three discover functions (ADR 0034 / 0042 / 0046) scan pairs and
return them; three name functions persist specific pairs. ADR
0049's classifier is the dual in read-only: given an arbitrary
pair, tell me which bucket it's in.

## Verification

- 270 → 276 tests pass (6 new: classify extends, independent,
  parallel, equal on same id, returns None for non-theory,
  neighborhood partitions pairs correctly).

## Implementation

- `v2/src/lib.rs` — `TheoryRelationKind` enum,
  `TheoryNeighborhood` struct, `classify_theory_pair`,
  `theory_neighborhood`.
- `v2/docs/decisions/0049-theory-relation-classifier.md` — this ADR.
