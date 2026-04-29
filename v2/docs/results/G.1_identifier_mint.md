# G.1 — Identifier minting proof-of-concept

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_g1_identifier_mint.log`](../../logs/2026-04-29_phase_g1_identifier_mint.log)
**Example**: [`examples/phase_g1_identifier_mint.rs`](../../examples/phase_g1_identifier_mint.rs)

## Goal

Smallest mechanism that derives **NEW identifiers** from existing ones via deterministic application of a recipe. First runtime step that grows v2's identifier *space* — Beta-1 grew the structure space over a fixed identifier pool; G.1 starts producing identifiers from identifiers.

Recipe: `mint_successor(token) := format!("succ({})", token)` — pure, deterministic, externally reproducible.

## Method

1. Seed RSet with a single self-loop `R("0", "0")`.
2. Apply `mint_successor` 5 times starting from `"0"`.
3. After each mint, write two edges:
   - `R(next, current)` — "next is the successor of current"
   - `R("__successor__", next)` — meta-R: "next was minted via successor"
4. Verify 4 properties.

## Result

```
seed RSet: 1 edges, 1 identifiers

step 1: 0 -> succ(0)
step 2: succ(0) -> succ(succ(0))
step 3: succ(succ(0)) -> succ(succ(succ(0)))
step 4: succ(succ(succ(0))) -> succ(succ(succ(succ(0))))
step 5: succ(succ(succ(succ(0)))) -> succ(succ(succ(succ(succ(0)))))

[1] determinism: mint_successor("0") == "succ(0)" (3/3 calls match)
[2] freshness: 5/5 minted ids absent from seed RSet
[3] anti-collision: 6 unique ids in chain (length 6)
[4] materializability: 5/5 chain edges present in RSet
```

Backwards walk via `R(?, current)` succeeds — full chain traversable in both directions through the existing R primitive (no new query API needed).

Meta-R query `R("__successor__", ?)` returns 5 ids — successor-derived identifiers are introspectable structurally.

## Verdict

**POSITIVE — all 4 properties verified**.

- **Determinism**: same input → same output, three independent calls return byte-identical strings. Externally-reproducible (commitment 4).
- **Freshness**: 5/5 minted ids absent from seed RSet — the chain genuinely grows the identifier space.
- **Anti-collision**: 6 unique strings in chain of length 6 — no recipe loops.
- **Materializability**: 5/5 chain edges present in `R(x, y)` form — generative output expresses cleanly in the existing primitive.

## What this slice produced

1. Working proof-of-concept that v2's R primitive supports identifier minting without any constitutional change. Adding a deterministic minting recipe + materializing results as R edges is sufficient.
2. Demonstration that minted identifiers are introspectable through normal meta-R queries (`left_of(SUCC_MARKER)`).
3. Bidirectional traversal of the minted chain works through `left_of` / `right_of` — no new traversal primitive needed.
4. **First concrete step toward integer construction**: Peano-style successor chain materialized in RSet form. `0`, `succ(0)`, `succ(succ(0))`, ... is the literal Peano construction; v2 now has the mechanism to produce it.

## Why this matters (vs Beta-1)

Beta-1 added Layer 2/3/4 abstractions over a fixed identifier pool (axioms, families). The system's *identifier space* didn't grow. G.1 establishes that identifier growth is achievable inside the constitution — the minting function is just a deterministic string transformer, materialization uses ordinary `RSet::add`.

This addresses the user's "v2 是要自动拓展的" critique at the identifier-layer: previously v2 could only *describe* incoming identifiers; now (in principle) it can *produce* them.

## Future implications

- **G.2**: express the recipe itself as a forward-applicable axiom in RSet — closing the loop so the system can autonomously apply it under scheduler control (rather than via hand-written example code).
- **G.3**: ADR formalizing the minting contract (determinism, anti-collision, lifecycle) for any future generative axiom kind.
- **G.4**: cross-precision adaptation — predicate axioms validate against substrates; generative axioms PRODUCE substrates. Need a different validation path.
- **Integer construction**: G.1's chain is structurally equivalent to ⟨0, S(0), SS(0), …⟩. Adding a length predicate or counting rule on top would yield natural-number arithmetic.
- **Caveat**: G.1 doesn't address *why* the runtime would mint — the demand-driven trigger (a drive that calls for new identifiers) is open. Beta side currently has no built-in pressure toward generative work; that's a separate design surface.

## Constitutional check

- **C1 (R singular)**: minted edges are plain R(x,y). ✓
- **C2 (R binary)**: edges are 2-arity. ✓
- **C3 (types as meta-R)**: `R(SUCC_MARKER, x)` declares `x` as type `successor-derived`. ✓
- **C4 (token identity)**: `mint_successor` is pure; same input → same string output. External code calling the same recipe produces token-equal ids — no implicit dedup needed because there's no ambiguity. ✓
- **C5 (similarity is structural)**: backwards-walk via `right_of` works on minted ids exactly as on any ids; no special-casing. ✓

All five commitments preserved.
