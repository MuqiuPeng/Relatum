# 0043: Indexed RSet + sampling-path for autonomous_pass

Status: Accepted
Date: 2026-04-24

## Context

ADR 0041 located the scale bottleneck in `find_instances_of` —
exhaustive enumeration of connected k-edge subgraphs matching a
candidate canonical form. ADR 0041 named two natural optimizations
and declined to do them; ADR 0043 is those two ADRs combined:

1. **Indexed RSet**: add source-keyed and target-keyed indices so
   `left_of(x)` / `right_of(y)` are O(|edges from x|) instead of
   O(|all edges|).
2. **Sampling-path for autonomous_pass**: an opt-in config flag
   that routes `autonomous_pass`'s instance-collection step
   through `sample_instances_of` (ADR 0024) instead of the
   exhaustive `find_instances_of`.

## Decision

### Indexed RSet

```rust
pub struct RSet {
    instances: HashSet<R>,
    by_source: HashMap<String, HashSet<R>>,   // new
    by_target: HashMap<String, HashSet<R>>,   // new
}
```

`add` and `remove` keep the indices synchronized with `instances`.
`left_of` and `right_of` read from the indices.

Equality is explicitly defined only by `instances` (a manual
`impl PartialEq`) — two RSets built via different insertion orders
compare equal regardless of index state.

### Sampling-path for autonomous_pass

```rust
pub struct AutonomousConfig {
    // ... existing fields
    pub instance_sampling: Option<SamplingMatchConfig>,   // new
}
```

When `Some(cfg)`, `autonomous_pass` calls
`self.sample_instances_of(&canon, cfg)` instead of
`find_instances_of(&canon)`. Trades completeness for tractability
on large graphs — sampling may under-report the full instance set.

`DriveConfig` has a matching `instance_sampling` field propagated to
every `DiscoverPatterns` action. Default `None` preserves the
exhaustive path.

## Alternatives considered

- **BTreeMap-based indices** for sorted iteration. Rejected —
  HashMap is faster on lookup; sorted iteration isn't needed here.
  If a future component wants sorted views, it can `.sorted()` on
  demand.
- **Make `instance_sampling` a runtime dial on `find_instances_of`
  itself**, not on `autonomous_pass`. Rejected — mixes the two
  contracts (`find_instances_of` promises exhaustive; sampling
  breaks that). Kept as a separate opt-in at the higher level.
- **Cache `canonical_structure` / `role_ids` per-pattern**.
  Considered; decided against for this ADR — the existing
  `pattern_roles` / `pattern_structure` queries go through the
  index, and are cheap enough after 0043. Revisit if a profile
  shows them still dominating.

## Consequences

### Measured speedup on scale benchmark

Re-running the ADR 0041 benchmark (release mode, same seed):

| edges | drive time pre-0043 | drive time post-0043 | speedup |
|-----:|--------------------:|---------------------:|--------:|
|  50  |       373 ms        |       364 ms         |   2%    |
|  100 |      2.26 s         |      2.04 s          |  10%    |
|  200 |     30.2 s          |     29.2 s           |   3%    |
|  400 |    255.1 s          |    207.8 s           |  18%    |

Modest but real. The indexed RSet speeds up every `left_of` /
`right_of` call, which appear in many places (check_reflexivity,
pattern query, collect_meta_ids, memberships_of, etc.). It does
**not** fix the deeper bottleneck — `find_instances_of`'s
enumeration still dominates pattern discovery at scale.

### Sampling path enables larger graphs

Sampling-mode `autonomous_pass` does not scale with edge count
the same way. It samples `sample_count` random walks of the target
size and returns matching ones; time is O(sample_count × k) per
candidate, independent of total edge count. At 1000–10000 edges
this is a meaningful win when completeness is not required.

Trade-off: sampling may miss instances, so `(N - 1) × k` reuse
savings may under-count, and the drive picks smaller reward deltas.
Caller chooses.

### Memory cost

Indexed RSet adds ~2× memory overhead (each edge is stored in
`instances`, `by_source`, and `by_target`). For β-scale this is
invisible; for very large RSets it's a real cost. Documented as
an honest trade-off.

### Backward compatibility

- Every existing test continues to pass (236 total, was 230).
- Default `AutonomousConfig` / `DriveConfig` set
  `instance_sampling: None` → exhaustive path preserved.
- RSet's public API is unchanged; callers see only speedup.

## Verification

- 230 → 236 tests pass (6 new: index-instance consistency, index
  survives remove, equality ignores indices, clone carries indices,
  sampling-mode autonomous_pass smoke test, drive-with-sampling
  smoke test).
- `cargo run --release --example scale_benchmark` shows the
  before/after timings above.

## Implementation

- `v2/src/lib.rs` — `RSet` field additions, `add`/`remove`/
  `extend`/`left_of`/`right_of` rewrites, manual `PartialEq`,
  `AutonomousConfig::instance_sampling` /
  `DriveConfig::instance_sampling`, branch in `autonomous_pass`,
  11 test-site patches.
- `v2/docs/decisions/0043-indexed-rset-and-sampling-path.md` —
  this ADR.
