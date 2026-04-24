# 0040: Drive auto-prune via counterfactual value

Status: Accepted
Date: 2026-04-24

## Context

ADR 0035 added `counterfactual_value` and `rank_by_counterfactual`
as a second-order signal — each named object's actual contribution
to `abstraction_score`. The ADR stated: "Nothing auto-prunes.
Using the signal to drive retraction decisions is a separate step."

Task 2 of the 1'→4' extension is that step: wire the counterfactual
ranking into the drive as a new `Prune` action so the system can
self-retract objects that aren't earning their keep.

Along the way ADR 0040 also fixes a metric oversight from ADR 0031
/ 0034: extension relations (ADR 0034) contributed zero to the
score but cost meta-R overhead, so their counterfactual value was
negative, and auto-prune would eat them. The fix is a small reward
term for extension relations.

## Decision

### Metric extension

```
score = Σ_pattern max(0, (N − 1) · k)
      + 2.0 · Σ_theory |members|
      + 1.0 · |extension_edges|     ← NEW in ADR 0040
      − 0.1 · |meta-R edges|
```

The new `+ 1.0 · |extensions|` term rewards each extension relation
enough to offset its 3-edge overhead (`-0.3`), giving net `+0.7`
per extension. That makes extensions "worth their keep" under the
metric and keeps auto-prune from removing them.

### DriveAction::Prune

```rust
pub enum DriveAction {
    DiscoverPatterns(AutonomousConfig),
    DiscoverTheory(AxiomDiscoveryConfig),
    Prune(f64),       // ← NEW
}

pub enum DriveActionResult {
    // … existing variants
    Pruned { object_ids: Vec<String> },
}
```

`Prune(threshold)` retracts every named object whose
`counterfactual_value` is strictly below `threshold`. Retraction
order: theories first (to release axiom references), then
extensions, then patterns. This avoids the "axiom still referenced"
failure path from `retract_axiom`.

### DriveConfig gets two new fields

```rust
pub struct DriveConfig {
    // … existing
    pub enable_prune: bool,       // default true
    pub prune_threshold: f64,     // default 0.0
}
```

`candidate_actions()` only includes `Prune` when `enable_prune` is
set. Default is on — the default drive now does exploration +
discovery + pruning in one loop. Callers who want the old-school
"only add, never remove" behavior opt out via `enable_prune: false`.

## Alternatives considered

- **Prune inline during retract_* calls**. Rejected — mixes
  discovery with value judgment at too low a level. Keeping
  Prune as a discrete action makes the drive trace readable
  (each step is either "discovered X" or "pruned Y").
- **Non-greedy pruning**. E.g., "try removing each; keep only the
  prune that maximizes the next-step score." Rejected for this
  ADR — the current metric is additive, so the greedy bulk prune
  is optimal.
- **Cascade pruning**. After pruning A, recompute counterfactuals
  and prune again. Rejected — `intrinsic_drive`'s loop already
  calls `drive_step` repeatedly; a subsequent step's
  `counterfactual_value` already reflects the post-prune state.
- **Larger extension reward weight**. Set `+ 2.0` to match theory
  weight. Rejected — extensions and theories have different
  "amounts of structural novelty"; the smaller `+ 1.0` is an
  honest acknowledgment that extension meta-R costs 3 edges vs. a
  theory's 1 registry + membership edges.

## Consequences

### The full drive loop is now a two-way process

Before 0040, drive was monotone: every step added objects, never
removed. After 0040, drive can also clean up. Idempotence at
saturation still holds (pruning nothing is a no-op; the drive
terminates when every action yields `Δ ≤ epsilon`).

### Counterfactual for extensions is now positive

Verified: `adr0040_extension_edges_now_reward_the_score` and
`adr0040_counterfactual_for_extension_is_positive_now`. Under the
new metric, extension counterfactual ≥ +0.7.

### Bulk metric shift

Every existing test that checked `score > 0` or `score == 0`
still passes. Extension-free RSets get the same score as before.
Extension-bearing RSets get a bump proportional to extension count.

### Limits

- **Metric still hand-tuned.** The `+1.0` for extensions and
  `+2.0` for theory members are still ad-hoc. A principled
  derivation (MDL bit cost) is future work.
- **No "reversible" prune.** Once Prune retracts an object, it's
  gone (per `retract_*` semantics). If the drive later wants it
  back, it must be re-named. Acceptable — the caller has the
  cloned RSet before the drive if they want to compare.
- **Pattern instances are not individually prunable.** Only
  whole patterns. Granularity is at the named-object level.
  Removing just one instance requires manual `retract_pattern` +
  re-naming a smaller set.

## Verification

- 217 → 222 tests pass (5 new: extension rewards score,
  extension CV positive, prune retracts negative-CV pattern,
  prune leaves positive-CV theory, enable_prune=false opt-out).

## Implementation

- `v2/src/lib.rs` — metric extension, `DriveAction::Prune`,
  `DriveActionResult::Pruned`, `DriveConfig` fields, Prune handler
  in `apply_drive_action`, inclusion in `candidate_actions`.
- `v2/docs/decisions/0040-auto-prune.md` — this ADR.
