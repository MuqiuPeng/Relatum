# ADR 0081 Phase 1.D Round 4 — multi-seed scan follow-up

**Status**: ✓ done (2026-05-11). Reinforces Round 2 retraction with N>1 evidence.
**Log**: [`logs/2026-05-11_bridge_multi_seed_scan.log`](../../logs/2026-05-11_bridge_multi_seed_scan.log)
**Example**: [`examples/bridge_multi_seed_scan.rs`](../../examples/bridge_multi_seed_scan.rs)
**Predecessor**: [`bridge_cross_substrate_canonical.md`](bridge_cross_substrate_canonical.md) (Round 2 retraction)
**Initiated by**: ARIS auto-review-loop Round 3 reviewer M3 + result-doc §13.6 follow-up list (both flagged single-seed fragility of the Round 2 retraction).

## Goal

Round 2 of the auto-review-loop replaced the methodologically empty OQ#2-self baseline with `Within(OQ#1, narrow_a) = 0.20` and retracted Phase 1.D's substrate-sensitive claim. The Round 3 reviewer (M3) and §13.6 both flagged that 0.20 was a single-seed value — possibly noise. This scan answers: across **all C(4,2) = 6 within-canonical-suite pairs** and **15 within-synth-DAG-family pairs** and **24 cross pairs**, what does the Jaccard distribution actually look like?

If the canonical-suite within-Jaccard has high mean and low std, Round 2's 0.20 was an outlier and the retraction was premature. If the mean is low and/or std is high, the retraction stands or is reinforced.

## Method

- Canonical-suite: OQ#1, narrow_a, OQ#2, long5k (v2's four established substrates).
- Synth-DAG family: 6 different graph-generation seeds, same generator function as Round 2.
- Discovery: `autonomous_pass(sizes 2-3)` at saturation budget (sample_count=400, top_m=20).
- Set equality: direct `CanonicalForm` (Round 1 W5 fix).
- All pairwise Jaccards computed.

## Results

### Within-canonical-suite (N=6 pairs)

```
Within(OQ#1,     narrow_a) = 0.2000
Within(OQ#1,     OQ#2)     = 0.1818
Within(OQ#1,     long5k)   = 0.2000
Within(narrow_a, OQ#2)     = 0.0000
Within(narrow_a, long5k)   = 1.0000
Within(OQ#2,     long5k)   = 0.0000

N=6  mean=0.2636  std=0.3406  min=0.0000  max=1.0000
```

### Within-synth-DAG (N=15 pairs)

```
All 15 pairwise Jaccards across 6 DAG seeds:
  10 pairs at 1.0000
   5 pairs at 0.8750

N=15  mean=0.9583  std=0.0589  min=0.8750  max=1.0000
```

### Cross-family (N=24 pairs)

```
                  DAG_0   DAG_1   DAG_2   DAG_3   DAG_4   DAG_5
     OQ#1        0.1875  0.1875  0.1875  0.1875  0.1875  0.1875
 narrow_a        0.0000  0.0000  0.0000  0.0000  0.0000  0.0000
     OQ#2        0.2632  0.2632  0.2632  0.2632  0.2632  0.2632
   long5k        0.0000  0.0000  0.0000  0.0000  0.0000  0.0000

N=24  mean=0.1127  std=0.1158  min=0.0000  max=0.2632
```

Note: cross values are constant across DAG seeds *per canonical-suite substrate*, because the DAG generator's canonical-form vocabulary is fully recovered at saturation (Within-synth-DAG mean ≈ 1.0).

## Statistical interpretation

| | mean | std | min | max | N |
|--|-----|-----|-----|-----|---|
| Within-canonical-suite | 0.2636 | **0.3406** | 0.0 | 1.0 | 6 |
| Within-synth-DAG | 0.9583 | 0.0589 | 0.875 | 1.0 | 15 |
| Cross | 0.1127 | 0.1158 | 0.0 | 0.2632 | 24 |

Key observation: **within-canonical mean exceeds cross mean by 0.151, but within-canonical std is 0.341 — more than twice the gap.** The "within > cross" difference is not statistically meaningful given the dispersion of within-canonical Jaccards.

By contrast, within-synth-DAG std=0.059 is small relative to its mean 0.958, so the DAG-family fingerprint IS a stable measurement.

## Why is the canonical-suite so heterogeneous?

The 6 within-canonical Jaccards split into three groups:

- **Identical canonical census** (Jaccard = 1.0): (narrow_a, long5k). Reason: narrow_a is just "OQ#1's regime A," and long5k's regime A is structurally identical at the size-2/3 motif level. They produce the same canonical set because they're effectively the same substructure.

