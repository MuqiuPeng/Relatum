# 0041: Scale benchmark (measurement only)

Status: Accepted
Date: 2026-04-24

## Context

Every v2 test to date ran at β-scale (< 20 edges). Multiple ADRs
flagged "scale behavior unknown" as an explicit limit. Task 3 of
the 1'→4' extension measures rather than optimizes: characterize
where the current pipeline's cliffs are, so future optimization
work is grounded.

## Decision

Add `v2/examples/scale_benchmark.rs`. Build deterministic random
RSets (inline xorshift64, fixed seed) at 50 / 100 / 200 / 400
edges over 20 identifiers. For each, measure:

- `discover_axioms` and `discover_axioms_minimal` wall-time
- `intrinsic_drive` wall-time, step count, final score
- `to_text` / `from_text` roundtrip time + byte count

No new library API. Pure instrumentation. Results captured in
`v2/logs/2026-04-24_scale_benchmark.log`.

## Alternatives considered

- **cargo bench with criterion**. Rejected — would introduce an
  external dev-dependency; v2 is deliberately zero-dep. A plain
  example with `std::time::Instant` is enough to expose the
  scaling shape.
- **Optimize as we measure**. Rejected — mixes measurement with
  change. ADR 0041 records the as-built envelope; optimization
  is a future ADR.

## Consequences

### Honest envelope

On my laptop with release builds:

| edges | drive time | final score | raw axioms | minimal |
|------:|-----------:|------------:|-----------:|--------:|
| 50    |    373 ms  |   811.20    |     0      |    0    |
| 100   |   2.26 s   |  1354.90    |     0      |    0    |
| 200   |   30.2 s   |   248.90    |     0      |    0    |
| 400   |  255.1 s   |     4.10    |    31      |    9    |

The interactive envelope (< 1 s) is around 50 edges; the tolerable
envelope (< minute) is around 200 edges.  Above that the full drive
loop takes minutes.

### The bottleneck is `find_instances_of`

Pattern discovery via `autonomous_pass` enumerates all connected
k-edge subgraphs matching a candidate canonical form. That scales
as `edges × avg_degree^(k-1)`. At fixed identifier count, increasing
edge count increases density quadratically, and enumeration
explodes. Axiom discovery, in contrast, scales only with template
count × `|ids|^num_vars`, which is nearly independent of edge count.

### The metric differentiates dense-random from sparse-random

Final score rises from 50 → 100 edges (more reuse opportunities)
then falls to 4.1 at 400 edges (near-complete graph has no
distinguished structural motifs). Not a bug — the metric correctly
reports that a dense random graph has **nothing abstract to name**.

### Persistence is not a bottleneck

Both `to_text` and `from_text` stay well under a millisecond per
100 KB. Byte count correlates with named-object density (meta-R
load), not raw edge count. ADR 0038's TSV format holds.

### Accidental-axiom issue surfaces

At 400 edges on 20 identifiers, 31 axioms appear at rate = 1.0
after minimization reduced to 9. These are **accidental**: random
graphs with few identifiers satisfy many universal rules by
coincidence. A "statistical significance vs. null baseline"
filter would be needed to distinguish genuine structure from
accident. Out of scope for this ADR; recorded.

## Verification

- `cargo run --release --example scale_benchmark` reproduces the
  table above (within ~10% wall-clock variance).
- All 222 tests still pass. Benchmark added zero library changes.

## Implementation

- `v2/examples/scale_benchmark.rs` — benchmark harness.
- `v2/logs/2026-04-24_scale_benchmark.log` — raw numbers + analysis.
- `v2/docs/decisions/0041-scale-benchmark.md` — this ADR.

## Open follow-ups (not scoped here)

- Route `autonomous_pass` through `sample_instances_of` (ADR 0024)
  for large RSets where exhaustive `find_instances_of` is too slow.
- Add O(1) source-index / target-index to `RSet` so repeated
  `left_of` / `right_of` amortize. Would straightforwardly widen
  the envelope 5–10×.
- Statistical-significance filter for axiom discovery on dense
  small-identifier-count graphs.
