# ADR 0081 Phase 1.D — Cross-substrate canonical-form comparison

**Status**: ⚠ Round 2 negative — original "substrate-sensitive" verdict is **retracted** (see §13). Engineering record kept; substantive claim withdrawn.
**Logs**:
- Original (hash-based): [`logs/2026-05-11_bridge_cross_substrate_canonical.log`](../../logs/2026-05-11_bridge_cross_substrate_canonical.log)
- Round 1 revised (direct CanonicalForm equality, W5 fix): [`logs/2026-05-11_bridge_cross_substrate_canonical_v2.log`](../../logs/2026-05-11_bridge_cross_substrate_canonical_v2.log)
- Round 1 null baseline (W3 fix; methodologically empty for OQ#2 — see Round 2 N1): [`logs/2026-05-11_bridge_null_baseline.log`](../../logs/2026-05-11_bridge_null_baseline.log)
- **Round 2 corrected null baseline (N1+N2 fix) — the authoritative number**: [`logs/2026-05-11_bridge_null_baseline_round2.log`](../../logs/2026-05-11_bridge_null_baseline_round2.log)

**Examples**:
- [`examples/bridge_cross_substrate_canonical.rs`](../../examples/bridge_cross_substrate_canonical.rs) (revised)
- [`examples/bridge_null_baseline.rs`](../../examples/bridge_null_baseline.rs) (added per W3)

**Predecessor**: [`bridge_lean_dep_probe_phase0.md`](bridge_lean_dep_probe_phase0.md)

---

## 0. ARIS auto-review-loop Round 1 disclosures

This document was rewritten after Round 1 of the ARIS auto-review-loop
flagged seven weaknesses in the initial 2026-05-11 draft. The reviewer
ran in a fresh sub-agent context with no access to the executor's
internal monologue. Round 1 score was 3/10 ("not ready"). The
following changes were made before re-review:

| Tag | Weakness | Fix |
|-----|----------|-----|
| W1 | "synthetic Lean dep" branding overstates Lean credibility | Substrate renamed to "synthetic layered random DAG" everywhere |
| W2 | Synthetic DAG is not a credible Lean proxy | Explicit non-Lean disclaimer; "Phase 1.E real Mathlib" called out as the only credible Lean test |
| W3 | Jaccard 0.26 uninterpretable without a null baseline | New experiment `bridge_null_baseline.rs` measures within-family Jaccards |
| W4 | Single-seed result not generalizable to "the family" | Acknowledged in §5; multi-seed scan deferred |
| W5 | Truncated 64-bit hash compared as canonical identity | Replaced with direct `CanonicalForm = Vec<(u64,u64)>` set equality |
| W6 | Overclaim about derived-lemma "merge" motif being Lean-specific | Reframed as "merge appears in the layered DAG family, absent from OQ#2-sequence" |
| W7 | "it generalizes" verdict too strong | Verdict narrowed to claims actually supported by data |

The Round 1 review itself is preserved verbatim in
[`review-stage/AUTO_REVIEW.md`](../../review-stage/AUTO_REVIEW.md) as
honest counter-evidence.

---

## 1. Goal

Following ADR 0081 Phase 0's GO signal (15 patterns minted on a
synthetic substrate vs OQ#2's ~7), this slice asks: are the
*canonical forms* distinct, or just more *instances* of the same
forms?

Method: extract `pattern_structure(pid)` from both substrates
post-`autonomous_pass`, set-compare via direct `CanonicalForm`
equality (W5 fix — was previously a 64-bit hash tag).

## 2. The second substrate is NOT Lean

The substrate previously labeled "synthetic Lean dep" is a purely
synthetic random DAG with a layered+clustered structure. It is:

- **NOT** Lean source.
- **NOT** a snapshot of Mathlib.
- **NOT** the output of any proof engine.
- It IS an 80-node random DAG with three layered phases (0–20: thin
  deps; 20–50: mid deps; 50–80: dense deps) plus 5-node clique
  clusters at 15-step intervals.

The loose intuition motivating its design: math-library dep graphs
tend to have small theorems depending on a few earlier small
theorems with topic clusters forming small cliques. We replicate
**that structural flavor** — not Lean itself. Henceforth in this
document the substrate is called the "synthetic layered random
DAG" (synth-DAG).

Any claim that v2 is "Lean-substrate-sensitive" requires real
Mathlib data. That is Phase 1.E, not this slice.

## 3. Headline result (cross-substrate Jaccard)

```
                                count
OQ#2 canonicals:                  9
Synth-DAG canonicals:            15
Shared:                           5
OQ#2-only:                        4
Synth-DAG-only:                  10
Jaccard(OQ#2, synth-DAG):     0.2632
```

(Numbers from [`logs/2026-05-11_bridge_cross_substrate_canonical_v2.log`](../../logs/2026-05-11_bridge_cross_substrate_canonical_v2.log)
using direct `CanonicalForm` set equality. The original
hash-tag-based comparison produced the same numbers, confirming
no collisions; the technique was nevertheless wrong in principle
and is now corrected.)

## 4. Null baseline — the W3 fix

The Round 1 reviewer correctly demanded: *what is the Jaccard
between two independent draws of the same substrate family?* Without
that anchor the cross-substrate 0.26 is uninterpretable — it could
indicate substrate-sensitivity, OR it could be the typical noise
floor of the discovery pipeline.

`bridge_null_baseline.rs` pre-registers two hypotheses and reports
three Jaccards:

```
H0 (v2 is doing subgraph census, no substrate-sensitivity):
    Jaccard_within ≈ Jaccard_cross

H1 (substrate-sensitive emergence):
    Jaccard_within > 0.7  AND  Jaccard_cross < 0.4
    (gap > 0.3)
```

Measured (single-seed):

```
Jaccard_OQ#2_self    = 1.0000   (same OQ#2 graph, different
                                  discovery RNG seeds)
Jaccard_DAG_self     = 1.0000   (two synth-DAG draws from
                                  same family, different
                                  graph-generation seeds)
Jaccard_cross        = 0.2632   (OQ#2 vs synth-DAG)

mean within-family   = 1.0000
gap (within - cross) = 0.7368
```

**Verdict: H1 supported.** Within-family Jaccards both exceed 0.7;
cross is below 0.4; gap exceeds 0.3. All pre-registered thresholds
met.

### 4.1 Honest caveats on the perfect within-family Jaccard

Within-family Jaccard = 1.0 is *suspiciously* clean. Two
non-exclusive explanations:

1. **Discovery saturation.** OQ#2 yields only 9 canonicals; the
   synth-DAG family yields 15. Both numbers are small enough that
   `sample_count=400 / top_m=20` may be saturating — finding *all*
   structurally-eligible canonicals of size 2-3 regardless of RNG.
   In that case Jaccard=1.0 is the *correct* number but is
   *trivially* the correct number.

2. **The substrate family really does have a small invariant
   canonical set at these sizes.** Two different 80-node random
   DAGs from the same layering+clustering family really do produce
   the same 15 canonicals — because the family's structural
   vocabulary at sizes 2-3 has only 15 distinct shapes.

Both interpretations support H1 (the cross is genuinely lower
than within-family). The conservative reading is: *under this
discovery budget, v2 produces a deterministic structural fingerprint
per substrate family, and the fingerprints differ between
substrate families*.

What would falsify this conservative reading: a multi-seed scan
in which within-family Jaccard drops below 0.7 on some seeds.
That is **W4**, deferred.

## 5. Shared canonicals (universal small motifs)

5 canonicals appear in both. These are the graph-theoretic
fundamentals any sufficiently connected directed graph contains:

```
edges=2  bidirectional pair          (2 roles, 2 edges)
edges=3  3-cycle                     (3 roles, 3 edges)
edges=3  star (hub of degree 3)      (4 roles, 3 edges)
edges=2  fork (1 source, 2 targets)  (3 roles, 2 edges)
edges=2  chain (length 2)            (3 roles, 2 edges)
```

These are the universal small-motif vocabulary. Both OQ#2 (tournament
/ lattice / star regimes) and the synth-DAG (layered + clustered)
contain them.

