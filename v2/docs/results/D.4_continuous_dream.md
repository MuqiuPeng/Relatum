# D.4 — Continuous dream-phase loop

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_d4_continuous_dream.log`](../../logs/2026-04-29_phase_d4_continuous_dream.log)
**Example**: [`examples/phase_d4_continuous_dream.rs`](../../examples/phase_d4_continuous_dream.rs)

## Goal

Phase Alpha-7..9 + Beta-1..6 used dream phase as a one-shot observation. D.4 runs dream phase **periodically** as part of a continuous loop, demoting whenever cross-precision drops below threshold.

## Method

For 6 phases × 300 ticks (= 1800 ticks total):
1. Run K=300 ticks with default scheduler
2. Compute cross-precision matrix (Alpha-7-style)
3. Identify lowest-mean theory
4. If mean < 0.50, retract; otherwise no-op
5. Repeat

## Result on OQ#1

| Phase | tick | lowest (cross-prec) | primary-rate | action |
|---|---|---|---|---|
| 0 | 300 | t_0 = **0.3248** | 0.4644 | DEMOTE |
| 1 | 600 | t_1 = 0.5273 | 0.5978 | no demote |
| 2 | 900 | t_1 = 0.5273 | 0.5978 | stable |
| 3 | 1200 | t_1 = 0.5273 | 0.5978 | stable |
| 4 | 1500 | t_1 = 0.5273 | 0.5978 | stable |
| 5 | 1800 | t_1 = 0.5273 | 0.5978 | stable |

After Phase 0's demote, runtime adds new theories (t_4 emerges from continued discovery). All survivors above 0.50 threshold by Phase 1; loop converges.

Phases 2-5 produce **identical numbers** — stream exhausts around tick 2000, runtime stabilizes, dream phase is a no-op.

## Verdict

**POSITIVE**. Three concrete observations:

1. **Continuous dream phase works** — 1 demote fired automatically; no manual intervention.
2. **Loop converges** — 4 stable phases post-demote with byte-identical state.
3. **Threshold is load-bearing** — t_1 at 0.5273 is just above 0.50; tighter threshold would demote it. Conservative 0.50 produces stable convergence.

## What this slice produced

1. Reusable dream-loop pattern for runtime experiments
2. Empirical confirmation that continuous dream loop on OQ#1 converges in 1 demote at K=300 cadence
3. Observation: cross-precision detected t_0 at the FIRST measurement (tick=300), matching Alpha-9's T=100 threshold-crossing finding
4. Idempotent post-convergence behavior (4 phases produce same numbers)

## Future implications

- D.4 demonstrates that dream phase can operate as a continuous safety net, not just a one-shot diagnosis
- For long-running streams with non-stationary regimes, periodic dream phase could detect drifting theory quality
- K tuning matters: K=300 is conservative (3 dream phases / 1000 ticks). K=100 would be aggressive but potentially expensive.
- Could be wired as a runtime ActionKind too (similar to B.5.1 pattern), making the loop part of the autonomous catalogue rather than an external driver

## Relationship to prior slices

| Slice | Dream phase usage |
|---|---|
| Alpha-7 | One-shot: build cross-precision matrix, observe |
| Alpha-8 | One-shot: cross-precision drives a single demote decision |
| Alpha-9 | Multi-T sweep: characterize signal convergence speed |
| **D.4** | **Continuous: dream phase runs periodically inside a loop** |

D.4 is the runtime-level realization of what Alpha-7..9 demonstrated as primitives.
