# Pure-tree probe — Round 9 follow-up to Round 6

**Status**: ✓ done (partial — n=80 size=4 killed at DAG_0 after 72min wall-clock).
**Log**: [`logs/2026-05-11_bridge_pure_tree_probe.log`](../../logs/2026-05-11_bridge_pure_tree_probe.log)
**Example**: [`examples/bridge_pure_tree_probe.rs`](../../examples/bridge_pure_tree_probe.rs)
**Predecessor**: [`bridge_structural_class_scan.md`](bridge_structural_class_scan.md) (Round 6); [`bridge_size4_scan.md`](bridge_size4_scan.md) (Round 7-8)

## Goal

Round 6 found that "tree + forward-DAG noise" failed H1 against synth-DAG: cross-Jaccard 0.78 (heavy overlap). The TREE builder added 80 random forward edges on top of a rooted-tree backbone, contaminating the tree signature with merge / cluster motifs that DAGs also have.

This probe asks: does a PURE rooted tree (only n-1 backbone edges, no forward-DAG noise) cleanly pass H1 against random-graph baselines?

If yes: pure-tree becomes the second H1-passing family after BIPARTITE (Round 7-8). Both would be structurally-constrained substrate classes that exclude motifs random DAGs include.

## Method

- 3 seeds × 3 families: pure-tree (n-1 edges), synth-DAG (random class), BIPARTITE (Round 7 baseline).
- 2 graph sizes × 3 motif sizes: n ∈ {40, 80} × size ∈ {2, 3, 4}.
- Saturation budget: sample_count=400, top_m=100 (post-Round-8 cap removal).

## Results

```
                    within(TREE)  max-cross(TREE)  H1
n=40 size=2          1.0000        0.5000          ✗ (BP × TREE shares 2 motifs at this scale)
n=40 size=3          1.0000        0.3758          ✓ SUPPORTED
n=40 size=4          0.9259        0.2213          ✓ SUPPORTED
n=80 size=2          1.0000        0.5556          ✗ (same as n=40)
n=80 size=3          1.0000        0.3879          ✓ SUPPORTED
n=80 size=4          (incomplete — killed at DAG_0 after 72min wall-clock)
```

### n=40 size=3 detail

```
WITHIN pure-TREE:  N=3 mean=1.0000 std=0.0000
WITHIN synth-DAG:  N=3 mean=0.9394 std=0.0429
WITHIN BIPARTITE:  N=3 mean=1.0000 std=0.0000
CROSS  TREE × DAG: N=9 mean=0.3758 std=0.0214
CROSS  TREE × BP:  N=9 mean=0.2727 std=0.0000
CROSS  DAG × BP:   N=9 mean=0.2818 std=0.0129
```

### n=40 size=4 detail

```
WITHIN pure-TREE:  N=3 mean=0.9259 std=0.0653
WITHIN synth-DAG:  N=3 mean=0.7756 std=0.0266
WITHIN BIPARTITE:  N=3 mean=0.9048 std=0.0673
CROSS  TREE × DAG: N=9 mean=0.2213 std=0.0269
CROSS  TREE × BP:  N=9 mean=0.1769 std=0.0185
CROSS  DAG × BP:   N=9 mean=0.1642 std=0.0209
```

### n=80 size=3 detail

```
WITHIN pure-TREE:  N=3 mean=1.0000 std=0.0000
WITHIN synth-DAG:  N=3 mean=0.9394 std=0.0429
WITHIN BIPARTITE:  N=3 mean=1.0000 std=0.0000
CROSS  TREE × DAG: N=9 mean=0.3879 std=0.0171
CROSS  TREE × BP:  N=9 mean=0.1667 std=0.0000
CROSS  DAG × BP:   N=9 mean=0.2909 std=0.0129
```

## Why size 2 fails

At size 2, pure-tree's canonical census is just `chain` (R(parent, child)). Bipartite at p=0.15 produces only L→R chains. Synth-DAG produces both chain and bidirectional pair. The 3 families share 2 of 2-4 canonical forms at size 2 — overlap forces cross-Jaccard ≥ 0.5.

Round 7-8 already noted size 2 is too small to discriminate structural classes in general. The interesting differentiation begins at size ≥ 3.

## Why size 4 at n=80 was abandoned

