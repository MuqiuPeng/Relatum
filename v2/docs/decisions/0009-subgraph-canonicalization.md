# 0009: Subgraph canonicalization (Weisfeiler–Lehman refinement)

Status: Accepted
Date: 2026-04-23

## Context

ADR 0008 gave us subgraph instances. ADR 0009 must answer: *given two
`Subgraph` values with (possibly) different identifiers, are they the
same pattern?*

Without this, β cannot recognize repetition at the subgraph level —
two chain fragments `{R(c2,c3), R(c3,c4)}` and `{R(p1,p2), R(p2,p3)}`
look unequal despite being structurally identical.

The pattern recognition must:
- Treat two subgraphs as equal iff they are isomorphic (same structure
  up to identifier renaming, direction preserved per commitment 2).
- Produce a comparable canonical form (Hash + Eq or Ord) so later
  mechanisms can group / deduplicate.
- Operate purely on the subgraph (not its parent RSet) — "same pattern"
  is a structural, not a contextual, claim.

## Decision

Implement Weisfeiler–Lehman (WL) style iterative refinement over each
subgraph's identifiers, producing a canonical edge list as the
output.

```rust
pub type CanonicalForm = Vec<(u32, u32)>;   // sorted edges over stable labels

impl Subgraph {
    pub fn canonicalize(&self) -> CanonicalForm;
    pub fn is_isomorphic_to(&self, other: &Subgraph) -> bool;
}
```

Algorithm:
1. Collect identifiers from the subgraph. Build within-subgraph in- and
   out-neighbor index per identifier.
2. Initial label for each identifier: `(out_degree, in_degree)` within
   the subgraph. Rank the distinct initial signatures to small integers
   (ordered by natural `Ord`), producing the initial integer labels.
3. Iterate: each identifier's new signature is
   `(current_label, sorted_out_neighbor_labels, sorted_in_neighbor_labels)`.
   Rank the distinct signatures to new integer labels.
4. Stop when the relabeling is stable (no new partitions) or after
   `|V| + 1` iterations (guaranteed convergence for WL-1).
5. Canonical form: for each edge `R(x, y)`, emit `(label(x), label(y))`;
   sort the resulting list. Two subgraphs are isomorphic iff they
   produce the same canonical form.

Determinism: labels are produced by *ranking* sorted-distinct signatures
— no hashing, no random seed. The algorithm is fully reproducible and
does not depend on process-global state.

## Alternatives considered

- **Hash-based WL labels.** Rejected. The standard stdlib hashers
  (`DefaultHasher`) use random seeds, making canonical forms
  non-deterministic across processes. Using a stable hash (SipHash
  with fixed key, xxHash, etc.) would work but introduces a hash
  collision probability where a deterministic rank-based scheme has
  none.
- **Full graph-isomorphism algorithm** (e.g., nauty-style). Rejected
  for now. WL-1 is not a complete isomorphism test — some non-isomorphic
  graphs (strongly regular, certain trees) collide. But at the small
  subgraph scale β is working with, WL-1 is effectively exact. If a
  later experiment surfaces a WL-1 false positive, a follow-up ADR
  can introduce a stronger backend.
- **Edge-multiset canonical form** (sorted multiset of
  endpoint-label pairs, without WL). Rejected as too weak: fails on
  simple examples like "3-cycle vs 3-chain" which have the same
  degree sequence but different structure.
- **Canonicalize against the parent RSet's IdentifierProfile.**
  Rejected. Pattern equality is structural, not contextual. A chain
  fragment embedded in a bigger RSet should still match a chain
  fragment standing alone; mixing in the parent context breaks this.
  Compound fingerprint (ADR 0007) already captures the contextual
  view when wanted.
- **Treat edge direction as ignorable** (canonicalize undirected
  skeleton). Rejected — commitment 2. Direction is intrinsic;
  `R(a,b)` and `R(b,a)` are structurally distinct.

## Consequences

- Two subgraphs are now comparable as patterns. `CanonicalForm` is
  `Vec<(u32, u32)>`, which derives `Hash + Eq + Ord + Clone` for free,
  making it usable as HashMap key or sort key in downstream code.
- **Cross-compound-class isomorphism becomes visible.** The ADR 0008
  experiment yielded 6 single-edge subgraphs across 4 different
  compound classes (chain head, tree branches, terminal descent ×2,
  isolated edge). Under canonicalization they all produce the same
  `CanonicalForm = [(1, 0)]` (a directed edge from a source to a sink).
  This is the first empirical finding that **compound fingerprint and
  structural pattern are different axes**: compound = contextual role
  in the RSet, canonical = pure structure of the subgraph.
- ADR 0010 will have to decide whether "pattern" means
  (a) canonical-form equivalence class,
  (b) (canonical-form × compound-fingerprint) pair,
  (c) something else.
  This ADR doesn't resolve that; it just exposes the distinction.
- WL-1 is a heuristic: false-positive collisions are possible in
  theory. For the graph scales expected in β's first experiments
  (subgraphs with ≤ 20 edges), collisions are vanishingly unlikely.
  Recorded as a known limitation; escape hatch is a stronger canonical
  form in a later ADR if needed.
- Cost: each iteration is `O(|V| + |E|)` for signature construction
  plus `O(|V| log |V|)` for ranking. At most `|V| + 1` iterations.
  Total: `O(|V|² log |V|)` in the worst case for a single subgraph.
  Fine at β's scale.

## Implementation

- Source: `v2/src/lib.rs` — `CanonicalForm`, `Subgraph::canonicalize`,
  `Subgraph::is_isomorphic_to`, helper `rank_labels`, plus tests.
- Example: `v2/examples/canonicalization.rs` — runs canonicalize on
  every subgraph produced by `compound_class_subgraphs` on the ADR 0007
  mixed graph and groups by canonical form.
- Experiment log: `v2/logs/2026-04-23_canonicalization.log` with the
  canonical forms, cross-class groupings, and the first-time answer to
  "do structurally-equal subgraphs span different compound classes?"
