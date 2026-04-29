# D.3.1 — Signals-disagree substrate (composite arbitration test)

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_d31_signals_disagree.log`](../../logs/2026-04-29_phase_d31_signals_disagree.log)
**Example**: [`examples/phase_d31_signals_disagree.rs`](../../examples/phase_d31_signals_disagree.rs)

## Goal

D.3 showed composite signal works mechanically but on OQ#1 both signals always rank t_0 lowest — arbitration moot. D.3.1 attempts a NARROW substrate (regime A only — diamond posets, no bipartite/equivalence/markers) hoping for signal disagreement.

## New stream

`src/test_substrates/narrow_a.rs` — only OQ#1's regime A (5 phases × 100 ticks). Diamond posets, no other regime types.

## Result

5 theories, 16 axioms after 1000 ticks:

| theory | primary | cross-prec | composite |
|---|---|---|---|
| **t_0** | **0.3991** | **0.2436** | **0.3213** |
| t_1 | 0.5957 | 0.5124 | 0.5541 |
| t_2 | 1.0000 | 1.0000 | 1.0000 |
| t_3 | 0.9427 | 0.7500 | 0.8463 |
| t_4 | 1.0000 | 1.0000 | 1.0000 |

All three signals pick **t_0** as bottom. **AGREE — arbitration moot**.

## Verdict

**NULL on hypothesis** (no disagreement), **POSITIVE on methodology**.

Methodologically, this is a structural finding: **on naturally-discovered theories, primary-rate and cross-precision correlate strongly because they measure the same underlying property** (axiom validity on data, just from primary-stream vs imagined-substrate angles).

Magnitudes differ — t_3 has primary 0.9427 but cross 0.7500. But ranking-by-bottom doesn't change.

## Why disagreement is structurally rare

For genuine signal disagreement, an axiom would need:
- HIGH primary-rate (correctly predicts edges in the actual stream)
- LOW cross-precision (wrongly predicts edges on imagined substrates)

This requires the axiom to be **regime-specific** (perfect on one regime, broken on others) AND for the primary stream to over-represent that regime. Naturally-discovered axioms tend to be either structurally valid (high on both) or structurally broken (low on both).

Engineering disagreement would require:
- Theory A: contains an axiom that's only valid on regime X
- Primary stream: 100% regime X → primary-rate = 1.0
- Imagined substrate from theory A: includes regime X structure ⇒ cross-precision also high
- ...still no disagreement

The architecturally-correct way to expose disagreement: an axiom whose primary stream gives high cumulative count but whose generated substrate is too narrow to verify it. Hard to engineer cleanly.

## Implications for D.3 composite signal

The composite signal's value is NOT primarily arbitration (signals tend to agree). The value is:
- **Robustness**: if one signal fails (e.g., insufficient predictions, or NaN), composite still has the other
- **Smoothing**: at small T, primary-rate is noisy; cross-precision is structural; blend is more stable
- **Defensive default**: if a future substrate produces disagreement, composite has a coherent answer

D.3 stays POSITIVE on those grounds. D.3.1 confirms the agreement strength is structural, not coincidence.

## What this slice produced

1. New `narrow_a` substrate (regime A only) for future targeted experiments
2. Empirical: signals AGREE even on narrow substrate
3. Methodological: composite arbitration is rare-event insurance, not common-case decision aid
4. Clarified D.3's value proposition: robustness over arbitration

## Follow-ups deferred

- Future substrate engineered for explicit disagreement (regime-specific axioms with biased primary stream)
- Multi-α composite sweep to find blend that minimizes worst-case verdict drift