## 6. OQ#2-only canonicals (4)

```
edges=4  4-edge graph on 4 nodes    — OQ#2 lattice ridges
edges=5  5-edge graph on 4 nodes    — OQ#2 dense clique (variant 1)
edges=5  5-edge graph on 4 nodes    — OQ#2 dense clique (variant 2)
edges=3  3-cycle (distinct variant) — distinct from shared 3-cycle
```

Dense small subgraphs from OQ#2's tournament / lattice / star
regimes. The synth-DAG's layered structure doesn't produce these
density profiles at sizes 2-3 within the sampled budget.

## 7. Synth-DAG-only canonicals (10)

10 distinct structural categories v2 mints on synth-DAG but not OQ#2:

```
edges=2  merge (two sources, one target)
edges=3  3-edge graph on 4 nodes   (4 variants)
edges=3  star (hub of degree 3)    (3 variants — beyond the shared one)
edges=3  3-edge triple             (2 variants)
```

Notable structural observations:

- **Merge (two-source-one-target).** This is a 2-in-degree node:
  "C ← A, C ← B." It appears in the synth-DAG family because the
  layered DAG explicitly emits multiple incoming edges per node
  (deps=2-5 in layers 1-2). It does NOT appear in OQ#2-sequence
  because OQ#2's tournament/lattice/star regimes don't natively
  emit 2-in-degree small subgraphs of this shape at size 2-3.

  **Important narrowing of the original claim (W6 fix):** the
  original draft called this "a structural signature of
  derived-lemma dependency." That framing is misleading — merge
  is a generic 2-in-degree pattern. Many natural graphs produce
  it (citation graphs, family trees, AST sharing). It is a
  signature of the *layered DAG with branching deps* family, of
  which derived-lemma dependency is one instance. The merge
  motif tells us: this substrate has 2-in-degree-1 small
  neighborhoods that OQ#2 does not.

