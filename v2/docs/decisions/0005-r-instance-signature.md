# 0005: R-instance signature (edge-level, endpoint profile pair)

Status: Accepted
Date: 2026-04-23

## Context

ADR 0004 established identifier-level signatures (0-hop profile). The
resulting equivalence classes cluster nodes by role but give no notion of
"edges playing the same structural role." That gap blocks compound pattern
detection: to say "these four edges are a chain of three," a mechanism
first needs to see the edges *as typed elements* and notice their
repetition.

Two design axes for compound patterns were surfaced in the progress note
after ADR 0004:
- **(a) edge-level patterns** — each edge is an atomic unit with a type
  derived from its endpoints.
- **(b) subgraph-level patterns** — a connected subgraph is the atomic unit.

(a) is chosen as the first step because:
- It is cheap and structurally isomorphic to the identifier-level
  signatures of 0004 (same machinery, lifted one level).
- Its failures will be informative about what subgraph-level machinery
  actually needs — which we do not yet know.

## Decision

Introduce `RSignature = (Signature, Signature)` — the ordered pair of
the endpoint signatures of an R instance.

New methods on `RSet`:

- `r_signature(&R) -> RSignature` — endpoint signature pair in (x, y) order.
- `r_equivalence_classes() -> HashMap<RSignature, HashSet<&R>>` — partition
  of all R instances by their edge-level signature.

Two R instances are structurally equivalent iff their signature pairs are
equal. Pair order matters — commitment 2 insists direction is intrinsic.

## Alternatives considered

- **Unordered pair** (multiset of endpoint signatures). Rejected: would
  collapse `R(x, y)` and `R(y, x)` when their endpoints have the same
  profiles, destroying directionality. Violates commitment 2 in spirit.
- **Subgraph pattern as the first primitive.** Deferred: requires a notion
  of "which subgraphs count as candidate patterns," which is an additional
  mechanism we have not designed. Edge-level is a cheaper, more informative
  waypoint.
- **Edge signature enriched with 1-hop context of endpoints.** Deferred
  per minimum-first. Upgrade the signature body later without changing
  the downstream API.
- **Self-loop special casing** (R(a, a) as a distinct kind). Not needed:
  tuple equality handles it — a self-loop gets signature `(P_a, P_a)` and
  naturally merges with any other edge that happens to have identical
  endpoint profiles. If this collapse turns out to hide meaningful
  distinctions, a future ADR can address it.

## Consequences

Predicted classifications on canonical graphs:

| Graph | Expected classes |
|---|---|
| n-chain (n ≥ 4) | 3: head-edge, middle-edges (merged), tail-edge |
| n-chain (n = 2) | 1: just the single edge |
| n-chain (n = 3) | 2: head-edge, tail-edge (no middle-middle) |
| k-cycle | 1: all edges merge |
| out-star (k spokes) | 1: all spokes merge |
| in-star (k spokes) | 1: all spokes merge |
| bidirectional chain | 3: out-from-end, in-to-end, middle-middle (bidir) |
| two disjoint chains | same as one chain, with each class populated twice |

These predictions serve as the experiment's oracle.

Cost: O(|R|) to compute all signatures and build the partition.

Upgrade path: replace the signature body (the two `Signature` components)
with richer values without changing `r_equivalence_classes`'s shape. The
"I want edge types to carry 1-hop context" upgrade is pure internal change.

## Implementation

- Source: `v2/src/lib.rs` (`RSignature`, `r_signature`, `r_equivalence_classes`, tests)
- Example: `v2/examples/edge_equivalence.rs`
- Experiment log: `v2/logs/2026-04-23_edge_equivalence.log`
