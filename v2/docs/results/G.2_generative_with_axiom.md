# G.2 — Generative output × existing axiom system

**Status**: ✓ done (2026-04-29)
**Log**: [`logs/2026-04-29_phase_g2_generative_with_axiom.log`](../../logs/2026-04-29_phase_g2_generative_with_axiom.log)
**Example**: [`examples/phase_g2_generative_with_axiom.rs`](../../examples/phase_g2_generative_with_axiom.rs)

## Goal

G.1 showed that `mint_successor` produces fresh, deterministic identifiers and materializes them as R edges. G.2 asks the next question: does the **existing** `forward_apply_axiom` machinery accept those edges as ordinary data — without any special handling?

If yes, generative output is first-class data; constitutional commitments hold end-to-end.

## Method

1. Build a 5-step minted chain: `R(succ(x), x)` for `x ∈ {0, succ(0), …, succ⁴(0)}`.
2. Register transitivity axiom (`ax_tpl_v3_p0-1_p1-2_c0-2`) via existing `register_axiom_with_intension`.
3. Iterate `forward_apply_axiom` to fixpoint.
4. Verify the full transitive closure of the chain materializes (15 pairs).
5. Round-trip via `to_text` / `from_text` — verify byte-identity is preserved.

## Result

```
chain ids: ["0", "succ(0)", "succ(succ(0))", "succ(succ(succ(0)))",
            "succ(succ(succ(succ(0))))", "succ(succ(succ(succ(succ(0)))))"]
seed rset: 5 edges
transitivity axiom: ax_tpl_v3_p0-1_p1-2_c0-2

  round 1: +4 edges
  round 2: +5 edges
  round 3: +1 edges
  round 4: +0 edges  ← fixpoint

inferred 10 edges, total chain pairs = 15 (= C(6,2))
missing from final rset: 0

critical edge R(succ⁵(0), 0) ∈ rset: TRUE

round-trip: 1525 bytes serialized, restored == original
identifier set identical, 5 minted ids preserved byte-for-byte
```

## Verdict

**POSITIVE — generative output integrates with existing axiom processing without modification.**

- Transitivity closes the full chain over minted identifiers
- Round-trip persistence preserves minted ids byte-for-byte
- No new code path required; existing `forward_apply_axiom` and `to_text`/`from_text` work as-is

## Why this is the integration test

Three things had to be true for G.2 to pass:

1. **Data-id classification**: minted ids are NOT in the meta-id set (no marker registered them as types/axioms/etc.), so `forward_apply` correctly enumerates them as candidates for variable binding.

2. **Variable binding accepts arbitrary strings**: the axiom processor doesn't gate on identifier shape — `succ(succ(0))` is just a string, indistinguishable from `n_0` or `42`.

3. **Persistence is shape-agnostic**: `to_text` only checks for tab/newline; `from_text` only splits on tab. Parentheses and nesting are passthrough. Commitment 4 (token identity) holds because string equality is byte-equality.

If any of these had additional gates, G.2 would have failed. None do — the constitution's "R is uninterpreted" stance pays off cleanly.

## What this slice produced

1. End-to-end demonstration that G.1's mint output is **first-class data** under the existing axiom processor
2. Confirmation that the transitive closure over a 6-id minted chain has the expected size (C(6,2) = 15)
3. Round-trip persistence proof: minted identifiers are stable across serialize/deserialize
4. **No code change to lib** — the integration was already there; G.2 made the property observable

## Future implications

- **G.3 (next)**: ADR formalizing the contract — given that ad-hoc minting works, write down what *kinds* of generative recipes are admissible (deterministic? collision-free? typed?).
- **G.4**: cross-precision generalization — generative axioms produce identifiers, not edges, so DreamCoder-style validation (predict edges on imagined substrate) doesn't apply directly. Need a different validation surface (e.g., does the chain extend coherently when added to substrates that already use these names?).
- **Toward integers**: combine G.1 (chain) + G.2 (transitivity = "less than" semantically) and the system has a viable Peano-natural-number embedding. Adding addition would be the next constructive step.
- **Caveat**: the axiom system reasons over minted ids, but it doesn't yet *cause* minting. The trigger ("when should the runtime mint?") is open — no drive currently calls for new identifiers.

## Constitutional check

- **C1 (R singular)**: all minted edges are R(x,y); transitivity closure also R(x,y). ✓
- **C2 (R binary)**: 2-arity preserved through closure. ✓
- **C3 (types as meta-R)**: no new compile-time type introduced; transitivity axiom is registered via meta-R as before. ✓
- **C4 (token identity)**: `succ(0) == succ(0)` by string equality before AND after round-trip. ✓
- **C5 (similarity is structural)**: closure derives purely from graph structure of the chain — `mint_successor`'s recipe shape didn't leak into the inference. ✓

All commitments preserved. Generative path is constitutionally clean.
