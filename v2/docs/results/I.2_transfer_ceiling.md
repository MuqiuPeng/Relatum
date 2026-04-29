# I.2 — Transfer ceiling test (OQ#1 axioms → OQ#2 rset)

**Status**: ✓ done (2026-04-30)
**Log**: [`logs/2026-04-30_phase_i2_transfer_ceiling.log`](../../logs/2026-04-30_phase_i2_transfer_ceiling.log)
**Example**: [`examples/phase_i2_transfer_ceiling.rs`](../../examples/phase_i2_transfer_ceiling.rs)

## Goal

I.1 showed strong transfer between OQ#1 and long5k (same regime types). C.2.1 predicted *catastrophic* transfer failure to OQ#2 (tournament + lattice + star, fundamentally different structure). I.2 actually runs that test.

## Method

1. Train rt_oq1 on OQ#1 (1000 ticks)
2. Build raw OQ#2 RSet via stream replay (no training)
3. For each axiom in axioms_OQ1, compute precision on OQ#2 rset
4. Compare to OQ#1's within-substrate precision

## Result

13 axioms total; 11 template axioms have predictions on both rsets (predicate axioms `ax_reflexivity` and `ax_antisymmetry` excluded).

| axiom | OQ#1 (within) | OQ#2 (across) | delta |
|---|---|---|---|
| **ax_tpl_v3_p0-1_p1-2_c0-2** (universal) | 1.0000 | 0.5756 | **−0.4244** |
| **ax_tpl_v2_p0-1_c0-0** (signal) | 0.6667 | 0.3472 | **−0.3194** |
| **ax_tpl_v2_p0-1_c1-1** (signal) | 0.5714 | 0.3472 | **−0.2242** |
| ax_tpl_v3_p0-1_p1-2_c2-0 | 0.4444 | 0.3487 | −0.0957 |
| ax_tpl_v3_p0-1_p2-1_c0-2 | 0.4500 | 0.4202 | −0.0298 |
| ax_tpl_v2_p0-1_c1-0 | 0.2667 | 0.4085 | **+0.1419** |
| ax_tpl_v3_p0-1_p0-2_c1-2 | 0.3600 | 0.4202 | **+0.0602** |
| ax_tpl_v3_p0-0_p1-2_c0-1 (noise) | 0.0750 | 0.0389 | −0.0361 |
| ax_tpl_v3_p0-0_p1-2_c1-0 (noise) | 0.0750 | 0.0389 | −0.0361 |
| ax_tpl_v3_p0-0_p1-2_c0-2 (noise) | 0.0643 | 0.0389 | −0.0254 |
| ax_tpl_v3_p0-0_p1-2_c2-0 (noise) | 0.0643 | 0.0389 | −0.0254 |

### Aggregate

- OQ#1 within precision (mean): **0.3671**
- OQ#2 across precision (mean): **0.2749**
- **Ratio: 0.7488** — partial transfer

## Verdict

**PARTIAL TRANSFER (ratio 0.75) — more nuanced than C.2.1's predicted catastrophic failure.**

The aggregate masks a bimodal pattern:
- **Signal/universal axioms collapse**: −0.42 (universal), −0.32, −0.22 — these were OQ#1's load-bearing predictors. On OQ#2 they lose significant precision.
- **Noise axioms stay flat**: −0.03 to −0.04 — already low on OQ#1, similarly low on OQ#2.
- **A few axioms IMPROVE**: `ax_tpl_v2_p0-1_c1-0` and `ax_tpl_v3_p0-1_p0-2_c1-2` predict OQ#2 BETTER than OQ#1 (+0.14 and +0.06). These were mid-range axioms on OQ#1; they happen to align with OQ#2's structure.

## Why this is more nuanced than C.2.1 predicted

C.2.1 predicted catastrophic failure based on the *axiom-discovery* path (no template axioms found on OQ#2 because of transitivity violations). That was about discovery — the system couldn't *find* axioms in OQ#2.

I.2 tests *transfer* — taking OQ#1-discovered axioms and applying them to OQ#2 data. The picture differs:
- Universal predictors (transitivity-style) suffer most because OQ#2 deliberately violates transitivity (tournament regime)
- Mid-range axioms with limited variable counts (2-var) sometimes survive — they encode simpler patterns that aren't regime-specific
- Noise axioms have no expected behavior, so they're robust to substrate change in a trivial way

## What this slice produced

1. Empirical answer to "does training transfer to fundamentally different substrates?": **partially — 75% precision retention on aggregate, but bimodal**
2. Refinement of C.2.1's prediction: catastrophic for *discovery*, partial for *transfer*
3. Per-axiom breakdown showing which axioms transfer (low-arity, mid-range) vs which collapse (universal, high-confidence)
4. Counterexample to "training is fully substrate-specific" hypothesis — some structure DOES generalize even across regime families

## Comparison: I.1 vs I.2

| dimension | I.1 (OQ#1 ↔ long5k) | I.2 (OQ#1 → OQ#2) |
|---|---|---|
| ratio | 1.0040 | 0.7488 |
| substrate similarity | high (same regimes) | low (different regimes) |
| top axiom impact | +0.14 (signal gains on longer stream) | −0.42 (signal collapses) |
| noise axiom impact | −0.02 to −0.03 | −0.03 to −0.04 |
| verdict | strong transfer | partial transfer |

I.1 + I.2 together establish the **transfer spectrum**:
- Same regimes → strong transfer (≈ 1.0)
- Different regimes (but same primitive R) → partial transfer (≈ 0.75)
- Adversarial substrates that violate axiom assumptions → would presumably push lower (untested, hypothesis: ≈ 0.5)

## Future implications

- **Transfer floor**: what's the worst-case transfer? Construct an adversarial substrate (e.g., NEGATIONS of OQ#1 patterns) and measure
- **Transferable subset**: identify which axioms transfer by structural property (low arity? non-cycle-dependent?) — predict transfer without testing
- **Fine-tuning recipe**: when transfer fails, can a small adaptation step (re-evaluate, re-rank, demote) recover most of the gap? OQ#1 → OQ#2 + 100 ticks of further training might converge
- **C.2.1's verdict refinement**: the "structural-bound" finding holds for axiom DISCOVERY but the boundary is softer for axiom TRANSFER. Worth a clarifying note in C.2.1's result file.

## Methodological note

The catastrophic-drop detector in this experiment used the criterion "precision halved AND original > 0.3". On OQ#2, no axiom passed this (universal dropped from 1.0 → 0.58, just barely above the halving threshold). The threshold is somewhat arbitrary; tightening to "precision dropped > 0.40" would catch the universal axiom case.

Bimodal aggregate behaviors don't reduce cleanly to a single ratio. Future transfer-related slices should report distribution statistics (median, quartiles) alongside means.
