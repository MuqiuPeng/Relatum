# Multi-substrate consolidation diagnostic — OQ#1 + long5k + OQ#2

**Status**: ✓ done (2026-05-01)
**Log**: [`logs/2026-05-01_phase_consolidation_multi_substrate_diagnostic.log`](../../logs/2026-05-01_phase_consolidation_multi_substrate_diagnostic.log)
**Example**: [`examples/phase_consolidation_multi_substrate_diagnostic.rs`](../../examples/phase_consolidation_multi_substrate_diagnostic.rs)
**ADRs validated across substrates**: [0070](../decisions/0070-shape-family-abstraction-layer.md), [0071](../decisions/0071-unified-theory-quality-report.md), [0072](../decisions/0072-intervention-policy-classifier.md)

## Goal

The OQ#1 diagnostic showed the consolidation triad works on its
native substrate. This slice answers two follow-up questions:

1. **Same-regime-type generalization**: do the SAME recommendations
   come out on long5k (a different stream with the SAME regime
   types per C.2)?
2. **Graceful degradation**: what does the pipeline produce on
   OQ#2 (a structurally-hostile substrate where C.2.1 found 0
   forward-applicable template axioms)?

## Setup

| substrate | ticks | source |
|---|---|---|
| OQ#1 | 1000 | `test_substrates::oq1::build_long_stream()` |
| long5k | 1500 | `test_substrates::long5k::build_5k_stream()` |
| OQ#2 | 4500 | `test_substrates::oq2::build_oq2_stream()` |

Per-theory imagined substrates: 4 each (NUM_GEN_IDS=15, density=0.05).

## Results

### Per-substrate state

| substrate | ticks | axioms | theories | L2 families | substrates | qualifying axioms |
|---|---|---|---|---|---|---|
| OQ#1 | 1000 | 13 | 4 | 6 | 4 | 11 |
| **long5k** | **1500** | **13** | **4** | **6** | **4** | **11** |
| OQ#2 | 4500 | 2 | 2 | **0** | 2 | **0** |

OQ#1 and long5k produce **identical structural state** — exactly
the C.2 finding ("same regime types → same shape families")
reproduced through the unified API. OQ#2 is starkly different:
13→2 axioms, 6→0 L2 families, 11→0 axioms with primary data.

### Recommendation distribution

| substrate | None | Shadow | FamilyDem | Repair | TheoryDem | Super | Merge | Manual |
|---|---|---|---|---|---|---|---|---|
| OQ#1 | 2 | 0 | 1 | 0 | 0 | 0 | 0 | 1 |
| **long5k** | **2** | **0** | **1** | **0** | **0** | **0** | **0** | **1** |
| OQ#2 | 0 | **2** | 0 | 0 | 0 | 0 | 0 | 0 |

OQ#1 ≡ long5k recommendation distribution. OQ#2 lands entirely
in `ShadowMonitor` — the correct response to "no data on any
quality dimension".

### Per-theory recommendations

