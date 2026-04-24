# 0046: Theory parallel relations

Status: Accepted
Date: 2026-04-24

## Context

After ADR 0034 (extends) and ADR 0042 (independence), the
theory-space relation set is:

| T_a vs T_b | member intersection | member comparison | current ADR |
|---|---|---|---|
| extends | ≠ ∅ | `a ⊋ b` or `b ⊋ a` | 0034 |
| independent | = ∅ | —  | 0042 |

There's a visible gap: two theories that **share** some axioms but
where **neither is a subset of the other** — partial overlap.
Antisymmetry + transitivity vs. antisymmetry + reflexivity both
contain antisymmetry but diverge. Neither extension nor
independence applies.

Task 5 of the 1''→5'' round adds this third case: **parallel**.

## Decision

### Definition

Two named theories `T_a` and `T_b` are **parallel** iff:
- `members(T_a) ∩ members(T_b) ≠ ∅`  (share something)
- Neither is a subset of the other  (neither extends the other)
- (Implicitly) `T_a ≠ T_b`

### Marker and encoding

```rust
pub const PARALLEL_MARKER: &str = "__parallel__";
```

Chain encoding, canonical direction (lex-smaller theory first),
same as ADR 0042 independence:

```text
R(__parallel__, par_N)      — registry
R(T_lo, par_N)              — canonical source
R(par_N, T_hi)              — canonical target
```

3 edges per parallel relation.

### API

```rust
impl RSet {
    pub fn name_theory_parallel(&mut self, a: &str, b: &str) -> Result<String, TheoryError>;
    pub fn parallel_edges(&self) -> Vec<&str>;
    pub fn parallel_endpoints(&self, par_id: &str) -> Option<(String, String)>;
    pub fn theories_parallel_to(&self, theory: &str) -> Vec<String>;
    pub fn discover_theory_parallels(&self) -> Vec<(String, String)>;
    pub fn retract_parallel(&mut self, par_id: &str) -> Result<usize, TheoryError>;
}
```

`name_theory_parallel` verifies:
1. Both theories exist, distinct.
2. Members overlap (non-empty intersection).
3. Neither is a subset of the other.

Failures hint at the appropriate alternative — "use independence
instead" or "this is an extends relation, not parallel."

## Alternatives considered

- **Fold parallel, extends, independent into one "theory-relation"
  marker with a kind tag**. Rejected — each has different
  symmetry / subset / disjointness invariants; unifying them in
  one marker hides that structure and loses compile-time
  differentiation.
- **Define parallel to *include* the extends case**. Rejected —
  extends is already its own relation. Parallel should be
  structurally distinct, signaling "different direction of
  divergence, neither subsumes the other."
- **Reward parallel edges in `abstraction_score`**. Not done —
  consistent with ADR 0042's choice not to reward independence
  edges. Future ADR could symmetrize if needed.

## Consequences

### Theory-space relation set is now three-complete

After ADR 0046, every pair of distinct theories falls into exactly
one of three categories by their member-set topology:
- **extends**: one strictly contains the other.
- **independent**: empty intersection.
- **parallel**: non-empty intersection, neither contains the other.

(Plus the trivial `equal` case where members are identical —
captured by `name_theory`'s id-reuse; no separate relation edge
needed.)

### Discovery completeness

`discover_theory_extensions`, `discover_theory_independences`,
and `discover_theory_parallels` together partition every
pair-of-distinct-theories. A future ADR could add a single
`classify_theory_pair(a, b) -> TheoryRelationKind` for uniform
query, but individual discovery functions are fine for now.

### No auto-prune risk (currently)

Parallel edges, like independence and extension edges, add to
meta-R cost (−0.3 tax) but don't get rewarded by
`abstraction_score`. Auto-prune with threshold 0 would retract
them. The same caveat as independence (ADR 0042); fixable by a
future metric revision.

### Commitment check

- 1–5 unaffected. The relation is purely structural (member-set
  set algebra).

## Verification

- 249 → 257 tests pass (8 new: valid parallel, rejects disjoint,
  rejects subset, canonical ordering, discover pairs, symmetric
  query, retract, meta-id inclusion).

## Implementation

- `v2/src/lib.rs` — `PARALLEL_MARKER`, six methods for parallel
  relations + `retract_parallel`, extended `collect_meta_ids`.
- `v2/docs/decisions/0046-theory-parallel.md` — this ADR.
