# 0075: Emergence kernel — audit and runtime integration

Status: Proposed
Date: 2026-05-06

Parents:
- [Reflection 0001 — meaning emerges with concept](../reflections/0001-meaning-emerges-with-concept.md)
- [Constitution amendment — strict reading](../constitution.md#strict-reading-differentiation-requires-registration)
- [0073 — phase pivot to concept emergence](0073-phase-pivot-concept-emergence.md) (now partially superseded)

Supersedes (in spirit, not in number):
- The earlier 0075 draft on "intrinsic drive from unexplained R" was
  withdrawn unrendered after the heavy-reading constitution amendment.
  This is the replacement.

## Context

The reflection-driven constitution amendment requires every act of
concept creation to be **atomic** in three facets: (a) mint a
concept token, (b) register participating tokens as instances of
it via meta-R, (c) never use per-token derived signature as
visible behaviour outside that atomic act.

ADR 0073's diagnosis "the system cannot create new concepts" was
written before this strict reading was applied. After the reading,
v2's existing pattern-naming pipeline can be re-audited:

- `Subgraph::canonicalize` (ADR 0009) operates only on the
  subgraph's own edges; initial labels are local (in-degree,
  out-degree) of nodes *within the subgraph*; WL refinement uses
  only neighbour labels within the subgraph. **No outer-RSet
  IdentifierProfile is consulted.** Therefore canonical form is a
  property of the subgraph, not of any token.
- `discover_motifs` (ADR 0016) groups sampled subgraphs by
  canonical form. **The bucket key is the canonical form, not a
  per-token signature.**
- `name_pattern_instances` (ADR 0010 / 0029) atomically writes:
  - `R(PATTERN_MARKER, p)` — concept exists
  - `R(ROLE_MARKER, role_i)` per role + `R(p, role_i)` —
    intension structure
  - `R(role_i, role_j)` for the canonical edges among roles
  - per-instance `R(p, instance_n)` + `R(instance_n, participant)`
    for every token participating in that instance — **this is
    the object-emergence facet**
- `autonomous_pass` (ADR 0018) wires the three together as a
  single closed loop.

Each step satisfies the strict reading. An audit was needed to
confirm the pipeline actually mints patterns (not just dead code)
and that participating tokens are explicitly registered.

## Audit result (2026-05-06)

`examples/phase_emergence_kernel_audit.rs` runs `autonomous_pass`
on each canonical substrate after its standard Phase 0:

```
substrate       ticks  total_edges  pre  post  new  total_instances
OQ#1            1000          375    0     7    7              105
long5k          1500          420    0     7    7              140
narrow_a         500          345    0     3    3               35
OQ#2            4500          245    2     7    5              172
```

Notes:
- `pre` = patterns existing before the audit's autonomous_pass
  call. OQ#2 had 2 — those came from the runtime's incidental
  pattern discovery during Phase 0; the audit added 5 more.
- `total_instances` aggregates across all minted patterns; each
  instance carries 1+ participating tokens registered as
  `R(instance_n, participant)` meta-R.
- Per-pattern participant counts (e.g. OQ#1's p_0 has 25
  distinct participants over 30 instances) confirm object
  emergence: 25 tokens that were previously only string-equal
  now carry the explicit property "is a participant in p_0
  instances."

### Surprise: OQ#2 is the most kernel-active substrate

The Emergence-1 substrate-diversity probe
(`docs/results/phase_emergence_1_substrate_diversity.md`) had
classified OQ#2 as the "blind spot" where Phase 0070-0072
produced nothing. It produced 0 template axioms and 0 shape
families. The diversity probe's verdict was that E3 (intrinsic
drive) would be most useful precisely there.

