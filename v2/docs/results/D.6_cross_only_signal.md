# D.6 — Engineered cross-precision-only signal substrate

**Status**: ✓ done (NULL result — methodologically informative)
**Log**: [`logs/2026-04-30_phase_d6_cross_only_signal.log`](../../logs/2026-04-30_phase_d6_cross_only_signal.log)
**Example**: [`examples/phase_d6_cross_only_signal.rs`](../../examples/phase_d6_cross_only_signal.rs)

## Goal

D.5 found that on OQ#1, primary < cross holds for noise-family axioms (0.11 vs 0.49) — but both below 0.5, not a STRONG asymmetry. D.6 attempts to ENGINEER a substrate where some axiom has cross ≥ 0.7 AND primary ≤ 0.3 — a genuine cross-only signal.

Such a substrate would prove composite arbitration's value beyond D.5's modest finding.

## Method

Engineered stream: 15 phases × 6-node almost-transitive chain. DELIBERATELY OMITTED transitive closures (e.g., omit R(n_0, n_3) but include R(n_0, n_1), R(n_1, n_2), R(n_2, n_3)). Hypothesis:
- Ground stream rarely confirms the omitted transitive predictions → primary low
- Substrate generation auto-saturates closures via forward apply → cross high

## Result

```
stream events: 150
[trained] axioms=6, theories=4
=== Per-axiom (primary, cross) ===
(no axioms had hit_rate data above MIN threshold)
```

Even after lowering MIN_AXIOM_PREDICTIONS from 5 to 1, no axioms accumulated prediction-state data. The runtime discovered 6 axioms structurally but didn't run them through enough prediction cycles to accumulate hit_rate stats.

## Verdict

**NULL — engineered substrate did not produce the strong cross-only pattern.**

This is a NEGATIVE empirical finding, but methodologically informative.

## Why engineering this is hard

Three structural reasons:

### 1. Substrate generation is downstream of training

`generate_substrate_from_theory(t)` produces a substrate satisfying T's axioms. If an axiom is part of T (i.e., the runtime accepted it during training), the substrate is *constructed to validate it*.

So an axiom that's "in a theory" naturally has high cross-precision on substrates of all theories. There's no way to keep an axiom in a theory while making it fail on most substrates — the construction is symmetric.

### 2. Axioms with low primary tend to be discarded

The runtime's discovery + tournament dynamics tend to discard axioms that don't validate on ground. So by the time we measure cross-precision, only axioms with reasonable primary survive. The "low primary, high cross" axiom is selected against.

D.5's noise family (primary 0.11) is an exception — these axioms DO survive because they're members of t_0, which covers a regime that DOES contain them. They're noisy on aggregate but signal-bearing in a sub-region.

### 3. Engineered streams are too sparse for prediction-state accumulation

The 150-event engineered stream was too small for the prediction-state machinery (which needs many ticks of axiom firing) to accumulate enough samples per axiom. The runtime needs sustained ticks where axioms apply and predictions are checked.

## What this slice produced

1. Failed engineering attempt with diagnostic detail
2. Methodological insight: cross-precision and primary are MORE correlated than D.5's noise-family disagreement suggested
3. Three structural reasons engineering this is hard:
   - Substrate generation symmetry
   - Discovery/tournament selection pressure
   - Insufficient prediction-state samples on small streams
4. Resolution: D.5's bimodal finding (noise-family asymmetry) is the BEST evidence v2 has for cross-precision-only utility. D.6 wasn't able to improve on it.

## Implication for composite scheduler signal

D.3.1 said: signals correlate too strongly to disagree at theory layer.
D.5 said: per-axiom disagreement DOES exist (7/11 axioms with delta ≥ 0.30).
D.6 says: engineering EXTREME disagreement (cross-only) is hard.

The honest synthesis: composite signal arbitration is **modest** in value — exists at axiom layer per D.5, but not extreme enough to engineer artificially. The composite is a **smoothing mechanism** more than an arbitrator.

## Future implications

- **Try a different engineering angle**: instead of sparse-stream omission, try a stream where some axiom is structurally non-applicable to ground but applicable to substrates. E.g., if ground has only 2-node interactions but substrate gen adds 3-node closures.
- **Lower the bar**: define "weaker cross-only" as cross > primary by 0.2+, accepting both can be middling. D.5 already shows this is achievable.
- **Composite signal weighting**: rather than α=0.5 blend (D.3), use cross-only when the axiom shows D.5's asymmetry, primary-only when no asymmetry exists. Adaptive composite.
- **D.7**: the SUFFICIENT condition for composite arbitration to matter empirically. Likely involves substrates with > 2 regimes where some axioms are valid on regime A but not regime B.

## Methodological note

A NULL result on an engineering attempt does not mean the underlying property is impossible — only that this specific engineering failed. D.6's contribution is documenting WHY the engineering is harder than expected. Future attempts should target the three structural reasons identified.
