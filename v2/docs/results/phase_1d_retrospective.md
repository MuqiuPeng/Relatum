# Phase 1.D 8-round retrospective — what survived 2026-05-11

**Status**: ✓ Phase 1.D experimentation closed (2026-05-11). Phase 1.E real Mathlib remains the natural next step.

This document consolidates 8 rounds of ARIS auto-review-loop + post-loop empirical follow-ups on ADR 0081 Phase 1.D's "substrate-sensitive emergent canonical-form" claim. It is the close-out summary, not a new result.

## Timeline

| Round | What happened | Doc |
|-------|---------------|-----|
| 0 | Original Phase 1.D claim: "v2 substrate-distinct emergence; 67% of canonicals substrate-novel" | [`bridge_cross_substrate_canonical.md`](bridge_cross_substrate_canonical.md) §§1-12 (preserved as historical record) |
| 1 | ARIS auto-review-loop Phase A: fresh sub-agent reviewer scored 3/10 not ready; surfaced 7 weaknesses W1-W7 | [`review-stage/AUTO_REVIEW.md`](../../review-stage/AUTO_REVIEW.md) Round 1 |
| 2 | Round 2 implementation: W1+W5+W6+W7 fixed; W3 addressed via new null-baseline experiment; reviewer scored 5/10; surfaced N1+N2+N3+N4 — including methodologically empty OQ#2-self baseline | AUTO_REVIEW.md Round 2 |
| 3 | Round 3 implementation: N1+N2 fixed via OQ#1-vs-narrow_a baseline and saturation probe; **H1 disconfirmed, Phase 1.D verdict retracted**; reviewer scored 7/10 (loop exit) | AUTO_REVIEW.md Round 3 + `bridge_cross_substrate_canonical.md` §13 |
| 4 | Multi-seed scan: 6 canonical-suite pairs + 15 within-DAG pairs + 24 cross — Round 2 single-seed value was typical; retraction reinforced N>1 | [`bridge_multi_seed_scan.md`](bridge_multi_seed_scan.md) |
| 5 | Multi-family scan: ER + SBM + synth-DAG **produce essentially identical canonical sets**; cross-family Jaccard ≈ within-family Jaccard ≈ 0.9 for random graphs | [`bridge_multi_family_scan.md`](bridge_multi_family_scan.md) |
| 6 | Structural-class scan: TREE × DAG (0.78) overlaps; BIPARTITE × DAG (0.33) distinguishable; TREE × BP (0.42) marginal | [`bridge_structural_class_scan.md`](bridge_structural_class_scan.md) |
| 7 | Size 4 scan (n=40, top_m=20): cross-class gap widens with size (0.25 → 0.35 → 0.38); **BIPARTITE first H1-passing family** (within 0.90, max cross 0.21) | [`bridge_size4_scan.md`](bridge_size4_scan.md) §§1-9 |
| 8 | Size 4 rerun with top_m=100 (cap-artifact check): random-family Jaccards essentially unchanged; **BIPARTITE H1 verdict sharper** (max cross drops to 0.16) | `bridge_size4_scan.md` Round 8 addendum |

## The arc

Original claim (Round 0) → surfaced weaknesses (Round 1) → corrected baseline disconfirms (Round 2-3 retraction) → retraction holds at N>1 (Round 4) → retraction generalizes across random-graph families (Round 5) → distinguishes structural classes (Round 6) → first family passes H1 at size 4 (Round 7) → confirmation under tighter measurement (Round 8).

## What is now known empirically

After 8 rounds of measurement on synthetic substrates:

### Established negatives

1. **v2 cannot distinguish substrates within the random-graph class** at sizes 2-4 under saturation budget. Erdős–Rényi, stochastic block model, and layered random DAG produce essentially identical canonical-form sets at sizes 2-3 (cross-Jaccard 0.91-0.95) and remain mutually indistinguishable at size 4 even after raising the top_m cap (cross 0.75-0.81 ≈ within 0.76-0.90).

