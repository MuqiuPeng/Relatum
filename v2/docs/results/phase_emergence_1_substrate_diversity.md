# Phase Emergence-1 — Substrate-diversity falsifiability probe

**Status**: ✓ done (2026-05-06); critical methodological finding
**Log**: [`logs/2026-05-06_phase_emergence_1_substrate_diversity.log`](../../logs/2026-05-06_phase_emergence_1_substrate_diversity.log)
**Example**: [`examples/phase_emergence_1_substrate_diversity.rs`](../../examples/phase_emergence_1_substrate_diversity.rs)
**Predecessor**: [`phase_emergence_1_concept_mining.md`](phase_emergence_1_concept_mining.md)
**ADRs touched**: [0074](../decisions/0074-phase-emergence-1-shape-co-occurrence-mining.md)

## Goal

The first Emergence-1 ship validated concept identity portability
on OQ#1 + long5k. But Phase 0072-A had already shown those
substrates converge to isomorphic RSets — so identity transfer
between them was structurally "free." This slice runs the same
mining loop on substrates expected to produce **structurally
distinct** RSets (narrow_a, OQ#2) to test whether concept
identity holds when the underlying RSet differs.

Predictions ranked from strongest to weakest:
- **Strong universality**: same concept id on all 4 substrates
- **OQ#1-clade only**: portable across OQ#1 + long5k, different
  ids on narrow_a / OQ#2
- **Substrate-specific**: each substrate produces its own ids
- **Null**: narrow_a / OQ#2 produce no concepts at all

## Result

### Outcome matrix

```
 substrate      ticks   ths   axs   fam  prop   reg
 OQ#1            1000     4    13     6     1     1
 long5k          1500     4    13     6     1     1
 narrow_a         500     4    13     6     1     1
 OQ#2 @ 1500     1500     2     2     0     0     0
 OQ#2 @ 3000     3000     2     2     0     0     0
 OQ#2 @ 4500     4500     2     2     0     0     0
```

### Concept id × substrate

```
concept id                  OQ#1   long5k  narrow_a  OQ#2
concept_4c2d2fde3b2d8360    ✓      ✓       ✓         —
```

3 of 6 substrates registered concepts; all 3 minted the **same
concept id** with the **same constituents**
(`shape_conclusion_c0-2`, `shape_premise_p0-1_p1-2`) and the
**same cross-precision** (1.0000).

OQ#2 produced only 2 theories with 2 axioms (`ax_antisymmetry`,
`ax_totality` — predicate axioms, not templates) and 0 L2
families. The propose pipeline requires `≥ 2` shape families;
OQ#2's stream doesn't fire any template axioms whose canonical
premises co-occur enough to mint families.

### Verdict's apparent reading

The probe printed `STRONG UNIVERSALITY — every minted concept id
is universal across substrates that produce concepts.` But this
verdict, read at face value, is **misleading without further
analysis**.

## What this finding actually reveals

Look closely at the matrix: **OQ#1 / long5k / narrow_a all
converge to identical Phase 0 state** — 4 theories, 13 axioms,
6 L2 families. This includes `narrow_a`, which is a deliberately
narrow stream containing only diamond posets across 5 phases
(500 events total). Despite being 1/10th the data of OQ#1, it
matures into the **same RSet topology** with the **same
discovered axioms** in the **same shape families**.

This is not a property of the concept-mining layer. It is a
property of v2's underlying axiom-discovery + canonicalization
pipeline.

### The real diagnosis

ADR 0073 named the curation/creation boundary as "the system
cannot mint new axiom shapes; the shape library is hard-coded."
This probe shows the consequence is even more total than that
phrasing suggested:

**v2's RSet output topology is determined by the shape library,
not by stream content.**

Any stream rich enough to fire template axioms produces the same
13 axioms grouped into the same 6 families. The stream determines
*which axioms have evidence to fire* (i.e., primary rates), not
*which axioms exist*. The "shape library is the hard ceiling"
claim is reframed: the ceiling collapses the entire output space
to a single RSet topology for any sufficiently rich stream.

OQ#2 is the lone exception, and it fails for a different reason:
its tournament/lattice/star regimes don't generate the subgraph
patterns that cause v2's template-discovery to instantiate any
template axioms. It surfaces only the two predicate axioms
(antisymmetry, totality). Predicate axioms aren't subject to
shape-family discovery (their structure is hard-wired), so 0
families minted.

### What this means for ADR 0074's universality claim

The probe's `STRONG UNIVERSALITY` print is **technically correct
but methodologically empty**. The 3 substrates that mint
concepts mint *the same concept* because they have *the same RSet*.
This proves only that:

1. The concept-mining algorithm is deterministic given an RSet
2. The deterministic id is portable

It does NOT prove the concept is universal across structurally
distinct RSets — because no two of the 3 minting substrates have
structurally distinct RSets. The structural-distinctness claim
that motivated this slice is empirically vacuous on the current
substrate suite.

### Re-categorizing the four predictions

The probe's outcome maps to **a hidden 5th prediction** that
wasn't in the list:

