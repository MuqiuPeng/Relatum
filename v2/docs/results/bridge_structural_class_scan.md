# ADR 0081 Phase 1.D Round 6 — structural-class scan

**Status**: ✓ done (2026-05-11). Adds nuance to Round 5's universal-vocabulary finding.
**Log**: [`logs/2026-05-11_bridge_structural_class_scan.log`](../../logs/2026-05-11_bridge_structural_class_scan.log)
**Example**: [`examples/bridge_structural_class_scan.rs`](../../examples/bridge_structural_class_scan.rs)
**Predecessor**: [`bridge_multi_family_scan.md`](bridge_multi_family_scan.md) (Round 5)
**Initiated by**: post-Round-5 question — does v2 distinguish structural CLASSES (tree, bipartite, random) even though it cannot distinguish within the "random" class (Round 5: ER ≈ SBM ≈ DAG)?

## Goal

Round 5 showed v2 at sizes 2-3 produces essentially identical canonical-form sets across ER, SBM, and synth-DAG — three different random-graph generative processes converge to a "universal small-motif vocabulary." This Round 6 asks: does that universality extend to STRUCTURALLY CONSTRAINED classes (tree, bipartite) whose motif censuses are provably different from random graphs?

Hypothesis H_class_sensitive (would partially revive Phase 1.D):
- TREE family: within > 0.7 AND cross to all other classes < 0.4.
- BIPARTITE family: within > 0.7 AND cross to all other classes < 0.4.
- If true: v2 IS substrate-sensitive at the structural-class level, just not within the random class.

## Method

- **canonical-suite** (carryover, 4 fixed): OQ#1, narrow_a, OQ#2, long5k.
- **TREE** (6 seeds): rooted random tree (n-1 = 79 edges) + 80 additional forward edges (i→j, i<j) for ~156 edges. Acyclic. Hybrid "tree + forward DAG noise" — not pure tree.
- **BIPARTITE** (6 seeds): nodes split L=0..40, R=40..80; edges only L→R with prob 0.10; expected ~160 edges. No L→L, no R→R, no R→L, no self-loops, no 3-cycles possible.
- **synth-DAG** (6 seeds): random-class representative (per Round 5 finding ER ≈ SBM ≈ DAG).
- Discovery: `autonomous_pass(sizes 2-3)` at saturation budget (sample_count=400, top_m=20).

## Results

### Within-class Jaccards

```
canonical-suite (C(4,2) = 6):   mean=0.2636  std=0.3406  [0.0000, 1.0000]
TREE            (C(6,2) = 15):  mean=1.0000  std=0.0000  [1.0000, 1.0000]
BIPARTITE       (C(6,2) = 15):  mean=1.0000  std=0.0000  [1.0000, 1.0000]
synth-DAG       (C(6,2) = 15):  mean=0.9583  std=0.0589  [0.8750, 1.0000]
```

TREE and BIPARTITE each saturate to **perfect invariance** — every pair of seeds within a class produces literally the same canonical set. TREE: 12 canonicals per instance. BIPARTITE: 5 canonicals per instance. (synth-DAG: 15 per instance.)

The structural class fully determines the size-2-3 canonical census; the random graph seeds within a class are irrelevant at this scale.

### Cross-class Jaccards

```
canonical × TREE:        N=24  mean=0.1165  std=0.1165  [0.0000, 0.2353]
canonical × BIPARTITE:   N=24  mean=0.1667  std=0.2041  [0.0000, 0.5000]
canonical × synth-DAG:   N=24  mean=0.1127  std=0.1158  [0.0000, 0.2632]
TREE × BIPARTITE:        N=36  mean=0.4167  std=0.0000  [0.4167, 0.4167]
TREE × synth-DAG:        N=36  mean=0.7813  std=0.0419  [0.6875, 0.8000]
BIPARTITE × synth-DAG:   N=36  mean=0.3333  std=0.0000  [0.3333, 0.3333]
```

### H_class_sensitive per class

Required for H1: within > 0.7 AND max cross < 0.4.

| Class | within | max_cross | H1 |
|-------|--------|-----------|-----|
| TREE | 1.00 ✓ | 0.78 (vs DAG) ✗ | not supported |
| BIPARTITE | 1.00 ✓ | 0.42 (vs TREE) ✗ | not supported (marginal) |
| synth-DAG | 0.96 ✓ | 0.78 (vs TREE) ✗ | not supported |
| canonical-suite | 0.26 ✗ | — | fails on within |

No class strictly clears H1, but BIPARTITE comes very close — its max cross (0.42 vs TREE) is barely over the 0.4 threshold. BIPARTITE vs synth-DAG is 0.33, sharply below the threshold.

## Interpretation

### What's actually going on

The cross-class pattern is structurally interpretable:

