# ADR 0081 Phase 1.D Round 7 — sizes 4 scan

**Status**: ✓ done (2026-05-11). **First H1-passing family** (BIPARTITE at size 4) in 7 rounds.
**Log**: [`logs/2026-05-11_bridge_size4_scan.log`](../../logs/2026-05-11_bridge_size4_scan.log)
**Example**: [`examples/bridge_size4_scan.rs`](../../examples/bridge_size4_scan.rs)
**Predecessor**: [`bridge_structural_class_scan.md`](bridge_structural_class_scan.md) (Round 6)

## Goal

Round 5+6 established that v2 at sizes 2-3 saturates to a near-universal small-motif vocabulary on random graphs at n=80. Open question: at size 4, does saturation break? Does within-family Jaccard drop faster than cross-family, exposing real substrate-sensitivity?

Pre-registered: H1 supported for any family if within_mean > 0.7 AND max cross_mean < 0.4 at size 4.

## Method

- n=40 (reduced from n=80 to keep size-4 autonomous_pass tractable; Round 5 BA-timing observation showed n=80 size-3 already risks 38min/instance for some structures, and size 4 grows worse).
- 4 families × 3 seeds: ER (p=0.05), SBM (4×10 blocks, p_within=0.15, p_cross=0.03), synth-DAG (proportionally scaled to n=40), BIPARTITE (20+20, p_cross=0.15).
- Saturation budget: sample_count=400, top_m=20.
- Sizes 2, 3, 4 each scanned separately for trend comparison.

## Results

### Per-size aggregate

```
            within_mean    cross_mean    gap
size=2      1.0000         0.7500        0.2500
size=3      0.9571         0.6101        0.3470
size=4      0.8153         0.4395        0.3758
```

**Trend**: as canonical size grows from 2 → 4, within drops (canonical space grows beyond saturation budget capacity) but cross drops MORE. Gap widens.

### Size 4 detail

Within (N=3 per family):

```
ER  size=4:  mean=0.7150 std=0.0342  [0.6667, 0.7391]    ⚠ at top_m=20 cap
SBM size=4:  mean=0.8207 std=0.0676  [0.7391, 0.9048]    ⚠ at top_m=20 cap
DAG size=4:  mean=0.8207 std=0.0676  [0.7391, 0.9048]    ⚠ at top_m=20 cap
BP  size=4:  mean=0.9048 std=0.0673  [0.8571, 1.0000]    natural census (6-7 canonicals)
```

Cross (N=9 per pair):

```
ER × SBM    = 0.7501    (random class internal, high)
ER × DAG    = 0.6866    (random class internal)
SBM × DAG   = 0.6995    (random class internal)
ER × BP     = 0.1447    ← sharply distinct
SBM × BP    = 0.1434    ← sharply distinct
DAG × BP    = 0.2128    ← sharply distinct
```

### H1 verdict per family

Required: within > 0.7 AND max cross < 0.4.

| Family | within_mean | max_cross_mean | H1 |
|--------|-------------|----------------|-----|
| ER | 0.715 ✓ | 0.750 ✗ | not supported |
| SBM | 0.821 ✓ | 0.750 ✗ | not supported |
| synth-DAG | 0.821 ✓ | 0.750 ✗ | not supported |
| **BIPARTITE** | **0.905 ✓** | **0.213 ✓** | **✓ SUPPORTED** |

**BIPARTITE is the first family to pass H1 cleanly at its pre-registered thresholds in 7 rounds of Phase 1.D experimentation.**

## Caveat — top_m=20 truncation artifact

ER, SBM, synth-DAG each produce exactly 20 canonicals per instance at size 4. This is the `top_m=20` cap, not the natural census. Each instance keeps its own "top 20" candidates by MDL gain, and seeds vary in which 20 they pick. This drives the within-random Jaccards to 0.72-0.82 (some overlap, some variance in the truncated set), and cross-within-random Jaccards similarly.

BIPARTITE produces only 6-7 canonicals at size 4 — below the cap. This is the natural BP census; cross-class comparison against random families is therefore meaningful (BP's 6-7 vs random's top-20).

The qualitative finding is unaffected: **BP × random is sharply distinguishable** (0.14-0.21) because BP's structural constraint (no L→L, no R→R, no self-loop, no 3-cycle) excludes most of the motifs that random families have. Even if random families had uncapped canonical sets, BP would still differ by the same exclusions.

But: comparing within-ER vs within-SBM vs within-DAG requires uncapped sampling to be a clean measurement. Future scan with top_m=100+ would tighten this.

## What this refines

After Round 7 the cumulative picture is:

1. **Saturation regime weakens with size**: at size 2 every family within is perfect (1.0); at size 3 random within drops to 0.94-0.96 with top_m=20; at size 4 down to 0.71-0.90 (BIPARTITE highest, random families lower due to cap artifact).

