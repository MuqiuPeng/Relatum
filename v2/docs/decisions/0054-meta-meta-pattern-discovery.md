# 0054: Meta-meta-pattern discovery (Phase D)

Status: Accepted (Phase D0 + D0+ implemented)
Date: 2026-04-26

## Context

ADR 0053 landed three M1 markers: `ESTABLISHED_MARKER` (for
patterns and theories that have proven their use over time) and
`SHARED_AXIOM_MARKER` (for axioms structurally bridging multiple
theories). The runtime now writes these edges, but **nothing
reads them**. If the rest of the system never consumes M1, then
M1 is decoration: a journal entry the runtime makes about itself
that no other mechanism cares about.

The promise from ADR 0052 — and the explicit motivation in
ADR 0053's summary — is that M1 facts become **subjects of
downstream discovery**. "What do all established patterns share?"
should be a query the system can ask of itself. Phase D is the
slice that makes that promise concrete.

The risk: turning the meta-inclusion knob on globally floods
discovery with structural noise from PATTERN_MARKER /
AXIOM_MARKER / role / instance edges. The same flag
(`include_meta_in_discovery`) already exists, but it's binary
and indiscriminate. Phase D needs targeted inclusion of just the
M1 subgraph plus its anchored objects.

## Decision

### What "meta-meta-pattern" means here

A **first-order pattern** is a pattern over data edges (the
default — `include_meta_in_discovery = false`). v1's entire
discovery pipeline operated here.

A **meta-meta-pattern** is a pattern whose canonical form
includes M1 markers as nodes. Examples (illustrative, not
prescriptive):

- "Pattern X is established AND has ≥ 2 instances AND owns
  ≥ 1 role." — captures *what shape established patterns share*.
- "Theory T is established AND every member axiom of T is
  shared." — captures *theories built entirely from
  cross-cutting machinery*.
- "Axiom A is shared AND appears in some established theory." —
  captures *which shared axioms are also experience-validated*.

These are not new ontology levels. They are first-order patterns
in v2's single-layer ontology — the only thing special is that
their anchor nodes happen to live in M1.

### Phase D0 — the smallest slice (implemented)

The implementation took a slightly more conservative slice than
this ADR's original sketch:

- **Filter on rset, not on `DiscoveryConfig`.** The
  `meta_subset_filter` field was *not* added to `DiscoveryConfig`.
  Instead, `RSet::discover_motifs_with_meta_subset(config, subset)`
  is a separate entrypoint that internally does the filtered edge
  selection and delegates to a private
  `discover_motifs_from_edges` helper. This avoids touching the
  20+ existing `DiscoveryConfig` literal-construction sites and
  keeps the option out of the default surface for callers that
  don't care.
- **Naming pipeline (D0+) followed in the next slice.**
  `find_instances_of` and `is_clean_subgraph` were extended with
  meta-subset variants (`find_instances_of_with_meta_subset`,
  `is_clean_subgraph_with_meta_subset`) that honor the same
  filter semantics. The `DiscoverMetaMetaPatterns` action now
  takes the top novel candidate, finds clean instances under
  the M1 view, and records them via
  `name_pattern_instances_with_policy(..., Intensional)`. The
  Intensional policy was deliberate: it writes Layer A (registry
  + roles + structural edges) but skips Layer B
  (instance-bound participant edges), which prevents the
  ESTABLISHED / SHARED_AXIOM markers from being pinned as
  literal participants of the new pattern. With Layer B off,
  the marker still appears in the pattern's *role* identifiers
  but no `R(<inst>, ESTABLISHED_MARKER)` edge gets minted.
  Loop-closure verified end-to-end: a runtime starting with 5
  ESTABLISHED-marked patterns names a meta-meta-pattern within
  ≤ 8 ticks.

#### `DiscoveryConfig::meta_subset_filter: Option<HashSet<String>>`

When `Some(set)`, `discover_motifs` includes only meta edges
whose endpoints (or marker) are in `set` — and excludes all other
meta. When `None`, behaves exactly as
`include_meta_in_discovery` does today (default `false`). The
two flags are independent: caller picks one regime explicitly.

Selection logic (conceptual):

