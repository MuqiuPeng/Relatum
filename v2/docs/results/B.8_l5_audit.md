# B.8 — Layer-5 super-super-meta audit

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_b8_l5_audit.log`](../../logs/2026-04-29_phase_b8_l5_audit.log)
**Example**: [`examples/phase_b8_l5_audit.rs`](../../examples/phase_b8_l5_audit.rs)

## Goal

B.7 minted 1 super-meta family (Layer 4) on OQ#1. To produce Layer 5, we'd need ≥ 2 L4 super-metas sharing an L3 nested family. Audit: what would yield more super-metas, on which substrate? Document the structural ceiling explicitly.

## Result

**Identical layer counts on OQ#1 and long5k:**

| layer | description | count |
|---|---|---|
| L1 | axioms | 13 |
| L2 | shape families | 6 (3 premise + 3 conclusion) |
| L3 | nested shape families | 2 (`meta_premise_p0-1`, `meta_premise_p1-2`) |
| L4 | super-meta-families | 1 (`super_shape_premise_p0-1_p1-2`) |
| L5 | super-super-meta-families | **0** |

The single L4 super-meta on each substrate contains both L3 nested families (they share `shape_premise_p0-1_p1-2`). For L5 to mint, we'd need ≥ 2 L4 super-metas sharing an L3 member — not present.

L5 prerequisite scan:
- `meta_premise_p0-1` appears in 1 L4 super-meta(s) — needs ≥ 2
- `meta_premise_p1-2` appears in 1 L4 super-meta(s) — needs ≥ 2

→ L5 would mint 0 on both substrates.

## Verdict

**STRUCTURAL-LIMIT — the current discovery kinds saturate at L4 for the OQ#1/long5k family of substrates.**

L4 is the deepest layer the current vocabulary supports. Further abstraction requires new discovery kinds, not bigger substrates.

## What would lift the ceiling

Three independent paths could yield ≥ 2 L4 super-metas:

### (a) More diverse premise / conclusion shapes

OQ#1's axiom enumeration produces premises from {p0-0, p0-1, p1-2}. That gives 6 L2 families and 2 L3 nested families. A substrate that produced premises like {p0-0, p0-1, p0-2, p1-2, p2-3} might yield 3-4 L3 nested families and multiple L4 super-metas.

**Limitation**: requires substrate engineering or more permissive axiom enumeration. Not easily inducible from data.

### (b) Additional L2 / L3 discovery kinds

Currently L2 = "shared canonicalized premise" or "shared conclusion edge". Adding kinds like:
- "shared variable arity" (3-var vs 4-var axioms)
- "shared structural symmetry" (R(x,y) ↔ R(y,x) flips)
- "shared conclusion orientation" (R(x,y) vs R(y,x))

would produce additional L2 families, hence more L3 candidates and likely more L4 super-metas. **Recommended path** — adds discovery axes, doesn't require new substrates.

### (c) Multi-substrate aggregation

Discover families across the union of multiple substrate runs. Different substrates produce different axioms; union gives more shapes. Not yet implemented. Risk: bookkeeping overhead.

## Why this parallels C.2.1's verdict

C.2.1 found OQ#2 (tournament + lattice + star) produced 0 template axioms because the substrate violated transitivity. That was a *substrate* structural bound — the discovery mechanism worked but had nothing to consume.

B.8 is the dual: the *abstraction-layer* mechanism is bottoming out. The substrate has rich enough axioms (13 on each), L2-L4 all populate, but the layer-recursion doesn't have enough material at L4 to produce L5.

Both findings reveal that v2's mechanisms are bounded by the structural diversity of their inputs. Beta-1's auto-extension is real but not unlimited.

## What this slice produced

1. Layer-by-layer count audit for OQ#1 and long5k
2. Diagnosis of the L5 prerequisite (≥ 2 L4 super-metas sharing an L3)
3. Specification of three paths to lift the ceiling (substrate diversity / new discovery kinds / cross-substrate aggregation)
4. Methodological alignment with C.2.1's structural-bound finding

## Future implications

- **B.8.1**: implement option (b) — add a "shared conclusion orientation" L2 discovery kind, re-audit to see if L5 mints
- **B.8.2**: implement option (c) — multi-substrate union family discovery
- **Stop saying "deeper recursion is always better"**: deeper layers add nothing if the input doesn't support them. L4 is the right ceiling for OQ#1.
- **Cross-substrate signal aggregation**: L4 saturation suggests the productive direction is *width* (more discovery kinds) not *depth* (more recursive layers)

## Verdict-of-method

When proposing recursive abstraction layers, audit the prerequisite at the next layer. If the prerequisite isn't met by the existing data flow, the layer won't populate — adding it is dead code. B.7 was the right place to stop on OQ#1; B.8 confirms that.
