# G.6 — Multi-arity generative recipe (addition)

**Status**: ✓ done (2026-04-30)
**Log**: [`logs/2026-04-30_phase_g6_addition_recipe.log`](../../logs/2026-04-30_phase_g6_addition_recipe.log)
**Example**: [`examples/phase_g6_addition_recipe.rs`](../../examples/phase_g6_addition_recipe.rs)

## Goal

G.1 demonstrated a unary generative recipe (successor). G.6 extends to a BINARY recipe and verifies ADR 0069's contract still holds for multi-arity. This is the next building block toward integer arithmetic (G.7).

## Recipe

```
mint_add(a, b) := format!("add({}, {})", a, b)
```

Materialization: each call writes TWO R edges (one per operand):

```
R(add(a, b), a)   — add(a,b) connects to a
R(add(a, b), b)   — add(a,b) connects to b
```

## Result

5 mints over a 3-id seed pool:
- `add(seed_0, seed_1)`, `add(seed_0, seed_2)`, `add(seed_1, seed_2)` (distinct operands → 2 edges each)
- `add(seed_0, seed_0)`, `add(seed_1, seed_1)` (same operand → R primitive deduplicates, 1 edge each)

All 5 ADR-0069 properties verified:

| # | property | status |
|---|---|---|
| 1 | determinism (binary) | ✓ — `mint_add("0", "0") == "add(0, 0)"` across 3 calls |
| 2 | freshness | ✓ — 5/5 minted ids absent from seed RSet |
| 3 | anti-collision | ✓ — all 5 distinct AND disjoint from input space |
| 4 | materializability | ✓ — 2 edges per mint, 8 unique edges total (after dedup of `(a,a)` cases) |
| 5 | persistence safety | ✓ — round-trip 398 bytes, byte-identical restore |

Backwards walk via `R(add(a,b), ?)` returns the operand pair (or singleton for `add(x,x)` after RSet dedup).

## Verdict

**POSITIVE — multi-arity generative recipes are constitutionally clean.**

All 5 contract properties hold; no escape from R primitive. The R-primitive's natural deduplication on `R(x, y) = R(x, y)` correctly handles `add(x, x)` cases — same edge written twice = one edge in rset.

## Why orientation is preserved

`mint_add(seed_0, seed_1) = add(seed_0, seed_1)` ≠ `mint_add(seed_1, seed_0) = add(seed_1, seed_0)`.

The recipe is **NOT** commutative at the recipe layer. If we want `add(a, b) = add(b, a)` (commutative semantics), that requires an EQUALITY axiom registered in v2 — orthogonal to the recipe.

This is the right separation: **the recipe specifies a pure deterministic mapping; semantic equivalences are independent meta-R declarations**. G.6 keeps these layers distinct.

## What this slice produced

1. Working multi-arity generative recipe (`mint_add`)
2. Materialization pattern: 2 R edges per mint (one per operand)
3. ADR 0069 contract verified for binary arity
4. R primitive's natural dedup property handles `add(x, x)` correctly without special casing
5. **Building block for G.7**: combine successor (chain) + addition (composition) → Peano arithmetic

## Future implications

- **G.7 (next)**: combine succ + add to express `add(succ(0), succ(0)) ≡ succ(succ(0))`. Requires equality axiom for closure.
- **N-arity recipes**: ADR 0069 covers it in principle — write N edges per mint. Tested at N=1 (G.1) and N=2 (G.6).
- **Symmetric recipes**: a commutative variant `mint_add_sym(a, b) := mint_add(min(a,b), max(a,b))` would canonicalize order — useful when commutativity is desired.
- **Composition closure**: applying `mint_add` to its own outputs (e.g., `mint_add(add(0,1), succ(2))`) works syntactically; semantic correctness needs G.7.

## Constitutional check

- **C1 (R singular)**: 2 R edges per mint. ✓
- **C2 (R binary)**: each materialization edge is 2-arity. ✓ Note: a TERNARY relation "add(a, b) = c" is NOT a single R edge in v2; it's 2 edges. The ternary semantics live in the structural pattern across multiple binary edges, per commitment 2.
- **C3 (types as meta-R)**: optional `R(__add__, x)` marker tags x as add-derived. ✓
- **C4 (token identity)**: same input pair → same string output, deterministically. ✓
- **C5 (similarity is structural)**: backwards walk via `left_of(add(a,b))` enumerates operands without special-casing. ✓

All commitments preserved.
