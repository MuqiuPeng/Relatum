# 0019: MDL-gain scoring

Status: Accepted
Date: 2026-04-23

## Context

Until now, candidate scoring has been **sample-frequency** (ADR 0016):
"how many of the N random-walk samples landed in this canonical form."
That is a useful search heuristic — it tells the system which
structural shapes recur in the data — but it is not a *principled*
criterion for whether a pattern is worth naming. A single edge shows
up in every sample because sampling starts from edges; by
sample-frequency alone, the trivial "single edge" pattern dominates.

A more principled criterion is Minimum Description Length (MDL):
prefer patterns whose naming yields the largest reduction in total
description cost. Design-notes lists MDL / compression gain as a
candidate evaluation rule. This ADR adds it.

**Interpretation of "compression" in v2.** Naming a pattern in v2
does not literally shrink the RSet — it *adds* meta-R instances
(registry, ownership, participant). Under a strict byte-count model,
naming is always a loss. The useful interpretation is therefore
*reusability gain*: the total description saving that hypothetical
users of the pattern (future classifications, lookups, inferences)
would realize by referring to `p_N` instead of spelling out each
instance's structure. This is MDL-inspired, not literal MDL.

Formula: `gain(P) = (N - 1) × k` where N = clean instance count and
k = canonical edge count.

Properties:
- N = 1 (singleton): gain = 0. No reuse, no naming value.
- N > 1: gain grows linearly in both N and k. Larger patterns and
  more frequent ones score higher, together.
- Integer-valued, easy to reason about, no logs or arbitrary units.

## Decision

### New primitives

```rust
impl RSet {
    /// MDL-inspired reusability gain of naming a pattern with this
    /// canonical form. Returns (N - 1) × k where N is the clean
    /// instance count (per `find_instances_of`) and k is the
    /// canonical size. Zero for empty canonical or zero-instance
    /// patterns.
    pub fn mdl_gain(&self, canonical: &CanonicalForm) -> f64;

    /// Re-score candidates by their MDL gain (replaces the
    /// sample-frequency score). Deterministic; no randomness.
    pub fn score_by_mdl(
        &self,
        candidates: Vec<MotifCandidate>,
    ) -> Vec<MotifCandidate>;
}
```

### Policy extension

`NamingPolicy` gains one field:

```rust
pub struct NamingPolicy {
    pub min_edges: usize,
    pub min_instances: usize,
    pub skip_meta_subgraphs: bool,
    pub attach_only: bool,
    pub min_mdl_gain: f64,    // NEW. Default: 0.0 (off).
}
```

`SkipReason` gains one variant:

```rust
pub enum SkipReason {
    BelowMinEdges { edges: usize, min: usize },
    BelowMinInstances { instances: usize, min: usize },
    AlreadyKnown,
    BelowMdlGain { gain: f64, min: f64 },  // NEW.
}
```

`consider_naming` applies the MDL threshold after the existing
min_edges and min_instances checks. If `min_mdl_gain > 0.0` and the
computed gain is below it, return
`Skipped(BelowMdlGain { ... })`.

### Default is off

`min_mdl_gain` defaults to 0.0, preserving existing behavior.
Callers who want MDL filtering set it explicitly.

## Alternatives considered

- **Use `N × k` (total edges covered).** Rejected — N=1 singletons
  score k > 0, but naming a one-time occurrence has no reuse value.
  `(N-1) × k` correctly zeros out singletons.
- **Information-theoretic formula** `(N-1) × k × log|V|` where V is
  the RSet's identifier count. Rejected as over-engineered for
  minimum-first. The ordering induced by `(N-1) × k` is the same
  after dropping the log|V| factor (it is constant across all
  candidates of the same RSet).
- **True byte-count MDL.** Rejected — naming literally adds
  R instances under the v2 encoding, so byte-count MDL would always
  recommend "name nothing." The reusability interpretation is the
  faithful analog.
- **Replace sample-frequency with MDL in `discover_motifs`.**
  Rejected. `discover_motifs` operates on samples (cheap); MDL
  requires `find_instances_of` (exhaustive). Keeping them as
  separate passes gives a clear cost model: sample first, MDL-score
  the survivors.
- **Put MDL threshold in `DiscoveryConfig`** instead of
  `NamingPolicy`. Rejected — the filter belongs at the
  naming-decision stage. `DiscoveryConfig` is about what to propose;
  `NamingPolicy` is about what to accept.
- **Make `mdl_gain` return an integer instead of f64.** Rejected —
  f64 aligns with the existing `MotifCandidate::score` type and
  leaves room for refined formulas (log factors, weighted terms)
  without a type change.

## Consequences

- **MDL scoring is opt-in.** Default policy unchanged; all ADRs
  0012–0018 semantics are preserved when `min_mdl_gain = 0.0`.
- **Singletons are easily filtered.** Setting `min_mdl_gain = 1.0`
  excludes any pattern that occurs only once.
- **Larger / more-frequent patterns are preferred.** A 4-edge
  pattern with 3 instances (gain 12) outranks a 2-edge pattern with
  2 instances (gain 2) even if they have equal sample-frequency.
  This matches the intuition that richer, more repeated structures
  are more valuable.
- **Cost: `mdl_gain` calls `find_instances_of`.** That is
  enumeration, which `v2_search_mode` flagged as non-ideal.
  Contained within the scoring step; not invoked during sampling.
  Users who find this expensive at scale can skip MDL or (later)
  replace `find_instances_of`'s internals.
- **Exact, deterministic scores.** Unlike sample-frequency, MDL
  gain is not stochastic; the same canonical on the same RSet
  always yields the same gain.

## Implementation

- Source: `v2/src/lib.rs` — `RSet::mdl_gain`, `RSet::score_by_mdl`,
  `NamingPolicy::min_mdl_gain`, `SkipReason::BelowMdlGain`,
  updated `consider_naming`.
- Tests: 5 new unit tests — mdl_gain zero for singleton, non-zero
  for repeats, score_by_mdl updates scores, min_mdl_gain filters
  singletons in `consider_naming`, `autonomous_pass` honors MDL
  threshold.
- Example: `v2/examples/mdl_scoring.rs` — autonomous_pass on the
  mixed graph at target_size 2 and 3, first with default policy
  then with min_mdl_gain > 0. Diff shows which candidates MDL
  filters.
- Experiment log: `v2/logs/2026-04-23_mdl_scoring.log`.