The kernel audit reverses this: **OQ#2 is the most pattern-rich
substrate by total instances (172 vs OQ#1's 105)**. The 84-
instance pattern p_3 alone is bigger than any pattern on OQ#1.
What was happening was that the *axiom path* (template
enumeration → predicate-axiom registration → theory naming) found
nothing on OQ#2's tournament/lattice/star regimes, while the
*pattern path* (subgraph canonicalize-and-name) was rich with
recurring substructure. Both paths exist in v2; only the axiom
path was consulted in Phase 0070-0072 work.

The diversity probe's "RSet collapse" finding was therefore
specifically about the axiom path. Pattern-path output is
genuinely substrate-distinct (different substrates produce
different participant counts and different instance counts at
each pattern size). The heavier diagnostic claim — "v2 collapses
every rich-enough stream to one RSet" — applies to the
axiom-discovery output, not to pattern-discovery output.

## Decision

Three actions:

### 1. Re-classify the existing pattern-naming pipeline as v2's emergence kernel

Recognise `discover_motifs` + `refine_candidates` +
`name_pattern_instances` + `autonomous_pass` as the v2 emergence
kernel. The audit confirms it satisfies the constitution's strict
reading: subgraph-canonicalization-derived bucketing,
atomic concept-mint with participating-token registration.

This is a conceptual re-classification, not new code. Update
ADR 0073's framing in subsequent commits / docs to reflect that
the kernel exists; the missing piece is **runtime integration**,
not the kernel itself.

### 2. Promote `DiscoverPatterns` in the rule-based scheduler

The `ActionKind::DiscoverPatterns` exists and the
`AutonomousRuntime` machinery wires it to `autonomous_pass`. What
is missing is for the rule-based scheduler to choose this action
with frequency proportional to the pattern-discovery utility on
the current rset. The integration:

- Increase `DiscoverPatterns`'s default priority so that
  Phase 0 substrates run pattern discovery alongside axiom
  discovery
- Pattern sizes scanned: 2, 3, 4, 5 (per the audit's empirical
  range)
- Use the existing `DriveConfig::default()`'s
  `discovery_config` (sample_count=200, top_m=10) as the runtime
  default

This is a small change to scheduler priorities + driver action
selection; no new ADR needed for the mechanism, this ADR
documents the priority bump.

### 3. Cross-substrate canonical-form comparison (follow-up)

The audit's "shared pattern id" output (e.g., p_2 appearing on
all 4 substrates) is misleading: pattern ids are per-RSet
counters that overlap by accident. To assess substrate
diversity at the pattern level, compare **canonical forms**, not
ids. This is a 1-day follow-up:

- For each substrate's minted pattern, extract its canonical
  form via `instance_subgraph(first_instance).canonicalize()`
- Build a cross-substrate canonical-form intersection /
  difference set
- Conclude whether different substrates produce structurally-
  distinct emergent patterns (expected: yes for OQ#2 vs the
  rest; possibly no among OQ#1 / long5k / narrow_a per the
  RSet-collapse finding)

This follow-up replaces the substrate-diversity probe's verdict
with a pattern-level one.

## What this changes for prior ADRs

- **ADR 0073** — the "system cannot create new concepts"
  diagnosis is corrected: the pattern-naming kernel mints
  concepts compliant with the strict reading; the diagnosis
  applies only to axiom-template invention (the shape grammar
  is still hard-coded), not to concept emergence in general.
  The E1 / E2 / E3 trichotomy is also obsolete — under the
  strict reading the three facets are inseparable, and the
  existing kernel demonstrates this in code.

- **ADR 0074** — concept mining via shape-family co-occurrence
  remains valid as documented but its standing changes: it is
  *implicit conceptualization* (per the constitution's heavy
  reading), useful as a curatorial tool but not a concept-
  creation act, because it does not register participating
  tokens as instances of the minted concept. The audit shows
  the existing pattern-naming kernel is the proper concept-
  creation path; ADR 0074's concept-id is more accurately
  described as a "shape-family co-occurrence label."

- **Substrate-diversity probe finding (Emergence-1)** — the
  "OQ#2 is the blind spot" verdict is corrected: OQ#2 is the
  blind spot of the *axiom path*. The pattern path is most
  active there.

## Alternatives considered

