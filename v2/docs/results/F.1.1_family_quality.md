# F.1.1 — Per-axiom cross-precision in family discovery

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_f11_family_quality.log`](../../logs/2026-04-29_phase_f11_family_quality.log)
**Example**: [`examples/phase_f11_family_quality.rs`](../../examples/phase_f11_family_quality.rs)

## Goal

F.1 shipped `axiom_cross_precision(ax, substrates)` — per-axiom quality scalar.
Beta-1 family discovery groups axioms by structural shape only — ignoring quality.

F.1.1 fuses them: for each shape family, summarize the quality of its members. Adds a quality dimension on top of the structural one.

## Method

1. Run runtime → discover axioms + theories
2. `discover_axiom_shape_families(2)` → 6 families (3 premise + 3 conclusion, per B.3)
3. Generate per-theory substrates (DreamCoder-style)
4. For each family member: compute `axiom_cross_precision`
5. Per-family aggregate: mean, std, min, max

## Result on OQ#1 @ 1000 ticks

| family | n_mem | mean | std | min | max |
|---|---|---|---|---|---|
| **shape_premise_p0-1** | 3 | **0.9298** | 0.0993 | 0.7894 | 1.0000 |
| **shape_premise_p0-1_p1-2** | 2 | **0.8947** | 0.1053 | 0.7894 | 1.0000 |
| shape_conclusion_c0-2 | 3 | 0.7598 | 0.2076 | 0.4936 | 1.0000 |
| shape_conclusion_c2-0 | 2 | 0.6415 | 0.1479 | 0.4936 | 0.7894 |
| shape_conclusion_c1-0 | 2 | 0.6415 | 0.1479 | 0.4936 | 0.7894 |
| **shape_premise_p0-0_p1-2** | 4 | **0.4936** | **0.0000** | 0.4936 | 0.4936 |

## Classification (mean ≥ 0.80 → signal; mean < 0.50 → noise; std < 0.05 → uniform)

- **Signal families (2)**: `shape_premise_p0-1`, `shape_premise_p0-1_p1-2`
- **Noise families (1)**: `shape_premise_p0-0_p1-2` (the variance-zero family)
- **Uniform families (1)**: `shape_premise_p0-0_p1-2` ← same family

## Verdict

**POSITIVE — quality dimension separates noise from signal families on OQ#1.**

Critical reproduction: F.1.1 confirms Beta-1's flagship empirical finding — the `shape_premise_p0-0_p1-2` family has **variance = 0.0000** across its 4 members. All 4 noise axioms behave identically under cross-precision. F.1.1 quantifies this from per-axiom data; Beta-1 had it from family-level aggregate; the two are now connected.

## What this slice produced

1. F.1.1 — per-family quality summary using F.1's `axiom_cross_precision`
2. Classification scheme: signal / noise / uniform based on mean and variance thresholds
3. Reproduction of Beta-1's variance-zero finding from a parallel-but-independent computation path
4. Conclusion-family quality (B.3 said conclusions don't capture quality): F.1.1 confirms — c0-2 has std 0.21 (high spread), c1-0 and c2-0 each have std 0.15. Conclusion-family means are middling (0.64-0.76), nowhere near the bimodal signal/noise gap that premise families show.

## Why F.1.1 + B.3 together close a B.3 caveat

B.3 found: conclusion families exist but don't capture the quality dimension. F.1.1 numerically confirms: conclusion families have spread > 0.15 (vs uniform = std < 0.05) AND middling means (0.64-0.76 vs noise 0.49 vs signal 0.89-0.93). They are genuinely intermediate — neither cleanly signal nor cleanly noise.

This means: on OQ#1, **structural premise IS the dominant quality axis**. Conclusion structure correlates with quality only weakly. This is a substrate-specific observation; on a different substrate, conclusion structure could dominate.

## Future implications

- **Family-level demote (B.2) refinement**: B.2 currently demotes when family mean < 0.65 AND variance < 0.05. F.1.1's classification could replace these heuristics with the signal/noise/uniform vocabulary directly.
- **Family-aware merge selector (F.2.1+)**: a "merge into the same family if families are both signal" rule
- **Cross-substrate family quality**: F.1.1 quality is per-substrate-set; computing across multiple substrates would identify families whose quality is *itself* substrate-stable (most universal predictors)
- **Visualization**: F.1.1's table is a starting point for a family-quality dashboard; future work could surface this in tooling