2. **The canonical-suite (OQ#1, narrow_a, OQ#2, long5k) is not a variance-bounded substrate family**. Pairwise within-Jaccards span [0.0, 1.0] with mean 0.26 std 0.34 — internally heterogeneous because each member is a specific hand-crafted stream regime.

3. **The original "67% of canonicals substrate-novel" framing was an artifact** of comparing v2's hand-crafted streams against ONE specific random-graph instance, conflating "hand-crafted vs generic random" with "substrate-sensitivity in general."

4. **v2's discovery pipeline does NOT scale on power-law (Barabási–Albert) graphs at saturation budget on n=80**. A single BA instance's size-3 autonomous_pass took 38 minutes (vs ER's ~100s). Documented as a quantitative scaling observation; not pursued further.

### Established positives (descriptive, not "emergent")

5. **v2 distinguishes structured stream substrates (canonical-suite) from generic random graphs** at sizes 2-3. Cross-canonical-suite × random ≈ 0.11-0.17, well below the within-random ≈ 0.9.

6. **v2 distinguishes BIPARTITE from random-graph baselines** sharply at sizes 2-4. At size 4 with top_m=100: within-BP = 0.90, max BP × random cross = 0.16. BIPARTITE clears the pre-registered H1 thresholds (within > 0.7 AND max cross < 0.4).

7. **Cross-class Jaccard gap widens with motif size** in the regime where saturation regime weakens (sizes 2 → 3 → 4: gap 0.25 → 0.35 → 0.38). Suggests sizes 5-6 might further widen the gap and expose more substrate-distinctions.

### Methodologically meaningful

8. **The ARIS auto-review-loop substantively improved the work**: 3 rounds of fresh sub-agent review caught an over-claim that would otherwise have shipped. Round 1's reviewer's seven weaknesses (W1-W7) were correct; Round 2's reviewer's four (N1-N4) were correct; Round 3's three (M1-M3) were correct. None of these were self-identifiable from inside the executor's context.

9. **"Pre-registered" hypotheses with explicit thresholds** (H1: within > 0.7 AND max cross < 0.4) gave the experimentation a falsifiable structure across all 8 rounds. The retraction was explicit because the threshold was explicit. Round 7-8's H1-passing finding for BIPARTITE is correspondingly a defensible positive, also against explicit thresholds.

## What was over-claimed in Round 0 and would now be removed

Original framing (preserved at start of `bridge_cross_substrate_canonical.md`):
- ✗ "67% of Lean canonicals are substrate-specific" — substrate was not Lean
- ✗ "First empirical evidence that v2's pattern path generalizes to natural-data structural categories" — no natural data tested
- ✗ "The bridge surfaces real structural novelty" — what was observed was hand-crafted-vs-random-graph census difference
- ✗ "Phase 2 of the bridge is empirically motivated" — synthetic data alone cannot motivate Phase 1.E

## What survives, narrowed

The defensible Phase 1.D claim after 8 rounds of measurement:

> **v2's canonicalization at sizes 2-4 reflects structural-class constraints.** Substrates with motif-excluding constraints (BIPARTITE: no 3-cycle, no self-loop, no within-part edges) produce canonical-form fingerprints sharply distinguishable from random-graph baselines (cross 0.13-0.16 vs within 0.90 at size 4 with top_m=100). v2 cannot distinguish substrates within the random-graph class at any size 2-4 tested. The canonical-suite of hand-crafted streams is internally heterogeneous (within mean 0.26 std 0.34) and is distinguishable from random-graph baselines (cross ≈ 0.12).

This is descriptive and quantitatively measured. It is also exactly classical subgraph-census behavior on substrates with structural exclusions — not "emergent substrate-sensitivity beyond classical motif census."

## Open questions Phase 1.D could not answer

1. **Does v2 do anything beyond classical subgraph census?** Not answered by synthetic-substrate measurements. Phase 1.E real natural data (Mathlib dep graph, arXiv citation graph) is the only test that could differentiate "v2 computes census" from "v2 produces substrate-sensitive emergence."

2. **Does sizes 5-6 widen the gap further?** Gap widens 0.25 → 0.35 → 0.38 from size 2 → 3 → 4. Plausible that sizes 5-6 reveal more substrate-distinctions, but compute cost scales rapidly. Not pursued.

3. **Does v2 distinguish other structurally-constrained classes (pure tree, geometric, small-world)?** Round 6 tested TREE-with-noise (overlap with random); pure tree at size 4 might pass H1 like BIPARTITE. Not pursued.

4. **Does v2's runtime (`AutonomousRuntime` + scheduler) produce different canonical sets than the static `RSet::from_text` path?** All rounds used static input for generated substrates. Round 7's TREE measurement noted forward-DAG noise contamination because the experimental TREE generator wasn't pure tree.

## What the auto-review-loop produced beyond the empirical findings

The 8-round process produced its own meta-result: a documented case of LLM-driven scientific self-correction. The original Phase 1.D claim was caught at auto-review stage and revised through pre-registered experiments that disconfirmed it. The work is preserved alongside its retraction in the same repository, with each retraction step traceable to specific reviewer-identified weaknesses.

For ARIS as a methodology, Phase 1.D is a positive demonstration. For v2 as a substantive research project, Phase 1.D is now a documented null result with one narrow positive (BIPARTITE × random distinguishable at sizes 2-4).

## Where to go from here

Three natural paths (recorded for whoever picks up next):

### Path A — Phase 1.E real natural data (highest paper value, biggest cost)

- Ingest a Mathlib dependency-graph snapshot, run v2's autonomous_pass at sizes 2-4, compare canonical-form census to:
  - The Phase 1.D synth-DAG result (does Mathlib differ structurally from random DAG?)
  - The Phase 1.D BIPARTITE result (does Mathlib's hierarchical structure look more like BIPARTITE than random?)
- Estimated effort: 1-3 weeks (downloads, parser, ETL, runtime, analysis).
- Risk: Mathlib's dep structure may fall into the "tree-like / sparse" zone where v2 cannot distinguish from random (Round 6 TREE × DAG = 0.78). Negative result is still publishable.

### Path B — ADR 0080 threshold tuning (Phase Emergence runtime hole)

- ADR 0080 learning-progress-aware drive was shipped but threshold-untuned; long-horizon OQ#2 runs hang.
- Tune LP_WINDOW and threshold to make sustained mode engage on long horizons.
- Estimated effort: 1-2 days.
- Closes a known runtime gap; orthogonal to Phase 1.D.

### Path C — Phase 1.D follow-ups (lower value after 8 rounds)

- Pure tree at size 4 (does it pass H1 like BIPARTITE?)
- Sizes 5-6 scan (does gap keep widening?)
- Sparser-graph families (does saturation break at density < 0.04?)
- All would refine the surviving narrow positive but won't change the overall picture.

## Files preserved

- Examples: `bridge_cross_substrate_canonical.rs`, `bridge_null_baseline.rs`, `bridge_multi_seed_scan.rs`, `bridge_multi_family_scan.rs`, `bridge_structural_class_scan.rs`, `bridge_size4_scan.rs`
- Result docs: `bridge_cross_substrate_canonical.md`, `bridge_multi_seed_scan.md`, `bridge_multi_family_scan.md`, `bridge_structural_class_scan.md`, `bridge_size4_scan.md`, this retrospective
- Logs: 9 log files spanning all 8 rounds
- Review record: `review-stage/AUTO_REVIEW.md` (verbatim Round 1-7 reviewer outputs + triage decisions)

## Closing note

The original Phase 1.D claim is retracted. The work that produced and corrected the claim is intact, traceable, and available as case material for ARIS methodology. v2's substrate-sensitivity question is now well-scoped: classical subgraph census at sizes 2-4 distinguishes structurally-constrained substrates from random graphs but does not distinguish within the random class. Phase 1.E real natural data is the only experiment that could move this conclusion beyond classical-census territory.
