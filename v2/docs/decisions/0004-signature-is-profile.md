# 0004: Signature = IdentifierProfile (0-hop, first pass)

Status: Accepted
Date: 2026-04-23

## Context

The next capability after profile is **structural similarity** — a way for
later mechanisms to ask "which identifiers play the same structural role?"
Design-notes calls this the basis for clustering and for type emergence.

Without a similarity mechanism, there is no way to move from per-identifier
observations (ADR 0003) to grouped roles, and no way for later abstraction
mechanisms to propose "these nodes are instances of the same pattern."

## Decision

**Signature = IdentifierProfile.** Same struct, aliased via
`pub type Signature = IdentifierProfile`. Two identifiers are structurally
equivalent iff their profiles are equal.

New methods on `RSet`:
- `signature(id) -> Signature` — alias for `profile(id)`.
- `equivalence_classes() -> HashMap<Signature, HashSet<&str>>` — partition
  of all identifiers into classes; each class is identifiers sharing a
  signature.

No richer signal at this layer. No neighbor information. No hop > 0.

## Alternatives considered

- **1-hop neighbor profile multiset.** More distinguishing: two middle-chain
  nodes near opposite ends look different because their neighbors' profiles
  differ. Deferred per the minimum-first practice. Activate when 0-hop
  demonstrably collapses things that should stay separate.
- **k-hop Weisfeiler–Lehman-style refinement.** Most distinguishing at the
  structural-similarity layer; standard heuristic for graph isomorphism.
  Expensive per iteration. Deferred; expected as the eventual upgrade path.
- **R-instance-level signature** (pair of endpoint profiles). Useful for
  comparing *edges* rather than *nodes*, but that is a distinct question
  from identifier-level similarity and is addressed by its own future ADR.
- **Motif-based signatures** (count of triangles, cycles, stars containing
  the node). Rich but complex; out of scope for the first pass.

## Consequences

- **Classification, not pattern naming.** 0-hop signatures answer "which
  nodes play the same role?" They do not answer "these four edges form a
  chain pattern." Compound-pattern machinery is a separate, later concern.
- **Expected collapses:**
  - In a forward chain of length n ≥ 3, the n−2 middle nodes collapse into
    one class. Position within middle is invisible.
  - In a cycle, all nodes collapse to one class. Rotational symmetry is
    (correctly) captured.
- **Cheap.** `equivalence_classes()` is O(|identifiers| · |R|) — one profile
  per identifier, bulk HashMap insertion.
- **Upgrade path is clean.** When collapses are too coarse, replace the
  body of `signature(id)` with a 1-hop variant; the downstream API does not
  change.

## Implementation

- Source: `v2/src/lib.rs` (`Signature` alias, `signature()`,
  `equivalence_classes()`, 6 new unit tests).
- Example: `v2/examples/structural_equivalence.rs` (demo on chain,
  bidirectional chain, cycle, star).
- Experiment log: `v2/logs/2026-04-23_structural_equivalence.log`.
