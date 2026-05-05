# Phase Emergence — Cross-substrate canonical-form comparison

**Status**: ✓ done (2026-05-06); replaces ADR 0073/0074 RSet-collapse verdict
**Log**: [`logs/2026-05-06_phase_emergence_canonical_form_diversity.log`](../../logs/2026-05-06_phase_emergence_canonical_form_diversity.log)
**Example**: [`examples/phase_emergence_canonical_form_diversity.rs`](../../examples/phase_emergence_canonical_form_diversity.rs)
**Predecessor**: [`phase_emergence_kernel_audit.md`](phase_emergence_kernel_audit.md)
**ADR**: [0075 — Emergence kernel audit and runtime integration](../decisions/0075-emergence-kernel-audit-and-runtime-integration.md) (piece 3)

## Goal

The kernel audit confirmed that v2's `autonomous_pass` mints
patterns on every substrate. It also flagged that comparing
**pattern ids** (p_0..p_N) across substrates is meaningless —
those are per-RSet counters that overlap by accident.

This slice runs the real cross-substrate-diversity test: take
each minted pattern's **canonical form** (the `Subgraph::
canonicalize` output, which is structurally invariant across
substrates per ADR 0009/0029) and compare set-membership
across substrates.

## Method

For each canonical substrate, run the standard runtime to its
Phase 0 horizon, then call `autonomous_pass` for sizes 2-5.
For each minted pattern p, retrieve `pattern_structure(p)` —
the registered canonical form, a stable `Vec<(u64, u64)>`.
Hash it to a 12-hex-char display tag (`can_<hex>`) for
table layout.

Two substrates "share" a canonical iff its hash appears in
both. The full canonical form is the actual identity; the tag
is just for display.

## Result

### Per-substrate canonical-form inventory

```
substrate    minted    canonicals
OQ#1              7    can_faefcad..., can_703239b..., can_1dcd38f...,
                       can_cb21943..., can_3d2da53..., can_ed6c192...,
                       can_bf91273...