DAG_0 at n=80 size=4 took **4361 seconds (72 min)** — the same BA-like scaling explosion observed in Round 5 (BA at n=80 size=3 took 38 min). DAG instances at n=80 size=4 with 236+ directed edges produce 36-42 canonicals each, and the discovery pipeline's subgraph candidate space grows combinatorially. Two more DAG instances + 3 BP instances would have taken ~3-4 more hours, exceeding the autonomous slice budget.

Killed. n=40 size=4 result + n=80 size=3 result jointly establish the conclusion.

## Verdict

**Pure-tree (no forward-DAG noise) is the SECOND H1-passing substrate family**, after BIPARTITE (Round 7-8). Both pass at sizes 3 (and pure-tree at size 4 on n=40 also).

The pattern is now clear:

> Substrate classes that EXCLUDE structural motif categories produce canonical-form fingerprints sharply distinguishable from random-graph baselines under v2's saturation-budget discovery.
>
>   - BIPARTITE: excludes 3-cycle, self-loop, within-part edges.
>   - PURE TREE: excludes ALL cycles (including 3-cycle, self-loop), bidirectional pairs, merge motifs.
>   - Random DAG: includes all the above motifs that BP/tree exclude.

Round 6's TREE-with-forward-DAG-noise was a methodological mistake: adding 80 random forward edges (essentially DAG noise) reintroduced the very motifs that pure tree excludes, contaminating the signature. Now corrected.

## What this confirms vs revises

### Confirms

- The Round 6 → 7 → 8 progression: structurally-constrained substrates pass H1; random-class substrates don't (within ≈ cross within random class).
- Saturation regime is real at sizes 2-3 on random graphs (within-DAG mean stays ~0.94 across n=40 and n=80).
- The 0.4 H1 threshold is empirically meaningful: BIPARTITE × DAG = 0.21-0.33, pure tree × DAG = 0.22-0.39 — both well below threshold.

### Revises

- Round 6's "TREE × DAG = 0.78" is **superseded** by this run's pure-tree × DAG = 0.22-0.39. The 0.78 was an artifact of forward-DAG noise edges.
- The Round 6 verdict ("only BIPARTITE distinguishable; TREE-with-noise overlaps") narrowed too aggressively. Pure structural classes DO distinguish.

## Implications for Phase 1.D narrative

The surviving narrow positive is now:

> **At sizes 3-4 under saturation budget, v2's canonicalization produces fingerprints that distinguish structurally-constrained classes (BIPARTITE, pure TREE) from random-graph classes (ER, SBM, synth-DAG). Within structurally-constrained classes, fingerprints are perfectly invariant (within-Jaccard ≈ 1.0). Within the random class, fingerprints are mutually indistinguishable (cross ≈ within ≈ 0.9).**

This is **measurable, defensible, and the same shape as classical subgraph census**: structural exclusions imply census exclusions imply distinguishability. v2 doesn't do anything beyond census at this scale.

For substrate-sensitivity at the **emergent abstraction** level (the original Phase 1.D claim), Phase 1.E real natural data remains the only test that could differentiate "v2 does subgraph census" from "v2 does emergent abstraction." This Round 9 finding doesn't change that picture.

## Open follow-up

- **BA scaling problem persists**. v2's discovery pipeline at saturation budget cannot tractably process power-law or large dense graphs (BA size=3 at n=80 → 38 min; DAG size=4 at n=80 → 72 min for one instance). Worth ADR-grade investigation if such substrates become target.
- **Pure tree at size 5-6**: untested. Would the pattern continue? Likely yes but compute cost would be prohibitive.
- **Other structurally-constrained classes**: planar graphs, regular graphs, etc. Each has its own motif exclusions; each should pass H1 under this framework.

## Files

- `examples/bridge_pure_tree_probe.rs`
- `logs/2026-05-11_bridge_pure_tree_probe.log`
- This doc

## Verdict

**Pure tree is the second H1-passing family at sizes 3-4 (n=40) and size 3 (n=80).** Round 6's TREE-with-noise result was contaminated; the corrected pure-tree measurement gives a clean cross-Jaccard 0.22-0.39 vs within-Jaccard 0.93-1.00. The "v2 distinguishes structural classes from random" claim now has two cleanly-supported examples (BIPARTITE + pure tree) instead of one. Still classical subgraph census, still not emergent abstraction, still Phase 1.E gating for any stronger claim.
