# 0021: Multi-size autonomous sweep

Status: Accepted
Date: 2026-04-23

## Context

`autonomous_pass` operates at one `target_size` per call. On a graph
that contains patterns at multiple sizes — a 2-chain, a 3-star, a
4-cycle — the caller currently invokes `autonomous_pass` three
times, each with a different size. That is a minor usability cost
but also a source of bugs: forgetting a size means missing patterns
that exist in the data.

`autonomous_sweep` is a thin wrapper that runs `autonomous_pass`
once per requested size and returns the outcomes grouped. Each
pass sees the *updated* RSet (patterns from earlier sizes are
already named), so sweeps naturally deduplicate across sizes.

## Decision

```rust
impl RSet {
    /// Run `autonomous_pass` once per target size. Each call uses the
    /// base `AutonomousConfig` with `discovery.target_size` set to
    /// the current size and `discovery.rng_seed` offset by the size
    /// so different sizes sample independently. Outcomes are grouped
    /// by size. Earlier sizes' patterns persist into later passes,
    /// so sweeps deduplicate naturally via the registry. ADR 0021.
    pub fn autonomous_sweep(
        &mut self,
        base: &AutonomousConfig,
        sizes: &[usize],
    ) -> Vec<(usize, Vec<AutonomousOutcome>)>;
}
```

Per size:
1. Clone `base`.
2. Override `discovery.target_size` with the current size.
3. Offset `discovery.rng_seed` by the size (so sizes sample
   differently: size 2 gets seed+2, size 3 gets seed+3, etc.).
4. Invoke `autonomous_pass(cfg)`.

Seeds offset by size ensures that running at size 2 and then size 3
doesn't use identical random walks; at the same time, each sweep
remains deterministic given the same base config.

## Alternatives considered

- **Take a `Vec<AutonomousConfig>`** so the caller fully specifies
  each pass. Rejected: most callers want the same policy and
  refinement across sizes; varying `target_size` is the only
  difference. Accept a base plus a list of sizes, expand internally.
- **Randomize seed independently per size.** Rejected: would make
  reproducibility across sweeps painful. A deterministic per-size
  offset from the base seed keeps reproducibility and achieves
  diversification.
- **Run sizes in parallel.** Rejected: `autonomous_pass` takes
  `&mut self`; sequential is cleaner, and each size needs to see
  the registry state from prior sizes so that rediscovered patterns
  are reported as `Existing` rather than re-named.
- **Skip sizes where no data edges of that size could exist** (e.g.,
  size > total data edges). Rejected: no harm in asking and getting
  empty outcomes; simpler to let `autonomous_pass` handle the case.

## Consequences

- Callers wanting "all patterns up to size k" call once instead of k
  times. Easy to bundle.
- Later sizes see earlier sizes' meta-R in the RSet. Since
  `discover_motifs` samples data edges only (meta-R excluded via
  `data_edges_sorted`), the meta-R from earlier sizes does not
  pollute later-size sampling. Order of sizes is therefore mostly
  cosmetic; outputs differ only in pattern id assignment order.
- Seed offset is a pragmatic choice; a future ADR could adopt a
  different strategy (per-size full re-seeding from a PRNG stream,
  for example) without changing the external API.

## Implementation

- Source: `v2/src/lib.rs` — `RSet::autonomous_sweep`.
- Tests: 3 new unit tests — empty sizes vec returns empty; single
  size matches a direct `autonomous_pass` call; multiple sizes
  accumulate patterns with `Existing` outcomes on a second sweep.
- Example: `v2/examples/autonomous_sweep.rs` — sweep over
  `[2, 3, 4]` on the mixed graph.
- Experiment log: `v2/logs/2026-04-23_autonomous_sweep.log`.
