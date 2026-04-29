# C.2.1 — OQ#2 non-overlapping regimes

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_c21_oq2_validation.log`](../../logs/2026-04-29_phase_c21_oq2_validation.log)
**Example**: [`examples/phase_c21_oq2_validation.rs`](../../examples/phase_c21_oq2_validation.rs)

## Goal

C.2 validated cross-precision and shape-family discovery on long5k, but long5k uses scaled-up versions of OQ#1's regimes. C.2.1 designs OQ#2 with **non-overlapping regimes** (tournament with violations, 4-element lattice with self-loops, star network with bidirectional edges) and tests whether the dream-phase signals still produce useful structure.

## OQ#2 design

3 regimes that don't share structure with OQ#1:
- **Regime T (1-1500)**: 5 phases × 6-node tournament with strategic transitivity violations (`(phase%2==0 && i==0 && j==5) skip` and `(phase%3==0 && i==5 && j==0) reverse`)
- **Regime L (1501-3000)**: 5 phases × 4-element diamond lattice with self-loops on all 4 elements
- **Regime S (3001-4500)**: 5 phases × star network (hub ↔ 4 leaves with bidirectional edges)

`src/test_substrates/oq2.rs`. 4500 total tick events across 3 regimes.

## Result on OQ#2 @ 1500 ticks

| metric | OQ#2 | (OQ#1 reference) |
|---|---|---|
| theories | 2 | 4 |
| axioms | 2 | 13 |
| template axioms | **0** | 11 |
| predicate axioms | 2 | 2 |
| episodes | 10 | 110 |
| shape families | **0** | 6 |
| cross-precision values | **none (—)** | 4 |

theories:
- `t_0`: {ax_antisymmetry, ax_totality}
- `t_1`: {ax_antisymmetry}

## Verdict

**PARTIAL/STRUCTURAL-LIMIT FINDING**.

The verdict from the example classifier is "MIXED — only theory discovery works", but the deeper finding is more interesting:

**OQ#2 by design produces only predicate axioms**:
- Regime T's transitivity violations break `transitivity` axiom (rate < 1.0 → not discovered)
- Regime L + Regime S's bidirectional edges break antisymmetry within those regimes; but on aggregate the runtime still picks antisymmetry as a partial theory member
- No regime's structure naturally produces a clean transitivity-shaped axiom

**Consequence**: predicate axioms don't forward-apply (they're constraints, not generators). Therefore:
- No premise-based shape families possible (no premise structure to share)
- Cross-precision returns `None` because `forward_apply_axiom` produces empty predictions

## Methodological lesson

Cross-precision and shape-family signals **structurally require forward-applicable (template) axioms**. They:
- Don't apply to predicate-axiom-only theories
- Don't generalize to substrates that don't expose template-axiom validity

This is not a flaw — it's a clean characterization of the signals' domain.

For substrates like OQ#2, alternative signals would be needed:
- Theory composition (`R(THEORY_MARKER, t)` + member set) is still available
- Predicate axiom counts could rank theories
- Constitutional structure could be checked manually

## Comparison vs C.2 (long5k)

| | OQ#1 | long5k | **OQ#2** |
|---|---|---|---|
| regime overlap with OQ#1 | self | partial (same regime types, scaled) | **none** |
| template axioms found | 11 | 11 | **0** |
| cross-precision applicable? | yes | yes | **no** |
| shape families discovered | 6 | 6 | **0** |

Cross-precision generalizes to substrates that share regime structure with OQ#1, but doesn't apply to fundamentally different substrate types.

## Future implications

- For substrates without template axioms, develop alternative quality signals (predicate axiom validation, constitutional checks, theory composition diversity)
- "Cross-precision" generalization claim should be qualified: it's regime-type-bound, not substrate-bound
- This validates the methodological caveat in C.2's result: cross-precision generalization is conditional on regime-type overlap

## What this slice produced

1. New `oq2` substrate (3 non-overlapping regimes) for future targeted experiments
2. Empirical structural-limit finding: cross-precision requires template axioms
3. Methodological qualifier on C.2's "generalizes" claim
4. Future-direction candidate: predicate-axiom-aware quality signals
