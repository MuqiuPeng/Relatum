# 0016: Motif discovery via sample-score-select (not enumeration)

Status: Accepted
Date: 2026-04-23

## Context

Three findings converge on this ADR:

1. ADR 0015's residual: the discovery pipeline (0007 + 0008 + 0009 +
   0012) misses asymmetric novel structures. It relies on
   `compound_class_subgraphs` which groups by compound fingerprint;
   asymmetric motifs don't form a clean single-group.
2. The user's explicit stance (now in memory as `v2_search_mode.md`):
   exhaustive enumeration is not the natural means. The system must
   choose a few diversified candidates, score them, and iteratively
   refine the best.
3. ADR 0015's `find_instances_of` itself uses BFS enumeration — it
   works at current scale but is philosophically at odds with the
   architecture. Motif discovery must not make the same mistake.

This ADR introduces the **first propose-score-select mechanism** in
v2. It is intentionally scoped small — "propose + score + select,"
skipping "refine" — because minimum-first and because the first
concrete refinement strategy is not yet evident without operating
the simpler loop. A later ADR can add refinement once we see where
the sample + rank output falls short.

## Decision

### The mechanism

```rust
pub struct DiscoveryConfig {
    pub target_size: usize,        // edge count of candidate subgraphs
    pub sample_count: usize,       // N: number of candidates to propose
    pub top_m: usize,              // M: keep top-M by score
    pub rng_seed: u64,             // deterministic reproducibility
}

pub struct MotifCandidate {
    pub canonical: CanonicalForm,
    pub representative: Subgraph,
    pub sample_frequency: usize,   // how many of the N samples hit this canonical
    pub score: f64,                // currently equals sample_frequency as f64
}

impl RSet {
    pub fn discover_motifs(&self, config: &DiscoveryConfig) -> Vec<MotifCandidate>;
}
```

### Algorithm

Three steps, in order:

1. **Propose (N diverse candidate subgraphs).** For each of N draws:
   a. Pick a seed edge at random from the RSet's *data* edges (meta-R
      excluded, per ADR 0010's layering).
   b. Grow the subgraph by random walk: at each step, pick a random
      adjacent edge (shares an identifier with the current subgraph,
      not already in it). Repeat until size equals `target_size`.
   c. If the seed's connected component is too small to reach
      `target_size`, discard this draw and continue.

2. **Score (sample-frequency).** For each distinct canonical form
   appearing in the samples, count how many samples produced it.
   That count is the candidate's `sample_frequency`. The candidate's
   score is currently `sample_frequency as f64` (identity).
   Distinctness of diverse candidates feeds back through canonical
   grouping — samples that are structurally identical collapse.

3. **Select top M.** Sort distinct canonicals by score descending,
   truncate to the first M, return.

**Randomness source:** an inline xorshift64 PRNG seeded from
`config.rng_seed`. Deterministic for tests; no external crate added.

### What counts as "diverse candidates"?

Diversification is delegated to the randomness of the seed pick and
the random walk. For the first pass, no explicit anti-correlation
between samples is enforced. If concentration becomes a problem
(e.g., most samples landing in the same region), a future ADR can
add seed-profile-aware rejection sampling.

### Refine is explicitly deferred.

The propose-score-select loop runs once per call. "Refine" — perturb
the top candidate and rescore — is left for ADR 0017 if it becomes
clear what move set and score function the refinement needs. Running
the minimum loop first tells us whether the top-M outputs are
already useful or whether they need local improvement.

## Alternatives considered

- **Enumerate all connected k-subgraphs** (like `find_instances_of`
  in ADR 0015). Rejected — directly contradicts the `v2_search_mode`
  memory. It's the current state of `find_instances_of`, and ADR
  0016 exists precisely so that a second version of the same mistake
  isn't coded.
- **Use `compound_class_subgraphs` as the source of candidates.**
  Rejected — reintroduces the fragmentation problem ADR 0015
  sidestepped. Sampling from raw data edges is more faithful to the
  structural space.
- **Score by MDL / compression gain.** Deferred. MDL needs a coding
  scheme over RSet entries and a counterfactual comparison — more
  machinery than this ADR should introduce at once. Sample-frequency
  is the cheapest meaningful scoring function and gives a baseline;
  MDL can replace or complement later.
- **Multi-step refinement loop with annealing.** Deferred. The
  minimum-first practice says to run the simpler version first, see
  what goes wrong, then add. No concrete motivation yet.
- **Require deterministic output without a seed.** Rejected — the
  whole point of sampling is stochastic exploration. Determinism is
  recovered via `rng_seed`; tests fix the seed to compare outputs.
- **Add `rand` crate dependency.** Rejected for now. Inline
  xorshift64 is 3 lines and sufficient. Adding a crate would be the
  first external dependency in v2 and warrants its own decision
  when there is more than one use case for it.
- **Integrate with `run_naming_pass`.** Deferred. Keep
  `discover_motifs` as a separate entry point. The caller decides
  whether to take the top candidate and name it (via
  `name_pattern_instances`). Mixing autonomous sampling with the
  existing pipeline is a follow-up decision.

## Consequences

- **First non-enumeration search mechanism in v2.** Every prior
  mechanism either scans the whole RSet or does deterministic
  grouping. `discover_motifs` is the first to make an explicit
  *choice* about what to look at, aligning with the architecture's
  commitment to system-intrinsic autonomy.
- **Stochastic output.** Reproducibility depends on `rng_seed`.
  Tests must pin the seed. Logs should record the seed.
- **Discovery gap on asymmetric structures is addressed in
  principle.** An asymmetric 3-chain or T-fork motif, if repeated
  across the RSet, has a chance of being sampled and scored. Whether
  it is *reliably* found depends on sample_count relative to RSet
  size.
- **No integration with naming yet.** Callers must explicitly take
  a `MotifCandidate` and invoke `name_pattern_instances` on its
  representative (or on a verified instance set via
  `find_instances_of`). Wiring is a later concern.
- **Sample budget is the main tuning parameter.** Too-small
  `sample_count` misses rare motifs; too-large wastes work.
  Experimentally we'll find the regime; no upfront answer.
- **The `find_instances_of` tension remains.** ADR 0015's enumeration
  is still there, still at odds with `v2_search_mode`. This ADR does
  not touch it. A future ADR can replace its guts with propose-
  score-select (given a target canonical, sample candidates and
  check matches) once the attach-pipeline pattern is well-tested.

## Implementation

- Source: `v2/src/lib.rs` — `DiscoveryConfig`, `MotifCandidate`,
  `RSet::discover_motifs`, private `sample_connected_subgraph` and
  `next_xorshift64` helpers.
- Tests: 5 new unit tests — empty RSet returns empty, deterministic
  under fixed seed, target size respected, top_m respected, finds
  the 2-chain canonical on the mixed graph (the dominant motif by
  frequency at size 2).
- Example: `v2/examples/motif_discovery.rs` — runs
  `discover_motifs` at sizes 2 and 3 on the canonical mixed graph,
  prints the top-M candidates per size.
- Experiment log: `v2/logs/2026-04-23_motif_discovery.log`.
