# 0015: Subgraph matching against named patterns

Status: Accepted
Date: 2026-04-23

## Context

ADR 0014 exposed a structural flaw: `run_naming_pass` uses
`compound_class_subgraphs` as its enumeration primitive, which
groups edges by compound fingerprint before extracting connected
components. Asymmetric structures (chains, trees) fragment when
their edges have distinct compound fingerprints. The experiment
demonstrated a fresh 2-chain `{u, v, w}` failing to attach to an
existing 2-chain pattern `p_2`, even though it is structurally
isomorphic.

The original plan was to defer the fix as "one use case, wait for
more." User pushed back: one reproducible failure is sufficient
evidence that the pipeline's enumeration strategy is wrong for
this purpose. This ADR fixes it.

Core insight: `compound_class_subgraphs` is a **discovery**
heuristic (find sites of repetition cheaply). **Matching** against
a known canonical form is a different problem. Treat them as two
distinct primitives; use the right one for each purpose.

## Decision

Introduce a subgraph-matching primitive and rewire `run_naming_pass`
to use it in attach-only mode.

### New primitive

```rust
impl RSet {
    /// Find every connected data subgraph whose canonical form equals
    /// `target`. Meta-R edges are excluded so patterns match against
    /// data, not against their own metadata.
    pub fn find_instances_of(&self, target: &CanonicalForm) -> Vec<Subgraph>;
}
```

### Algorithm

BFS-style enumeration of connected edge sets, pruned by size,
deduplicated by sorted edge-tuple key, then filtered for "cleanness":

1. Let `k = target.len()` (target edge count).
2. Filter RSet edges to "data only" (exclude meta-R tokens).
3. For each data edge as a seed, expand by adding any edge that
   shares an identifier with the current set. Stop at size `k`.
4. At size `k`, canonicalize the edge set; if equal to `target`,
   record it.
5. Dedup via `HashSet<Vec<R>>` keyed on sorted edges so each
   connected edge-set is visited once.
6. **Cleanness filter:** retain only subgraphs whose participant set,
   restricted to the data-edge portion of the RSet, induces exactly
   `k` edges. Reject "embedded" cases (e.g., a 2-chain whose three
   nodes also close back into a 3-cycle — the cycle has three data
   edges among those nodes, not two, so the 2-chain is embedded
   rather than clean). The filter preserves ADR 0010's canonical-
   recovery invariant: a clean instance's participant set uniquely
   determines its structure when restricted to data edges.

Worst-case complexity is combinatorial in the RSet size. At β's
experimental scale (tens to low-hundreds of edges, `k ≤ 5` typical)
it runs fast enough. Pruning by endpoint-profile sketch can be
added later if experiments outgrow the naive approach.

### Rewire `run_naming_pass` for attach-only

Under `attach_only = true`, the pass semantics change:

- Iterate named patterns in the RSet.
- For each pattern, reconstruct its canonical form from its first
  instance's participants.
- Call `find_instances_of(pattern_canonical)` to enumerate matching
  subgraphs in the data.
- Filter out any instance whose participant set is already recorded
  (dedup, same as before).
- Attach each remaining instance via `name_pattern_instances`.
- Record a per-pattern decision.

Under `attach_only = false` (the default), behavior is unchanged:
discovery uses `compound_class_subgraphs` as before.

`SkipReason::NoMatchingPattern` is removed. It was defined in
ADR 0014 for the old attach flow, where canonicals without matches
were rejected. Under the new attach flow, we never iterate canonicals
without matches — we iterate known patterns — so the variant has no
producer and is dead code. Removing it keeps the public enum minimal.

## Alternatives considered

- **Keep compound-class enumeration and accept the fragmentation
  limit.** Rejected after user pushback. One reproducible
  counter-example (fresh 2-chain failing to attach) is sufficient
  evidence that the enumeration strategy is wrong for the attach
  use case. Deferring "until more use cases" would accumulate silent
  failures.
- **Replace compound-class enumeration entirely.** Rejected.
  Compound-class enumeration remains the right primitive for
  *discovery* — it surfaces sites of repetition without a target
  canonical. Rip-and-replace would regress discovery. The right
  split is "compound-class for discovery, subgraph matching for
  verification."
- **Keep `NoMatchingPattern` for manual callers.** Deferred. No
  current producer; re-introducing it when a real use case appears
  is cheaper than carrying dead variants.
- **Use subgraph matching for *both* discovery and attach.**
  Rejected. Discovery doesn't know what to match against — its
  whole point is to find candidate canonicals emergent from the
  data. Compound-class enumeration is the right heuristic.
- **Add endpoint-profile sketch pruning now.** Deferred per
  minimum-first. The naive enumeration is fast enough at current
  scale; adding sketches later is a pure performance refinement
  that won't change outcomes.

## Consequences

- **Attach-only handles asymmetric structures correctly.** A fresh
  2-chain, tree-branch, or any other structure whose edges don't
  share a compound fingerprint will attach to a matching pattern.
  This is the core fix.
- **Two enumeration primitives coexist.** `compound_class_subgraphs`
  for discovery, `find_instances_of` for verification. Each has its
  place; neither claims to solve the other's problem.
- **Attach-pass decision structure simplifies.** Each decision is
  per-pattern (not per-canonical-group). Every returned entry
  describes "did this pattern gain new instances?"
- **No impact on discovery.** Default `run_naming_pass` behavior is
  identical to ADR 0012. Existing discovery tests pass unchanged.
- **Loss: `SkipReason::NoMatchingPattern` goes away.** Tests and
  examples that matched on it are updated.
- **Cost: naive enumeration is combinatorial.** O(|data|^k) in
  worst case for pattern-of-size-k. At β scale this is acceptable;
  larger graphs will need pruning. Documented as a known limit,
  not blocking.

## Implementation

- Source: `v2/src/lib.rs` — new `find_instances_of` method plus
  private helper `expand_connected`; rewired attach branch of
  `run_naming_pass`; removed `SkipReason::NoMatchingPattern`;
  private `data_edges` helper.
- Tests: 3 new unit tests — `find_instances_of` detects asymmetric
  chain after naming, attach-only attaches asymmetric chain to p_2,
  `find_instances_of` returns empty for novel canonical. Existing
  attach-only tests updated for the new semantics.
- Example: `v2/examples/subgraph_matching.rs` — runs the same
  discovery + new-data scenario as ADR 0014's `attach_only`
  example and shows the asymmetric chain now attaching.
- Experiment log: `v2/logs/2026-04-23_subgraph_matching.log`.
