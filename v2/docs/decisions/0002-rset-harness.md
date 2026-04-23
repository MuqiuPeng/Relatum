# 0002: RSet as the observation harness

Status: Accepted
Date: 2026-04-23

## Context

v2 needs a place to hold R instances and a way to observe them. Before any
abstraction mechanism (object emergence, pattern detection, type naming) can
function, it needs a stable surface to query: "what R instances exist, which
identifiers appear, what are the incoming/outgoing edges at this identifier."

This decision establishes that surface — deliberately thin, with zero
interpretation. The goal is to keep the observation layer and the abstraction
layer separable, so later mechanisms can be added without disturbing the base.

## Decision

Introduce `RSet`, a deduplicated set of R instances backed by `HashSet<R>`,
with the following observation methods:

- `add(R) -> bool`, `extend(iter)`, `contains(&R)` — mutation and membership.
- `len()`, `is_empty()`, `iter()` — enumeration.
- `identifiers()` — all tokens appearing on either side, as `HashSet<&str>`.
- `left_of(id)`, `right_of(id)` — instances with `id` in the respective slot.

No salience, similarity, or type machinery at this layer.

## Alternatives considered

- **`Vec<R>` with external dedup.** Rejected: commitment 4 (token-based
  identity) makes dedup a semantic requirement, not an optimization. Encoding
  it structurally via `HashSet` + `PartialEq/Eq/Hash` on `R` is the honest choice.
- **`BTreeSet<R>`.** Rejected: ordering adds a structure v2 is not ready to
  commit to. The constitution says nothing about order.
- **Trait-based abstraction over storage.** Rejected as premature. One
  implementation first; generalize when an alternative is actually needed
  (e.g., streaming or persistent storage).
- **Expose `HashSet<R>` directly.** Rejected: couples consumers to the current
  impl and prevents future refinement.

## Consequences

- Dedup is automatic and structurally guaranteed.
- Every subsequent mechanism can assume the observation surface is stable.
- The methods bias observation toward per-identifier views — if R-instance-level
  observations become primary, the API will need to be extended or
  reorganized. Not a problem for the current phase.

## Implementation

- Commit `5abdb73` — `v2: add RSet — observation API over R instances`.
- Source: `v2/src/lib.rs`.
- Tests: 6 behaviors covering dedup, direction-distinctness, identifier
  collection, slot partitioning, chain representability.
