# F.2 — Family-aware merge candidate signal

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_f2_family_aware.log`](../../logs/2026-04-29_phase_f2_family_aware.log)
**Example**: [`examples/phase_f2_family_aware_merge.rs`](../../examples/phase_f2_family_aware_merge.rs)

## Goal

Alpha-5 picked merge candidates by member-set Jaccard, excluding subset pairs. F.2 adds **family-signature complementarity** as a parallel selection signal: each theory's signature = set of shape families containing any of its members. Pairs with disjoint signatures cover different structural niches.

## Result on OQ#1 @ 1000 ticks

Theory family signatures:

| theory | signature size | families covered |
|---|---|---|
| t_0 | 6 | all 6 (broad noisy + signal) |
| t_1 | 5 | 5 (no shape_premise_p0-0_p1-2 noise) |
| t_2 | 2 | shape_conclusion_c0-2, shape_premise_p0-1_p1-2 |
| t_3 | 3 | conclusion_c0-2, premise_p0-1, premise_p0-1_p1-2 |

Pairwise complementarity (1 − signature Jaccard):

| pair | sig_jaccard | complementarity |
|---|---|---|
| t_0, t_1 | 0.8333 | 0.1667 |
| **t_0, t_2** | 0.3333 | **0.6667** |
| t_0, t_3 | 0.5000 | 0.5000 |
| t_1, t_2 | 0.4000 | 0.6000 |
| t_1, t_3 | 0.6000 | 0.4000 |
| t_2, t_3 | 0.6667 | 0.3333 |

Best by signature complementarity: **(t_0, t_2)** at 0.6667.

## Verdict

**POSITIVE on signal**. F.2 produces a distinct selection signal from Alpha-5's membership-Jaccard. The two heuristics surface different "best" pairs:

| picker | best pair | rationale |
|---|---|---|
| Alpha-3++++ Jaccard (highest membership) | (t_0, t_1) | most overlap (subset+noise) |
| Alpha-5 smart (non-subset highest Jaccard) | (t_2, t_3) | non-trivial overlap, no subset |
| **F.2 signature complementarity** | **(t_0, t_2)** | most disjoint family signatures |

## Caveat

(t_0, t_2) being best by signature complementarity doesn't mean it's the **right** merge target. t_2 is high-quality (rate 1.0); merging with noisy t_0 would dilute it. A composite picker would AND complementarity with **both-above-quality-threshold** to avoid this.

The signal works; integration with composite logic deferred to a future slice (F.2.1?).

## What this slice produced

1. `theory_family_signature` helper (per-theory shape-family coverage set)
2. Pairwise complementarity computation
3. Empirical: distinct merge selection signal on OQ#1
4. Methodological: family-aware signal needs combination with quality gate to avoid diluting good theories

## Future implications

- F.2.1: combine complementarity with both-above-threshold for production-quality merge picker
- F.2 + F.3: cross-precision-driven merge could use family signature as a regularizer
- Family signatures could serve as cheap theory-similarity fingerprints in future tournament cycles
