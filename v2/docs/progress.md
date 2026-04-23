# v2 Progress Log

Chronological record. Append-only (except typo fixes). Each entry dated;
entries link to the ADR that governs the step.

---

## 2026-04-23

### Project restructure
v1 archived under `v1/` at tag `v1.0`. v2 scaffolded at `v2/` with an empty
Rust package: only the `R(x, y)` struct and three invariant tests
(construction, directionality, token-based identity).

Decision: [0001-project-restructure](decisions/0001-project-restructure.md).
Commits: `b8aaa84`, `79541ea`.

### RSet harness
Added `RSet` — a deduplicated R-instance container with observation methods
(`identifiers`, `left_of`, `right_of`). Zero interpretation at this layer.

Decision: [0002-rset-harness](decisions/0002-rset-harness.md).
Commit: `5abdb73`.

### IdentifierProfile (first pass)
Added `profile(id)` returning `(degree_out, degree_in, slots)`. First-pass
answer to the "object emergence" question: structurally salient identifiers
can be found by comparing profiles, but the profile itself makes no salience
judgment. Richer granularity (neighbor sets, self-loop flag, co-occurrence,
multi-hop) documented in the ADR as deferred candidates.

Decision: [0003-identifier-profile](decisions/0003-identifier-profile.md).
Commit: `8886d53`.

### Traceability infrastructure
Added `docs/practices.md`, `docs/decisions/` (with index), `docs/progress.md`,
and `logs/` directory. Backfilled ADRs 0001–0003 to cover the work above.

Commit: `6a1f4f9`.

### Structural signature — first pass
`signature(id)` aliased to `profile(id)`; added `equivalence_classes()` on
`RSet`. Two identifiers are structurally equivalent iff their 0-hop profiles
are equal. 6 new unit tests cover chain / cycle / star / disjoint-union
collapse behavior.

Ran the first v2 experiment (`examples/structural_equivalence.rs`) over
six canonical small graphs. Findings:
- Role classification works as intended — head/middle/tail in chains,
  pivot-vs-leaves in stars, full collapse in cycles.
- Disjoint unions merge equivalent roles across components without extra
  machinery.
