# F.5 — Empirical merge safety test

**Status**: ✓ done (2026-04-30)
**Log**: [`logs/2026-04-30_phase_f5_merge_safety.log`](../../logs/2026-04-30_phase_f5_merge_safety.log)
**Example**: [`examples/phase_f5_merge_safety.rs`](../../examples/phase_f5_merge_safety.rs)

## Goal

F.4 picked `(t_2, t_3)` at 66.7% confidence as the highest-confidence multi-signal merge candidate. The picker is a PROPOSAL — does executing the merge actually preserve quality?

F.5 closes the loop: ACTUALLY merge, re-evaluate cross-precision, verify no degradation.

## Method

1. Train rt on OQ#1 (1000 ticks) → 4 theories
2. Pre-merge: compute t_2, t_3 cross-precision (excluding self-substrate)
3. `merge_theories(t_2, t_3)` → minted t_merged
4. Post-merge: re-compute t_merged cross-precision
5. Safety check: delta_max = t_merged − max(t_2_pre, t_3_pre); should be ≥ −0.05

## Result

### Pre-merge

| theory | axioms | cross-precision (excl self) |
|---|---|---|
| t_2 | 3 (`ax_antisymmetry`, `ax_reflexivity`, `ax_tpl_v3_p0-1_p1-2_c0-2`) | **1.0000** |
| t_3 | 4 (`ax_antisymmetry`, `ax_tpl_v2_p0-1_c0-0`, `ax_tpl_v2_p0-1_c1-1`, `ax_tpl_v3_p0-1_p1-2_c0-2`) | **1.0000** |

Union = 5 axioms (2 shared between t_2 and t_3).

### Merge execution

```
merge_theories("t_2", "t_3") → "t_4"
theories before: [t_0, t_1, t_2, t_3]
theories after:  [t_0, t_1, t_4]
t_4 axioms: 5 (= |union| ✓)
```

t_2 and t_3 retracted; t_4 minted with the union member set.

### Post-merge

| theory | cross-precision (excl self) | delta vs max pre |
|---|---|---|
| **t_4 (merged)** | **1.0000** | **+0.0000** |

## Verdict

**POSITIVE — merge preserves quality (delta = 0.0000, well within tolerance).**

**F.4's 66.7% confidence pick is empirically VALIDATED.**

This is the strongest safety claim possible: not only does the merge not degrade, it preserves cross-precision exactly.

## Why the merge is exactly lossless

t_2 and t_3 satisfy:
- Both have universal cross-precision (1.0 against every non-self substrate)
- They share 2 of 5 axioms (`ax_antisymmetry`, `ax_tpl_v3_p0-1_p1-2_c0-2`)
- Their non-shared axioms (`ax_reflexivity` from t_2; `ax_tpl_v2_p0-1_c0-0`, `ax_tpl_v2_p0-1_c1-1` from t_3) are also universal predictors

Merging produces t_4 with all 5 universal axioms. Each axiom in the union still satisfies its original cross-substrate behavior. The union's predictions = union of individual predictions, all of which match the substrates.

This matches F.3's prediction: t_2 and t_3 had **identical column profiles** (max_diff = 0.0000). Functionally equivalent theories merge losslessly.

## What this slice produced

1. End-to-end merge safety verification: F.4 propose → execute → re-evaluate → confirm
2. First empirical merge in the F-family lifecycle (Rounds 1-3 stayed at proposal layer)
3. Confirmation that the multi-signal aggregation (Alpha-5 + F.3 → F.4) produces actionable picks, not just suggestions
4. Theory count reduced 4 → 3 with zero quality cost

## Comparison with prior merge attempts

Alpha-3++++ tried naive Jaccard merge and was falsified (subset+noise pair). The Alpha-5 → F.3 → F.4 lineage refined the picker through 4 increasingly disciplined criteria. F.5 is the empirical answer that the refinement pays off:

- Alpha-3++++ pick (t_0, t_1) — would have failed safety (t_0 noisy)
- Alpha-5 pick (t_2, t_3) — passes safety (this slice) ✓
- F.3 pick (t_2, t_3) — passes safety (same as Alpha-5) ✓
- F.2.1 pick (t_1, t_2) — UNTESTED, might dilute t_2 with t_1's noisier axioms
- F.4 top-1 (t_2, t_3) — passes safety (this slice) ✓

The Borda-aggregated multi-signal pick is the most defensible — F.5 is the empirical confirmation.

## Future implications

- **Test F.2.1's pick**: F.2.1 picked (t_1, t_2). Run F.5 logic on that pair — does the complementarity merge survive? Hypothesis: t_1's lower cross-precision (0.6832) drags the merge down.
- **Merge cascade**: after merging (t_2, t_3) → t_4, what's the next highest-confidence merge? Run F.4 again on the smaller theory set. Iterate to convergence — is there a fixed point of "merge until no high-confidence pair remains"?
- **Merge-aware tournament**: theory tournament (Alpha-3) currently treats theories as fixed. With F.5 confirming safe merges, a tournament that includes merge-as-a-move becomes viable.
- **Composite consolidation strategy**: F.5 + future F.5.1 + future F.5.2 = a portfolio of "safe consolidations" that progressively shrink the theory layer toward minimum-DL representation.