- **Multiple stars distinguishable by canonical form.** Synth-DAG
  produces 3 star variants beyond the shared one — likely
  reflecting different hub-spoke incidence patterns in the
  clustered structure. OQ#2 produces 1 star canonical. v2's
  canonicalization correctly distinguishes incident-pattern
  variants without collapsing them.

- **5 distinct 4-node 3-edge variants.** The cluster structure
  (5-node interlinked bundles) yields many distinct 4-node
  sub-shapes under sampling.

## 8. What this slice supports — *narrowly*

1. **OQ#2 sequence structure and layered-DAG-with-clusters
   structure produce distinguishable canonical-form populations.**
   Cross-family Jaccard 0.26; within-family Jaccard 1.0; gap 0.74.
   This is real differentiation, not pipeline noise.

2. **v2's canonical-form machinery is structurally informative.**
   Different substrate families fingerprint to different small-motif
   distributions. This is a precondition for any downstream
   substrate-specific reasoning.

3. **Multi-variant emergence works.** Where OQ#2 has 1 star,
   synth-DAG has 4 (3 unique + 1 shared). v2's canonicalization
   correctly distinguishes them — stars with different incidence
   patterns aren't collapsed into a single pattern.

## 9. What this slice does NOT support

- **NOT a claim about Lean substrate-sensitivity.** The synth-DAG
  is not Lean. That requires Phase 1.E real Mathlib data.

- **NOT a claim of "v2 generalizes to natural-data structural
  categories beyond hand-crafted tests."** The synth-DAG IS
  hand-crafted; it is just hand-crafted differently from OQ#2.
  Both are synthetic.

- **NOT a multi-seed result.** Each Jaccard reported is single-seed
  per family. Variance across seeds is W4, deferred.

- **NOT a claim that the cross-substrate Jaccard 0.26 is
  *predictive* of any real-world performance.** It is one
  measurement on one pair of synthetic substrates.

## 10. Significance vs prior cross-substrate comparisons

ADR 0075 piece 3 (2026-05-06) compared OQ#2 against OQ#1-clade
substrates and found Jaccard 0.17 — OQ#2 was structurally distinct
from the canonical synthetic suite.

This slice (2026-05-11) compares OQ#2 against a synth-DAG family
and finds Jaccard 0.26.