```text
let edge in rset:
    if endpoints are both data:
        include
    elif at least one endpoint is in set OR edge.marker is in set:
        include
    else:
        exclude
```

The selectivity matters: with `set = {ESTABLISHED_MARKER,
SHARED_AXIOM_MARKER}`, the discovery sees data + M1 *only*, and
ignores the bulk of meta plumbing (role registries, instance
ownership, premise/conclusion chains, theory relation edges).
That keeps the meta-meta-discovery's hypothesis space small and
the resulting patterns interpretable.

#### `ActionKind::DiscoverMetaMetaPatterns`

Runtime action that calls `discover_motifs` with
`meta_subset_filter = Some({ESTABLISHED, SHARED_AXIOM})`,
applies a strict-mode `name_pattern_instances` to the top
candidates, and lets ADR 0040's existing low-value pruning lane
retract any whose counterfactual goes negative.

The action mirrors the existing `DiscoverPatterns` dispatch
shape: same naming policy, same instance-sampling control flow,
same `last_improved_tick` accounting. Discovered patterns enter
ObjectHistory and become eligible for C0 promotion just like any
other pattern. **This is the loop closure**: M1 facts feed
discovery, discovery produces patterns, patterns get promoted
back into M1 if they prove useful. Whether that loop is healthy
is the open question Phase D exists to answer.

#### Frontier integration

A new `FrontierKind::MetaMetaCandidate`. Surfaced when:

- `rset.contains` ≥ `min_m1_edges_for_meta_meta` edges with
  `ESTABLISHED_MARKER` or `SHARED_AXIOM_MARKER` (default 5);
- AND no `MetaMetaCandidate` is already pending.

Picked in Expand mode, alongside `TheoryCandidate` /
`PatternCandidate`. Subject to the same B1+ cooldown machinery
as DiscoverPatterns (a separate counter — meta-meta is allowed
to fail several times before being suppressed, since it's
exploratory).

### Phase D1 (sketch, deferred)

**Bias frontier priorities by M1 evidence.** A pattern whose
instances overlap heavily with `R(?, ESTABLISHED_MARKER)` ids
gets a priority boost — the runtime is preferring to extend
already-validated regions. Cheap heuristic, no new state.

### Phase D2 (sketch, deferred)

**Closed-loop self-evaluation.** Once a meta-meta-pattern is
discovered and promoted to ESTABLISHED, run a controlled
ablation: temporarily mask its instances and re-run the
counterfactual evaluator. If abstraction_score drops more than
some threshold, the meta-meta-pattern is *load-bearing* — keep.
Else demote. This puts the runtime's promotion claims to a
falsifiable test, in line with the v2 failure-bar memory.

## Alternatives considered

- **Skip Phase D; flip the existing flag.**
  `include_meta_in_discovery = true` does include M1 — but it
  *also* includes every other meta edge. The hypothesis space
  blows up; the resulting patterns are dominated by structural
  artifacts (role/instance/axiom-shape), not by the
  experience-with signal we care about. Not worth the noise.
- **Build an ephemeral sub-rset containing data + M1 only,
  discover there, name in the original.** Equivalent in result
  to the filter knob, but creates a transient RSet that must be
  reconciled with the main one. Bookkeeping not worth saving the
  one extra parameter.
- **A separate `MetaRSet` type.** Same problem as the ephemeral
  sub-rset approach, plus it splits the ontology — violates v2's
  single-layer commitment.
- **Pattern over ESTABLISHED_MARKER alone (ignore SHARED_AXIOM).**
  Cleaner in scope, but loses the most interesting M1 facts:
  axioms that bridge theories *are* the system's discovered
  abstractions about its own theories. Excluding SHARED_AXIOM
  means D0 can't see them. Include both.

## Non-goals

- **Truly higher-order discovery.** Patterns over patterns over
  patterns is recursive and unbounded; v2's commitment to a
  single-layer ontology means there is no level-3 marker.
  Meta-meta-patterns are still first-order patterns over a
  larger graph.
- **A new "meta-meta-pattern" object type.** Discovered
  meta-meta-patterns are normal patterns, named via the existing
  `name_pattern_instances` API and tracked in
  `ObjectHistory.patterns`. The only thing special about them is
  what they happen to reference.
