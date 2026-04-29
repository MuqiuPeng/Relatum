# B.3 — Shared-conclusion family kind

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_beta_3_with_conclusion_families.log`](../../logs/2026-04-29_phase_beta_3_with_conclusion_families.log)

## Goal

Extend `discover_axiom_shape_families` with a second family kind: **shared canonicalized conclusion**. Test if grouping axioms by what they predict (rather than what they require) captures a meaningful quality dimension.

## Implementation

Single function call now mints both family kinds:
- `shape_premise_<canonical>` — axioms with identical premise edge sets (B.1)
- `shape_conclusion_c<x>-<y>` — axioms with identical canonicalized conclusion edge (NEW)

Predicate axioms still skipped (no template). One unit test added (`adr0068_shape_family_conclusion_kind`); existing test updated to expect the new conclusion family. 542 lib tests pass.

## Result on OQ#1

Total families discovered: **6** (3 premise + 3 conclusion).

Premise families (B.1):
| family | n | mean | var | structural verdict |
|---|---|---|---|---|
| shape_premise_p0-0_p1-2 | 4 | 0.5099 | **0.000000** | NOISE FAMILY |
| shape_premise_p0-1 | 3 | 0.9057 | 0.017776 | MIXED |
| shape_premise_p0-1_p1-2 | 2 | 0.8586 | 0.019998 | MIXED |

Conclusion families (B.3, new):
| family | n | mean | var | structural verdict |
|---|---|---|---|---|
| shape_conclusion_c0-2 | 3 | 0.7484 | 0.040113 | MIXED |
| shape_conclusion_c1-0 | 2 | 0.6136 | 0.010737 | MIXED |
| shape_conclusion_c2-0 | 2 | 0.6136 | 0.010737 | MIXED |

## Verdict

**POSITIVE on mechanism / MIXED on empirical utility.**

Mechanism: family discovery correctly extended; new family kind shipped, idempotent, queryable via existing API.

Empirical: **conclusion families do not capture quality dimension on OQ#1**. All three conclusion families have variance > 0.01, indicating divergent cross-precision profiles among members.

## Why conclusion families fail on OQ#1

Looking at `shape_conclusion_c0-2` (3 members predicting R(0,2)):
- `ax_tpl_v3_p0-0_p1-2_c0-2`: 0.5099 (noise — `p0-0` premise)
- `ax_tpl_v3_p0-1_p1-2_c0-2`: 1.0000 (transitivity — pure signal)
- `ax_tpl_v3_p0-1_p2-1_c0-2`: 0.7354 (mid-tier)

Spread 0.49. The same conclusion shape can be reached from a noisy premise (`p0-0`) or a clean premise (`p0-1_p1-2` = transitivity). Conclusion alone does NOT determine quality.

## Empirical lesson

**Premise structure determines axiom quality more than conclusion structure** (at least on OQ#1).

This explains why B.1's premise-shared families found a clean variance-zero noise cluster, but B.3's conclusion-shared families don't. Quality is rooted in what an axiom REQUIRES (premise binding pattern), not what it CLAIMS (conclusion shape).

## Future implications

- Family-level demote (B.2) should only consider premise families with low variance
- Conclusion families remain useful for OTHER analyses (e.g., "which axioms predict the same edge type?") — just not quality
- Future B.X family kinds could combine premise + conclusion structure or use semantic signatures (e.g., "axioms that produce reverse edges")

## What this slice produced

1. Shared-conclusion family kind shipped in `discover_axiom_shape_families`
2. 6 families on OQ#1 (3 premise + 3 conclusion) — vocabulary doubled
3. Empirical finding: premise > conclusion as quality predictor
4. 1 new unit test; 542 lib tests pass