- **canonical-suite × any random class ≈ 0.12**. The canonical-suite's hand-crafted stream regimes produce motifs absent from random graphs. Round 5 finding.
- **BIPARTITE × synth-DAG = 0.33** (sharply different). Bipartite's constraint (no L→L, no R→R, no 3-cycle, no self-loop) excludes motifs that synth-DAG has (3-cycle, self-loop, dense 4-edge subgraphs). The canonical sets share only the chain / fork / merge motifs that survive both constraints.
- **TREE × synth-DAG = 0.78** (heavily overlapping). Both are acyclic, both have similar low-degree motifs. Tree's 12 canonicals are nearly a subset of DAG's 15. The 0.78 reflects DAG having a few additional cluster-derived motifs (the 4-edge variants).
- **TREE × BIPARTITE = 0.42** (moderately different). Tree allows any-direction edges; bipartite is L→R only. Some motifs (chain length 2) appear in both; others (specific role configurations) don't.

### What this says about v2's "substrate-sensitivity"

This refines the Round 5 picture:

1. **WITHIN structurally-constrained classes**: v2 saturates to perfect invariance (Jaccard = 1.0 across all seeds). The class fully determines the size-2-3 canonical census.

2. **BETWEEN structurally-distinct classes**: v2 produces meaningfully different canonical sets when the structural constraints differ enough to exclude different motifs. BIPARTITE vs synth-DAG (cross 0.33) is the cleanest example.

3. **BETWEEN structurally-similar classes**: v2 fails to distinguish them. TREE (acyclic) ≈ synth-DAG (acyclic with random structure) cross = 0.78; their constraint sets overlap too much.

4. **WITHIN the canonical-suite (hand-crafted streams)**: heterogeneous — each stream regime has its own composition, so within-suite is high-variance.

### Does this revive Phase 1.D?

**No, but partially refines the surviving narrow positive:**

The Round 5 conclusion was "v2 at sizes 2-3 cannot distinguish substrate families." Round 6 refines this to:

> v2 at sizes 2-3 cannot distinguish substrates within the same structural class (random vs random, or hand-crafted-stream vs hand-crafted-stream). It CAN distinguish substrates from structurally-distinct classes when the classes have non-overlapping motif vocabularies (BIPARTITE vs DAG, canonical-suite vs random). It CANNOT distinguish structurally-similar classes (TREE vs DAG).

This is not "substrate-sensitive emergent abstraction." It is "v2 computes the size-2-3 motif census, and motif censuses trivially differ when the substrate's structural constraints differ by construction." A classical graph-theory subgraph-census routine would produce the same finding.

For the original Phase 1.D claim (v2 produces emergent substrate-sensitive canonical categories beyond hand-crafted tests), Round 6 does not provide support. It just adds quantitative shape to the null result.

## Open follow-ups

- **Phase 1.E real Mathlib** — still the gating experiment for any natural-data substrate-sensitivity claim. Round 6 confirms synthetic data, no matter how diverse, will only show "motif-census reflects structural constraints."
- **Sizes 4-6 scan** — at bigger canonical sizes, the saturation regime may break and structurally-similar classes (TREE vs DAG) may produce distinguishable canonical sets.
- **Sparser-graph families** — at density ≪ 0.04, the canonical-form set may be small enough that within-class saturation doesn't hold, exposing whether v2 distinguishes anything at low density.
- **A pure tree (without forward-DAG noise)** — would push TREE × DAG cross lower (no merge/cluster motifs). Worth running as a follow-up if the structural-class line is pursued.

## Files

- `examples/bridge_structural_class_scan.rs`
- `logs/2026-05-11_bridge_structural_class_scan.log`
- This doc

## Verdict

**v2's size-2-3 canonicalization correctly distinguishes structural classes when their constraint vocabularies differ.** BIPARTITE vs synth-DAG (cross 0.33) is the cleanest empirical demonstration. WITHIN each structurally-constrained class (TREE, BIPARTITE, synth-DAG), v2 saturates to perfect or near-perfect invariance (Jaccard 0.96-1.00 across seeds).

This is a descriptive measurement of v2's classical-subgraph-census behavior, not evidence of substrate-sensitive emergent abstraction. For Phase 1.D's original claim to revive, an experiment is needed where v2 distinguishes substrates that classical motif census CANNOT distinguish — that is empirically inaccessible at sizes 2-3 on synthetic data, and is precisely what Phase 1.E real Mathlib was designed to test.

The Round 5 "universal small-motif vocabulary" finding is refined: v2's vocabulary is universal WITHIN structurally-equivalent classes, and is class-determined ACROSS structurally-distinct classes. The substrate-sensitivity claim's surviving form is now:

> *Under saturation budget at sizes 2-3, v2 distinguishes substrates if and only if their structural constraints exclude different motifs from the size-2-3 canonical vocabulary.*

This is graph theory, not v2-specific cognition. Phase 1.E remains the only test that can change the picture.