| theory | OQ#1 | long5k | OQ#2 |
|---|---|---|---|
| t_0 | `FamilyDemote(shape_premise_p0-0_p1-2, Uniform)` | `FamilyDemote(shape_premise_p0-0_p1-2, Uniform)` | `ShadowMonitor("no data...")` |
| t_1 | `Manual(...)` | `Manual(...)` | `ShadowMonitor("no data...")` |
| t_2 | `None` | `None` | (no t_2 — only 2 theories on OQ#2) |
| t_3 | `None` | `None` | (no t_3) |

OQ#1 and long5k produce **byte-identical recommendation strings**
for every theory id. The numerical values behind them differ
slightly:

| dim | OQ#1 t_0 | long5k t_0 |
|---|---|---|
| primary mean | 0.3759 | 0.3640 |
| cross mean | 0.6835 | 0.6835 |
| t_3 primary | 0.9144 | 0.9673 |

But the `summary_class` and the chosen recommendation step are
identical — confirming that ADR 0072's thresholds aren't
brittle to within-regime-family numerical drift.

## Sanity verdict

| substrate | result | meaning |
|---|---|---|
| OQ#1 | ✓ 3/3 | t_2/t_3 None; t_0 noise-targeting |
| long5k | ✓ 3/3 | same expectations as OQ#1 |
| OQ#2 | ✓ graceful | 2 ShadowMonitor recommendations; no aggressive (FamilyDemote/TheoryDemote) calls on a 0-family substrate |

**STRONGLY POSITIVE: triad works across all 3 substrates.**

## Why OQ#2's graceful degradation matters

OQ#2 is the structurally-hostile case. The runtime saw 4500
ticks of tournament + lattice + star data and discovered:

- 2 axioms (the predicate axioms `ax_reflexivity` and
  `ax_antisymmetry`; template axioms fail because OQ#2's
  tournament regime deliberately violates transitivity)
- 2 theories built from those axioms
- **0 L2 families** (Beta-1's `discover_axiom_shape_families`
  excludes predicate axioms; no template axioms → no families)
- **0 axioms with primary hit-rate data** (predicate axioms
  don't accumulate per-axiom hit rates via the template path)

The classifier sees:
- `summary_class == Indeterminate` (Step 0 of decision tree)
- → `ShadowMonitor { reason: "no data on any quality dimension" }`

This is the **correct** response to genuinely-insufficient data.
A naive classifier might:
- Recommend `TheoryDemote` ("no signal must mean it's bad")
- Crash on empty data structures
- Default to `Manual` (less informative than `ShadowMonitor`)

Instead the triad surfaces "we have no evidence; observe more"
as a diagnostic message. **The Indeterminate-vs-Mixed
distinction (introduced in ADR 0071) is load-bearing here**:
without it, OQ#2's empty-data theories would be classed as
Mixed (everything is missing → mean = 0.0 → < 0.50 → Noise),
producing a wrong TheoryDemote recommendation.

## What this validates beyond the OQ#1 single-substrate test

1. **Cross-substrate consistency**: same regime types → same
   recommendation set. The classifier isn't OQ#1-overfitted.
2. **Numerical robustness**: t_0's primary mean drifts 0.3759
   → 0.3640 across substrates, but the recommendation stays
   identical. Thresholds aren't on a knife edge.
3. **Hostile-substrate degradation**: pipeline doesn't crash on
   OQ#2; produces semantically-correct ShadowMonitor instead of
   misleading aggressive recommendations.
4. **Indeterminate as a real signal**: the summary class
   distinguishing "no data" from "data but mixed" is empirically
   load-bearing on OQ#2. Removing it would convert ShadowMonitor
   recommendations into incorrect TheoryDemote recommendations.

## What this does NOT validate

- **Adversarial substrates**: OQ#2 is hostile by failing to
  produce template axioms. A more adversarial test (an OQ#3
  with template axioms that LIE — e.g., axioms predicting
  randomly-mismatched edges) is open.
- **Long-horizon stability**: each substrate is run once. Whether
  recommendations are stable across re-runs (or under
  perturbation of substrate generation seeds) isn't tested.
- **Recommendation execution**: still pure read; nothing acts on
  the recommendations.

## What this slice produced

1. End-to-end multi-substrate diagnostic example
2. Empirical confirmation that OQ#1 ↔ long5k produce identical
   recommendations (byte-identical strings; minor numerical
   drift in stats)
3. Empirical confirmation that OQ#2 degrades gracefully via
   `ShadowMonitor` rather than misleading aggressive
   recommendations
4. Demonstration that the `Indeterminate` summary class is
   load-bearing in practice (specifically on OQ#2)
5. A reusable cross-substrate diagnostic battery — running the
   triad on a new substrate is now ~5 lines of code

## Future implications

- **CI / regression**: this example is small and fast (~10s).
  Adding it as part of a release-gate "diagnostic battery" run
  would catch threshold drift / regression in a single run.
- **New substrate validation**: when adding a new substrate
  (engineered or otherwise), running it through this diagnostic
  surfaces "what does the runtime think of you" — a cheap
  characterization tool.
- **Threshold tuning**: numerical drift between OQ#1 and long5k
  is ~0.01-0.05 on aggregate stats. This empirically bounds how
  much margin the classifier has on its thresholds — useful
  when ADR 0072's constants are revisited.
- **Open question**: would `OQ#3 = engineered transitivity-honest
  but tournament-shaped` recover template-axiom discovery? Could
  inform substrate engineering for richer test coverage.

## Verdict

**STRONGLY POSITIVE — 3-substrate cross-validation.** The
consolidation triad behaves consistently on same-regime-type
substrates and degrades gracefully on hostile ones. The
"experiment heap → structural system" turning point (user's
2026-04-30 strategic critique) is now validated empirically
across the substrate landscape, not just on the canonical OQ#1.
