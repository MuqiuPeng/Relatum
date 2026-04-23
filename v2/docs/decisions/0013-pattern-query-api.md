# 0013: Pattern query API (first use of named meta-R)

Status: Accepted
Date: 2026-04-23

## Context

ADR 0012 closed the β layer. Named patterns are stored as meta-R;
the naming-pass driver can run idempotently; the pipeline is
complete from raw edges to named structural types. But the meta-R
currently sits passively — nothing reads from it. The gamma log
explicitly flagged this: "the RSet now contains meta-R that
captures structure — but nothing yet *uses* this to do anything."

ADR 0013 introduces the minimum set of queries that let meta-R
start paying rent. Three questions a caller should be able to ask:

1. **Does this subgraph match a known pattern?** ("classify")
2. **What patterns does this identifier participate in?** ("membership")
3. **What concrete edges make up this instance?** ("reconstruction")

All three are derivable from existing state; this ADR just names
them and wraps them.

## Decision

Add four public query methods on `RSet`:

```rust
impl RSet {
    /// Return the named pattern whose canonical form matches `sg`, or None.
    pub fn classify_subgraph(&self, sg: &Subgraph) -> Option<&str>;

    /// Return the pattern that owns this instance, or None if the argument
    /// is not a recognized instance identifier.
    pub fn pattern_of(&self, instance_id: &str) -> Option<&str>;

    /// Return every (pattern_id, instance_id) pair in which `id` is a
    /// participant.
    pub fn memberships_of(&self, id: &str) -> Vec<(&str, &str)>;

    /// Reconstruct the concrete subgraph of an instance — the RSet edges
    /// whose endpoints are both in the instance's participant set.
    pub fn instance_subgraph(&self, instance_id: &str) -> Subgraph;
}
```

All four are thin compositions of existing APIs
(`find_pattern_matching`, `right_of`, `patterns`, `participants_of`,
`iter`, `Subgraph::from_edges`).

## Alternatives considered

- **Skip the ADR and let callers compose the existing primitives.**
  Rejected because these four are named concepts that will be
  reused by downstream work (automatic attach on `add`, cross-graph
  pattern transfer). Naming them once reduces repetition and makes
  the queries discoverable from `RSet`'s API surface.
- **Combine into a single `PatternView` struct** carrying pattern,
  instance, participants, and reconstructed subgraph together.
  Deferred. Callers mix and match; premature bundling limits
  flexibility. A `PatternView` helper can be added later as a
  convenience over these four primitives.
- **Return owned `String` instead of `&str`.** Rejected — the
  identifiers live in the RSet already; owning them would copy
  every query. Borrows are sufficient and conventional here.
- **Cache instance → pattern pointers.** Rejected as premature
  optimization. `pattern_of` scans `self.patterns()` to find the
  owner, which is `O(|patterns|)`. At the scale β operates on,
  this is negligible; caching adds invalidation complexity.

## Consequences

- Meta-R becomes *queryable*. Previously it was only the record of
  a naming event; now it answers structural questions from code.
- Enables the next step beyond β: automatic attach on `add`.
  A caller that has a subgraph in hand and wants to know "does
  adding this to my RSet extend any existing pattern" can compose
  `classify_subgraph` with `run_naming_pass`.
- The query API does *not* add or modify RSet contents. All four
  methods are `&self`. This is the first post-β layer that
  explicitly distinguishes *observation* from *action* in the
  named-pattern world.
- `memberships_of` returns the cartesian product of all patterns
  the identifier participates in. If the same identifier is a
  participant in two different patterns' instances (e.g., a node
  shared by a chain and a cycle), both memberships are returned.
  This is the expected multi-membership.
- `instance_subgraph` is the formalization of "the canonical
  recovery invariant" from ADR 0010. It produces exactly the
  subgraph whose canonical form matches the pattern's canonical
  form, under the stated invariant.

## Implementation

- Source: `v2/src/lib.rs` — four query methods, added near the
  existing ADR 0010 query block (`patterns`, `instances_of`,
  `participants_of`, `find_pattern_matching`).
- Tests: 5 new unit tests — classify returns known pattern,
  classify returns None for unknown canonical, pattern_of round
  trip, memberships_of for a multi-membership identifier,
  instance_subgraph canonical round-trip.
- Example: `v2/examples/pattern_queries.rs` — runs γ default pass
  on the mixed graph, then demonstrates each query in turn,
  including a "fresh subgraph classified against the registry" case.
- Experiment log: `v2/logs/2026-04-23_pattern_queries.log`.
