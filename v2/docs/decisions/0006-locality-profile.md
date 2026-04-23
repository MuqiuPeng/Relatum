# 0006: Locality profile for R instances

Status: Accepted
Date: 2026-04-23

## Context

ADR 0005's experiment log flagged a concrete collision: the 3-cycle and
the out-star both collapse to a single `RSignature` class. Endpoint
profiles cannot distinguish them because the distinction lives in *how
edges connect via shared identifiers* — the "locality" axis — which
endpoint-pair signatures do not see.

Three candidates were considered for the next layer:
- **α** locality / co-occurrence (this ADR),
- **β** compound pattern as meta-R instance (ADR 0005's commitment 3 link),
- **γ** self-driven triggering of pattern-naming.

α is chosen first because (i) it directly addresses the cycle-vs-star
collision, (ii) it is still a pure derivation from the RSet and needs no
new state or drive, and (iii) its output is the raw structural signal β
will need to attach names to.

### Note on γ's dormancy

Everything built so far (ADRs 0002–0005, and α below) is deterministic
derivation from the RSet: given the same input, the same output. There
is no choice point. γ's concern — "when does the system *decide* to
act?" — does not yet apply because no mechanism has a choice. γ will
become load-bearing at β, where naming a pattern requires choosing
*which* pattern(s) to name; naming every repeated class would drown the
system. Recording this here so the γ deferral is retrievable.

## Decision

Introduce `LocalityProfile` — four counts describing an R instance's
immediate neighbors via shared identifiers:

```
LocalityProfile {
    co_left:  usize,   // other edges sharing the left (x) endpoint
    co_right: usize,   // other edges sharing the right (y) endpoint
    forward:  usize,   // edges e' with self.y == e'.x  (this flows into e')
    reverse:  usize,   // edges e' with self.x == e'.y  (e' flows into this)
}
```

Method `locality_profile(&R) -> LocalityProfile` on `RSet`. Counts exclude
the edge itself in all four categories — "other edges."

## Alternatives considered

- **Multiset of neighbor R-signatures** (not just counts). Richer — would
  preserve which kind of neighbor is adjacent, not just how many. Deferred
  per minimum-first; counts are sufficient to separate cycle from star,
  which is the immediate motivation.
- **Undirected neighbor set** (bool per id-sharing, no position split).
  Rejected. Collapses direction and would not distinguish, e.g.,
  "chain successor" from "chain predecessor." Commitment 2 requires
  direction-preserving observations.
- **Extend `RSignature` to include `LocalityProfile`.** Deferred. Keeping
  `RSignature` narrow (endpoints only) and `LocalityProfile` separate
  lets later mechanisms pick which axis to condition on. A future ADR
  can introduce a combined signature if needed.
- **2-hop locality** (neighbors-of-neighbors). Deferred. Known limitation:
  1-hop collides chain-middle with cycle-edge (both have
  `co_left=0, co_right=0, forward=1, reverse=1`). Distinguishing these
  needs 2-hop context. Implement only when this collision blocks progress.

## Consequences

Predicted profiles on the motivating examples:

| Graph / edge | co_left | co_right | forward | reverse |
|---|---|---|---|---|
| 3-cycle (any edge) | 0 | 0 | 1 | 1 |
| out-star (any spoke, k=3) | 2 | 0 | 0 | 0 |
| in-star (any spoke, k=3) | 0 | 2 | 0 | 0 |
| 4-chain head edge R(a,b) | 0 | 0 | 1 | 0 |
| 4-chain middle edge R(b,c) | 0 | 0 | 1 | 1 |
| 4-chain tail edge R(c,d) | 0 | 0 | 0 | 1 |

Cycle and star are now separated. Chain middle and cycle still collide
at 1-hop; this is a known, recorded limitation.

Cost: for each edge, counting co_left / co_right / forward / reverse is
one pass over the RSet — O(|R|) per edge, O(|R|²) for the whole set.
Acceptable at the current experiment scale; if locality becomes a
frequent call in a hot path, index structures can be added later.

Upgrade path: if counts collide on examples we care about, the path is
to replace the count fields with neighbor-signature multisets (α') or
to add 2-hop locality (α''). Downstream call sites just ask for
`locality_profile(r)` and read the fields — structure of the API holds.

## Implementation

- Source: `v2/src/lib.rs` (`LocalityProfile`, `locality_profile`, tests).
- Example: `v2/examples/locality.rs` — cycle vs star vs chain side-by-side.
- Experiment log: `v2/logs/2026-04-23_locality.log`.