2. **Cross-class gap widens with size**: gap = 0.25 → 0.35 → 0.38. Size 4 starts to distinguish what size 2 cannot.

3. **BIPARTITE H1 passes cleanly at size 4** (and was marginal at sizes 2-3, max cross 0.42-0.50). This is the first H1-supported result in the auto-review series.

4. **Random-class internal indistinguishability persists at size 4**: ER × SBM = 0.75, ER × DAG = 0.69. The Round 5 "universal small-motif vocabulary" finding extends to size 4 within the random class.

## Does this revive Phase 1.D?

**Partially, with strict scope.** The defensible empirical claim is now:

> v2's pattern discovery at sizes 2-4 produces canonical-form fingerprints that meaningfully distinguish BIPARTITE substrates from random-directed-graph substrates under saturation budget. Within-BIPARTITE Jaccard = 0.90 at size 4; BIPARTITE × random cross Jaccard = 0.14-0.21. H1's pre-registered thresholds (within > 0.7, max cross < 0.4) are satisfied for BIPARTITE.

This is NOT the original Phase 1.D claim ("v2 produces substrate-distinct structural emergence beyond classical motif census"). It IS a quantitative description of v2's motif-census behavior on a substrate with strong structural exclusions.

Whether BIPARTITE-vs-random is "emergent substrate-sensitivity" or "trivially correct subgraph census" is a definitional question. A classical subgraph-census algorithm would also produce different canonicals for BIPARTITE vs random graphs — that's what subgraph census is for.

The narrow finding that survives:

> v2's canonicalization at sizes 2-4 IS sensitive to structural constraints that exclude motifs. It is NOT sensitive to within-random-class generative-process differences.

This is descriptive and defensible. It is much narrower than Phase 1.D originally claimed, but it represents real measured behavior.

## Final state across 7 rounds

| Round | Finding | Status |
|-------|---------|--------|
| 0-1 | "v2 substrate-distinct emergent abstraction; 67% novel" | Withdrawn |
| 2-3 | Retraction shipped; ARIS loop exit at 7/10 | Stands |
| 4 | Multi-seed: canonical-suite is not variance-bounded | Stands; strengthened |
| 5 | Multi-family: ER ≈ SBM ≈ synth-DAG universal vocabulary | Stands; major reinforcement |
| 6 | Structural-class: BIPARTITE × random distinguishable; TREE × DAG overlap | Stands; refinement |
| **7** | **BIPARTITE passes H1 cleanly at size 4 (within 0.90, max cross 0.21)** | **First H1-passing family; partial Phase 1.D revival under strict scope** |

## What this leaves open

- **Top_m cap effect**: re-run size 4 at top_m=100+ to remove the random-family truncation artifact. Predicted: within-random drops, cross-random stays roughly same — sharper retraction within random class.
- **Sizes 5-6**: gap widening trend (0.25→0.35→0.38) suggests it might keep growing. Test if other families approach H1 thresholds at size 5-6.
- **Pure tree** (no forward-DAG noise): Round 6 noted TREE × DAG = 0.78 was inflated by my hybrid tree. Pure tree at size 4 likely produces a much smaller distinct canonical set, possibly passing H1 like BIPARTITE.
- **Phase 1.E real Mathlib**: still the gating experiment for natural-data claims. Mathlib's dep structure is sparser+more hierarchical than synthetic dense random graphs; it MAY produce a distinct enough canonical census.

## Files

- `examples/bridge_size4_scan.rs`
- `logs/2026-05-11_bridge_size4_scan.log`
- This doc

## Verdict

**BIPARTITE is the first substrate family across 7 rounds of Phase 1.D experimentation to clearly pass the pre-registered H1 thresholds.** At size 4 with n=40 saturation budget, within-BIPARTITE Jaccard = 0.90 and max BIPARTITE × random cross-Jaccard = 0.21 — both inside the pre-set windows (within > 0.7, cross < 0.4).

This is real substrate-sensitivity at the canonical-form level, but ONLY for substrates with strong structural exclusions (BIPARTITE excludes 3-cycle, self-loop, L→L, R→R, R→L). It is NOT general "v2 distinguishes substrate families" — within the random-graph class, v2 still cannot distinguish ER from SBM from synth-DAG.

The defensible Phase 1.D claim is now:

> *v2's canonicalization at sizes 2-4 reflects structural-class constraints in the canonical-form set. Substrates with constraint-excluded motifs (e.g., BIPARTITE) produce canonical fingerprints sharply distinguishable from random-graph baselines.*

This is descriptive and verifiable. It is also exactly what classical subgraph census would produce. Whether v2 does anything beyond classical census remains the open question that Phase 1.E real natural data must answer.
