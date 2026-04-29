# G.7 — Integer arithmetic embedding (constructive scaffold)

**Status**: ✓ done (2026-04-30)
**Log**: [`logs/2026-04-30_phase_g7_integer_embedding.log`](../../logs/2026-04-30_phase_g7_integer_embedding.log)
**Example**: [`examples/phase_g7_integer_embedding.rs`](../../examples/phase_g7_integer_embedding.rs)

## Goal

Combine G.1's successor recipe + G.6's addition recipe + transitivity to express integer **order** ("less than" / "greater than"), and document where arithmetic **equivalence** (`add(1, 1) ≡ 2`) requires the equality-axiom layer.

## Method

Three plies:

- **Ply A** — Successor chain (G.1): mint 5 levels of `succ(...)` from seed `"0"`. 6 chain ids total.
- **Ply B** — Transitivity closure (G.2): apply `R(x,y) ∧ R(y,z) → R(x,z)` to fixpoint. Produces strict total order.
- **Ply C** — Addition mints (G.6): mint `add(a, b)` for 4 chain pairs. Re-close transitivity. Observe boundary.

## Result

### Ply A — chain materializes

```
0, succ(0), succ²(0), succ³(0), succ⁴(0), succ⁵(0)
```

5 direct chain edges `R(succⁿ(0), succⁿ⁻¹(0))`.

### Ply B — full transitive closure

After `forward_apply_axiom(transitivity)` to fixpoint:
- **15 directed chain pairs** present in rset (= C(6,2))
- Strict total order verified: for every distinct (i, j), exactly one of `R(chain[i], chain[j])` or `R(chain[j], chain[i])` holds.
- Semantic reading: `R(succⁿ(0), succᵏ(0))` ↔ "n is greater than k"

This is **Peano arithmetic's "<" relation** materialized inside v2.

### Ply C — addition mints + boundary

4 addition mints registered:
- `add(succ(0), succ(0))` = `add(succ(0), succ(0))`
- `add(succ(0), succ(succ(0)))`
- `add(succ(succ(0)), succ(succ(0)))`
- `add(0, succ(succ(0)))`

Each writes 2 operand edges (per G.6). Transitivity re-closure on full rset.

## Boundary observation

`add(succ(0), succ(0))` should arithmetically equal `succ(succ(0))` (1 + 1 = 2). Are they structurally equivalent in v2?

| identifier | neighborhood size |
|---|---|
| `add(succ(0), succ(0))` | 2 edges (just operand pointers) |
| `succ(succ(0))` | 8 edges (chain edges + transitive order pairs) |

**Not structurally equivalent.** The two identifiers occupy disjoint structural positions.

## Verdict

**POSITIVE on the scaffold; open on full arithmetic.**

Achievements:
- Ply A: ✓ successor chain materializes
- Ply B: ✓ transitivity yields Peano "<" relation
- Ply C: ✓ addition mints integrate as new identifiers

Boundary:
- `add(N_a, N_b) ≡ N_(a+b)` is NOT auto-derivable. Requires:
  - **Equality axiom** asserting `add(x, y) = z when chain-position(x) + chain-position(y) = chain-position(z)`
  - This is the existing v2 equality axiom layer (ADR 0044 / 0047)
- Future slice (G.8 / G.9): construct the equality axiom, verify closure produces `add(succ(0), succ(0)) = succ(succ(0))`

## Why the boundary is at equality, not at structure

In v2, identifiers are tokens (commitment 4). Two strings are equal IFF byte-equal. `add(succ(0), succ(0))` and `succ(succ(0))` are different strings, hence different identifiers.

To mint them as equivalent requires an explicit semantic statement — an equality axiom. This is **the right architecture**: equivalence is a declared assertion, not an inferred property of the recipe layer.

For arithmetic, the natural equality axiom would be:
```
∀ a, b such that {a, b} ⊂ chain
  add(a, b) = z
  where z is chain[chain_position(a) + chain_position(b)]
```

Computing `chain_position` is itself a structural query (count predecessors via transitivity). So the equality axiom needs ACCESS to the chain order — which Ply B provides. The pieces are in place; G.7 stops at scaffolding because the equality slice is its own design surface.

## What this slice produced

1. End-to-end integer **order** materialized inside v2 (Peano "<" relation)
2. Concrete demonstration that addition mints CAN be added without breaking the chain
3. Clear boundary statement: arithmetic **equivalence** is the next step, requiring equality axioms
4. Proof of viability for the integer-construction direction the user asked about ("距离构造出整数概念还有多久")
5. Specification of the natural equality axiom needed to close the gap

## Future implications

- **G.8 / equality closure**: write the equality axiom that asserts `add(succⁿ(0), succᵐ(0)) = succⁿ⁺ᵐ(0)`. This is the slice that ACTUALLY produces "1 + 1 = 2" inside v2.
- **Subtraction, multiplication**: same template. `mint_sub`, `mint_mul` recipes + their equality axioms.
- **Recursive definitions**: `mul(x, succ(y)) = add(x, mul(x, y))` is the inductive case. Whether v2's axiom system can express recursive definitions is its own question.
- **The user's original question**: "距离构造出整数概念还有多久?" — G.7 answers concretely: **the order relation is here NOW, the equality closure is one slice away, basic arithmetic closure is two**. The cognitive primitive of "integer" is structurally constructible inside v2 within the next 2-3 slices.

## Constitutional check

- C1 (R singular): all materialized edges are R(x,y). ✓
- C2 (R binary): 2-arity preserved. ✓
- C3 (types as meta-R): would extend to `R(NUMBER_MARKER, succⁿ(0))` if needed. ✓
- C4 (token identity): every minted id is byte-deterministic. ✓
- C5 (similarity is structural): the order relation + boundary observation both rely on graph structure alone. The boundary is honest about which equivalences ARE structural and which require declared axioms. ✓

All commitments preserved through the integer scaffold.
