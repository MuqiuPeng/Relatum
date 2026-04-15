# Dual-Signal Analysis: Associativity as the Most Valuable Axiom

## Two Independent Measurements

### Signal 1: Model Space Rarity (Enumeration)

Source: exhaustive enumeration of all 19,683 binary operations on {0,1,2}.

```
Axiom                 Models   % of total   Rarity rank
─────────────────────────────────────────────────────
(none)                18748    95.25%       —
commutativity           729     3.70%       1 (least rare)
identity                243     1.24%       2
associativity           113     0.57%       3 (most rare)
```

Associativity eliminates 99.43% of all operations. It is the single strongest structural constraint on binary operations.

### Signal 2: Closure Space Indispensability (Ablation)

Source: ablation analysis on Z₃'s propositional-logic-style evaluation and group axiom discovery.

From the chain identity analysis:
```
Structure          Chain identities   Associativity present?
───────────────────────────────────────────────────────────
Z₃ (abelian)       11                 yes (+ 10 commutativity-derived)
S₃ (non-abelian)    1                 yes (only associativity)
Z₄× (monoid)       11                 yes (+ commutativity)
```

S₃ has exactly 1 chain identity — pure associativity — with no commutativity padding. Removing it collapses the entire chain inference capability.

From the cross-structure comparison:
- 11 chain identities in Z₃ ∩ 1 in S₃ = only associativity survives
- Commutativity-derived identities are structure-specific (Z₃/V₄ only)
- Associativity is the only universal chain identity

## The Convergence

```
                        Model Space        Closure Space
                        (enumeration)      (ablation/cross-structure)
────────────────────────────────────────────────────────────────
Associativity           most rare (0.57%)  only universal chain identity
                        strongest filter   irreplaceable across all groups

Commutativity           medium (3.70%)     structure-specific (Z₃, V₄ only)
                        weak filter        replaceable (S₃ works without it)

Identity                medium (1.24%)     discoverable from data
                        moderate filter    has verification rules
```

Both signals independently identify associativity as the most structurally significant axiom. This is not a coincidence — it reflects a deep fact about binary operations:

**Associativity is rare because it imposes a global constraint** (every triple must satisfy it), unlike commutativity (pairwise) or identity (single element). The same global nature makes it the most productive inference rule — it connects any three elements through a chain, enabling the richest closure.

## Quantitative Summary

| Metric | Associativity | Commutativity | Identity |
|--------|:---:|:---:|:---:|
| Model rarity (% eliminated) | 99.43% | 96.30% | 98.76% |
| Cross-structure universality | 3/3 groups | 2/3 groups | 3/3 groups |
| Chain identity count (S₃) | 1 | 0 | — |
| Verification rules | — | — | 4 per structure |
| Universal generative rules | — | — | 2 per concept |
| Formally equivalent to | G2 | — | G3 |

## Implication for the Core Conjecture

The research plan's core conjecture:

> Mathematical concept evolution ≈ selection under some pressure function.

The dual-signal result provides concrete evidence:

1. **Selection pressure exists**: associativity is objectively distinguishable from other axioms by both rarity and indispensability
2. **The pressure is computable**: model count + ablation score are both algorithmic, no human judgment needed
3. **The outcome matches history**: associativity was indeed one of the first axioms formalized (Cayley, 1854), predating the systematic study of commutativity or identity

A system equipped with both signals would rank associativity highest among candidate axioms — arriving at the same conclusion mathematicians reached historically, through a purely computational process.