- **Completely disjoint** (Jaccard = 0.0): (narrow_a, OQ#2), (OQ#2, long5k). Reason: OQ#2's tournament/lattice/star regimes produce dense small subgraphs (4-edge and 5-edge canonicals on 4 nodes — see Phase 1.D §6); narrow_a and long5k's regime-A-style diamond posets don't. The canonical sets share **nothing**.

- **Partial overlap** (Jaccard 0.18-0.20): (OQ#1, narrow_a), (OQ#1, OQ#2), (OQ#1, long5k). Reason: OQ#1 spans 4 regimes (A, B, C, D), so it shares regime-A motifs with narrow_a and long5k, and a couple of regime-B/C motifs with OQ#2.

This is structurally interpretable — the canonical-suite is *deliberately* heterogeneous, and the heterogeneity reflects regime composition. **The "canonical suite" is not a substrate family in the sense Round 2's null baseline was treating it as.**

## Did Round 2 retract too aggressively?

No, but the retraction prose can be sharpened.

Round 2's verdict — "H1 not supported; Within(OQ#1, narrow_a) = 0.20 ≈ Cross(OQ#1, DAG) = 0.19" — is now seen as one slice of a more general statistical truth:

- The canonical-suite is not a family with a tight within-Jaccard distribution (std=0.34, range [0,1]).
- Some pairs share everything (narrow_a-long5k=1.0); some pairs share nothing (narrow_a-OQ#2=0.0).
- "Substrate-sensitivity" cannot be inferred from cross-Jaccard alone; it requires a within-family baseline with low variance, which the canonical-suite does not provide.

The retraction therefore stands and is strengthened by N>1 data, NOT weakened.

## Surviving narrow positive (Round 3 M1 reinforced)

The synth-DAG family does have a tight invariant fingerprint:
- mean Jaccard 0.958, std 0.059, all 15 pairs ≥ 0.875.
- 10 of 15 pairs identical at saturation.
- This is real signal **about the layered-random-DAG generator**, not about v2.

The generator's structural degrees of freedom at sizes 2-3 are exhausted by ~14-15 canonical motifs. Two different seeds of the same generator hit the same set. This is informative about the generator's structural rigidity, not about v2's substrate-sensitivity.

## Implications for the ADR 0081 Phase 1.D narrative

Final state of the substrate-sensitivity claim across 4 rounds:

| Round | Substantive claim | Status |
|-------|------------------|--------|
| 0 (Phase 1.D as published) | "v2's pattern emergence produces substrate-distinct structural categories; 67% of synth-DAG canonicals are substrate-novel; Phase 2 motivated." | Withdrawn at Round 2. |
| 1 (Auto-review-loop W1-W7) | "v2 is substrate-sensitive on canonical-form level; H1 supported under pre-registered thresholds." | Withdrawn at Round 2. |
| 2 (corrected null baseline) | "Phase 1.D's substrate-sensitive verdict is retracted; the 0.26 cross is not interpretable as evidence of substrate-sensitivity." | Stands. |
| 3 (framing tweaks) | "Pipeline runs; surviving positive is DAG-generator invariance, not v2 substrate-sensitivity." | Stands. |
| 4 (this scan, N>1) | "Within-canonical-suite distribution mean 0.26 std 0.34 with bimodal extremes (0.0, 1.0). Cross mean 0.11 std 0.12. Gap < 1 std of within-canonical. Round 2 retraction reinforced; new observation that the 'canonical suite' is not a family in the variance-bounded sense." | Stands; **strengthened by N>1**. |

## What this leaves open

- **Phase 1.E real Mathlib.** Untouched. Still the next test for any substrate-sensitivity claim.
- **Multi-family scan**: this slice did N=6 within-canonical, N=15 within-synth-DAG, N=24 cross. Adding other generative families (Erdős–Rényi, preferential attachment, planted-partition) would test whether `Within(family pair) > Cross(family vs DAG)` for *those* families.
- **Sizes 4-6 canonical scan**: canonical counts at sizes 2-3 are 4-15; bigger sizes may give more statistical power per pair.
- **Stream-seeded canonical suite**: building parameterized OQ#1 / OQ#2 / long5k generators (varying internal seeds) would enable a true within-substrate variance measurement, currently impossible because all four are deterministic.

## Files

- `examples/bridge_multi_seed_scan.rs`
- `logs/2026-05-11_bridge_multi_seed_scan.log`
- This doc

## Verdict

The Round 2 retraction stands and is strengthened. The within-canonical-suite Jaccard is highly heterogeneous (std=0.34); there is no general "canonical-family fingerprint" against which to anchor a substrate-sensitivity claim. The only surviving narrow positive — that the synth-DAG generator family produces a tight invariant motif vocabulary (mean Jaccard 0.96) — is a property of the generator, not of v2's substrate-sensitivity machinery. Phase 1.E (real Mathlib) remains the necessary next experiment.