- **Implicit triggering.** Phase D actions are scheduled, not
  baked into discovery's default code path. A caller using
  `RSet::autonomous_pass` directly does NOT get meta-meta
  discovery for free.

## Verification plan

For Phase D0:

1. Existing 381 tests pass unchanged.
2. New tests:
   - **filter scope — empty M1**: `meta_subset_filter` set,
     no ESTABLISHED / SHARED_AXIOM edges in the rset →
     `discover_motifs` returns the data-only result (filter
     doesn't add anything when its target is absent).
   - **filter scope — some M1**: rset has data + a few M1
     edges → discovery sees data + M1 *only*; structural
     artifacts (role chains, etc.) are excluded.
   - **filter scope — pure M1**: rset has only M1 edges, no
     data → discovery still works (M1 anchors are valid
     samples).
   - **action dispatch — gate**: frontier has fewer than
     `min_m1_edges_for_meta_meta` M1 edges → no
     `MetaMetaCandidate` item.
   - **action dispatch — promote**: ≥ threshold M1 edges
     present → item appears, dispatched, `discover_motifs`
     called once, episode recorded.
   - **loop closure (smoke)**: in a runtime with several
     ESTABLISHED patterns sharing a sub-shape, run for ≥ N
     ticks → a meta-meta-pattern is discovered and named, and
     its `ObjectHistory` entry is created.
3. End-to-end: a hand-crafted scenario where three first-order
   patterns share an instance-shape AND all three are
   ESTABLISHED. After Phase D0 fires, the system has named a
   fourth pattern whose canonical form references
   `ESTABLISHED_MARKER`. This is the "system asking itself
   *what do my established patterns share?*" demo.

## Open questions (for the implementation, not blocking acceptance)

1. **Strict vs lax matching for meta-meta**. The existing
   discovery pipeline uses isomorphism for canonicalization. M1
   markers are atoms — they canonicalize to themselves. Should
   meta-meta-patterns require the marker node to be the
   *same* marker, or are markers role-equivalent (so
   ESTABLISHED and SHARED_AXIOM nodes can collapse during
   canonicalization)? Suggest "same marker" — markers carry
   semantics, treating them as interchangeable defeats the
   purpose.
2. **Cooldown counter sharing**. If `DiscoverPatterns` and
   `DiscoverMetaMetaPatterns` share the B1+ counter, an
   unproductive D0 pass burns the regular discovery's budget.
   Almost certainly want separate counters — TBD wiring.
3. **Persistence of discovered meta-meta-patterns**. Same as
   regular patterns: rset checkpoint round-trips them. No new
   work — just confirm.
4. **Termination property**. If meta-meta-patterns become
   ESTABLISHED, then *they* could feed another meta-meta pass
   (meta-meta-meta…). The single-layer ontology commitment
   means this is fine in principle (still first-order
   patterns), but in practice the hypothesis space could
   explode. Suggest D0 ships with a hard cap on the number of
   ESTABLISHED → meta-meta cycles per `run_bounded` invocation.

## Touched ADRs

- **ADR 0018** `autonomous_pass` — Phase D0's action calls into
  the same discovery pipeline; no API change.
- **ADR 0029** pattern naming — discovered meta-meta-patterns
  use the same `name_pattern_instances` machinery.
- **ADR 0040** `Prune` — meta-meta-patterns that fail
  counterfactual cleanup go through the existing prune lane.
- **ADR 0043** sampling — the sample-based discovery path
  inherits this filter knob; instance-sampling matching gets
  the same M1-only view.
- **ADR 0052** Phase D was deferred; this ADR fills it in.
- **ADR 0053** M1 markers are the *input* to meta-meta
  discovery. This ADR is the answer to ADR 0053's implicit
  question: "what for?"

## Summary

Phase D is M1's payoff — the moment the runtime starts treating
its own experience as material for further abstraction. D0 is
the smallest slice that lets the system pattern over its own
ESTABLISHED / SHARED_AXIOM subgraph specifically, without
flooding discovery with all of meta-R.

Implementation lands in a follow-on commit if the design
survives review. Status: **Proposed**.
