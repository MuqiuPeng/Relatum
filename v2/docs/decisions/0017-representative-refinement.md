# 0017: Representative refinement for motif candidates

Status: Accepted
Date: 2026-04-23

## Context

ADR 0016's sample-score-select surfaces motif candidates but
makes no guarantee about the quality of each candidate's
*representative* — the specific `Subgraph` instance attached to a
canonical form. The experiment showed the first candidate at
target_size=2 was `{R(k1,k2), R(k3,k1)}` — structurally a 2-chain,
but embedded in the 3-cycle (participants `{k1, k2, k3}` induce
3 data edges, not 2; not a *clean* instance per ADR 0015's
cleanness filter).

For downstream use (especially the future "motif → pattern"
pipeline in ADR 0018), the caller typically wants a clean
representative: one whose participant set induces exactly `k`
edges, satisfying ADR 0010's canonical-recovery invariant.

ADR 0017 adds a refinement step that takes a list of
`MotifCandidate`s and, for each, tries to improve the
representative — specifically, tries to find a *clean* representative
with the same canonical form when the input representative is not
clean.

## Decision

### Refinement strategy

**Targeted re-sampling, not local edge-swap hill climb.**

Why re-sampling rather than local swaps: a pure edge-swap hill
climb cannot escape local structural neighborhoods. A 2-chain
embedded in a 3-cycle cannot be swapped to a clean 2-chain because
every single-edge swap within the cycle stays within the cycle.
Escape requires a long jump — exactly what re-sampling from a new
seed provides.

Re-sampling is still in the propose-score-refine family (per the
`v2_search_mode` memory): the refinement proposes new candidates
with a constraint ("must match target canonical AND be clean"),
scores trivially (pass/fail), and accepts the first match.

### API

```rust
pub struct RefinementConfig {
    pub max_tries: usize,   // per-candidate re-sampling budget
    pub rng_seed: u64,
}

impl RSet {
    pub fn is_clean_subgraph(&self, sg: &Subgraph) -> bool;
    pub fn refine_candidates(
        &self,
        candidates: Vec<MotifCandidate>,
        config: &RefinementConfig,
    ) -> Vec<MotifCandidate>;
}
```

### Algorithm

For each candidate `c` in the input:

1. If `c.representative` is already clean (via
   `is_clean_subgraph`), leave it unchanged.
2. Otherwise, for up to `max_tries` attempts:
   a. Draw a random-walk subgraph of size `c.canonical.len()` from
      the RSet's data edges (same sampler as ADR 0016's
      `sample_connected_subgraph`).
   b. Canonicalize it. If canonical matches `c.canonical` AND the
      subgraph is clean, accept as the new representative and stop.
3. If no clean representative is found within the budget, keep the
   original.

The RNG is threaded through the whole candidate list so one
refinement call is deterministic under `rng_seed`.

## Alternatives considered

- **Local edge-swap hill climb.** Rejected as primary strategy.
  Cannot escape tight structural neighborhoods (cycle interiors
  etc.). Could be added as a complement in a future ADR, but
  alone it is insufficient.
- **Simulated annealing / basin hopping.** Deferred. The
  re-sampling approach already jumps between basins via random
  seeds — the philosophical thing SA brings (energy-function
  guided moves) is absent here because the goal is binary
  (clean / non-clean), not a continuous objective.
- **Canonical refinement** (search for nearby canonicals with
  higher score). Rejected for this ADR — it is "what motif" drift,
  not "where to pin the same motif." A separate ADR if ever needed.
- **Expose the refinement as a free function.** Rejected. Making
  it a method on `RSet` matches the rest of the API and reuses
  the internal sampler and cleanness check.
- **Return a `Result` with error for unsuccessful refinement.**
  Rejected. "No clean representative available" is a legitimate
  outcome, not an error. Callers inspect `is_clean_subgraph` on
  the output if they need to know.
- **Refine in-place with `&mut`.** Rejected. The function takes
  `Vec<MotifCandidate>` by value and returns a new Vec —
  consistent with functional composition and avoids aliasing the
  input.

## Consequences

- **Motif candidates can now carry clean representatives when
  the data contains one.** This makes them directly usable by any
  downstream consumer that assumes ADR 0015's cleanness invariant
  (including the future motif → pattern pipeline).
- **Budget-limited.** If a clean representative exists but is
  rarely sampled, the refinement might miss it. `max_tries` is
  tunable per call. Empirically at β's scale, a few dozen tries
  is sufficient.
- **Determinism is preserved** via a single RNG thread per call.
- **Small API expansion.** `is_clean_subgraph` is exposed to
  callers that want to make their own cleanness-aware decisions
  (was previously private to `find_instances_of`'s retain call).
- **Does not change `discover_motifs`'s output.** Refinement is an
  optional post-step. The caller composes:
  `rs.refine_candidates(rs.discover_motifs(&config), &refine_cfg)`.

## Implementation

- Source: `v2/src/lib.rs` — `RefinementConfig`,
  `RSet::is_clean_subgraph` (extracted from the in-line check
  inside `find_instances_of`), `RSet::refine_candidates`.
- Tests: 4 new unit tests — refine on clean-already is no-op,
  refine on non-clean with clean alternative available produces
  a clean rep, refine on non-clean with no alternative is
  unchanged, determinism under fixed seed.
- Example: `v2/examples/motif_discovery.rs` extended with a
  refinement pass to show before/after representatives.
- Experiment log: `v2/logs/2026-04-23_motif_refinement.log`.