Both fall in the 0.15-0.30 range = "substantively distinguishable
without being completely disjoint." The null-baseline measurements
in this revision (within-family Jaccard 1.0, gap 0.74) anchor that
range as *not* the pipeline's noise floor — it represents real
cross-family differentiation given the discovery budget.

## 11. Follow-ups

- **Phase 1.E** — real Mathlib extraction. Without this, we have
  no Lean claim at all.
- **W4 follow-up** — multi-seed scan of within-family Jaccard. Goal:
  confirm Jaccard_within > 0.7 across ≥10 seeds, not just one.
- **Bigger discovery budget.** Re-run at `sample_count=4000 /
  top_m=200` to see if the canonical sets stabilize at the same
  count or grow.
- **Multi-size canonical scan.** Currently sizes 2-3; sizes 4-6
  may produce more discriminating canonical sets.
- **Theory comparison.** Both substrates' theory_candidate sets
  are empty after `autonomous_pass`. Investigate why.

## 12. Files

- `examples/bridge_cross_substrate_canonical.rs` (revised, W1+W5+W6 fixes)
- `examples/bridge_null_baseline.rs` (new, W3 fix)
- `logs/2026-05-11_bridge_cross_substrate_canonical.log` (original)
- `logs/2026-05-11_bridge_cross_substrate_canonical_v2.log` (revised)
- `logs/2026-05-11_bridge_null_baseline.log` (new)
- This result doc (revised)

## 13. Round 2 update — H1 is NOT supported by the corrected baseline

**Honest negative finding** added 2026-05-11 after ARIS Round 2 review.

The Round 2 reviewer flagged two methodological problems with the Round 1 baseline:

- **N1** — The OQ#2 "within-baseline" varied only the discovery RNG, not the input graph. It was a sampler-determinism check, not a substrate-family variance test.
- **N2** — Within-family Jaccard = 1.0 was equally consistent with H1 (genuine convergence) and H0 (saturation under sample_count=400). The original experiment did not discriminate.

The Round 2 revision of `bridge_null_baseline.rs` fixes both:
- Replaces OQ#2-self with **OQ#1 vs narrow_a** — two genuinely different canonical-suite graphs.
- Adds a **saturation probe** at sample_count=50, top_m=5. Under H1, within-family Jaccard should stay high even at low budget; under H0/saturation, within-family Jaccard collapses.

### 13.1 Round 2 measurements (log [`logs/2026-05-11_bridge_null_baseline_round2.log`](../../logs/2026-05-11_bridge_null_baseline_round2.log))

```
Saturation budget (sample_count=400, top_m=20):
  Within(OQ#1, narrow_a)      = 0.2000      ← ALSO low
  Within(DAG_A, DAG_B)        = 1.0000
  Cross(OQ#1, DAG_A)          = 0.1875
  Cross(OQ#2, DAG_A)          = 0.2632
  within_mean=0.6000 cross_mean=0.2253 gap=0.3747

Low budget (sample_count=50, top_m=5):
  Within(OQ#1, narrow_a)      = 0.0000      ← drops further
  Within(DAG_A, DAG_B)        = 0.7778      ← drops from 1.0
  Cross(OQ#1, DAG_A)          = 0.2500
  Cross(OQ#2, DAG_A)          = 0.1333
  within_mean=0.3889 cross_mean=0.1917 gap=0.1972
```

### 13.2 What these numbers say

