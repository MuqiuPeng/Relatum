# 0003: IdentifierProfile as first-pass structural observation

Status: Accepted
Date: 2026-04-23

## Context

The design notes list "object emergence" as the first capability: recognize
stable identifiers in the R instance stream. Commitment 4 states that identity
is token-based — every distinct token is already an object. So "emergence"
does not mean creating objects from nothing; it means identifying which
tokens are structurally salient enough to deserve further attention.

A first-pass mechanism is needed to supply salience-related observations.
The mechanism itself should not make salience judgments; it should provide
the raw observations that downstream mechanisms can judge from.

## Decision

Introduce `IdentifierProfile`, a structural summary of one identifier:

- `degree_out: usize` — appearances in the left (x) slot.
- `degree_in: usize` — appearances in the right (y) slot.
- `slots: SlotPattern` — one of `None | LeftOnly | RightOnly | Both`.

Plus methods on `RSet`:

- `profile(id) -> IdentifierProfile` — includes zero-profile for absent ids
  (deliberately, since commitment 4 makes "absent" a count, not an error).
- `profiles() -> HashMap<&str, IdentifierProfile>` — bulk variant.

No judgment is made. Two identifiers with the same profile are
"structurally equivalent at profile granularity" — interpretation of that
equivalence is left to later mechanisms.

## Alternatives considered

- **Full neighbor sets** (list of in-neighbor ids, list of out-neighbor ids).
  Deferred. Neighbor sets encode raw-identifier information that obscures
  structural similarity (two middle-of-chain nodes would not look similar
  because their neighbors are different identifiers). Structural similarity
  needs features abstracted from specific identifiers.
- **Self-loop flag.** Deferred; derivable from checking `left_of(id)` for
  `R(id, id)`. Add a direct accessor only if self-loop logic becomes common.
- **Co-occurrence** (which identifiers appear together across multiple R
  instances). Richer signal, more expensive. Deferred until needed.
- **Multi-hop reachability profile.** More distinguishing but much more
  expensive. Deferred. If 0-hop profiles fail to separate obviously distinct
  roles, consider 1-hop neighbor-profile multisets first, then k-hop.
- **Merge `Profile` and `Signature`** (skip this ADR, go straight to 0004).
  Rejected: profile is an observation; signature is a use of profile for
  similarity. Keeping them separate makes it easier to expand signature
  without changing profile (e.g., a signature that uses 1-hop neighbor
  profiles rather than the raw profile).

## Consequences

- Profile is O(|R|) per identifier to compute; O(|R| · |identifiers|) for
  the bulk `profiles()` call. Acceptable for early experiments.
- Profile distinguishes endpoints from middles in a chain, but does *not*
  distinguish positions within the middle (e.g., `a2` and `a4` in a 5-node
  chain have the same profile). This limitation is expected to surface when
  mechanisms need to name "the third node of a chain"; at that point,
  refinement moves from "deferred" to "required."
- `SlotPattern::None` exists for ids that are queried but absent from the
  set. Surprising at first; matches the token-based-identity commitment
  which makes every string a potential identifier.

## Implementation

- Commit `8886d53` — `v2: add IdentifierProfile — first-pass structural salience observation`.
- Source: `v2/src/lib.rs`.
- Tests: 6 behaviors covering absent-id zero profile, slot distinction,
  degree counts, self-loop, bulk profiles, chain endpoint asymmetry.