long5k            7    same 7 as OQ#1 (different instance counts)
narrow_a          3    can_1dcd38f..., can_ed6c192..., can_bf91273...
                       (subset of OQ#1's set)
OQ#2              7    can_faefcad..., can_1dcd38f...,
                       can_61c8938..., can_d22cf47..., can_a24624e...,
                       can_356b874..., can_ff8b08e...
```

### Canonical × substrate matrix (instance counts)

```
canonical              size    OQ#1   long5k   narrow_a   OQ#2
can_1dcd38f6674f1878      3      25       50         25     25     ← universal
can_faefcad1cdc772d7      2      30       30          —      9
can_703239bca97249ae      2      15       15          —      —
can_3d2da53b81a90ad7      4      15       15          —      —
can_cb21943a71a9eb8e      3      10       10          —      —
can_bf91273eb195d4e9      5       5       10          5      —
can_ed6c192b563a80e3      5       5       10          5      —
can_61c89385fc0342a0      3       —        —          —     84     ← OQ#2-only, biggest
can_d22cf47563c30091      5       —        —          —     30     ← OQ#2-only
can_a24624ecca927e67      3       —        —          —     20     ← OQ#2-only
can_356b87478aee25fa      2       —        —          —      1     ← OQ#2-only
can_ff8b08ea746bf094      2       —        —          —      3     ← OQ#2-only
```

12 distinct canonical forms total. 1 universal (every substrate),
6 shared by OQ#1+long5k+narrow_a (some), 5 OQ#2-only.

### Pairwise Jaccard

```
              OQ#1   long5k   narrow_a   OQ#2
OQ#1          1.00     1.00       0.43   0.17
long5k        1.00     1.00       0.43   0.17
narrow_a      0.43     0.43       1.00   0.11
OQ#2          0.17     0.17       0.11   1.00
```

### Diversity verdict

`SUBSTRATE-DISTINCT` — 5 canonical forms appear on exactly one
substrate (all 5 are OQ#2-only). The pattern path produces
structural diversity that the axiom path collapses.

## What this confirms vs revises

### Confirms

- **OQ#1 ≡ long5k at pattern level** (Jaccard 1.00).
  Consistent with Phase 0072-A's cross-precision parity finding
  and Emergence-1's RSet topology collapse for those two
  substrates. The RSet-collapse property generalises from
  axiom path to pattern path *for these substrates specifically*
  — not surprising, since both produce identical 4-theory /
  13-axiom / 6-family RSets.
- **narrow_a is a true subset of OQ#1/long5k** (Jaccard 0.43,
  3 ⊂ 7). Narrow stream produces a smaller pattern inventory
  drawn from the same canonical-form universe. Not a
  structurally distinct substrate.

### Revises

- **OQ#2 is genuinely substrate-distinct**, not just "the
  blind spot." Jaccard 0.17 with OQ#1/long5k means **5 of OQ#2's
  7 minted canonicals are unique to it**. The biggest pattern
  on any substrate (84 instances) is OQ#2-only.
- **The "RSet collapse" diagnosis** from Emergence-1 is
  precisely scoped: collapse holds within the OQ#1-clade
  (OQ#1, long5k, narrow_a), but not across all v2 substrates.
  OQ#2 sits outside the clade and produces its own structural
  vocabulary at the pattern level.
- **The "OQ#2 produces nothing learnable" claim** (Emergence-1
  diversity probe) is wrong. OQ#2 is the **most pattern-rich
  substrate by instance count** (172 total per the kernel
  audit; the 84-instance p_3 alone exceeds any other
  substrate's largest pattern).

## What "OQ#2-only canonicals" mean structurally

Without going into per-canonical visualization (a future
diagnostic), we know structurally:
- OQ#2 substrates have 3 regimes (tournament + lattice + star)
- OQ#1 / long5k / narrow_a all have diamond-poset-flavoured
  regimes
- The 5 OQ#2-only canonicals correspond to subgraph structures
  emerging from tournaments, lattices, stars — structures that
  don't form in the diamond-flavoured substrates
- The 1 universal canonical (`can_1dcd38f...`, size=3) is
  almost certainly a triangle or simple chain shared by all
  4 substrates
- The 1 OQ#2-shared-with-OQ#1/long5k canonical (`can_faefcad...`,
  size=2) likely captures a 2-edge motif present in both
  substrate families (e.g., self-loop + outgoing edge)

A future audit could map each canonical to its concrete subgraph
shape; not in scope for this slice.

## What this means for ADR 0073's "shape library is the hard ceiling" claim

ADR 0073's claim has now been further refined three times:

1. **Original claim**: v2 cannot mint new concepts because the
   shape library (axiom templates) is hard-coded.
2. **Reflection 0001 / heavy reading**: v2 also can't mint
   concepts properly because the existing pseudo-creation paths
   (ADR 0074) don't register participating tokens.
3. **Kernel audit (ADR 0075 piece 1)**: the pattern path
   already mints constitution-compliant concepts on every
   substrate. The "cannot mint new concepts" diagnosis was
   wrong; it should have been "axiom shape grammar is hard-
   coded but pattern minting is unrestricted."
4. **This slice (ADR 0075 piece 3)**: the substrate-diversity
   gap also resolves at the pattern level. v2 *does* produce
   different vocabularies on different substrates, just via
   pattern naming, not via axiom discovery.

The honest current standing: **v2's emergent vocabulary is
substrate-distinct via the pattern path; the axiom-template
grammar remains hard-coded. The hard ceiling is therefore
strictly on axiom-template invention, not on concept emergence
broadly.**

## Implications for next phases

- The kernel audit's recommendation to integrate
  `DiscoverPatterns` into the runtime's high-priority action
  set (ADR 0075 piece 2) becomes more urgent: **right now the
  pattern vocabulary is empirically richest exactly on the
  substrate the runtime currently learns least from**. With
  scheduler integration, OQ#2-class substrates would gain a
  large amount of structural knowledge during normal runtime
  operation.

- ADR 0072's quality / cross-precision / intervention
  framework was designed for axioms and theories. Patterns
  are now first-class concept-creation outputs and are
  visibly more diverse across substrates than axioms. A
  future ADR may extend the quality framework to patterns
  — pattern cross-precision (predict R from imagined
  pattern instances), pattern-aware intervention (prune
  pattern X if it co-occurs with low-quality pattern Y),
  etc.

- The migration from "concept curation" (Phase 0070-0072) to
  "concept emergence" (Phase Emergence-*) was correct in
  direction but may have under-counted how much emergence
  v2 already does. The next phase boundary may be
  "pattern-aware curation" — applying ADR 0072's machinery
  to the pattern path.

## What was not measured

- **Concrete subgraph visualization** for the 12 distinct
  canonicals. Each `can_<hex>` is currently opaque — we know
  size and instance count, but not "this is a triangle" or
  "this is a star". A future helper that renders a canonical
  form back to a visual subgraph would let humans interpret
  individual emergent patterns.

- **Pattern quality on each canonical**. The kernel audit
  + this slice count instances and participants but don't
  validate quality (cross-precision, primary rate). Patterns
  are not currently evaluated like axioms / theories;
  closing that gap is a separate ADR.

- **Stability across runs**. RNG seeds in
  `phase_emergence_canonical_form_diversity` are fixed;
  the canonical hashes should be deterministic. But a stress
  test — vary seed, confirm the canonical-form set is stable
  — would harden the empirical claim. Not done here.

## Files

- Example: `examples/phase_emergence_canonical_form_diversity.rs`
- Log: `logs/2026-05-06_phase_emergence_canonical_form_diversity.log`
- This result: `docs/results/phase_emergence_canonical_form_diversity.md`

No library code changes. Observational only.

## Verdict

**Phase Emergence's pattern path produces real substrate
diversity.** v2's "blind spot" diagnostic was a self-inflicted
artifact of looking only at axioms. Pattern-level concept
emergence has been working all along, and on OQ#2 it produces
the most novel structural vocabulary of any substrate measured.
The next concrete step is integrating this dormant capability
into the runtime's normal operation (ADR 0075 piece 2).