**Alt A: design a new emergence kernel from scratch.** Was the
original plan after the constitution amendment. Rejected after
the audit revealed the existing pipeline is already compliant.
A fresh design would duplicate work.

**Alt B: keep the existing kernel hidden behind manual API.**
Status quo before this ADR. Rejected because the runtime
currently does not exercise the kernel autonomously, leaving the
system functionally without concept emergence in the main loop —
even though the capability exists.

**Alt C: merge axiom discovery with pattern discovery into a
unified action.** Tempting, but axiom discovery produces
template-axiom instances (whose templates are pre-registered) and
pattern discovery produces emergent patterns (whose forms are
mined). They serve different roles; mixing them risks confusing
"emergence" with "instantiation under pre-registered concepts."
Keep them separate, with separate action priorities.

## Consequences

**Now possible:**
- The runtime autonomously mints emergent patterns (and their
  participating-token registrations) during normal operation,
  not just under explicit experiment scripts
- Concept emergence is queryable via the same APIs as ADR 0010 /
  0018 — `rset.patterns()`, `rset.instances_of(p)`,
  `rset.left_of(instance)` for participants
- Cross-substrate emergent-pattern comparison becomes a
  concrete experiment (the canonical-form-comparison follow-up)
- ADR 0072's intervention machinery may eventually reference
  patterns as well as theories — patterns are first-class
  candidates for the same quality / cross-precision validation
  loop already shipped

**Now harder:**
- Pattern-discovery cost. `discover_motifs` with sample_count=200
  per call is cheap, but called every Phase 0 pass it adds
  measurable overhead. The runtime priority bump should default
  to a moderate cadence (e.g., once every 50–100 ticks during
  Phase 0), tuned via experiment.
- Pattern lifecycle. Once minted, patterns persist in the rset
  indefinitely unless retracted. Currently no retraction policy
  exists for emergent patterns analogous to ADR 0072's
  `RetractShapeFamily`. A future ADR may add
  `RetractPattern`-with-rationale.

**Now deferred (until pattern path is observably active in
runtime):**
- ADR 0074's concept-mining lifecycle revisions
- Patterns-vs-shape-families subsumption
- Pattern-aware drive metric (the original "intrinsic drive"
  agenda that the withdrawn 0075 draft attempted; reframed,
  this would now be "drive computed from R uncovered by both
  axioms AND patterns" — but that's still a free-standing
  metric and falls under ADR 0059 unless / until a fresh
  bucket-key design avoids the per-token-signature trap)

## Implementation

This ADR ships in three pieces:

1. **Audit example** (already shipped, see
   `docs/results/phase_emergence_kernel_audit.md`) —
   `examples/phase_emergence_kernel_audit.rs` +
   `logs/2026-05-06_phase_emergence_kernel_audit.log`. Demonstrates
   the kernel works on all 4 canonical substrates.

2. **Scheduler integration** (next commit) — adjust
   `RuleBasedScheduler` so `DiscoverPatterns` is selected
   periodically during Phase 0, not only on explicit invocation.
   Tune cadence so the runtime gains pattern-discovery autonomy
   without paying excessive cost.

3. **Cross-substrate canonical-form comparison** (follow-up
   commit) — replaces the substrate-diversity probe's
   axiom-only verdict with a pattern-level one, producing the
   "real" substrate-diversity finding.

Pieces 2 and 3 may land in successor commits; piece 1 is the
empirical foundation.

## Open questions

- **What pattern size to prefer?** The audit ran sizes 2-5; size
  3 was modal in instances. Default cadence may scan only one
  size per call (rotating) to bound cost.
- **When to prune?** Patterns minted during early Phase 0 may
  later prove low-quality once more axioms / theories
  consolidate. ADR 0072-style `recommend_intervention` extended
  to patterns would resolve this; deferred to a future ADR.
- **Object identity across patterns**. A token can be a
  participant in many pattern-instances; do we eventually want
  a per-token "concept membership index" view? Currently this
  is a query (`rset.right_of(token)`), but might warrant a
  helper.

These are decided in implementation or deferred.