The pre-registered H1 thresholds (saturation: within > 0.7 AND cross < 0.4) require **both** within-family Jaccards to clear 0.7. They do not. **Within(OQ#1, narrow_a) = 0.2 at saturation budget** is essentially identical to **Cross(OQ#1, DAG_A) = 0.19**. Two members of the canonical synthetic family are structurally as different from each other as one of them is from a synth-DAG.

Interpreted strictly:

- **The earlier (Round 1) verdict was wrong.** It was sustained by a methodologically empty within-baseline. The corrected within-baseline shows there is no general "substrate-family fingerprint" at the canonical-form level under this discovery configuration.

- **What remains supported (with Round 3 M1 caveat)**: the layered-random-DAG generator family has a small invariant size-2/3 motif vocabulary that the discovery pipeline saturates on at sample_count=400 — Within(DAG_A, DAG_B) = 1.0 — and that the discovery pipeline partially recovers at sample_count=50 — Within(DAG_A, DAG_B) = 0.78. This is a property of THE GENERATOR'S small-motif structure, not a property of v2's substrate-sensitivity. Framing it as "v2 produces a stable fingerprint" would smuggle a v2-capability claim back in.

- **The cross-substrate Jaccard 0.26 between OQ#2 and synth-DAG** — the original headline of Phase 1.D — is now seen as not meaningfully different from the within-canonical-suite Jaccard 0.20 **at this single seed**. The structural-distinctness claim collapses into "two graphs with different generative processes produce different size-3 subgraph censuses." That is the Round 1 reviewer's W2/W6 dismissal exactly: graph theory, not emergence. Round 3 M3 correctly noted that the single-seed nature of Within(OQ#1, narrow_a) = 0.20 means the negative verdict is stated with somewhat more confidence than a single configuration licenses — "fails the threshold at this single configuration" is the rigorous phrasing.

### 13.3 What we now claim, very narrowly

After Round 2 the only defensible substantive claims are:

1. **v2's discovery pipeline computes the size-2-3 canonical census on each input graph**, and different input graphs produce different censuses. This is trivially true and not a property of v2 specifically.

2. **The synth-DAG generator alone produces a stable canonical fingerprint across seeds.** Within(DAG_A, DAG_B) = 1.0 at saturation, 0.78 at low budget. This is informative about that particular DAG generator (small invariant canonical vocabulary at sizes 2-3), not about v2's substrate-sensitivity in general.

3. **The cross-substrate Jaccard 0.26 reported in Phase 1.D is not interpretable as evidence of substrate-sensitivity** without a within-family baseline that exceeds it consistently — and the corrected within-baseline does not.

### 13.4 What we explicitly retract

- **Retracted**: "v2's pattern emergence machinery produces substrate-distinct structural categories" as a property of v2.
- **Retracted**: "Jaccard 0.26 consistent with prior 0.17 = substrate-sensitive without over-fitting" as a generalizing inference.
- **Retracted**: §10's "both Jaccards in the 0.15-0.30 range" framing as N=2 range-fitting that the Round 2 reviewer correctly called out (N4) and that the corrected within-baseline now disconfirms.
- **Retracted**: Round 1's "H1 SUPPORTED" verdict. It depended on the methodologically empty OQ#2-self baseline.

### 13.5 What this means for the bridge (ADR 0081)

The pipeline still runs end-to-end; that was never the contested claim. What Phase 1.D failed to establish is that the minted canonicals are *informatively substrate-sensitive in a way that distinguishes v2 from a subgraph-census routine*. Phase 1.E (real Mathlib) remains the next experiment; Phase 1.D's role is now downgraded from "substantive finding" to "honest negative null-baseline check."

The original Phase 0 GO signal (richer pattern population on synth-DAG than OQ#2) survives — that's about count, not about distinctness. But "the bridge surfaces substrate-distinct structural emergence" was an over-reading that Round 2 has corrected.

### 13.6 Open questions left by Round 2

- **Multi-seed scan on canonical-suite pairs.** Is Within(OQ#1, narrow_a) = 0.2 robust, or did this single seed happen to land on a bad value? **Addressed by Round 4 multi-seed scan — see §14 below.**
- **Bigger canonical sets.** OQ#1 only mints 4 canonicals; small sets make Jaccard noisy. Re-running at sizes 4-6 may produce larger canonical sets and more stable Jaccards.
- **Different generative families.** Erdős–Rényi, preferential attachment, planted-partition — does the within-vs-cross pattern look the same across the structural-graph zoo, or is it idiosyncratic?
- **Within-substrate-with-resampled-stream.** OQ#2 and OQ#1 are fully deterministic. Building stream-seeded variants of OQ#2 would enable a true OQ#2-self baseline. That's a substrate-engineering task.

## 14. Round 4 — multi-seed scan (W4 follow-up)

The Round 3 reviewer (M3) and §13.6 above both flagged that Round 2's `Within(OQ#1, narrow_a) = 0.20` was a single-seed value. A multi-seed scan was run on 2026-05-11 to test whether 0.20 was outlier or typical.

Full follow-up doc: [`bridge_multi_seed_scan.md`](bridge_multi_seed_scan.md).
Log: [`logs/2026-05-11_bridge_multi_seed_scan.log`](../../logs/2026-05-11_bridge_multi_seed_scan.log).
Example: [`examples/bridge_multi_seed_scan.rs`](../../examples/bridge_multi_seed_scan.rs).

### 14.1 Numbers

```
Within-canonical-suite (N=6 pairs from {OQ#1, narrow_a, OQ#2, long5k}):
   mean = 0.2636   std = 0.3406   min = 0.0000   max = 1.0000

   per-pair:
      Within(OQ#1, narrow_a)     = 0.2000   ← Round 2 single seed (TYPICAL)
      Within(OQ#1, OQ#2)         = 0.1818
      Within(OQ#1, long5k)       = 0.2000
      Within(narrow_a, OQ#2)     = 0.0000
      Within(narrow_a, long5k)   = 1.0000
      Within(OQ#2, long5k)       = 0.0000

Within-synth-DAG (N=15 pairs from 6 DAG seeds):
   mean = 0.9583   std = 0.0589   range [0.8750, 1.0000]

Cross (N=24, 4 canonical × 6 DAG):
   mean = 0.1127   std = 0.1158   range [0.0000, 0.2632]
```

### 14.2 What this says

**Round 2 retraction is reinforced**, not premature:

- Within-canonical mean 0.26 exceeds Cross mean 0.11 by **only 0.15**, while within-canonical std is **0.34** — more than twice the gap. The "within > cross" difference is **not statistically meaningful** given the dispersion.

- Within-canonical Jaccards are **bimodal-ish**: 2 pairs share nothing (0.0), 1 pair shares everything (1.0), 3 pairs share ~20%. The "canonical suite" is not a substrate family in the variance-bounded sense Round 2's null baseline implicitly assumed.

- Round 2's single-seed 0.20 is **typical**, not outlier (3 of 6 within-canonical pairs are 0.18-0.20).

- The synth-DAG family genuinely IS tight: mean 0.96, std 0.06. This survives as the only positive signal, but it is a property of **the layered-random-DAG generator** (small invariant motif vocabulary at sizes 2-3), not of v2 substrate-sensitivity. Round 3 M1 framing stands.

### 14.3 Surprising N=6 observations worth noting

- `(narrow_a, long5k) = 1.0` — narrow_a's canonical set is a strict subset of long5k's (both anchored on regime-A diamond posets). They have IDENTICAL canonical fingerprints at sizes 2-3.
- `(narrow_a, OQ#2) = 0.0` and `(OQ#2, long5k) = 0.0` — completely disjoint canonical sets between OQ#2 and the narrow_a/long5k family. The dense small subgraphs OQ#2 emits (4-5 edge canonicals on 4 nodes) are absent from regime-A-style substrates.
- `narrow_a` and `long5k` have **zero** canonical overlap with the synth-DAG family. Only OQ#1 (Jaccard 0.19) and OQ#2 (Jaccard 0.26) share anything with the DAG.

These observations are structurally informative about WHICH substrates have which motifs, but they do not support a general "v2 is substrate-sensitive" claim — they reflect the specific compositional content of each pre-built substrate.

### 14.4 Final state of the Phase 1.D claim across 4 rounds

| Round | Substantive claim | Status |
|-------|------------------|--------|
| 0 (original) | "v2 produces substrate-distinct emergence; 67% novel; Phase 2 motivated" | Withdrawn (Round 2) |
| 1 (W1-W7 fixes) | "H1 supported at pre-registered thresholds" | Withdrawn (Round 2) |
| 2 (N1+N2 corrected baseline) | "Phase 1.D verdict retracted; cross 0.26 not substrate-sensitivity evidence" | Stands |
| 3 (M1-M3 framing) | "Surviving positive is DAG-generator invariance, not v2 capability" | Stands |
| 4 (this scan) | "Within-canonical mean 0.26 std 0.34; gap < 1 std; canonical suite is not a variance-bounded family" | Stands; **strengthened by N>1** |

## 15. Round 5 — multi-family scan (universal small-motif vocabulary)

Per user direction "扩展 retraction 实证基础" (after Round 3 ARIS loop exit), a Round 5 scan added 3 more generative families (Erdős–Rényi, Barabási–Albert, stochastic block model) at 6 seeds each, all n=80 with ~250 directed edges, plus a full within / cross matrix at sizes 2-3 saturation budget.

Full follow-up doc: [`bridge_multi_family_scan.md`](bridge_multi_family_scan.md). Log: [`logs/2026-05-11_bridge_multi_family_scan.log`](../../logs/2026-05-11_bridge_multi_family_scan.log).

### 15.1 The killer finding

**Three different random-graph families (ER, SBM, synth-DAG) at sizes 2-3 produce essentially identical canonical-form sets.**

```
Within-family Jaccards:
  ER (N=15):         mean=0.87  std=0.05
  SBM (N=15):        mean=0.95  std=0.04
  synth-DAG (N=15):  mean=0.96  std=0.06

Cross-family Jaccards:
  ER × SBM (N=36):       mean=0.91  std=0.05    ← essentially = within
  ER × synth-DAG (N=36): mean=0.91  std=0.05    ← essentially = within
  SBM × synth-DAG (N=36): mean=0.95  std=0.05   ← essentially = within
```

ER vs SBM cross-Jaccard (0.91) is numerically indistinguishable from ER's own within-family Jaccard (0.87). For random-graph families at this scale, v2's canonical-form output is the **same regardless of generative process**.

### 15.2 Cross to canonical-suite (consistent with Round 4)

```
canonical × ER:   N=24  mean=0.12  std=0.13
canonical × SBM:  N=24  mean=0.12
canonical × DAG:  N=24  mean=0.11  std=0.12
```

The canonical-suite differs sharply from random-graph families (cross ≈ 0.12), but is internally heterogeneous (Round 4 within-canonical mean 0.26 std 0.34). The original Phase 1.D 0.26 OQ#2-vs-synth-DAG cross sits comfortably inside this 0.0-0.33 cross-to-random range.

### 15.3 BA scaling observation (separate)

BA (Barabási–Albert, m=3) was attempted but skipped after a single instance took **38 minutes** for size=3 autonomous_pass (vs ER's ~100s). v2's discovery pipeline at saturation budget does **NOT scale on power-law hub-rich structures at n=80**. Documented as a quantitative scaling observation about v2's discovery pipeline, not a substrate-sensitivity result. ADR-grade investigation warranted if BA-style natural graphs (web link, scientific citation) become target substrates.

### 15.4 What this tells us

- **v2's pattern discovery at sizes 2-3 saturates on a universal small-motif vocabulary for random directed graphs of density ~0.04 on n=80**. ~13-16 canonicals, invariant to the generative process.
- **The "substrate-sensitive emergence" claim is now triply retracted**:
  - Round 2: cross ≈ within-OQ#2-sampler (initial baseline)
  - Round 4: within-canonical-suite mean ≈ cross mean (with high std)
  - Round 5: **all random-graph families produce the same canonical set; v2 cannot distinguish them**
- **What v2 DOES distinguish at sizes 2-3**: hand-crafted stream substrates (canonical-suite) FROM "generic random graph" — cross ≈ 0.12 vs within-random ≈ 0.91. The 0.26 OQ#2-vs-synth-DAG Jaccard is exactly this, not "substrate-sensitivity in general."

### 15.5 Updated surviving narrow positive

After Round 5 the "narrow positive" of Round 3 M1 (DAG-generator-family fingerprint) is no longer special — ER and SBM produce the SAME fingerprint as synth-DAG. The discovery saturation regime is the explanation. The only surviving non-trivial measurement is:

> **v2 at sizes 2-3 distinguishes "structured stream substrate" (canonical-suite) FROM "generic random graph" (ER/SBM/DAG) under this discovery configuration. Cross-Jaccard ≈ 0.12 vs within-random ≈ 0.91.**

This is a descriptive measurement, not an emergent-cognition capability. Phase 1.E real Mathlib remains the gating experiment for any claim about v2 on real-world data.

### 15.6 Final state of the Phase 1.D claim across 5 rounds

| Round | Substantive claim | Status |
|-------|------------------|--------|
| 0 (original) | "v2 produces substrate-distinct emergence; 67% novel; Phase 2 motivated" | Withdrawn (Round 2) |
| 1 (W1-W7 fixes) | "H1 supported at pre-registered thresholds" | Withdrawn (Round 2) |
| 2 (N1+N2 baseline) | "Phase 1.D verdict retracted; cross 0.26 not substrate-sensitivity evidence" | Stands |
| 3 (M1-M3 framing) | "Surviving positive is DAG-generator invariance, not v2 capability" | Stands |
| 4 (multi-seed N>1) | "Canonical suite is not variance-bounded family" | Stands; strengthened |
| **5 (multi-family)** | **"v2 sizes 2-3 produces universal small-motif vocabulary across random-graph families; cannot distinguish ER from SBM from synth-DAG"** | **Stands; substantially strengthens retraction** |

## 16. Round 6 — structural-class scan (refinement)

Asked: does v2 distinguish structural CLASSES (tree, bipartite, random) even though Round 5 showed it cannot within the random class?

Full follow-up doc: [`bridge_structural_class_scan.md`](bridge_structural_class_scan.md). Log: [`logs/2026-05-11_bridge_structural_class_scan.log`](../../logs/2026-05-11_bridge_structural_class_scan.log).

### 16.1 Numbers

```
Within-class:
  canonical-suite (N=6): mean=0.26  std=0.34  [0.00, 1.00]
  TREE (N=15):           mean=1.00  std=0.00  [1.00, 1.00]   ← PERFECT
  BIPARTITE (N=15):      mean=1.00  std=0.00  [1.00, 1.00]   ← PERFECT
  synth-DAG (N=15):      mean=0.96  std=0.06  [0.88, 1.00]

Cross-class:
  canonical × any random: 0.11-0.17
  BIPARTITE × synth-DAG:  0.33    ← sharp distinction
  BIPARTITE × TREE:       0.42    ← moderate
  TREE × synth-DAG:       0.78    ← heavy overlap (both acyclic)
```

### 16.2 What this refines

- **Structurally-constrained classes saturate to perfect invariance** (Jaccard = 1.0 across all seeds within TREE / BIPARTITE).
- **BIPARTITE vs synth-DAG is the cleanest cross-class distinction** at cross-Jaccard 0.33 — bipartite excludes 3-cycle, self-loop, L→L motifs that synth-DAG has.
- **TREE vs synth-DAG fails to distinguish** at cross-Jaccard 0.78 — both are acyclic; TREE's 12 canonicals are mostly a subset of DAG's 15. (Caveat: this "TREE" includes forward-DAG noise edges; pure tree might score lower.)
- **No class strictly passes H1** at the within>0.7 AND max-cross<0.4 thresholds. BIPARTITE is marginal (max cross 0.42 vs TREE).

### 16.3 Updated surviving narrow positive

> Under saturation budget at sizes 2-3, v2 distinguishes substrates if and only if their structural constraints exclude different motifs from the size-2-3 canonical vocabulary.

This is graph theory (subgraph census reflecting structural constraints), not v2-specific cognition or "emergent substrate-sensitivity." Phase 1.E real natural-data substrates remain the only experiment that could change the picture.

### 16.4 Final state across 6 rounds

| Round | Status |
|-------|--------|
| 0-1 | Original claim Withdrawn |
| 2-3 | Retracted with framing tweaks |
| 4 | N>1 confirmation of canonical-suite heterogeneity |
| 5 | Universal small-motif vocabulary across random-graph families |
| **6** | **Class-constraint-determined canonical census; bipartite cleanly distinguishable from random-DAG; tree-with-noise overlaps with random-DAG** |
