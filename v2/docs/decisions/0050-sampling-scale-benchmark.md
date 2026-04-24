# 0050: Large-scale sampling-mode benchmark

Status: Accepted
Date: 2026-04-24

## Context

ADR 0041 characterized the v2 pipeline at exhaustive-mode β-scale
(interactive to ~50 edges, tolerable to ~200). ADR 0043 added the
opt-in sampling-mode path for `autonomous_pass`. Task 4 of the
1'''→5''' round: actually run the sampling path at the scales
ADR 0041 couldn't reach, and publish the numbers.

## Decision

Add `v2/examples/sampling_scale_benchmark.rs`. Build deterministic
random RSets at 100 / 200 / 500 / 1000 edges, run the full drive
loop in both modes where feasible, record timings and final
scores.

No new library API. The benchmark purely exercises ADR 0043's
existing `instance_sampling` knob.

## Results

Release mode on same machine:

| edges | ids | mode       | steps | final score | wall time    |
|------:|----:|-----------|------:|------------:|-------------:|
|   100 |  48 | sampling   |   3   |   1 159.80  |   1.72 s     |
|   100 |  48 | exhaustive |   2   |   3 948.40  |   2.39 s     |
|   200 |  50 | sampling   |   3   |   1 109.00  |   2.46 s     |
|   200 |  50 | exhaustive |   2   |  20 237.70  |  38.22 s     |
|   500 | 100 | sampling   |   3   |   1 784.40  |  13.97 s     |
|  1000 | 100 | sampling   |   3   |   1 059.30  |  16.53 s     |

Exhaustive skipped at ≥ 500 edges — ADR 0041's growth curve puts
it at 10+ minutes there.

## Consequences

### The envelope pushes out ~20×

From ~50 edges interactive in ADR 0041 to ~1000 edges interactive
(< 17 s) in sampling mode. This puts v2 in reach of
small-to-medium relational datasets (graphs with hundreds to low
thousands of edges).

### Sampling under-reports reuse savings

At 200 edges, sampling's final score is 5.5% of exhaustive's. The
reuse-savings term dominates the metric, and sampling misses
many instances. Callers comparing scores across RSets must use a
consistent mode.

### Neither mode is "correct"

- Exhaustive: **complete** pattern enumeration but intractable.
- Sampling: tractable but **stochastic under-estimate**.

Both report the same structural types; they differ on per-type
instance counts. For many purposes (is there structure? what
kind?), either is sufficient. For "how many instances of each
type", only exhaustive is authoritative.

### Future work flagged (not done)

- Adaptive `sample_count` based on new-instance-discovery rate.
  Currently caller picks a number; could be data-driven.
- Lower-bound pattern counts with exhaustive probes on a random
  subset of named types; use sampling elsewhere. Hybrid mode.

## Verification

- `cargo run --release --example sampling_scale_benchmark` reproduces
  the table above (within 10% wall-time variance).
- All 276 prior tests still pass (no library change).

## Implementation

- `v2/examples/sampling_scale_benchmark.rs` — the benchmark.
- `v2/logs/2026-04-24_sampling_scale_benchmark.log` — raw data +
  analysis.
- `v2/docs/decisions/0050-sampling-scale-benchmark.md` — this ADR.
