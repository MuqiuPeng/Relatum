# D.3 — Composite scheduler signal

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_d3_composite_signal.log`](../../logs/2026-04-29_phase_d3_composite_signal.log)
**Example**: [`examples/phase_d3_composite_signal.rs`](../../examples/phase_d3_composite_signal.rs)

## Goal

Blend Alpha-3+'s primary-stream rate and Alpha-8's cross-precision mean into a single composite tournament score:
```
composite(theory) = α · primary_rate + (1-α) · cross_prec_mean
```
Test if composite is more decisive than either alone, especially at small T where signals may disagree.

## Method

For each T ∈ {100, 200, 350, 500, 1000}: fresh runtime, run T ticks, compute primary-rate / cross-precision-mean / composite (α=0.5) per theory, identify the bottom theory by each signal.

## Result on OQ#1

All three signals consistently rank t_0 lowest at every T (rank-tie). Decisive metric is **first T to cross the 0.50 demote threshold**:

| T | primary (t_0) | cross (t_0) | composite (t_0) | composite < 0.50? |
|---|---|---|---|---|
| 100 | 0.5790 | **0.3756** | **0.4773** | ✓ |
| 200 | 0.5064 | 0.3248 | 0.4156 | ✓ |
| 350 | **0.4267** | 0.3248 | 0.3757 | ✓ |
| 500 | 0.4129 | 0.3248 | 0.3688 | ✓ |
| 1000 | 0.3757 | 0.3248 | 0.3502 | ✓ |

Threshold-crossing speed:
- primary: T = **350**
- cross: T = **100**
- **composite: T = 100** (matches cross)

## Verdict

**POSITIVE** on mechanism — composite math is correct, decisive at the speed of the faster of the two signals (cross-precision, in this case).

**Not yet differentiating** on OQ#1: cross-precision dominates the composite at every T because it's the more decisive signal. Primary-rate's slower convergence pulls the composite down only marginally.

## What this slice does NOT show

The hypothesized value of composite is **arbitration when signals disagree**. On OQ#1 they always agree on t_0 as bottom, so composite tracks cross-precision. The arbitration scenario would need a substrate where:
- primary-rate ranks one theory lowest
- cross-precision ranks a different theory lowest
- composite arbitrates per α

This would be a clear test of when composite > either alone. Future deferred slice (D.3.1?): construct or find such a substrate and re-run.

## Future implications

- Composite signal is now a usable scheduler input (no API change needed; just compute both signals and blend in tournament)
- α tuning: future α-sweep could reveal optimal blend
- For OQ#1-class substrates where signals agree, composite ≈ cross-precision — use cross alone if cost-sensitive
- The lookahead value of composite is in robustness: even if one signal goes haywire on a new substrate type, composite still has the other to fall back on

## What this slice produced

1. Composite signal example demonstrating equal-blend mechanism
2. Empirical confirmation: composite crosses threshold at T=100 (no slower than cross alone)
3. Methodological framing: composite's value is arbitration, not speed (speed comes from cross alone)
4. Future-slice candidate: construct a substrate where primary and cross disagree
