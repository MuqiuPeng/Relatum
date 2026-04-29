# ADR 0069 — Identifier minting (generative axioms): contract for growing v2's identifier space (2026-04-29)

## Status

Accepted as design contract; lib API surface deferred to G.4+.
G.1 + G.2 examples demonstrate the mechanism is constitutionally
clean; this ADR documents the rules for any future runtime mechanism
that mints new identifiers.

## Context

v2's structural extensions (Beta-1 through B.7) all grew the
**structure space** over a fixed identifier pool: axioms, theories,
families, nested families, super-meta-families. None grew the
**identifier space** itself.

The user's "v2 是要自动拓展的" critique applies at two layers:
- **Structure layer** — addressed (partially) by Beta-1's auto-extension
- **Identifier layer** — *not yet addressed*

A system that can only describe identifiers it already knows about
cannot construct integers, names, or any genuinely new concepts.
G.1 and G.2 (Phase Alpha+, 2026-04-29) probed identifier-layer
extension via a successor recipe `0 → succ(0) → succ(succ(0)) → …`;
both passed. ADR 0069 codifies what made that work and what any
future generative axiom must satisfy.

## Decision

**A generative axiom is a deterministic recipe that derives new
identifiers from existing ones, materialized as ordinary R(x, y)
edges.**

The recipe is a function `mint :: token → token` (or more generally,
`tokens → token`) satisfying four properties.

### The four contract properties

#### 1. Determinism

`mint(t)` returns the same string for the same input across processes,
runs, and machines. Implementation must use only the input string —
no clocks, RNGs, hash-state, allocator addresses, or other
non-deterministic data.

**Why**: Commitment 4 says identity is token-based and there is no
implicit dedup. If two processes minted different tokens for the
same conceptual recipe step, an externally-supplied identifier
matching one would not match the other — silent breakage.

#### 2. Anti-collision (with input space)

`mint(t)` produces a token that does not collide with the input
token: `mint(t) ≠ t` and ideally `mint(t) ∉ {any prior chain
member}`. Practical recipes use a structural prefix or wrapper
(e.g., `format!("succ({})", t)`) which guarantees the output
contains a syntactic feature absent from the seed.

**Why**: A self-collision (`mint(t) = t`) breaks chain freshness.
A chain-member collision creates a cycle in the materialized
edges, which may or may not be desired but is not the default
contract.

#### 3. Materializability

The recipe's output is expressible as one or more R edges using
only the input identifier(s) and the freshly minted identifier(s).
No auxiliary data structures, side channels, or compile-time
metadata required.

**Why**: Commitment 1 (R singular). If the recipe needs anything
beyond R edges to be useful, it has escaped the primitive.

For successor: one edge `R(mint(t), t)` suffices to encode the
"successor of" relationship.

#### 4. Persistence safety

Minted identifiers serialize cleanly through the canonical text
form (`to_text` / `from_text`). They contain no tab, no newline.
After round-trip, the byte-sequence of every minted id is identical.

**Why**: Without this, minted ids couldn't survive runtime restart
without changing identity — silently violating commitment 4 across
process boundaries.

### Lifecycle and tagging

Generative axioms differ from existing axiom kinds:

| kind | input matched on | output |
|---|---|---|
| **template axiom** (`ax_tpl_*`) | existing edges | new edges over **existing** ids |
| **predicate axiom** (`ax_reflexivity`, `ax_antisymmetry`, …) | existing edges | constraint check (no new edges) |
| **generative axiom** (G.2+) | existing ids | new edges introducing **new** ids |

Generative axioms should register under their own marker:

```
R(GENERATIVE_AXIOM_MARKER, gen_<id>)
```

Membership of a token in the generative-derived set:

```
R(<recipe_marker>, <derived_token>)
```

(e.g., `R("__successor__", "succ(0)")` from G.1)

This keeps generative output distinguishable from template-discovered
edges while still living entirely within the R primitive.

### Cross-precision applicability

DreamCoder-style cross-precision (Phase Alpha-7+) validates
*predictions of edges* on imagined substrates. Generative axioms
do not predict edges over a fixed substrate — they introduce
identifiers that didn't exist on the substrate.

Therefore: **cross-precision as currently defined does NOT apply to
generative axioms.** Until G.4 specifies an alternative metric,
generative axioms should be excluded from cross-precision-driven
demote and from `discover_axiom_shape_families` quality summaries.

This is documented as an asymmetry; the alternative (forcing
cross-precision to apply to generative output) would corrupt
the existing predicate-axiom validation pipeline.

## Constitution check

- **C1 (R singular)**: minted edges are plain R(x,y). No new primitive
  needed. ✓
- **C2 (R binary)**: 2-arity preserved. ✓
- **C3 (types as meta-R)**: generative-axiom registration uses a
  marker exactly as Beta-1's shape families do. ✓
- **C4 (token identity)**: the determinism property is a direct
  encoding of commitment 4. External code calling the same recipe
  produces token-identical output by construction. ✓
- **C5 (similarity is structural)**: minted-id graph membership is
  queryable via `right_of` / `left_of` like any data id; no
  special-cased traversal. ✓ (Verified in G.1: backwards walk works.)

## Why an ADR (and not just an example)

G.1 + G.2 prove the mechanism works in two specific cases. ADR 0069
forward-binds future code: *any* future generative recipe must
satisfy the four contract properties or it is not a valid v2
mechanism.

Without this contract, a future direction could naively add a
"random-id minter" or "timestamp-tagged id" mechanism, both of which
would break commitment 4. Codifying the contract now prevents that
class of regression.

## Empirical evidence (G.1 + G.2)

[G.1](../results/G.1_identifier_mint.md) verifies determinism,
freshness, anti-collision, materializability across a 5-step
successor chain.

[G.2](../results/G.2_generative_with_axiom.md) verifies that the
existing `forward_apply_axiom` machinery accepts minted edges as
ordinary data — no special handling, no new code path. Round-trip
serialization preserves all 5 minted ids byte-identically.

The 6-id minted chain has 15 directed (i > j) pairs. Transitivity
applied to fixpoint produces all 15. The "succ(succ(succ(succ(succ(0))))) →
0" closure edge is the witness that generative output composes
cleanly with declarative reasoning.

## Future deferred slices

- **G.4**: a validation metric for generative axioms (cross-precision
  doesn't apply; alternatives include "does the chain extend
  coherently across substrates?", "do composed predicate-axioms
  apply meaningfully to the minted ids?")
- **G.5**: a runtime drive that *demands* new identifier creation.
  Currently no Drive calls for minting. Without one, generative
  axioms are inert.
- **G.6**: multi-arity recipes (e.g., `mint(a, b) → "pair(a, b)"`
  for Cartesian product construction)
- **G.7**: integer arithmetic embedding — combining the successor
  chain with addition recipe `add(x, y) := apply succ y times to x`.
  The first viable target for "v2 constructs an integer concept".

## What this is NOT

- Not a new axiom-storage format. Generative axioms reuse the existing
  template-axiom intension when their output is a known shape; the
  novelty is that the *bound variables* in the conclusion can come
  from minting rather than from the existing identifier set.
- Not a license to mint freely. The contract is restrictive: each
  generative axiom must be a documented recipe with explicit
  determinism. There is no "generic mint" primitive.
- Not yet integrated with the scheduler. Generative axioms will need
  their own `ActionKind` and frontier item — analog to Beta-1's
  `DiscoverAxiomShapeFamilies` integration in B.5.1.

## Status

Accepted as contract. G.1 and G.2 demonstrate one valid recipe
(successor) end-to-end. ADR 0069 binds future generative work to
this four-property contract.
