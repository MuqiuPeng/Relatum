# 0024: Sampling-based `sample_instances_of`

Status: Accepted
Date: 2026-04-23

## Context

`find_instances_of` (ADR 0015) enumerates every connected clean
subgraph of a target size. It is exact but `O(|data|^k)` in the
worst case. On large RSets or large target sizes, this is
prohibitive. More importantly, it is philosophically at odds with
the `v2_search_mode` memory, which says the system should *propose
candidates and choose*, not enumerate exhaustively.

ADR 0024 adds a sampling-based companion, `sample_instances_of`,
that uses random walks to propose candidates, filters to those
whose canonical matches the target, and returns the set of distinct
clean instances found within a sample budget. The guarantee is
weaker (may miss instances); the cost is much smaller.

The exhaustive `find_instances_of` is **not** replaced. Two
primitives coexist:

- **Exact** (`find_instances_of`): correctness, small scale.
- **Approximate** (`sample_instances_of`): scalability, philosophical
  alignment, acceptable miss rate.

## Decision

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingMatchConfig {
    pub sample_count: usize,
    pub rng_seed: u64,
}

impl RSet {
    /// Sampling variant of `find_instances_of`. Runs `sample_count`
    /// random walks of length `target.len()` over data edges, keeps
    /// those whose canonical equals `target` and whose participants
    /// cleanly induce exactly `k` data edges, dedups by participant
    /// set, and returns the distinct clean instances found.
    ///
    /// Never over-returns (every entry is a verified match). May
    /// under-return (sampling can miss instances). Deterministic
    /// under `rng_seed`. ADR 0024.
    pub fn sample_instances_of(
        &self,
        target: &CanonicalForm,
        config: &SamplingMatchConfig,
    ) -> Vec<Subgraph>;
}
```

Algorithm:

1. Let `k = target.len()`. If zero, return empty.
2. Collect data edges via `data_edges_sorted` (deterministic).
3. For each of `sample_count` attempts:
   - Run `sample_connected_subgraph` to propose a connected k-subgraph.
   - If its canonical equals `target` AND `is_clean_subgraph` is
     true, record it.
4. Dedup the recorded set by participant identifier multiset. Two
   candidates with the same clean participants are the same instance.
5. Return the deduped list.

## Alternatives considered

- **Replace `find_instances_of` entirely with sampling.** Rejected.
  Correctness users (attach pass, library transfer) need exhaustive
  guarantees. Downgrading their foundation would regress ADR 0015
  and ADR 0023. Coexistence is the honest path.
- **Share implementation between sampling and exhaustive via a
  trait / generic.** Rejected for now; the algorithms are different
  in kind (BFS enumeration vs. random-walk sampling). A shared
  abstraction would be forced.
- **Allow caller to pick "best effort" in find_instances_of via a
  flag.** Rejected. Two separately-named functions make the
  guarantee explicit at the call site. Flags on `find_instances_of`
  would make reading existing callers harder.
- **Return a count estimate instead of instances.** Deferred.
  Returning the instances themselves subsumes counting (`.len()`);
  a future ADR could add a `estimate_instance_count` if only the
  cardinality is wanted.
- **Use MDL gain directly on the sampled set.** `mdl_gain` still
  calls `find_instances_of` internally. A follow-up could add
  `sample_mdl_gain` that estimates gain from the sampling result;
  orthogonal to this ADR.

## Consequences

- **Scalability.** On a 1,000-edge graph with k=4, exhaustive is
  hard; sampling with 200 draws is trivial.
- **Loss is quantifiable.** The probability of missing an instance
  that occupies fraction `p` of subgraph-space is roughly `(1-p)^N`
  for N samples. Callers who care about loss can increase
  `sample_count` and re-run (determinism holds under different
  seeds).
- **`find_instances_of` semantics unchanged.** Existing code that
  relied on exhaustive counts (ADR 0019 MDL, ADR 0023 transfer,
  ADR 0015 attach) is untouched.
- **Philosophical alignment.** v2 now has a non-enumeration path
  for the matching problem, available for callers who want to use it.
- **Over-returns ruled out by construction.** Every result is
  canonical-checked AND cleanness-checked. A caller can trust that
  results are valid instances; only completeness is weakened.

## Implementation

- Source: `v2/src/lib.rs` — `SamplingMatchConfig`,
  `RSet::sample_instances_of`.
- Tests: 4 new — empty canonical returns empty, sampling on no-match
  target returns empty, sampling with enough budget approximates
  exhaustive count on the mixed graph (compare to
  `find_instances_of`), determinism under fixed seed.
- Example: `v2/examples/sample_instances.rs` — compare
  `find_instances_of` (exhaustive) and `sample_instances_of`
  (sampling) side by side across target canonicals and sample budgets.
- Experiment log: `v2/logs/2026-04-23_sample_instances.log`.
