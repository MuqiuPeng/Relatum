# 0007: Compound signature probe (observation before β)

Status: Accepted
Date: 2026-04-23

## Context

ADR 0006 closed the cycle-vs-star collision at the locality layer but
left three β-layer questions open (see ADR 0005 / 0006 logs):

- (i) What shape links a pattern-name identifier to its member edges?
- (ii) What counts as a *nameable* pattern?
- (iii) What triggers naming (γ)?

This ADR addresses (ii) empirically, ahead of committing to a β
mechanism. The question is: **if we compose the three observation
layers we already have** — endpoint signature (`RSignature`, ADR 0005)
and locality profile (`LocalityProfile`, ADR 0006) — **do the resulting
compound classes already expose natural pattern candidates?**

This is an observation probe, not a new ontological commitment. It
adds a tiny utility (`EdgeFingerprint` = `(RSignature, LocalityProfile)`
+ `edge_fingerprint(&R)`) and runs a single mixed-graph experiment to
see what falls out. The ADR documents the probe and its findings; its
success criterion is that the results either (a) suggest a natural
answer to (ii), or (b) tell us a specific new signal is needed.

## Decision

Add two items to `v2/src/lib.rs`:

- `pub type EdgeFingerprint = (RSignature, LocalityProfile);`
- `impl RSet { pub fn edge_fingerprint(&self, r: &R) -> EdgeFingerprint }`

Run one experiment: a mixed graph consisting of a disjoint union of a
5-chain, a 3-cycle, an out-star with three spokes, a small tree
(three edges), and a single isolated edge. Compute `edge_fingerprint`
for every edge, partition by value, inspect the result.

No new commitment is made by this ADR. It is a probe, and a specifically
scoped one: a single mixed graph, analyzed by eye.

## Alternatives considered

- **Skip the probe, go straight to β.** Rejected per minimum-first: the
  β design hinges on (ii), and (ii) is more honestly answered with one
  cheap experiment than with architectural speculation.
- **Longer sweep across many graphs.** Deferred. One mixed graph is
  enough to see the *shape* of compound-class behavior; if it leaves
  specific open questions, a targeted follow-up probe is cheaper than a
  broad upfront sweep.
- **Also include identifier-level signatures in the fingerprint.**
  Rejected as redundant — `RSignature` already embeds the endpoint
  identifier signatures.
- **Introduce `edge_fingerprint_classes()` method now.** Deferred.
  Convenient but not load-bearing; the experiment binary can do the
  bucketing inline. Add a method only when a second caller wants it.

## Consequences

- If the compound classes already produce a natural set of
  pattern-repetition candidates (e.g., "these 3 edges are structurally
  equivalent *and* locally situated the same way"), question (ii) has
  an empirical answer: nameable patterns are compound classes with
  size > k, for some k to be fixed.
- If compound classes over-merge (many different structural roles land
  in one class), the probe has told us the signal is still too coarse
  and we need a richer layer (2-hop locality, or a locality signature
  that uses neighbor R-signatures rather than counts). That is also a
  useful finding.
- If compound classes under-merge (even obvious repetitions split into
  singletons), β's mining threshold must be lower than "class size > 1"
  or must consider across-graph data. Useful finding.
- The chain-middle / cycle-edge 1-hop collision is carried forward
  unchanged; this probe does not attempt to break it.

## Implementation

- Source: `v2/src/lib.rs` (`EdgeFingerprint`, `edge_fingerprint`,
  small test suite).
- Example: `v2/examples/compound_signature.rs` — the mixed graph run.
- Experiment log: `v2/logs/2026-04-23_compound_signature.log` with the
  observed compound classes and the findings for (ii).