- 0-hop is **not** sufficient for naming compound patterns (e.g., "this
  is a chain of three"); that is a separate, later mechanism.

0-hop signatures are adopted as the role-classification layer; no immediate
upgrade to 1-hop needed. Open questions for the next layer (pattern
detection) are listed at the end of the experiment log.

Decision: [0004-signature-is-profile](decisions/0004-signature-is-profile.md).
Log: [logs/2026-04-23_structural_equivalence.log](../logs/2026-04-23_structural_equivalence.log).

Commit: `1437569`.

### R-instance signature — edge-level (first pass)
Lifted the signature machinery one level: `RSignature = (Signature, Signature)`
(ordered endpoint profiles), with `r_signature(&R)` and
`r_equivalence_classes()` on `RSet`. 6 new unit tests plus an
`edge_equivalence.rs` example covering the same six canonical graphs as
the identifier-level demo.

All six ADR-0005 predictions verified. Key findings:
- **First "repetition inside a single graph."** The 5-chain's middle-middle
  edges `R(a2,a3)` and `R(a3,a4)` merge into one class — the first
  single-graph-derived multi-member class not caused by pure symmetry.
  This is the signal a later pattern-mining layer can mine.
- **Direction is preserved.** Bidirectional chain produces three distinct
  classes (out-from-end, in-to-end, middle-middle) because pair order matters.
- **Stars reduce to "one edge type repeated."** Both out-star and in-star
  collapse their spokes; shape of the compound-pattern definition starts
  to come into view.
- **Cycles and stars collide at this layer** — both go to a single class.
  Distinguishing them requires a locality / co-occurrence signal, which
  is the motivation for the next ADR.

Decision: [0005-r-instance-signature](decisions/0005-r-instance-signature.md).
Log: [logs/2026-04-23_edge_equivalence.log](../logs/2026-04-23_edge_equivalence.log).

Commit: `5b8d116`.

### Locality profile (α)
Added `LocalityProfile { co_left, co_right, forward, reverse }` and
`locality_profile(&R)` on `RSet`. Counts of four kinds of 1-hop neighbor
relations: share left endpoint, share right endpoint, forward chain
(this.y == other.x), reverse chain (this.x == other.y). Directional by
design per commitment 2. 6 new unit tests; `examples/locality.rs` covers
the six canonical graphs plus an explicit chain-middle / cycle-edge
collision check.

All ADR 0006 predictions matched. Principal result:

- **Cycle vs star separated.** Cycle edge: `(0,0,1,1)`. Out-star spoke:
  `(2,0,0,0)`. In-star spoke: `(0,2,0,0)`. Three distinct locality
  fingerprints where the edge signature (0005) saw only "one class per graph."
- **Bidirectional chain structure becomes visible.** Endpoint edges and
  interior edges have different locality profiles, separating them
  cleanly where 0005's edge classes only grouped them by direction.
- **Known 1-hop collision locked.** Chain-middle and cycle-edge both have
  `(0,0,1,1)` — deferred, with a test that fails if the collision is ever
  intentionally broken by a 2-hop upgrade.

### Note on γ's dormancy (observation, not decision)
Every mechanism in v2 so far is deterministic derivation from the RSet:
there are no choice points, so self-driven triggering (γ) has nothing to
trigger. γ becomes load-bearing at β (compound pattern naming), where
"which patterns to name" is genuinely a choice. Captured in ADR 0006's
Context to avoid later confusion about why γ was postponed this long.

Decision: [0006-locality-profile](decisions/0006-locality-profile.md).
Log: [logs/2026-04-23_locality.log](../logs/2026-04-23_locality.log).

Commit: `1cbcf5f`.

### Compound signature probe (0007) — observation before β
Added `EdgeFingerprint = (RSignature, LocalityProfile)` and
`edge_fingerprint()` as a small probe utility. Ran on a mixed 14-edge
graph (5-chain + 3-cycle + 3-spoke star + 3-edge tree + 1 isolated edge)
to see what compound classes fall out.

Result: 14 edges partition into 7 compound classes, with sizes
`{5, 3, 2, 1, 1, 1, 1}`.

Key findings:
- **Biggest class (5) is the predicted 1-hop collision** — 2 chain-middle
  edges merged with all 3 cycle edges. Naive "name the biggest class"
  would pick the known false-merge.
- **Genuine cross-structure merge (size 2)** — chain-tail `R(c4,c5)` and
  tree-leaf-edge `R(t2,t4)` share a signature. Structurally both are
  "edge descending into a terminal node"; naming this would abstract
  "terminal descent." Likely a legitimate pattern.
- **Star spokes (size 3)** form a clean same-component class — the most
  textbook repetition.
- **4 of 7 classes are singletons.** Chain head and tree edges don't
  repeat; "pattern from size-1 classes" is not free.

β question (ii) answered empirically: **size > 1 is necessary but not
sufficient**. The biggest candidate is a false merge. A sanity filter is
needed. Two forms identified:
- (a) **2-hop tie-breaker** — cheap, breaks the specific collision.
- (b) **Subgraph coherence check** — closer to the design-notes goal of
  naming whole structures (like "integer chain").

γ also takes concrete shape for the first time: its first real job is
applying this sanity filter.

Decision: [0007-compound-signature-probe](decisions/0007-compound-signature-probe.md).
Log: [logs/2026-04-23_compound_signature.log](../logs/2026-04-23_compound_signature.log).

Commit: `ef6b332`.

### Subgraph extraction (β step 1 — first of 4 planned ADRs)
Added `Subgraph` struct and `Subgraph::connected_components_of` (plain
BFS with identifier-sharing adjacency). Added `compound_class_subgraphs`
on `RSet` for the common lift from compound fingerprint to subgraph
instances.

10 new tests. Applied to the ADR 0007 mixed graph:

- 14 edges → 7 compound classes → **9 subgraph instances**.
- The 5-member false-merge class (chain-middle + cycle) split into 2
  subgraphs: a 3-edge cycle and a 2-edge chain fragment. Sanity filter
  (b) is operational.
- The 2-member "terminal descent" class split into 2 disjoint
  single-edge subgraphs (chain-tail and tree-leaf share no identifiers).
- Star spokes stayed as one 3-edge subgraph — star-ness preserved as a
  unit.
- Singletons stayed as singletons.

β now has its pattern-instance unit: a Subgraph. Next ADR (0009) will
define when two Subgraph values represent the same pattern
(isomorphism via Weisfeiler-Lehman-style refinement, planned).

Decision: [0008-subgraph-extraction](decisions/0008-subgraph-extraction.md).
Log: [logs/2026-04-23_subgraph_extraction.log](../logs/2026-04-23_subgraph_extraction.log).

Commit: `88097a8`.

### Subgraph canonicalization (β step 2 / 4 — ADR 0009)
Added Weisfeiler-Lehman-1 refinement to `Subgraph`: `canonicalize()`
returns a deterministic `CanonicalForm` (sorted edge list over stable
integer labels), `is_isomorphic_to(other)` checks structural equality.
Rank-based labels (no hashing) keep the canonical form reproducible
across processes. 10 new unit tests cover chains, cycles, stars, vees,
direction sensitivity, cross-identifier isomorphism.

Applied to the 9 subgraph instances from ADR 0008. Sequence
**7 → 9 → 4** across the full pipeline:

- **7** compound classes (ADR 0007)
- **→ 9** subgraph instances after connectivity split (ADR 0008; false
  merges open up)
- **→ 4** pattern classes after structural merge (ADR 0009; instances
  with different identifiers collapse)

Pattern classes on the mixed graph:
- **P1** `[(0,0),(0,0),(0,0)]` — 3-cycle; 1 instance
- **P2** `[(1,0)]` — single edge; **6 instances across 5 compound classes**
  (chain head, chain tail, tree leaf edge, tree branches, isolated edge)
  — first empirical cross-compound-class pattern
- **P3** `[(1,0),(1,0),(1,0)]` — 3-spoke out-star; 1 instance
- **P4** `[(1,2),(2,0)]` — 2-edge forward chain; 1 instance

**Key finding for ADR 0010:** canonical-form identity is necessary but
too coarse on its own — P2 "single edge" pattern has 6 instances which
would dominate any naive naming rule. The log argues for splitting the
concerns: ADR 0010 establishes structural pattern identity; ADR 0011
(γ) decides *which* structural patterns are worth naming (threshold /
MDL / non-triviality filter).

Compound class C3 (the known 1-hop false merge) correctly maps to TWO
different pattern classes — P1 (cycle) and P4 (chain fragment) —
confirming the full pipeline rejects the false merge.

Decision: [0009-subgraph-canonicalization](decisions/0009-subgraph-canonicalization.md).
Log: [logs/2026-04-23_canonicalization.log](../logs/2026-04-23_canonicalization.log).

Commit: `0646dc8`.

### Pattern naming as meta-R instances (β step 3 / 4 — ADR 0010)
Added `PATTERN_MARKER = "__pattern__"`, `PatternError`, and five
methods on `RSet`: `name_pattern_instances`, `patterns`,
`instances_of`, `participants_of`, `find_pattern_matching`. Encoding
is the three-shape convention: `R(PATTERN_MARKER, p)` registers a
pattern, `R(p, inst)` owns an instance, `R(inst, participant)` lists
participants. Canonical form is intentionally not stored — recovered
on demand via `Subgraph::from_edges` + canonicalize. 8 new unit tests
(empty list, empty subgraph, non-isomorphic reject, dedup, collision
guard, canonical round-trip, shared participant across patterns, and
the 6-instance single-edge case from ADR 0009's P2).

Ran the full pipeline (0007 → 0008 → 0009 → 0010) on the mixed graph:

- 14 original R instances → 49 after naming (35 meta-R added).
- 4 canonical-form groups → 4 named patterns: p_0 (3-cycle, 1 inst),
  **p_1 (single edge, 6 insts)**, p_2 (3-spoke star, 1 inst),
  p_3 (2-edge chain, 1 inst).
- p_1 is the P2 cross-compound-class pattern made durable: it lists
  six instances whose participants span five original compound classes.
- Every R-instance addition matches the oracle count
  (4 registry + 9 ownership + 22 participant = 35).

**Key observation for ADR 0011 (γ):** p_1 being dominant is exactly
the case that motivates γ. Naming every canonical class surfaces
trivial patterns ("just one edge") alongside substantial ones. "Which
patterns deserve naming" needs a policy — the γ layer's first job.

The feedback loop is now live but not exploited: compound classes
computed on the post-naming RSet would include `__pattern__`, `p_N`,
and `p_N_i_M` nodes as structural neighbors, potentially enabling
patterns-of-patterns discovery. Re-running is γ's choice.

Decision: [0010-pattern-naming-as-meta-r](decisions/0010-pattern-naming-as-meta-r.md).
Log: [logs/2026-04-23_pattern_naming.log](../logs/2026-04-23_pattern_naming.log).
