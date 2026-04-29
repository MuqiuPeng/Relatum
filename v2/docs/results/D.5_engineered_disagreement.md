# D.5 — Per-axiom primary/cross disagreement on OQ#1

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_d5_engineered_disagreement.log`](../../logs/2026-04-29_phase_d5_engineered_disagreement.log)
**Example**: [`examples/phase_d5_engineered_disagreement.rs`](../../examples/phase_d5_engineered_disagreement.rs)

## Goal

D.3.1 found that primary-rate and cross-precision rank theories the same way on OQ#1 and on the engineered narrow_a substrate, leading to NULL for the composite-arbitration hypothesis at the theory layer. D.5 zooms in: do the signals disagree at the **axiom** layer, even when theory rankings agree?

## Method

For every axiom on OQ#1 with sufficient predictions (≥ 5):
1. Read primary hit-rate from `prediction_state.hit_rate(ax)`
2. Compute cross-precision from `axiom_cross_precision(ax, substrates)`
3. Compute |primary − cross|
4. Identify axioms with disagreement ≥ 0.30 threshold

## Result on OQ#1 @ 1000 ticks

11 axioms had data on both signals.

| axiom | primary | cross | diff | family | disagree? |
|---|---|---|---|---|---|
| ax_tpl_v2_p0-1_c1-1 | 0.8476 | 1.0000 | 0.1524 | shape_premise_p0-1 |  |
| **ax_tpl_v3_p0-0_p1-2_c0-1** | 0.1162 | 0.4936 | 0.3774 | noise family | ★ |
| **ax_tpl_v3_p0-1_p0-2_c1-2** | 0.5044 | 0.8125 | 0.3081 | (none) | ★ |
| ax_tpl_v3_p0-1_p2-1_c0-2 | 0.5477 | 0.7859 | 0.2382 | conclusion_c0-2 |  |
| **ax_tpl_v3_p0-1_p1-2_c2-0** | 0.4679 | 0.7894 | 0.3214 | (mixed) | ★ |
| **ax_tpl_v2_p0-1_c1-0** | 0.4113 | 0.7894 | 0.3780 | (mixed) | ★ |
| **ax_tpl_v3_p0-0_p1-2_c0-2** | 0.1095 | 0.4936 | 0.3841 | noise family | ★ |
| ax_tpl_v3_p0-1_p1-2_c0-2 | 1.0000 | 1.0000 | 0.0000 | (universal) |  |
| **ax_tpl_v3_p0-0_p1-2_c2-0** | 0.1095 | 0.4936 | 0.3841 | noise family | ★ |
| **ax_tpl_v3_p0-0_p1-2_c1-0** | 0.1162 | 0.4936 | 0.3774 | noise family | ★ |
| ax_tpl_v2_p0-1_c0-0 | 0.8958 | 1.0000 | 0.1042 | premise_p0-1 |  |

**7 of 11 axioms (64%) exhibit per-axiom disagreement ≥ 0.30.**

Pearson correlation r(primary, cross) = **0.9776** — high but not perfect.

## Verdict

**POSITIVE — primary and cross genuinely disagree at the axiom layer.**

- All 4 noise-family members have `primary ≈ 0.11` but `cross ≈ 0.49` — the disagreement is concentrated in the structural noise group.
- 3 additional axioms show the same direction: primary < cross, by 0.30+
- The 4 axioms WITHOUT disagreement are either (a) universal (cross=1.0, primary=0.85+) or (b) genuinely high-quality (both signals high)

D.3.1 was right to flag NULL at the theory layer — both signals rank theories the same. But the magnitudes differ at the axiom layer, and a composite-α blend operates on those magnitudes (not rankings).

## What this means for D.3 composite signal

D.3 shipped α=0.5 composite blend. D.3.1 said "no arbitration value" because rankings always agreed. D.5 corrects this:

- At the **theory** layer, composite has no arbitration value (rankings agree)
- At the **axiom** layer, composite IS arbitrating: an α=0.5 blend produces magnitudes between primary (low) and cross (high). For the noise family, primary alone says "demote aggressively" (0.11), cross alone says "borderline" (0.49), composite says "demote conservatively" (0.30).

The arbitration is real; it's just at fine granularity rather than coarse.

## Why the noise family shows this pattern systematically

`shape_premise_p0-0_p1-2` axioms have premise `R(0,0) ∧ R(1,2)`. Bindings:
- Variable 0: any node with self-loop
- Variable 1, 2: any pair with a forward edge between them

Many bindings exist in OQ#1's ground stream → many predictions → most don't match (primary low ≈ 0.11).

On a substrate generated from a theory containing these axioms, the substrate generation CONSTRUCTS edges to satisfy the axiom (in part) → cross-precision elevated to ~0.49.

So substrate generation is *partial* — it doesn't replicate ground sparsity, but it doesn't fully satisfy axioms either. The 0.49 cross score reflects this.

## What this slice produced

1. Per-axiom primary vs cross scatter on OQ#1
2. Identification of 7 axioms with primary/cross disagreement ≥ 0.30
3. Diagnosis: noise family is the systematic source of disagreement
4. Resolution of D.3.1's NULL at axiom granularity — composite arbitration is real

## Future implications

- **Per-axiom composite signal**: instead of theory-aggregated composite (D.3), compute per-axiom and use it for axiom-level demote
- **Disagreement-based filter**: axioms with high |primary − cross| are more interesting (they're where signals disagree); could be a discovery prior
- **Substrate generation audit**: the systematic 0.49 cross score for the noise family suggests substrate generation has a structural floor (or ceiling). Worth inspecting `generate_substrate_from_theory` semantics.
- **D.5's surprise**: didn't need to engineer a substrate; OQ#1 already contains the disagreement at finer granularity. D.3.1's "engineer a stream" framing was over-aggressive.
