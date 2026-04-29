# G.3 — ADR for identifier minting

**Status**: ✓ done (2026-04-29)
**ADR**: [`docs/decisions/0069-identifier-minting.md`](../decisions/0069-identifier-minting.md)

## Goal

Codify the contract that any future generative axiom must satisfy.
G.1 + G.2 demonstrated one valid recipe (successor) end-to-end —
G.3 forward-binds future code: any future generative recipe must
satisfy the contract or it is not a valid v2 mechanism.

## What ADR 0069 specifies

**Four contract properties for any generative axiom**:

1. **Determinism** — `mint(t)` returns the same string every time across processes
2. **Anti-collision (with input space)** — output ≠ any input or prior chain member
3. **Materializability** — output expressible as R(x,y) edges using only inputs and freshly minted ids
4. **Persistence safety** — minted ids survive `to_text` / `from_text` byte-identically

**Lifecycle and tagging**: generative axioms differ from template/predicate axioms; should register under their own marker, with derived tokens recorded under a per-recipe marker (e.g., `R("__successor__", "succ(0)")`).

**Cross-precision applicability**: explicitly *does not apply* to generative axioms. They produce identifiers, not predictions over a fixed substrate. Generative axioms must be excluded from cross-precision-driven demote until G.4 specifies an alternative metric.

## Why an ADR (and not just an example)

Without this contract, a future direction could naively add a "random-id minter" or "timestamp-tagged id" mechanism, both of which would silently break commitment 4. Codifying the contract now prevents that class of regression.

## What this slice produced

1. ADR 0069 — design contract for generative axioms (no code change)
2. README.md updated with 0067-0069 entries (had been missing 0067 and 0068 from index)
3. Forward-binding for G.4-G.7 future work

## Future implications

- Provides a checklist any future generative recipe must pass
- Distinguishes generative from template/predicate axiom kinds clearly
- Documents the cross-precision asymmetry — saves G.4 work from accidentally trying to force-fit cross-precision

## Verdict

ADR drafted and accepted. G.1 + G.2 stand as the empirical witnesses; G.3 is the contract they fit.