> **v2-vocabulary-bound universality**: the concept id is
> universal across every substrate that exercises v2's full
> axiom-shape library; substrates that don't (OQ#2) don't mint
> concepts at all. Universality is real but its scope is
> v2-internal vocabulary, not the underlying R-only ontology.

This is the actual evidential standing of ADR 0074's mining
layer post-probe.

## What this changes for E1 / E2 / E3

This probe does not invalidate Emergence-1's mechanism. The
propose / validate / register loop works correctly: it deterministic-
ally surfaces the unique signal-class shape-shape pattern from
each available RSet. What the probe falsifies is the secondary
claim that **registered concepts represent transfers between
genuinely different RSets**.

For the next steps:

### Implications for E1 follow-ups

- **AxiomComplete intervention** (Emergence-1.5) is unaffected:
  it gates on local theory-vs-concept structure, not on cross-
  substrate transfer. Still worth shipping.
- **Re-validation cadence** is unaffected.
- The case for **richer substrates** (or relaxed
  `min_theories`) is now first-priority for the mining layer's
  empirical credibility. Without an RSet that produces a
  *different* set of shape families from OQ#1's, concept-mining
  can only register one concept ever.

### Implications for E2 (object lifting)

E2 promotes structural patterns in R to candidate object tokens.
This probe suggests E2 will encounter the same RSet-topology
collapse: any stream rich enough to exercise v2's axioms will
produce the same equivalence classes / hub structures. E2's
substrate diversity test must therefore include an *intentionally
narrow* substrate (like narrow_a) that fires fewer axioms,
producing a smaller RSet.

### Implications for E3 (intrinsic drive)

E3's "what is currently unexplained" signal depends on having an
R subset that no current axiom predicts. On a stream where v2's
13 axioms cover everything (OQ#1 / long5k / narrow_a all
converge here), E3's signal will be zero or near-zero. The
diversity probe therefore predicts E3's value is most visible on
**substrates that *don't* fire v2's full axiom library** — i.e.
substrates v2 currently fails to learn anything from, like
**OQ#2**. E3 is most useful precisely where the existing system
is empty.

This is a useful inversion. OQ#2's "failure" (only 2 axioms, no
families, no concepts) is exactly where E3's drive should
activate. Phase Emergence-2 (E3) would prove its value on OQ#2,
where Phase 0070-0072 produced nothing learnable.

### Implications for breaking the RSet-topology collapse

The deeper agenda implied by this probe: v2 needs a way to
**vary its own RSet output topology** in response to stream
content, beyond just "which axioms fire." Candidate mechanisms:

1. **Substrate-conditional shape canonicalization**: allow
   premise-key buckets to depend on stream-derived structural
   priors, not just template syntax. Hand-coded canonicalization
   is too coarse.
2. **Axiom-shape mining itself** (the original ADR 0073 E1 step
   that ADR 0074 weakened to "shape *family* mining"): if the
   system could mint a new `AxiomTemplate` from data — true
   axiom invention — different streams would produce different
   axiom sets and therefore different RSets. This is the strong
   form of E1 that ADR 0074 deferred.
3. **Object lifting (E2)** providing a way to reshape the
   identifier space: lifted equivalence classes change which
   axioms apply where, perturbing the RSet.

All three live downstream of E1. None ships in this slice.

## What this slice produced

1. **Empirical verification that ADR 0074's propose-validate-
   register loop works deterministically across all substrates
   that exercise v2's axiom library** (3 of 4 tested substrates).
2. **A new methodological discovery**: the substrates considered
   "structurally distinct" (OQ#1 vs long5k vs narrow_a) are in
   fact RSet-isomorphic at Phase 0 maturity. Genuine substrate
   diversity, in v2's current state, requires either making a
   substrate v2 can't currently learn from (OQ#2) or adding
   mechanisms that vary RSet topology with stream content.
3. **A reframing of ADR 0073/0074's "universality" claim**:
   universality holds within v2's hard-coded vocabulary; the
   universality of the mined patterns is therefore inherited
   from the universality of the vocabulary, not independently
   established by mining.
4. **A new strategic case for E3 (intrinsic drive)**: E3's value
   is highest precisely on substrates where Phase 0070-0072
   produced nothing — OQ#2 is the natural empirical target.

## Verdict

**Substrate-diversity probe DOES NOT yet falsify ADR 0074, but
the apparent confirmation is methodologically empty.** The probe
revealed a deeper structural fact about v2: any sufficiently
rich stream produces the same RSet, so "cross-substrate
universality" of concept ids is currently a corollary of v2's
fixed axiom vocabulary, not an independent property of the
mining layer.

The honest standing of ADR 0074 post-probe: the mining layer is
correctly implemented and produces deterministic, useful
concepts; **but the pre-probe universality claim should be read
as v2-vocabulary-bound, not ontology-level, until further
substrates with structurally distinct RSets exist.**

Open follow-up: build or find a substrate that produces a
genuinely different RSet (different theory count, different
shape families, different axiom set) to test whether concepts
re-mint with new constituents under different structural inputs.
This is the prerequisite for any meaningful "concept transfer"
claim.
