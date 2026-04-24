# 0051: Adaptive drive config

Status: Accepted
Date: 2026-04-24

## Context

ADR 0050 showed sampling mode pushes the drive's working envelope
to 1000+ edges. But callers still need to pick `sample_count`,
`pattern_sizes`, and whether to enable sampling at all — hardcoded
numbers that don't adapt to the RSet at hand.

Task 5 of the 1'''→5''' round: give `RSet` a method that auto-tunes
a `DriveConfig` based on the RSet's own size and density. This is
the smallest meaningful step toward "the system picks its own
parameters" without building a full adaptive-learning loop.

## Decision

### `RSet::adaptive_drive_config(base) -> DriveConfig`

Takes a base config, returns a tuned one. Rules:

1. **Drop sizes that can't fit**: `pattern_sizes.retain(|&k|
   data_edges >= k)`. Can't discover a 3-edge pattern in a
   2-edge graph.
2. **Scale discovery sample_count**: `discovery_config.sample_count
   = (data_edges * 2).clamp(50, 1000)`. Larger graphs explore more
   candidates, but capped so degenerate inputs don't blow up.
3. **Enable instance_sampling at scale**: if `data_edges > 300`
   and `instance_sampling` is `None`, set it to
   `SamplingMatchConfig { sample_count: (edges*2).clamp(100, 2000), rng_seed }`.
4. **Respect explicit caller choices**: if `instance_sampling` is
   already `Some(...)`, leave it alone.

`base` is taken by value; the function returns a new `DriveConfig`.
`self` is read-only.

### What's NOT auto-tuned

- `naming_policy` — left to caller; policy choice is more about
  semantic preferences than scale.
- `axiom_config` — likewise; strictness / confidence thresholds
  are domain-specific.
- `max_steps` / `epsilon` — caller picks the budget.
- `enable_prune` / `prune_threshold` — policy.

## Alternatives considered

- **A full `DriveConfig::auto()` that returns a fresh default**.
  Rejected — callers benefit from specifying partial config
  (naming policy, axiom settings) and having the adaptive layer
  only adjust scale-dependent fields.
- **Mid-run adaptation**: after each drive step, re-evaluate the
  config. Rejected for this ADR — the drive's greedy semantics
  already handle "no improvement → halt"; dynamic re-tuning in
  the middle of a run is a bigger concept.
- **Learn from prior drives**: record which configs worked on
  similar RSets, pick by similarity. Deferred — requires a prior
  corpus which v2 doesn't have.
- **Expose all heuristic thresholds as config**. Rejected for
  this ADR — the numbers (300, 50/1000, 100/2000) are clearly
  heuristic; a future ADR can parametrize if use cases surface.

## Consequences

### The drive becomes "one-call ready" at scale

```rust
let cfg = rs.adaptive_drive_config(DriveConfig::default());
let trace = rs.intrinsic_drive(&cfg);
```

Previously the caller had to manually size-check and choose
sampling. Now `DriveConfig::default()` + `adaptive_drive_config`
handles scale. This is the first v2 mechanism where the system
**chooses its own performance parameters** based on inspecting
its own state.

### Not yet "fully autonomous"

Still caller-triggered (they invoke `intrinsic_drive`) and
caller-scoped (they pick `max_steps`). But the friction of
manual parameter tuning is now gone at the scale layer.

### Interaction with ADR 0043 sampling path

Caller who's already chosen sampling explicitly gets their choice
preserved. Caller leaving it `None` gets auto-enabled at
`data_edges > 300`. This threshold is the point where ADR 0041's
exhaustive-mode growth starts biting.

## Verification

- 276 → 282 tests pass (6 new: small RSet no-sampling, large
  enables sampling, drops too-big pattern sizes, scales
  sample_count correctly, respects explicit sampling, clamps
  extreme counts).

## Implementation

- `v2/src/lib.rs` — `RSet::adaptive_drive_config`.
- `v2/docs/decisions/0051-adaptive-drive-config.md` — this ADR.
