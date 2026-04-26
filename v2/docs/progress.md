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

Commit: `64c0ace`.

### Meta-R feedback probe (ADR 0011 — probe before γ)
Observation-only ADR. No `lib.rs` changes. Ran
`compound_class_subgraphs` on the canonical mixed graph before and
after ADR 0010 names all four canonical groups. (γ is now ADR 0012
after this probe was inserted.)

Results:
- Baseline: 7 compound classes / 9 subgraph instances / 14 edges.
- Post-naming: **22 compound classes / 31 subgraph instances / 49
  edges.** 3× class growth, 3.5× edge growth.
- Class kinds split 10 data-only / 5 meta-only / 7 mixed.

Main finding: the feedback loop **does** produce novel structure,
but most of it is **predictable from the encoding convention**:
- `R(__pattern__, p_N)` forms a 4-spoke out-star.
- `R(p_N, p_N_i_M)` ownership trees are out-stars per pattern.
- `R(p_N_i_M, participant)` edges are participant-fan-outs per
  instance.

These are artifacts of the three-shape encoding (ADR 0010). Naming
them would re-discover the encoding itself — circular.

One genuinely new structural form surfaces: data patterns like
3-cycle become 3-spoke stars when re-encoded as
instance-to-participants trees. Still encoding-derivative, but
worth noting.

**Recommendation for ADR 0012 (γ):** default no iteration on the
enlarged RSet. If iteration is enabled, include an "artifact filter"
that excludes k-spoke stars generated by the encoding. γ's primary
job is the relevance filter for naming — suppressing ADR 0009's P2
"single edge" trivial pattern — not iteration.

Decision: [0011-meta-r-feedback-probe](decisions/0011-meta-r-feedback-probe.md).
Log: [logs/2026-04-23_meta_feedback_probe.log](../logs/2026-04-23_meta_feedback_probe.log).

Commit: `a6e6326`.

### γ naming-pass driver and relevance filter (β step 4 / 4 — ADR 0012)
Closes the β layer. Added `NamingPolicy { min_edges, min_instances,
skip_meta_subgraphs }`, `SkipReason` (with `BelowMinEdges`,
`BelowMinInstances`, `AlreadyKnown`), `NamingDecision`, and two new
methods on `RSet`: `consider_naming` (one-shot policy wrapper) and
`run_naming_pass` (the γ driver). Private helpers
`filter_known_instances` and `collect_meta_ids`. Default policy
`min_edges=2, min_instances=1, skip_meta_subgraphs=true` suppresses
ADR 0009's trivial single-edge P2 pattern, allows singleton
instances, and keeps iteration dormant.

6 new unit tests — default suppresses single edge, lowering
min_edges allows it, min_instances threshold, mixed-graph full pass
names 3 / skips 1, tighter policy names 0, and idempotence under
default via participant-set dedup.

On the mixed graph:
- Default: 3 patterns named (cycle, star, chain), 1 skipped (single
  edge). 14 → 30 edges (vs 14 → 49 in ADR 0010's unfiltered naming).
- Second pass idempotent: 3 AlreadyKnown + 1 BelowMinEdges, zero
  new entries.
- Tighter policy (`min_instances=2`): 0 patterns named.
- Permissive policy (`min_edges=1`): 4 patterns named, matches ADR
  0010's baseline.

**β layer now complete** — extraction (0008), canonicalization
(0009), naming (0010), and γ (0012) together implement the
minimum-viable autonomous-abstraction path from the design notes.
Full sequence:
  RSet → compound_class_subgraphs (0007) →
  connected_components_of (0008) → canonicalize (0009) →
  run_naming_pass (0012 driving 0010).

Known open directions (post-β, not next-ADR material):
- MDL-based relevance scoring (alternative to min_edges / min_instances).
- Automatic trigger (γ on every add, rather than explicit).
- Cross-graph patterns.
- Pattern retraction.
- Downstream exploitation — consuming named patterns to suggest
  attachments, answer structural queries, etc.

Decision: [0012-gamma-naming-pass](decisions/0012-gamma-naming-pass.md).
Log: [logs/2026-04-23_gamma_naming_pass.log](../logs/2026-04-23_gamma_naming_pass.log).

Commit: `da22182`.

### Pattern query API — first use of named meta-R (ADR 0013)
Added four `&self` query methods on `RSet`: `classify_subgraph`,
`pattern_of`, `memberships_of`, `instance_subgraph`. All are thin
compositions over ADR 0010 / 0009 primitives — no new state, no
new ontological commitments. 5 new unit tests; 77 total.

Demonstrates the first concrete use of named meta-R: classifying
fresh structure against the registry.

From the example run on the default-γ mixed graph (3 patterns
named: p_0 3-cycle, p_1 3-star, p_2 2-chain):

- `classify_subgraph({m1→m2, m2→m3, m3→m1})` → **p_0**  (fresh
  cycle on unseen identifiers matches the named 3-cycle pattern)
- `classify_subgraph({u→v, v→w})` → **p_2**  (fresh chain on
  unseen identifiers matches the named 2-chain)
- `classify_subgraph({h→a, h→b})` → **None**  (2-spoke star is
  not a named pattern)
- `memberships_of("c3")` → **[(p_2, p_2_i_0)]**
- `pattern_of("p_0_i_0")` → **p_0**; `pattern_of("k1")` → **None**

This is the first step where meta-R is actively read, not just
recorded — the "meta-R pays rent" transition.

Decision: [0013-pattern-query-api](decisions/0013-pattern-query-api.md).
Log: [logs/2026-04-23_pattern_queries.log](../logs/2026-04-23_pattern_queries.log).

Commit: `e9ff017`.

### Attach-only mode for naming pass (ADR 0014)
Added `attach_only: bool` to `NamingPolicy` (default false) and a new
`SkipReason::NoMatchingPattern` variant. When `attach_only = true`,
`run_naming_pass` rejects candidate groups whose canonical form
doesn't match any existing named pattern, preserving the registry
as a stable artifact. 3 new unit tests; 80 total pass.

Two-phase workflow demonstrated in the example:
1. Discovery pass (default policy) names p_0 cycle, p_1 star, p_2 chain.
2. New data added: fresh 3-cycle {m1,m2,m3}, fresh 2-chain {u,v,w},
   novel T-fork {q1,q2,q3,q4}.
3. Attach-only pass → the fresh 3-cycle attaches to p_0 as a second
   instance; nothing else is named.

**Surfaced limitation — compound-class fragmentation.** The fresh
2-chain `{u, v, w}` did not attach to p_2 even though it is
structurally isomorphic. Root cause: `compound_class_subgraphs`
groups edges by compound fingerprint, then extracts connected
components per group. An asymmetric structure (like a 2-chain)
whose edges have *different* compound fingerprints fragments across
compound classes and cannot be reassembled as a single subgraph.

- Cyclic / symmetric structures (cycles, stars) survive: every edge
  has the same endpoint-profile pair, all land in one compound class.
- Asymmetric structures (chains, trees) can fragment if the new
  data's identifiers don't share profiles with already-named
  participants.

This is not a bug in 0014 but a pipeline-scope limit. It was
implicit in the discovery pipeline too; 0014's attach experiment
just constructed a case that makes it visible.

Next ADR (0015, provisionally): introduce subgraph *matching* —
enumerate connected subgraphs of a named pattern's size, canonicalize
each, compare — to reach the asymmetric structures that
fragmentation misses.

Decision: [0014-attach-only-mode](decisions/0014-attach-only-mode.md).
Log: [logs/2026-04-23_attach_only.log](../logs/2026-04-23_attach_only.log).

Commit: `e273354`.

### Subgraph matching against named patterns (ADR 0015)
Replaces compound-class enumeration with direct subgraph matching
in the attach pass. New `RSet::find_instances_of(&CanonicalForm)
-> Vec<Subgraph>` enumerates connected data subgraphs of the
matching size and canonical form, with a cleanness filter:
participants must induce exactly `k` data edges in the RSet (reject
embedded cases). `run_naming_pass` branches on `attach_only`:
discovery uses compound-class enumeration (unchanged); attach uses
per-pattern subgraph matching. Removed
`SkipReason::NoMatchingPattern` (no producer under new semantics).

User's point — one reproducible failure (the ADR 0014 asymmetric
2-chain) is sufficient evidence that the pipeline is wrong for this
purpose — drove the priority.

**Results on the mixed graph (subgraph_matching example):**
- Phase 2: attach on original data. p_2 1 → 4 instances
  (`{c2,c3,c4}` + `{c1,c2,c3}` + `{c3,c4,c5}` + `{t1,t2,t4}`).
  Discovery had found only the interior fragment; attach surfaces
  the overlaps and the tree branch.
- Phase 4: after adding `{u,v,w}` chain + T-fork + new 3-cycle.
  p_0 1 → 2; p_2 4 → 6 (picks up `{u,v,w}` and the T-fork's
  `{q1,q3,q4}` 2-chain).
- Phase 5: second attach is a no-op. Truly idempotent.

Cleanness filter is load-bearing: without it, the cycle's three
consecutive edge pairs all canonicalize to 2-chain and would be
recorded as three p_2 instances on the same participants
`{k1,k2,k3}`, breaking ADR 0010's canonical-recovery invariant.
With it, only instances whose participants induce exactly `k`
edges are admitted.

Two enumeration primitives now coexist cleanly:
- `compound_class_subgraphs` (ADR 0007): discovery heuristic.
- `find_instances_of` (ADR 0015): verification primitive.

Known residual: discovery itself (ADR 0007/0008) still fragments
asymmetric structures. Naming novel asymmetric structures requires
subgraph-*motif* discovery (enumerate connected subgraphs at sizes
k, group by canonical, name by frequency) — candidate for a
future probe. 83 tests pass.

Decision: [0015-subgraph-matching](decisions/0015-subgraph-matching.md).
Log: [logs/2026-04-23_subgraph_matching.log](../logs/2026-04-23_subgraph_matching.log).

Commit: `6148efb`.

### Motif discovery via sample-score-select (ADR 0016)
First non-enumeration search mechanism in v2, per the
`v2_search_mode` memory principle. Added `DiscoveryConfig`,
`MotifCandidate`, and `RSet::discover_motifs`. Random-walk
sampling from data edges (inline xorshift64 PRNG for determinism,
no external crate), scoring by canonical-form frequency among the
sample, top-M selection.

Results on the canonical mixed graph at size 3 (200 samples):
- 3-chain canonical `[(1,3),(2,0),(3,2)]`: 70 samples (35%)
- 3-star `[(1,0),(1,0),(1,0)]`: 42 samples (21%)
- **3-tree-branch `[(2,0),(3,1),(3,2)]`: 37 samples (18.5%)** —
  the asymmetric motif ADR 0015 flagged as unreachable by
  compound-class discovery
- 3-cycle `[(0,0),(0,0),(0,0)]`: 34 samples (17%)

The asymmetric tree motif surfaces naturally through random-walk
sampling — propose-score-select reaches shapes that the
deterministic grouping pipeline fragments. 5 new unit tests;
88 total pass.

Kept explicitly deferred: refinement step (ADR 0017 territory if
needed), MDL scoring, automatic motif-to-pattern naming
pipeline.

Known caveat: motif representatives may be non-clean (e.g., the
2-chain representative at size=2 was `{R(k1,k2), R(k3,k1)}` which
is embedded in the 3-cycle). Discovery reports structural
recurrence; cleanness verification remains `find_instances_of`'s
job, by design — motif ≠ clean instance.

Decision: [0016-motif-discovery-via-sampling](decisions/0016-motif-discovery-via-sampling.md).
Log: [logs/2026-04-23_motif_discovery.log](../logs/2026-04-23_motif_discovery.log).

Commit: `3a702af`.

### Representative refinement (ADR 0017)
Added `RefinementConfig`, `RSet::refine_candidates`, and the public
helper `RSet::is_clean_subgraph`. Refinement strategy: for each
motif candidate whose representative is embedded (not clean), do
targeted re-sampling within a tries-budget and accept the first
clean alternative with the same canonical form. Re-sampling rather
than edge-swap hill-climb because single-edge swaps cannot escape
tight structural neighborhoods (e.g., 2-chains inside cycles).

Also **fixed a determinism bug**: `discover_motifs`,
`find_instances_of`, and `refine_candidates` now use
`data_edges_sorted` (lexicographic by (x, y)) instead of
HashSet-iteration order. Before: same seed + same RSet produced
different output across process runs. After: cross-run
reproducible. The pipeline's promised determinism now holds in
full.

4 new unit tests; 92 total pass. Explicit demo in
`motif_refinement.rs` constructs a non-clean 2-chain (embedded in
the 3-cycle `{k1,k2,k3}`) and refines it to a clean 2-chain in
the 5-chain data `{c2,c3,c4}`.

Sample-score-refine is now complete:
  `discover_motifs` → propose + score + select
  `refine_candidates` → polish representatives
  (next in 0018) → name as pattern when clean + novel

Decision: [0017-representative-refinement](decisions/0017-representative-refinement.md).
Log: [logs/2026-04-23_motif_refinement.log](../logs/2026-04-23_motif_refinement.log).

Commit: `766817e`.

### Autonomous pass — abstraction loop closes (ADR 0018)
Composes `discover_motifs` → `refine_candidates` → `find_instances_of`
→ `name_pattern_instances` into a single entry point.
`AutonomousConfig` bundles the three sub-configs. Per-candidate
`AutonomousOutcome` is one of: `NewPattern`, `Existing`, `Skipped
{ NoCleanInstance | PolicyFiltered }`. 4 new unit tests; 96 total pass.

**Key result — first full autonomous run on the mixed graph:**

```
Pass 1 (fresh RSet, target_size=3, 200 samples):
  NewPattern p_0  3-chain   [(1,3),(2,0),(3,2)]   2 instances
  NewPattern p_1  3-cycle   [(0,0),(0,0),(0,0)]   1 instance
  NewPattern p_2  3-tree    [(2,0),(3,1),(3,2)]   1 instance  <-- asymmetric
  NewPattern p_3  3-star    [(1,0),(1,0),(1,0)]   1 instance

  14 data edges + 28 meta-R added = 42 total.

Pass 2 (same config): 4 × Existing, zero deltas. Idempotent.
```

The asymmetric 3-tree — flagged by ADR 0015 as unreachable by
compound-class discovery — is named autonomously. The autonomous
loop is not just "equivalent to" deterministic discovery; it
*extends* what discovery can reach, in the direction
design-notes asked for.

The system:
1. Sampled from its own data (random walks).
2. Scored candidates by their own canonical-form frequency.
3. Refined representatives against its own cleanness criterion.
4. Named the novel canonicals as meta-R instances.

All five commitments hold: only R; direction preserved; types as
meta-R instances; token-based identity; structural similarity.

**Autonomous abstraction loop is operational at a minimum-viable
scale.** Remaining open directions (multi-size passes, attach-only
integration, MDL scoring, cross-graph transfer, pattern retraction)
are refinements rather than completions.

Decision: [0018-autonomous-pass](decisions/0018-autonomous-pass.md).
Log: [logs/2026-04-23_autonomous_pass.log](../logs/2026-04-23_autonomous_pass.log).

Commit: `93fef37`.

### MDL-gain scoring (ADR 0019)
Opt-in reusability filter for naming. Added
`RSet::mdl_gain(canonical) → usize` computing `(N − 1) × k` from
`find_instances_of(canonical)`, `RSet::score_by_mdl` for
re-ranking candidates by MDL, `NamingPolicy::min_mdl_gain: usize`
(default 0 = off), `SkipReason::BelowMdlGain { gain, min }`, and
an MDL check branch in `consider_naming`. Integer arithmetic
throughout — `(N − 1) × k` is always a non-negative integer, so
Eq-derivability on `SkipReason` is preserved.

Key comparison on the mixed graph at target_size=3 (200 samples):

  canonical                      sample_freq    mdl_gain
  3-chain  [(1,3),(2,0),(3,2)]         67          3
  3-cycle  [(0,0),(0,0),(0,0)]         51          0
  3-tree   [(2,0),(3,1),(3,2)]         44          0
  3-star   [(1,0),(1,0),(1,0)]         27          0

Frequency scores how often a structure is sampled; MDL scores how
often it truly appears in a reusable form. Singletons have sample
frequency but zero MDL gain.

`autonomous_pass` with `min_mdl_gain=1` names only the 3-chain
(the sole canonical with N≥2 clean instances on this graph).
The three singleton canonicals are filtered with
`Policy(BelowMdlGain 0<1)`. With `min_mdl_gain=0` (default), all
4 still get named — backward-compatible.

5 new unit tests; 101 total pass.

Interpretation: in v2's encoding, naming adds meta-R rather than
removing data, so strict byte-count MDL would always recommend
naming nothing. `(N − 1) × k` is an MDL-inspired *reusability*
proxy — the description saving that downstream callers would
realize. Documented as such in the ADR.

Decision: [0019-mdl-scoring](decisions/0019-mdl-scoring.md).
Log: [logs/2026-04-23_mdl_scoring.log](../logs/2026-04-23_mdl_scoring.log).

Commit: `cfea4cf`.

### Pattern retraction (ADR 0020)
The registry is now bidirectional. Added `RSet::remove` (single-edge
removal, dual of `add`), `RetractionError`, `RetractionSummary`, and
`RSet::retract_pattern` which removes exactly the meta-R edges
belonging to a named pattern (registry entry + ownership edges +
participant edges) and leaves data edges untouched. 5 new unit tests;
106 total pass.

On the canonical mixed graph (autonomous_pass already named 4
patterns, RSet size 42):

- `retract_pattern("p_1")` (the 3-cycle) removed 5 meta-R edges
  (1 instance × 3 participants + 1 ownership + 1 registry).
  RSet: 42 → 37.
- All 5 sampled data edges still present afterward.
- Unknown pattern errors with `RetractionError::UnknownPattern`.
- Re-running autonomous_pass after retraction re-discovers the
  3-cycle and mints it as p_4 (not p_1 — mint_pattern_id walks
  forward from current count rather than filling gaps). The
  retracted slot stays vacant. ID gaps are acceptable because
  pattern identifiers are opaque tokens.
- `find_pattern_matching` returns None for the retracted
  canonical, which is the correctness guarantee we need.

Enables experimentation loops: try a naming, inspect, roll back.
Does not yet support cascading retraction or soft delete; those
are future concerns if they surface.

Decision: [0020-pattern-retraction](decisions/0020-pattern-retraction.md).
Log: [logs/2026-04-23_pattern_retraction.log](../logs/2026-04-23_pattern_retraction.log).

Commit: `8b0809c`.

### Multi-size autonomous sweep (ADR 0021)
`RSet::autonomous_sweep(base, sizes)` runs `autonomous_pass` once
per target size, each with `target_size` overridden and `rng_seed`
offset by the size so sizes sample independently. Later sizes see
earlier-named patterns in the registry, producing `Existing`
outcomes for already-registered canonicals. 3 new unit tests;
109 total pass.

Sweep over `[2, 3, 4]` on the mixed graph produced 7 distinct
patterns:
  size 2: 2-chain (4 inst), 2-star (4 inst)
  size 3: 3-chain (2), 3-tree (1), 3-star (1), 3-cycle (1)
  size 4: 4-chain (1, the whole 5-node chain as a 4-edge motif)
RSet 14 → 83 edges. Second sweep on same sizes: zero new
patterns, idempotent.

Decision: [0021-autonomous-sweep](decisions/0021-autonomous-sweep.md).
Log: [logs/2026-04-23_autonomous_sweep.log](../logs/2026-04-23_autonomous_sweep.log).

Commit: `faa791c`.

### Autonomous + attach composition (ADR 0022)
`RSet::autonomous_and_attach` — runs `autonomous_pass` then
`run_naming_pass(attach_only=true)`, returning both outputs.
Natural incremental-data workflow: autonomous handles novel
canonicals; attach handles new instances of pre-existing ones.
Autonomous first is strictly inclusive. 3 new tests; 112 total.

Incremental demo: prime with autonomous_pass (4 patterns on mixed
graph), add `{q1→q2→q3→q4}` (new 3-chain instance), run
autonomous_and_attach → attach phase adds 1 new p_0 instance
(3-chain count 2 → 3). Autonomous phase reports `Existing` for the
3-chain canonical since p_0 already exists.

Decision: [0022-autonomous-and-attach](decisions/0022-autonomous-and-attach.md).
Log: [logs/2026-04-23_autonomous_and_attach.log](../logs/2026-04-23_autonomous_and_attach.log).

Commit: `795262e`.

### Cross-graph pattern transfer (ADR 0023)
Patterns become portable. `RSet::canonical_library()` extracts all
named canonicals as an identifier-free `Vec<CanonicalForm>`.
`RSet::attach_canonicals(library, policy)` applies them to any
target RSet — named if the target contains them, skipped with
`NoCleanInstance` if not. Same `AutonomousOutcome` enum as
`autonomous_pass`. 4 new tests; 116 total pass.

Demonstrated with graph A (mixed, 4 patterns) → graph B (two chains
+ a cycle, no shared identifiers). B receives 2 patterns (cycle,
3-chain) with correct instance counts for its own data; star and
tree canonicals skip with `NoCleanInstance` because B has neither.

Decision: [0023-cross-graph-transfer](decisions/0023-cross-graph-transfer.md).
Log: [logs/2026-04-23_cross_graph_transfer.log](../logs/2026-04-23_cross_graph_transfer.log).

### Sampling-based `sample_instances_of` (ADR 0024)
Philosophical-alignment companion to `find_instances_of`. Uses
`sample_connected_subgraph` random walks, filters to canonical-
matched clean subgraphs, dedups by participant set. Never
over-returns; may under-return. Deterministic under fixed seed.
`find_instances_of` unchanged — callers needing exhaustiveness
(attach, transfer, MDL) keep it. 4 new tests; 120 total pass.

On the mixed graph, at N=50 the sampled count equals exhaustive
for all six target canonicals (2-chain, 2-star, 3-chain, 3-cycle,
3-star, 3-tree). Small graphs saturate quickly; large-graph
demonstrations of under-counting not needed for β.

Decision: [0024-sample-instances](decisions/0024-sample-instances.md).
Log: [logs/2026-04-23_sample_instances.log](../logs/2026-04-23_sample_instances.log).

### Hierarchical discovery probe (ADR 0025)
Adds `DiscoveryConfig::include_meta_in_discovery: bool` (default
false). When true, `discover_motifs` samples from data + meta-R
instead of data only. 2 new tests; 122 total pass.

**Verdict: negative.** Probe on the post-autonomous mixed graph:
baseline (meta excluded) → 4 candidates, all data-level;
probe (meta included) → 9 candidates, 8 of 9 touch meta. Those 8
are predictable encoding artifacts (stars and trees in the
pattern-registry hierarchy — ADR 0011 already documented these).
The data-only 3-cycle drops from freq 99 to 5 under dilution.

A real hierarchical-pattern mechanism needs richer canonicals
(pattern-id labels, not just integer node labels), resolving
matching, and scoring that rewards true composition over encoding
repetition. This is out of scope for a "simple lifting" ADR.
Closing the direction empirically.

Decision: [0025-hierarchical-discovery-probe](decisions/0025-hierarchical-discovery-probe.md).
Log: [logs/2026-04-23_hierarchical_probe.log](../logs/2026-04-23_hierarchical_probe.log).

### Gradient-descent refinement probe (ADR 0026)
Three passes of investigation:

1. **Initial probe (negative).** Single-start gradient descent from
   the current representative stuck in a local minimum across 4
   config variants. Concluded "don't promote."
2. **Follow-up probes (revised: usable).** Added
   `gradient_refine_from_uniform` (stuck) and
   `gradient_refine_multistart` (finds clean chain at 30+ random
   starts). Revised verdict: usable with budget, just ~45× more
   expensive than random re-sample (ADR 0017) on β-scale graphs.
   Methodological lesson saved to memory:
   `v2_probe_methodology.md` — run multi-start variants before
   declaring a technique dead.
3. **Value assessment & removal.** User asked "what did this bring"
   — honest answer: methodological value is real (saved to memory),
   code value on β-scale graphs is zero. Under minimum-first,
   removed the implementation. `GradientRefineConfig`, the four
   `gradient_refine_*` methods, the sigmoid / gradient helpers,
   the example, and the five unit tests all deleted. Test count
   127 → 122.

What persists:
- ADR 0026 (three-phase history inside).
- Experiment log with all observations.
- Memory entry `v2_probe_methodology.md`.
- Git commit `4fc8b67` contains the working reference
  implementation if anyone revisits.

Decision: [0026-gradient-refine-probe](decisions/0026-gradient-refine-probe.md).
Log: [logs/2026-04-23_gradient_refine.log](../logs/2026-04-23_gradient_refine.log).

### Axiom discovery probe: extensional → intensional (ADR 0027)
User pointed out that v2's pattern machinery is extensional
(finite motifs) and asked whether it can construct a poset
(intensional axiom-level concept). Chose option C: system
discovers axioms from data.

Added:
- `AxiomTemplate { num_vars, premise: Vec<EdgeTemplate>, conclusion }`
- `AxiomEvidence`, `AxiomDiscoveryConfig`
- `ReflexivityEvidence`, `AntisymmetryEvidence`, `PosetCheck`
- `RSet::discover_axioms(config)` — enumerate template space
  (max_vars=3, max_premise=2 by default), canonicalize variables,
  evaluate each against data, return strict-rate-1.0 survivors
- `RSet::check_reflexivity()`, `check_antisymmetry()`, `check_poset()`
- 6 unit tests; 128 total pass

**Verdict: POSITIVE.** On a diamond poset:
- Transitivity discovered as template `[R(0,1), R(1,2)] → R(0,2)`
  with 16 bindings at rate 1.0 ← canonical partial-order axiom,
  recovered entirely from edge observations
- `check_poset()` returns `is_poset=true` with full evidence
- On a raw chain: 0 axioms + `is_poset=false` (correct)
- On a symmetric graph: symmetry discovered as `[R(0,1)] → R(1,0)`

Caveat: 25 axioms on diamond, most are consequences of universal
reflexivity. Subsumption / minimization is an obvious next step
but out of scope.

Architecture note: axioms are Rust values, NOT meta-R instances.
Encoding rules-with-variables in R is a bigger design question
deferred to a future ADR.

v2 now has a **first-order axiomatic inference primitive** on top
of the extensional machinery. Posets, equivalence relations,
symmetric relations can be detected as properties, not just as
collections of motifs.

Decision: [0027-axiom-discovery-probe](decisions/0027-axiom-discovery-probe.md).
Log: [logs/2026-04-24_axiom_discovery.log](../logs/2026-04-24_axiom_discovery.log).

### Axiom discovery: rigorous blind test (ADR 0027 follow-up)
User asked for a harsher test of the discovery mechanism without
leaking any test information into the library. Added
`examples/axiom_rigorous_test.rs` — 8 mathematically well-defined
cases (transitive chain, equivalence, strict partial order,
almost-transitive with one broken binding, random sparse, tolerance,
total order, complete bipartite). Library code unchanged.

Result: every discoverable axiom (within the ≤2-edge-premise / ≤3-var
positive-implication template form) was found on inputs where it
holds and absent on inputs where it doesn't. No false positives on
the broken-transitive, random, or bipartite cases. Honest caveat
recorded in the log: high counts on cases where reflexivity holds
universally (45 / 37 / 25) — pointed at subsumption as the immediate
next fix.

Log: [logs/2026-04-24_axiom_rigorous.log](../logs/2026-04-24_axiom_rigorous.log).

### Axiom subsumption (ADR 0028)
User's response to the rigorous log: flagged that "no false positives"
is a bounded claim (inside the template space, not over all axioms),
that the discovery is still axiom-level not theory-level, and that
the noise is not cosmetic — it's an upper-layer blocker for concept
synthesis. Subsumption is therefore a precondition for further work.

Three mechanisms added on top of ADR 0027:

1. **Structural template canonicalization.** Previous canonicalizer
   only normalized by first-use variable order (invariant under
   renaming but not permutation). New canonicalizer picks the
   lex-smallest form over all variable permutations (≤ 24 for
   max_vars=4). Collapses transitivity's two 0027 variants to one.
2. **Subsumption by universal reflexivity.** When every data
   identifier has a self-loop, axioms with conclusion R(v, v) are
   entailed by reflexivity alone and dropped.
3. **Subsumption by premise weakening.** If axiom A has a premise
   that's a subset of B's under some variable mapping preserving
   the conclusion, then A is strictly stronger and B is dropped.

Composed as `RSet::discover_axioms_minimal(config)`. Raw
`discover_axioms` unchanged.

Effect on the rigorous 8-case battery:

| Case                | raw(0027) | minimal(0028) |
|---------------------|----------:|--------------:|
| transitive chain    |  2        |  1            |
| equivalence         | 45        |  5            |
| strict partial order|  2        |  1            |
| broken transitive   |  0        |  0            |
| random              |  0        |  0            |
| tolerance           | 37        |  1            |
| total order         | 25        |  1            |
| bipartite           |  0        |  0            |

Cases 1, 3, 6, 7 reduce to exactly one canonical axiom. Case 2
keeps 5 (symmetry + 4 independent transitivity variants under
equivalence — compositional subsumption would need a theorem prover,
deliberately out of scope).

Tests: 128 → 134 (6 new tests for canonicalizer collapse, reflexivity
subsumption, premise weakening, and per-case minimal output).

Decision: [0028-axiom-subsumption](decisions/0028-axiom-subsumption.md).
Log: [logs/2026-04-24_axiom_subsumption.log](../logs/2026-04-24_axiom_subsumption.log).

### Intension vs extension split (ADR 0029)
Audit triggered by the "property relations, not fact relations"
feedback. Found that ADR 0010's three-shape encoding was dominated
by extension (N instance registrations + N·k participant bindings),
with the type's intension implicit (canonical form recovered at
query time from the first instance). Commitment 3 ("types are
meta-R") was therefore only half-expressed in the code.

Added Layer A — pattern intension — always written on first mint:
- `R(__pattern__, p_N)` — registry (existing)
- `R(__role__, p_N_role_i)` × k — role registry (new `__role__` marker)
- `R(p_N, p_N_role_i)` × k — pattern owns its roles
- `R(p_N_role_i, p_N_role_j)` × e — structural edges over roles,
  using the first instance's sorted-id-index mapping (preserves
  multiplicity; canonical form of the role-subgraph equals the
  pattern's canonical form)

Added `PatternRecordingPolicy` enum:
- `Intensional` — Layer A only; no instance records
- `InstancesOnly` — Layer A + `R(p_N, p_N_i_M)` per instance
- `FullBindings` — Layer A + instances + participant edges
  (ADR 0010 legacy; default)

Added queries: `roles()`, `pattern_roles(p)`, `pattern_structure(p)
-> Option<CanonicalForm>`, `is_role(id)`. `find_pattern_matching`
reads Layer A first, falls back to 0010's first-instance recovery
for legacy RSets. `retract_pattern` tears down all four layer-A
edge families. `instances_of` and `memberships_of` now filter role
ids (they share the `R(p, *)` shape).

Meta-R cost on the ADR 0007 mixed graph (4 types, 9 instances, 22
total participants):
- Intensional: 37 edges (Layer A only)
- InstancesOnly: 46 edges
- FullBindings: 68 edges

Layer A is constant per-type (≈ 2k + e + 1 per pattern). Scales
much better than 0010 when the same type is revisited many times:
same 4 types with 1000 instances each = 37 vs ~16k.

Constitution updated with a clarifying footnote on commitment 3
(intension = meta-R, extension = instrumentation policy).
Commitment text itself unchanged.

Tests: 134 → 143 (9 new covering Layer A writes, each policy mode,
legacy fallback, retraction, meta-id collection).

Decision: [0029-intension-extension-split](decisions/0029-intension-extension-split.md).
Log: [logs/2026-04-24_intension_extension.log](../logs/2026-04-24_intension_extension.log).

### Theory objects: conjunctive concept naming (ADR 0030)
The four-phase plan (A → C → B → D) starts here. User's A: bundle
multiple axioms into a single named theory object in meta-R, so the
system can hold "this is an equivalence-shaped relation" as one
entity rather than as a list of fragments.

Added reserved markers `__axiom__` and `__theory__`, stable
predicate ids `ax_reflexivity` and `ax_antisymmetry`, deterministic
template-axiom ids via `axiom_template_id` / `axiom_id_to_template`.
New API:
- `discover_theory(config) -> Theory` — runs
  `discover_axioms_minimal` + reflexivity/antisymmetry predicates,
  packages everything as a Theory struct
- `name_theory(&[axiom_ids]) -> Result<String, TheoryError>` —
  verifies every member still holds on the current RSet, writes
  `R(__theory__, t_N)` + `R(t_N, ax_i)` membership, reuses existing
  theory id when member set matches
- `retract_theory`, `theories`, `theory_axioms`,
  `theories_containing`, `axioms`, `is_axiom`, `is_theory`
- `collect_meta_ids` extended to include axiom/theory markers + ids

Theory fingerprints on the 8-case rigorous battery:
- transitive chain: {trans, antisym}
- equivalence: {sym, refl, trans, 3 trans-variants} (6 members)
- strict partial order: {trans, antisym} — same fingerprint as chain
- broken transitive: {antisym} — transitivity correctly absent
- random sparse: {antisym} — vacuously antisymmetric
- tolerance: {sym, refl} — transitivity correctly absent
- total order: {trans, refl, antisym} — poset fingerprint
- bipartite: {antisym} — vacuously antisymmetric

Identical theories reuse the same `t_N` via member-set equality
(structural identity, not label-based). Axiom intension (premise /
conclusion structure of the axiom itself as meta-R) is deferred to
B. Concept library injection rejected as off-philosophy (commitment
5: no external labels).

Tests: 143 → 155 (12 new). Commitment 3 extends one more step —
theories' *membership* is now materialized in meta-R, though
individual axioms are still name-only until B.

Decision: [0030-theory-objects](decisions/0030-theory-objects.md).
Log: [logs/2026-04-24_theory_discovery.log](../logs/2026-04-24_theory_discovery.log).

### Intrinsic drive + global evaluation (ADR 0031)
Task C of the A→C→B→D sequence. First v2 mechanism where the system
self-triggers: chooses among its own abstraction capabilities, when
to invoke them, and when to stop — all driven by an internal scalar
score rather than external calls.

Added `RSet::abstraction_score(&self) -> f64`:
- Σ pattern reuse savings `max(0, (N-1)·k)`
- Plus `2.0 × Σ theory member counts`
- Minus `0.1 × meta-R edge count` (overhead tax)

Added `drive_step` (try all candidate actions on a clone, apply
best-improving) and `intrinsic_drive` (loop until saturation).
Candidate action space:
- `DiscoverPatterns(size)` for each `pattern_sizes` entry — wraps
  `autonomous_pass`
- `DiscoverTheory` — wraps `discover_theory` + `name_theory`

On four characteristically different inputs, the drive picked
different action orders that reflected the inputs:
- mixed graph: patterns(2) → theory, final 14.7
- equivalence: theory → patterns(4), final 14.6
- strict poset: patterns(3) → theory, final 11.0
- random sparse: patterns(2) → theory, final 4.3

The final score discriminates "how much is there to understand"
(structured inputs ~14.7 vs random ~4.3). `adr0031_drive_is_idempotent_
after_saturation` confirms the loop halts and is stable on re-run.

Tests: 155 → 162 (7 new). This closes the long-pending
"self-driven triggering" capability from MEMORY.md's wishlist and
gives v2 its first external-visible signal of abstraction depth.

Decision: [0031-intrinsic-drive](decisions/0031-intrinsic-drive.md).
Log: [logs/2026-04-24_intrinsic_drive.log](../logs/2026-04-24_intrinsic_drive.log).

### Axiom intension as meta-R (ADR 0032)
Task B of the A→C→B→D sequence. ADR 0030 registered axioms by name
only; this ADR gives each template axiom its full structural
intension in meta-R — the promise commitment 3 makes for every
type-level object.

Added three reserved markers: `__axiomvar__`, `__premise__`,
`__conclusion__`. Every template axiom now carries:
- `n` variables `ax_X_var_i` (registry + ownership)
- `m` premise-edge nodes (registry + ownership + chain source/target)
- One conclusion-edge node (registry + ownership + chain)

Each premise / conclusion edge becomes a 3-node chain
`var_x → edge_node → var_y`. Direction of R encodes source vs.
target, so no per-edge src/tgt markers are needed. Total per
template axiom: 2n + 4m + 4 edges (18 for transitivity, 12 for
symmetry).

Predicate axioms (`ax_reflexivity`, `ax_antisymmetry`) remain
registry-only — their semantics is in the predicate checkers, not
in the template form.

New API:
- `register_axiom_with_intension(id)` — writes the chain encoding
  (called automatically from `name_theory`)
- `axiom_variables(ax)`, `axiom_premise_edges(ax)`, `axiom_conclusion(ax)`
- `reconstruct_axiom_template(ax) -> Option<AxiomTemplate>` — inverse
  of the intension write
- `retract_axiom(ax)` — removes the full stack, refuses if a theory
  references it

Roundtrip tests confirm that storing + reading back yields the
original template exactly for both transitivity and symmetry.
Axiom intension does not leak into data-layer discovery (verified
with a before/after `discover_axioms` equality check).

Tests: 162 → 170 (8 new). `collect_meta_ids` extended to cover the
three new markers and all axiom internal ids. commitment 3 now
lands for every named meta-R object in v2 (patterns, theories,
axioms) except predicate axioms, which require a richer template
language.

Decision: [0032-axiom-intension](decisions/0032-axiom-intension.md).

### Defeasible axioms (ADR 0033)
Task D of the A→C→B→D sequence, final phase. Admits axioms at
rate < 1.0 with support threshold. Motivating case: the "almost-
transitive" rigorous battery input (4-chain closure minus one
edge) previously returned zero axioms under strict `rate == 1.0`;
now at `min_rate = 0.5` it surfaces transitivity with rate 0.667,
support 2/3 — the system can report "this almost holds."

Added `AxiomDiscoveryConfig::min_rate: f64` (default 1.0, preserves
strict behavior). Relaxed the `discover_axioms` check from
`rate == 1.0` to `rate >= min_rate`. Guarded `discover_axioms_minimal`
to skip subsumption when `min_rate < 1.0` (subsumption assumes
strict soundness and would not compose correctly on defeasible
rules).

Preserved: discover_theory still strict; intrinsic_drive still
scores strict axioms only; subsume_* free functions unchanged.

Tests: 170 → 176 (6 new: strict-default unchanged, defeasible
surfaces near-axioms, minimal skips subsumption in defeasible,
strict minimal still subsumes, rate invariant across evidence,
loose threshold yields at least as many).

Decision: [0033-defeasible-axioms](decisions/0033-defeasible-axioms.md).

### Theory extension relations (ADR 0034)
First of the approved five-step extension (1→2→3→4→5). Adds theory-
to-theory extension as a first-class meta-R object: T_sub extends
T_super when `members(T_sub) ⊇ members(T_super)`.

New marker `__extends__`. Extension edge encoded as three-edge chain:
`R(__extends__, ext_N)` + `R(T_sub, ext_N)` + `R(ext_N, T_super)`.
Same direction-as-role convention used for axiom internal chains.

API: `name_theory_extension`, `extension_edges`, `extension_endpoints`,
`theory_extends`, `theory_extended_by`, `discover_theory_extensions`
(read-only pair scan).

First higher-order relation in v2 meta-R — all prior meta-R linked
objects to definitions; this links objects to each other. Tests:
176 → 182. collect_meta_ids extended for EXTENDS_MARKER and every
ext_N.

Decision: [0034-theory-extension-relations](decisions/0034-theory-extension-relations.md).

### Counterfactual value / meta-metric (ADR 0035)
Task 2 of 1→5. Adds a second-order signal on top of ADR 0031's
drive: for each named object, how much does it contribute to the
global score?

`counterfactual_value(id) -> Option<f64>` clones self, retracts the
object, returns (before − after). `rank_by_counterfactual()` gives
a descending ranking of all retractable named objects.

Also added `retract_extension` (symmetric with retract_theory /
_pattern / _axiom).

Supported object kinds: patterns, theories, extensions, and axioms
not yet bound to any theory (blocked for axioms in theories since
retract_axiom refuses; caller must retract theories first).

Truth invariant: predicted drop ≡ actual drop after retract,
verified numerically (`adr0035_counterfactual_respects_actual_
retract_behavior`).

Tests: 182 → 188 (6 new).

Decision: [0035-counterfactual-value](decisions/0035-counterfactual-value.md).

### Empty-premise templates (ADR 0036)
Task 3 of 1→5. Partially closes the "predicate axioms have no
template form" gap flagged since ADR 0027. Admits empty-premise
templates with single-variable self-loop conclusion:
`[] ⇒ R(0,0)` — reflexivity as a template.

Config: `AxiomDiscoveryConfig::include_empty_premise: bool`, default
`false` (backward compat). When true, `enumerate_axiom_templates`
prepends the empty-premise case; `evaluate_template_recursive`
already handles empty premise trivially (every binding counts).

Reflexivity now has two ids: `ax_reflexivity` (predicate, via
check_reflexivity, used by discover_theory) and `ax_tpl_v1_c0-0`
(template, via opt-in discover_axioms). They coexist; discover_theory
unchanged for backward compatibility.

Antisymmetry (equality conclusion) and totality (disjunction) still
require template-language extensions beyond this ADR.

Defeasible + empty-premise: partial reflexivity at rate 0.5
correctly surfaces on a half-reflexive graph.

Tests: 188 → 194 (6 new).

Decision: [0036-empty-premise-templates](decisions/0036-empty-premise-templates.md).

### Compositional subsumption (ADR 0037)
Task 4 of 1→5. Finally addresses the equivalence-relation "5
minimal axioms" residue noted since ADR 0030/0028: the four
transitivity variants are all derivable from {sym, any one variant}
under composition, but ADR 0028's direct premise-weakening couldn't
see that.

Added `template_derivable_from(target, sources)` — forward-chaining
derivability check on fresh nodes. Seeds target's premise, iterates
sources as closure rules to fixpoint, checks if target's conclusion
appears. Sound only in strict mode (rate = 1.0 sources).

Added `subsume_by_composition(axioms) -> axioms` — iteratively drop
axioms derivable from the rest, in descending template-key order
for determinism.

Added `RSet::discover_axioms_minimal_compositional(config)` — runs
`discover_axioms_minimal` then applies composition subsumption.
Strict-mode only; defeasible mode passes through.

Effect on equivalence: 5 → 2 (symmetry + one transitivity-shape
axiom). Strict poset / total order unchanged (already 1 axiom).
`discover_axioms_minimal` itself NOT changed — composition is an
opt-in extra step.

Tests: 194 → 200 (6 new). The "4 transitivity variants are
compositionally subsumed" gap noted since ADR 0030 is finally
closed.

Decision: [0037-compositional-subsumption](decisions/0037-compositional-subsumption.md).

### RSet text persistence (ADR 0038)
Final task 5 of 1→5. Smallest-possible persistence: line-oriented
TSV, `x\ty\n`, sorted lex so byte-identical across processes.
Blank / `#`-prefixed lines are skipped on read for hand editing.

`RSet::to_text()` / `RSet::from_text(&str)` with `PersistenceError`
enum for tab / newline / malformed-line. `RSet` now derives
`PartialEq, Eq` so roundtrip tests can use `assert_eq!`.

Because every named meta-R object (patterns, roles, theories,
axioms with intension, extensions) is already encoded as R
instances, persistence is a single Set round-trip — nothing
special-cased. Tested end-to-end.

No external dependencies (no serde / no JSON). v2 stays zero-dep.

Tests: 200 → 209 (9 new).

Decision: [0038-persistence](decisions/0038-persistence.md).

### Totality as predicate axiom (ADR 0039)
Task 1 of the second five-step extension (1'→4'). Adds the third
predicate axiom on par with reflexivity / antisymmetry. `check_
totality` verifies every unordered pair (x, y) satisfies
`R(x,y) ∨ R(y,x)`; `discover_theory` now includes `ax_totality`
when it holds; `name_theory` accepts and verifies it.

Total orders now have a distinguishing fingerprint (`{trans, refl,
antisym, totality}`) vs. non-total posets. Predicate-only — template
language cannot yet express disjunctive conclusions.

Tests: 209 → 217 (8 new).

Decision: [0039-totality-predicate](decisions/0039-totality-predicate.md).

### Drive auto-prune via counterfactual value (ADR 0040)
Task 2 of 1'→4'. Closes the "nothing auto-prunes" limit from ADR
0035. Drive gains a third candidate action `Prune(threshold)` that
retracts every named object with counterfactual value strictly
below the threshold.

Metric extended with `+ 1.0 · |extension_edges|` so extensions
(ADR 0034) are no longer net-negative under overhead tax. Gives
them positive CV, keeping auto-prune from eating them.

`DriveConfig` gains `enable_prune: bool` (default true) and
`prune_threshold: f64` (default 0.0). Drive is now a two-way
process — add via Discover actions, remove via Prune — while
remaining idempotent at saturation.

Retraction order inside Prune: theories first (release axiom
refs), extensions, patterns. Avoids retract_axiom's "still
referenced" failure.

Tests: 217 → 222 (5 new).

Decision: [0040-auto-prune](decisions/0040-auto-prune.md).

### Scale benchmark (ADR 0041)
Task 3 of 1'→4'. Measurement-only ADR. Built
`examples/scale_benchmark.rs` to characterize the β-scale envelope
on deterministic random graphs.

Findings:
- 50 edges → drive completes in ~0.4 s
- 100 edges → ~2.3 s
- 200 edges → ~30 s
- 400 edges → ~255 s
- Interactive envelope ≈ 50 edges; tolerable ≈ 200.

Bottleneck: `find_instances_of` in `autonomous_pass` scales as
edges × avg_degree^(k-1). Axiom discovery scales ~linearly
(nearly independent of edge count at fixed identifier count).
Persistence (to_text / from_text) is not a bottleneck — sub-ms per
100 KB.

Accidental-axiom effect surfaces at dense small-id graphs: 31
templates at rate 1.0 (9 after minimization) on a 400-edge
20-id graph. Recorded as known limit; needs statistical-
significance filter to address.

No new library API. Two natural optimization follow-ups noted:
(a) route autonomous_pass through sample_instances_of,
(b) add source-/target-index to RSet.

Decision: [0041-scale-benchmark](decisions/0041-scale-benchmark.md).
Log: [logs/2026-04-24_scale_benchmark.log](../logs/2026-04-24_scale_benchmark.log).

### Theory independence relations (ADR 0042)
Task 4 of 1'→4'. Companion to ADR 0034's `extends`: a symmetric
relation saying two theories share no axioms. Together with
extends, theories now have basic "theory-space geography" in
meta-R.

New marker `__independent__`. Chain encoded in canonical
direction: `R(T_lo, ind_N) + R(ind_N, T_hi)` with `T_lo < T_hi`
lex, so the pair has exactly one stored form regardless of
argument order.

API: `name_theory_independence`, `independence_edges`,
`independence_endpoints`, `theories_independent_from` (symmetric),
`discover_theory_independences`, `retract_independence`.

Verification at name time: both theories exist, distinct, member
sets disjoint. Tests: 222 → 230 (8 new).

Note: independence edges are not currently rewarded by the drive
metric (only extensions are, per ADR 0040). A future ADR can
symmetrize this if usage warrants.

Decision: [0042-theory-independence](decisions/0042-theory-independence.md).

### Indexed RSet + sampling-path integration (ADR 0043)
Task 1 of the 1''→5'' round. Combines the two optimizations noted
in ADR 0041. `RSet` now maintains `by_source` / `by_target`
HashMaps synced with `instances` via `add`/`remove`. `left_of` and
`right_of` are O(edges-at-id) instead of O(all-edges). Manual
`PartialEq` ignores indices (equality = same `instances`).

`AutonomousConfig.instance_sampling: Option<SamplingMatchConfig>`
opts into sampling-path: when `Some`, `autonomous_pass` routes
instance collection through `sample_instances_of` (ADR 0024)
instead of exhaustive `find_instances_of`. `DriveConfig` gains
the same flag propagated to each DiscoverPatterns action. Default
stays exhaustive.

Scale benchmark (release) drive-time: 373 → 364 ms (50e), 2.26 →
2.04 s (100e), 30.2 → 29.2 s (200e), 255 → 208 s (400e). 10–18%
speedup from indexing; the deeper bottleneck (find_instances_of
enumeration) remains — sampling mode is the lever when that
matters.

Tests: 230 → 236 (6 new). Zero public API regression.

Decision: [0043-indexed-rset-and-sampling-path](decisions/0043-indexed-rset-and-sampling-path.md).

### Extended template language: equality + disjunction (ADR 0044)
Task 2 of 1''→5''. Antisymmetry and totality enter the template
family, without disturbing the existing `AxiomTemplate` type.

Two new sibling types: `EqualityAxiomTemplate` (conclusion is
`v_a = v_b`) and `DisjunctiveAxiomTemplate` (conclusion is
`R(c_1) ∨ R(c_2) ∨ …`). Unified `ExtendedAxiomEvidence` enum
carries rate + support across all three families.

`discover_antisymmetry_template`, `discover_totality_template`,
`discover_extended_axioms` merge all three families with shared
`min_rate` / `min_evidence` filtering. Rate behavior on the
rigorous battery:
- diamond poset: antisym rate 1.0 (premise only met at self-loops)
- equivalence: antisym rate < 1.0 (R(a,b) ∧ R(b,a) holds for a ≠ b)
- total order: totality rate 1.0
- diamond: totality rate < 1.0 (incomparable pair exists)

Not yet integrated: meta-R intension, subsumption, composition,
discover_theory fingerprint — all edge-family only. Documented
as deferrals.

Tests: 236 → 243 (7 new).

Decision: [0044-extended-template-language](decisions/0044-extended-template-language.md).

### Axiom confidence: Wilson score + null-baseline (ADR 0045)
Tasks 3+4 of 1''→5'' (combined). Closes the "how confident should
I be" gap noted since ADR 0041's scale benchmark. Every AxiomEvidence
now carries:
- `posterior_lower_95` / `posterior_upper_95`: Wilson score 95%
  CI on the binomial proportion `s/n`. Corrects `rate` for small
  N (diamond poset's N=2 transitivity: CI lower < 0.5 despite
  rate 1.0).
- `null_baseline_prob`: `p_edge^N` under iid Bernoulli null;
  small = statistically surprising.

Fields populate in `evaluate_axiom_template`; no default filter
applied. Callers opt into strict acceptance via
`ev.posterior_lower_95 > 0.8` or `ev.null_baseline_prob < 0.01`.

Dense random graphs (ADR 0041's 400-edge case) now report
null_baseline_prob ≈ 1.0 on all axioms — a filter threshold of
0.01 would drop the accidental 31.

Three AxiomEvidence struct-literal test sites patched with the
new fields (non-breaking at contract level, breaking at literal
level).

Tests: 243 → 249 (6 new).

Decision: [0045-axiom-confidence](decisions/0045-axiom-confidence.md).

### Theory parallel relations (ADR 0046)
Task 5 of 1''→5''. Fills the gap between extends (one subsumes the
other) and independent (no overlap): **parallel** — two theories
share some members but neither is a subset of the other.

Marker `__parallel__`, chain encoding `R(T_lo, par_N) + R(par_N,
T_hi)` (canonical direction, lex-smaller first), same shape as
independence.

API: `name_theory_parallel`, `parallel_edges`,
`parallel_endpoints`, `theories_parallel_to` (symmetric),
`discover_theory_parallels`, `retract_parallel`. Name-time
verification rejects disjoint pairs (use independence) and subset
pairs (use extends), with helpful messages.

The three theory-space relations now partition every
pair-of-distinct-theories:
- extends: one strictly contains the other
- independent: empty intersection
- parallel: non-empty intersection, neither contains the other

Plus the trivial `equal` case handled by `name_theory`'s id-reuse.

Tests: 249 → 257 (8 new).

Decision: [0046-theory-parallel](decisions/0046-theory-parallel.md).

### Extended axiom id codec (ADR 0047)
Task 1 of 1'''→5'''. Fills the gap left in ADR 0044: equality and
disjunctive templates now have deterministic ids and can be
accepted by `name_theory`.

Format:
- Edge: `ax_tpl_v{n}_p..._c...` (0030)
- Equality: `ax_eq_v{n}_p..._eq{a}-{b}` (new)
- Disjunctive: `ax_disj_v{n}_p..._d..._d...` (new)

`verify_axiom_holds` now dispatches the three prefixes in order.
`name_theory` can bundle axioms of all three families into one
theory — on a total order: {transitivity, antisymmetry via
equality form, totality via disjunctive form} all co-exist.

Not yet: meta-R intension / subsumption / composition for non-
edge axioms. Deferred.

Tests: 257 → 265 (8 new).

Decision: [0047-extended-axiom-ids](decisions/0047-extended-axiom-ids.md).

### Confidence filters in discovery config (ADR 0048)
Task 2 of 1'''→5'''. Finishes ADR 0045 — the posterior and null-
baseline fields are now wirable through `AxiomDiscoveryConfig`:
- `min_posterior_lower: f64` (default 0.0) — drops small-support
  axioms whose Wilson CI lower falls below threshold.
- `max_null_baseline: f64` (default 1.0) — drops dense-random
  accidents whose null probability exceeds threshold.

Defaults preserve ADR 0027 behavior. `discover_axioms`,
`discover_axioms_minimal`, `discover_axioms_minimal_compositional`
all compose the filters automatically.

Demonstrated on complete-graph-on-4-ids: `max_null_baseline = 0.5`
drops every axiom as accidental. On diamond poset:
`min_posterior_lower = 0.7` drops transitivity (only 2 bindings).

Tests: 265 → 270 (5 new).

Decision: [0048-confidence-filters](decisions/0048-confidence-filters.md).

### Theory relation classifier + neighborhood (ADR 0049)
Task 3 of 1'''→5'''. Meta-view over ADRs 0034/0042/0046. One call
gives the relation kind for any pair of named theories; another
groups every other theory by its relation to a given one.

`TheoryRelationKind` enum has 5 values: Equal, Extends, ExtendedBy,
Independent, Parallel. Every distinct pair of named theories falls
into exactly one.

`classify_theory_pair(a, b)` — HashSet-compare member sets, return
the kind in O(|axioms|). Returns None if either id isn't a theory.

`theory_neighborhood(t) -> TheoryNeighborhood` — for every other
named theory, classify and group. Returns sorted lists per kind.

Read-only. Callers who want to persist a relation still use the
specific `name_theory_*` functions.

Tests: 270 → 276 (6 new).

Decision: [0049-theory-relation-classifier](decisions/0049-theory-relation-classifier.md).

### Large-scale sampling-mode benchmark (ADR 0050)
Task 4 of 1'''→5'''. Runs the drive loop at 100/200/500/1000 edges
in sampling mode, compares to exhaustive at overlapping sizes.

Results:
- sampling mode scales to 1000 edges in ~16.5s
- exhaustive tolerable only to ~200 edges (~38s)
- sampling under-reports instance counts (final score ~5-29% of
  exhaustive's at overlapping sizes)

Interactive envelope pushed from ~50 edges (ADR 0041) to ~1000
edges (ADR 0050) — 20× expansion, at the cost of stochastic
under-counting.

No library changes — pure benchmark ADR exercising ADR 0043's
existing knob.

Decision: [0050-sampling-scale-benchmark](decisions/0050-sampling-scale-benchmark.md).
Log: [logs/2026-04-24_sampling_scale_benchmark.log](../logs/2026-04-24_sampling_scale_benchmark.log).

### Adaptive drive config (ADR 0051)
Task 5 of 1'''→5''', final in this round. Makes DriveConfig
RSet-aware.

`RSet::adaptive_drive_config(base) -> DriveConfig` reads the RSet's
data-edge count and tunes:
- drops pattern_sizes that don't fit (`k > data_edges`)
- scales `discovery_config.sample_count` to `(edges*2).clamp(50, 1000)`
- enables `instance_sampling` when `edges > 300` (if caller left it None)
- preserves every explicit caller choice (naming_policy,
  axiom_config, explicit instance_sampling, etc.)

First v2 mechanism where the system **picks its own performance
parameters** based on inspecting itself. Not full autonomy (still
triggered externally) but removes the manual scale-tuning step at
the pipeline's widest lever.

Tests: 276 → 282 (6 new).

Decision: [0051-adaptive-drive-config](decisions/0051-adaptive-drive-config.md).

### Autonomous runtime — Phase A0 skeleton (ADR 0052)
After a multi-round design conversation, landed ADR 0052 as the
overall autonomous-runtime architecture. Status promoted from
Proposed to Accepted concurrent with Phase A0 implementation.

Design summary: v2 + runtime module (NOT v3); five modules
(runtime / scheduler / memory / environment / evaluator); memory
two-tier (M0 durable Rust struct now, M1 declarativized meta-R
deferred); budget is step-count only; non-goals explicitly enumerated.

Phase A0 landed: `src/runtime/mod.rs` with `AutonomousRuntime`,
`LifecycleState` (Booting/Running/Sleeping/Stopped), `RuntimeMode`
(Expand/Consolidate/Reflect), `BudgetState`, `Episode`, `Memory`
(ring buffer + cap), `ActionKind`, `SchedulerDecision`, `ActionPlan`,
`Scheduler` trait + `StubScheduler`, `Environment` trait +
`NoOpEnvironment`, `Event`.

`run_bounded(max_ticks)` main loop: poll env → apply events →
scheduler.choose → execute (DiscoverTheory wired; DiscoverPatterns
and Prune stubbed for A1/A2) → record episode.

Tests (10 new): bounded-tick runs N episodes, diamond poset gets
its full theory named on first tick, score monotone non-decreasing
under stub, memory respects cap, run_bounded is additive, Stop /
Sleep decisions halt loop.

Phase A1 (real Frontier + rule-based Scheduler), A2 (mode
machine), A3 (sleep/wake + checkpoint), B (history kicks in)
queued. No v3 rename.

Tests: 282 → 292 (10 new).

Decision: [0052-autonomous-runtime-architecture](decisions/0052-autonomous-runtime-architecture.md).

### Runtime Phase A1 — Frontier + Rule-based Scheduler
Continuing ADR 0052. A1 moves from "stub spin loop" to
"value-guided loop":

- `FrontierKind` (TheoryCandidate / PatternCandidate /
  LowValueObjectForPrune), `FrontierStatus`, `FrontierItem`,
  `Frontier` with `refresh(rset, tick)` enumerating candidates from
  current state, sorted by priority descending, `mark_dirty()`
  flag.
- `FrontierTarget` (WholeRSet / PatternSize / Pattern / Theory)
  added to `ActionPlan` so actions know where to operate.
- `SchedulerContext<'a>` bundles rset + memory + frontier + mode +
  tick for the scheduler. `Scheduler::choose` trait signature
  changed accordingly.
- `RuleBasedScheduler` — picks top frontier item, maps kind to
  `ActionKind`, returns `Sleep` on empty frontier or after
  `max_zero_streak` unproductive episodes.
- `execute_action` now handles `DiscoverPatterns` (wires to
  `autonomous_pass` with per-target size) and
  `PruneLowValueObjects` (retracts by target kind or bulk-negative-CV).
- `collect_meta_ids` made `pub(crate)` so runtime can compute
  data-edge counts.
- `run_bounded` main loop: poll events → apply → mark dirty →
  refresh if dirty → scheduler.choose(&ctx) → dispatch.

Tests (11 new):
- `a1_frontier_proposes_theory_candidate_on_diamond`
- `a1_frontier_omits_theory_candidate_after_naming`
- `a1_frontier_proposes_pattern_candidates`
- `a1_rule_based_runs_and_sleeps` — full diamond poset run
- `a1_deterministic_trace_reproducible` — same seed twice → byte-
  identical episode trace
- `a1_empty_frontier_triggers_sleep`
- `a1_frontier_dirty_after_action`
- `a1_pattern_candidate_priority_decreases_with_size`
- `a1_frontier_sorted_by_priority_desc`
- `a1_mark_dirty_leaves_items_intact`
- `a1_rule_based_zero_streak_triggers_sleep`

Tests: 292 → 303 (11 new). Phase A2 next (mode machine).

### Runtime Phase A2 — mode machine
ADR 0052 Phase A2. Adds Expand / Consolidate / Reflect transitions
on top of A1's frontier-driven scheduler.

New types:
- `ActionKind::UpdateTheoryRelations` — execute scans named theory
  pairs and persists missing extension/independence/parallel edges
  via `classify_theory_pair`.
- `FrontierKind::TheoryNeedsRelations` — Frontier proposes when
  ≥ 2 named theories have at least one pair lacking a relation
  edge.
- `ModeTransition` struct + `Memory.mode_transitions: VecDeque`
  with `record_mode_transition` and `max_mode_transitions` cap.

Mode-aware scheduler:
- **Expand**: pick TheoryCandidate / PatternCandidate items.
  Switch to Consolidate when `recent_positive_discovers >=
  min_recent_gains` AND consolidate work exists. Falls back to
  Reflect when no expand work.
- **Consolidate**: pick LowValueObjectForPrune /
  TheoryNeedsRelations items. Switch to Reflect when consolidate
  work is empty.
- **Reflect**: pure state-machine — no Execute. Returns SwitchMode
  to Expand or Consolidate if work exists, else Sleep.

`SwitchMode` to the same mode is a no-op (no log entry); only
real transitions get logged.

`MinimizeAxioms` deferred — its semantics (rename theories with
smaller member sets) requires careful design around object
identity; future ADR.

Tests (10 new A2):
- Frontier proposes / omits TheoryNeedsRelations correctly
- UpdateTheoryRelations actually persists independence edges
- Mode transitions logged; same-mode is no-op
- Consolidate mode processes consolidate work
- Reflect never returns Execute
- Multi-theory chain walks Expand → Consolidate or Reflect
- Mode transitions cap respected
- Deterministic-trace property holds across modes

Tests: 303 → 313. Phase A3 next (sleep/wake + checkpoint).

### Runtime Phase A3 — sleep/wake + checkpoint round-trip
ADR 0052 Phase A3. The runtime now stays inside the main loop while
`Sleeping` and wakes on data events; it also serializes its mutable
state to text and rebuilds itself from that text.

New types:
- `LifecycleTransition` (and `Memory.lifecycle_transitions: VecDeque`
  with `record_lifecycle_transition` and
  `max_lifecycle_transitions` cap, default 200).
- `should_wake(events: &[Event]) -> bool` — public free fn. True iff
  any event is `AddEdge` / `RemoveEdge`. Bare `Tick` does NOT wake.

Main-loop changes (`run_bounded`):
- Loop condition no longer breaks on `Sleeping`; only `Stopped`
  exits.
- Each tick: poll env → compute `wake_signal = should_wake(&events)`
  → apply events → if `Sleeping` and `wake_signal`: transition to
  Running; if `Sleeping` and not: `continue` (skip scheduler /
  frontier-refresh / dispatch entirely — no episode added).
- New helper `transition_lifecycle(to, reason)` is the single seam
  for state changes: records the transition in memory, mutates
  `self.lifecycle`, and on entry to `Sleeping` or `Stopped`
  snapshots a checkpoint into `last_checkpoint: Option<String>`.

Checkpoint format (hand-rolled, no serde — mirrors `RSet::to_text`
TSV, ADR 0038):

```
# v2 runtime checkpoint v1
[meta]
tick<TAB>N
episode_counter<TAB>N
steps_since_last_gain<TAB>N
current_score<TAB>F
lifecycle<TAB>Running|Sleeping|Stopped|Booting
mode<TAB>Expand|Consolidate|Reflect
max_episodes<TAB>N
max_mode_transitions<TAB>N
max_lifecycle_transitions<TAB>N
actions_per_tick_cap<TAB>N

[rset]
<RSet::to_text() output>

[episodes]
id<TAB>tick<TAB>mode<TAB>action<TAB>tgt_kind<TAB>tgt_value<TAB>before<TAB>after<TAB>delta

[mode_transitions]
tick<TAB>from<TAB>to<TAB>reason

[lifecycle_transitions]
tick<TAB>from<TAB>to<TAB>reason
```

API:
- `AutonomousRuntime::checkpoint_text(&self) -> Result<String, String>`
- `AutonomousRuntime::from_checkpoint_text(text: &str) -> Result<Self, String>`
  — restores rset / lifecycle / mode / tick / counters / score /
  memory; uses default `StubScheduler` + `NoOpEnvironment`. Caller
  swaps in real scheduler / environment before resuming.
- File I/O is the **caller's** responsibility; the runtime stays
  pure. ADR 0052 § Memory M0 ("durability vs. declarativeness").

Side change: `ModeTransition.reason` and
`LifecycleTransition.reason` are `String` (not `&'static str`) so
they round-trip through serialization. One existing A2 test updated
accordingly.

Tests (12 new A3):
- `should_wake` truth table (data events / Tick / empty)
- Pre-sleeping runtime stays asleep under NoOpEnvironment for
  full tick budget; no episodes added
- Sleeping runtime wakes on AddEdge (verified via lifecycle log,
  not final state — wake may be followed by another Sleep)
- Bare `Tick` event does not wake
- Sleep entry logs a `LifecycleTransition` with reason
  `scheduler_sleep`
- `last_checkpoint` populated on sleep entry; format header
  matches
- Round-trip preserves rset, lifecycle, mode, tick, episodes,
  transitions, caps
- Round-trip is byte-idempotent (`text → restore → text` equals
  original)
- Resumed runtime can advance further ticks
- Lifecycle-transition cap respected (LRU eviction)
- End-to-end Running → Sleeping → Running cycle in one bounded run
  (TickGatedEnv injects an event mid-flight)

Tests: 313 → 325. Phase A complete; Phase B next (`ObjectHistory` +
`PolicyStats` + `SyntheticStreamEnvironment`).

### Runtime Phase B0 — history + stats + synthetic-stream env
ADR 0052 § Phase B starts. B0 introduces the three new structures
listed in the ADR and wires the runtime to populate them as a
side-effect of dispatch. **No** history-aware scheduling rule is
added in B0 — that is B1's job. The aim of B0 is to get data
flowing so B1 has signal to consume.

New types:
- `ObjectHistory` — `first_seen_tick`, `last_seen_tick`,
  `last_improved_tick`, `times_selected_as_focus`, `times_pruned`,
  `last_counterfactual_value`, `stability_estimate` (Option, kept
  `None` until B1 lands a rolling EMA).
- `ObjectHistoryStore` — three name-indexed maps
  (`patterns` / `axioms` / `theories`).
- `PolicyStats` — `action_counts`, `action_positive_delta_counts`,
  `mode_transition_counts`, `wake_count`, `sleep_count`,
  `stop_count`. ADR's `RegimeKey` bucketing is deferred until a
  regime signal is wired in.
- `SyntheticStreamEnvironment` — replays a `Vec<(u64, Event)>`
  schedule where the `u64` is matched against an internal poll
  index. Sorted on construction; events drain in order; stops
  early once it hits a future-tick entry.

Runtime integration:
- `execute_and_record` snapshots pattern/theory id sets before and
  after each action. New ids → `ObjectHistory::new_at(tick)`.
  Removed ids → bump `times_pruned`. All present ids advance
  `last_seen_tick`; positive-delta episodes also advance
  `last_improved_tick`. Targeted Pattern / Theory plans bump
  `times_selected_as_focus`.
- `execute_and_record` increments `policy_stats.action_counts`
  every dispatch and `action_positive_delta_counts` when delta > 0.
- Mode-transition dispatch increments
  `mode_transition_counts[(from, to)]`.
- `transition_lifecycle` increments `sleep_count` on entering
  Sleeping, `wake_count` on Sleeping → Running, `stop_count` on
  entering Stopped.
- Required: `ActionKind` and `RuntimeMode` derive `Hash` to be
  HashMap keys.

Checkpoint coverage: B0 does **not** serialize
`object_history` / `policy_stats`. `from_checkpoint_text` restores
them as empty. A B0 test asserts this boundary so B1 inherits an
explicit invariant. Existing A3 round-trip tests stay green
because they assert on serialized fields only.

Tests (12 new B0):
- ObjectHistory recorded on first DiscoverTheory; `first_seen_tick`
  matches the run tick; `last_improved_tick` populated when delta
  positive
- `last_seen_tick` advances across multi-tick run
- `action_counts` increments per dispatch; `*_positive_delta_*`
  bumps when delta > 0
- Mode-transition counts logged on SwitchMode
- Wake count = 1 after one wake-on-event; sleep_count behavior
  matches expectation
- Stop count = 1 after Stop dispatch
- SyntheticStream: events fire on the scheduled poll index
- Back-dated events (tick 0) fire on first poll
- **Drip-feed scenario (verification #5 partial)**: empty RSet
  fed a 4-node diamond poset over 9 ticks via SyntheticStream;
  runtime ends with all 9 edges and ≥ 1 named theory
- Pruning a theory increments `times_pruned`
- Targeted Theory plan increments `times_selected_as_focus`
- Restored runtime starts with empty history + stats (B0 boundary)

Tests: 325 → 337. Phase B1 next (regime-aware scheduling rule
that consumes ObjectHistory; checkpoint coverage of stats stores).

### Phase A verification — 8-case battery + drip-feed
ADR 0052 § Verification plan:
- #1 (282 prior tests pass) — verified continuously throughout
  A0–B0; nothing broken.
- #2 (≥ 30 new runtime tests across the three families) — far
  exceeded: 55 runtime tests across A0–B0 (10 + 11 + 10 + 12 + 12).
- #3 (8-case rigorous battery, fingerprint match) — landed.
  `a_verification_8_case_battery_matches_direct_discovery`
  reuses the cases from ADR 0027's
  `examples/axiom_rigorous_test.rs`; for each case, asserts the
  runtime's named theory's member axioms (sorted) equal what
  `rs.discover_theory(&cfg)` returns when called directly. Also
  checks the runtime stabilizes (`Sleeping`) within 60 ticks.
  `a_verification_8_case_battery_is_deterministic` runs each
  case twice and asserts byte-identical `(members, tick,
  lifecycle)`.
- #4 (NoOp termination) — covered by A3's
  `a3_runtime_stays_sleeping_under_noop_environment`.
- #5 (drip-feed) — covered first by B0's
  `b0_synthetic_stream_drives_runtime_to_named_theory` (existence
  check) and now fully by
  `a_verification_drip_feed_diamond_full` (all 9 edges arrive,
  ≥ 1 theory named, `is_poset == true` at the end).

Tests: 337 → 340 (+3 verification tests). All five Phase-A
verification predicates of ADR 0052 are now satisfied. Phase A
is closed; Phase B1 is the next ADR-0052 work item.

### Runtime Phase B1 — mode-thrash gate (first stats-driven rule)
ADR 0052 § Phase B / B1 first deliverable. The scheduler now
**queries** the stats pipeline B0 set up. Concretely: the
RuleBasedScheduler refuses a `SwitchMode(target)` once the pair
`(current, target)` has accumulated `>= max_mode_oscillations`
transitions in either direction in
`policy_stats.mode_transition_counts`. The decision becomes
`Sleep` instead.

Default `max_mode_oscillations = 4`. Rationale: each
Expand↔Consolidate round-trip costs two transitions; 4 = two
round-trips. After two unproductive cycles, the rset is most
likely stable and re-evaluating from cold (post-wake) is cheaper
than thrashing.

Implementation: every previous direct
`SchedulerDecision::SwitchMode(target)` site now goes through
`switch_or_sleep(ctx, target)`, which consults
`would_thrash(ctx, current, target)` first. The Reflect-mode
"all out of work" `Sleep` path is unchanged.

Tests (4 new B1):
- `would_thrash` with empty stats returns false.
- `would_thrash` triggers when forward + reverse counts hit
  the threshold; an unrelated pair stays untouched.
- Reflect mode with thrashed (Expand, Reflect) pair returns
  `Sleep` even though expand work exists.
- Below-threshold case still returns `SwitchMode(target)` —
  guards against off-by-one.

Tests: 340 → 344 (+4). B2 next: checkpoint coverage of
`object_history` + `policy_stats` so resumed runtimes inherit
their thrash history (without it, the gate is reset on every
boot).

### Runtime Phase B2 — checkpoint covers history + stats
ADR 0052 § Phase B / B2. Closes the boundary B0 left explicit:
`object_history` and `policy_stats` now round-trip through the
checkpoint. A resumed runtime inherits the B1 thrash gate's
input — without this, the gate resets on every boot.

Six new sections appended to the checkpoint format, after
`[lifecycle_transitions]`:

```
[object_history_patterns]
<id>\t<first>\t<last_seen>\t<last_improved>\t<focus>\t<pruned>\t<cv>\t<stability>

[object_history_axioms]
<same row schema>

[object_history_theories]
<same row schema>

[policy_stats_action_counts]
<action_kind>\t<total>\t<positive>

[policy_stats_mode_transition_counts]
<from>\t<to>\t<count>

[policy_stats_lifecycle_counts]
wake\t<n>
sleep\t<n>
stop\t<n>
```

Encoding choices:
- `Option<u64>` and `Option<f64>` use the sentinel `-` for `None`.
  `-` cannot legally start an unsigned integer, and we don't
  serialize negative zeros that begin with `-`, so it is
  unambiguous.
- All maps are emitted in sorted-key order so `text → restore →
  text` is byte-identical (preserves the A3 idempotent property).
- Action counts that are 0 are not written; the parser drops zero
  entries to keep the in-memory map sparse.

Tests:
- The B0 boundary test
  `b0_history_and_stats_default_after_checkpoint_restore` is
  replaced by `b2_history_and_stats_round_trip` — full equality
  for both stores after a real run.
- `b2_checkpoint_with_stats_is_idempotent` extends A3's idempotent
  property to the larger format.
- `b2_thrash_history_survives_resume` plants a thrash count of 4
  in `(Expand, Reflect)`, round-trips, and asserts the count is
  preserved.
- `b2_optional_fields_round_trip_none_and_some` covers both
  branches of every `Option` field on `ObjectHistory`.

Tests: 344 → 347 (net +3 — one B0 boundary test was repurposed
into a B2 test, three new B2 tests added). Phase B0/B1/B2
complete; the runtime can now be hibernated and resumed without
losing its accrued thrash gate or pattern lifetime data. Next
step is open — possible directions include richer history-aware
rules (cool down patterns whose `last_improved_tick` is stale)
or starting Phase C (selective declarativization to meta-R).

### Phase A verification — captured run report
Added a rerunnable reporter
`examples/phase_a_verification.rs` that prints, per case, the
runtime's tick / lifecycle / theory fingerprint match status
plus the named axioms. Captured a snapshot of its output into
`logs/2026-04-25_phase_a_verification.log`.

Snapshot summary (B2 era, 2026-04-25):
- All 8 rigorous-battery cases reach `Sleeping` and **match=OK**
  against direct `discover_theory`. Axiom counts:
  transitive_chain 3 · equivalence_3_classes 6 ·
  strict_partial_order_diamond 2 · almost_transitive 1 ·
  random_sparse 1 · tolerance 2 · total_order 4 ·
  complete_bipartite 1.
- Drip-feed diamond ends with `is_poset=true`, 3 named theories
  (one per "settled" intermediate state during edge ingestion;
  the runtime correctly names a theory at each plateau as the
  rset evolves). The first theory `t_0` already contains
  reflexivity + symmetry + a transitivity-shaped axiom.

To regenerate: `cargo run --example phase_a_verification`.

### Runtime Phase B1+ — DiscoverPatterns hit-rate cooldown
Second stats-driven scheduling rule on top of B1's mode-thrash
gate. RuleBasedScheduler now consults
`policy_stats.action_counts` and `action_positive_delta_counts`
for `ActionKind::DiscoverPatterns`. When the runtime has tried
DiscoverPatterns at least
`min_pattern_attempts_before_cooldown` times (default 5) AND
the positive-delta hit rate is below `min_pattern_hit_rate`
(default 0.1 = 10%), PatternCandidate items are skipped and the
scheduler prefers TheoryCandidate.

Falls back through the normal mode chain when no TheoryCandidate
exists either: Consolidate work → Reflect → Sleep. The
mode-thrash gate from B1 still applies at every SwitchMode site,
so a cooled-out runtime cannot oscillate forever between Expand
and Reflect.

Implementation seam: `pattern_cooldown_active(ctx)` reads stats;
`has_expand_work(ctx)` was upgraded from associated to method
form so it can consult cooldown state — Reflect now reports
"no expand work" when the only available expand work is
cooled-out PatternCandidate, avoiding a wasted Reflect → Expand
→ Reflect bounce.

Tests (5 new B1+):
- Cooldown inactive when attempts < threshold (3 < 5).
- Cooldown active on bad rate above the floor (1/20 < 10% with
  20 ≥ 5).
- Cooldown inactive on healthy rate (5/10 = 50%).
- Cooled scheduler falls back to TheoryCandidate even when
  PatternCandidate has higher priority (synthetic priority=999).
- Cooled scheduler with no TheoryCandidate available falls back
  to SwitchMode(Consolidate), not Sleep — confirms the chain
  walks correctly past cooldown.

Existing 347 tests stayed green: their attempt counts on
DiscoverPatterns either stay below the 5-attempt floor or the
hit rate stays well above 10%, so the gate never fires
spuriously.

Tests: 347 → 352 (+5).

### Runtime Phase B3 — stale-pattern Prune injection
Third stats-driven scheduling rule. Closes the Phase B
history → action loop in the simplest way: `ObjectHistory.last_improved_tick`
now gates a *new* `LowValueObjectForPrune` source on the frontier.
A named pattern becomes a stale-prune candidate when both:

- its age (`tick - first_seen_tick`) ≥ `min_pattern_age_for_staleness`
  (default 50), AND
- its staleness (`tick - last_improved_tick`, or `age` when
  `last_improved_tick` is `None`) ≥ `max_pattern_staleness_ticks`
  (default 30).

Stale prune items piggyback on the existing Consolidate / Prune
lane — no new dispatch path. Priority is fixed at 0.5, below the
typical negative-counterfactual prune (`-cv * 2.0`, normally ≥ 1.0),
so a counterfactually-bad pattern still preempts a merely-stale
one. If the same pattern already has a Prune item (e.g. from
negative cv), the staleness pass skips it — no double-injection.

Implementation seam:
- `StalenessConfig { max_pattern_staleness_ticks, min_pattern_age_for_staleness }`
  added next to `Frontier`; `Frontier` gains a `staleness` field
  with a sensible default.
- `Frontier::refresh_stale_prune(&ObjectHistoryStore, tick)`
  appends staleness items and re-sorts by priority. Idempotent
  against repeat calls in the same tick.
- Main loop in `AutonomousRuntime::run` calls
  `refresh_stale_prune` immediately after `refresh`, scoped to
  the same `dirty` gate. No checkpoint changes — frontier is
  recomputed on resume.
- Theory staleness deferred. Theories are harder-won than
  patterns; pruning a stale theory is a stronger signal than a
  staleness threshold can carry. Will revisit when the runtime
  has both stale-theory data and a counterfactual signal to
  combine.

Tests (6 new B3):
- Pattern age below the 50-tick floor → no injection (even when
  staleness window has elapsed since `first_seen`).
- Long-unimproved pattern (`first_seen=0`, `last_improved=None`,
  `tick=100`) → injected with `prune_stale_<id>_<tick>` id.
- Recently-improved pattern (`stale_since=5 < 30`) → skipped.
- Staleness pass leaves the existing negative-cv Prune item alone
  (no duplicate, original priority preserved).
- Two consecutive `refresh_stale_prune` calls on the same state
  produce the same item count (idempotent).
- When both negative-cv and stale items exist, the negative-cv
  one ranks first after sort.

Existing 352 tests stayed green: their horizons are well below
50 ticks, so the staleness floor is never reached.

Tests: 352 → 358 (+6). Phase B0/B1/B1+/B2/B3 complete; the
runtime now feeds history *and* acts on it. Phase C
("selective declarativization to meta-R") is a deferred follow-on
ADR — drafted separately as ADR 0053.

### ADR 0053 (Proposed) — selective declarativization (M1)
Phase C of ADR 0052, scoped before any code lands. Splits meta-R
into two classes: the existing "kind-of" facts (PATTERN_MARKER,
THEORY_MARKER, …) and a new "experience-with" class — facts the
runtime declares about how it has *used* the things in the first
class.

Phase C0 (smallest viable slice): a single new marker
`ESTABLISHED_MARKER`. A named pattern earns
`R(p_x, ESTABLISHED_MARKER)` when it has been stable for ≥ K
ticks (default 100) AND has been referenced by ≥ M episodes
(default 5). Promotion runs on Reflect entry; demotion piggybacks
on `retract_pattern` (cascade — no separate demotion machinery).
C1 and C2 (theories, shared axioms) are sketched and deferred.

No code yet. ADR carries the verification plan (5 tests +
end-to-end), open questions (counter source for M, reentry across
checkpoint), and four explicit alternatives that were rejected
(skip C; numeric attribute on existing edges; per-tick churn;
sub-categorized markers). Status: Proposed — implementation
lands in a follow-on commit if the design holds up against
review.

Tests: unchanged (358).

### Phase C0 — ESTABLISHED_MARKER (M1, slice 1)
ADR 0053 implementation. The runtime now declares its first
"experience-with" meta-R fact: a named pattern that has been
alive ≥ 100 ticks (`PromotionConfig::min_pattern_age_for_promotion`)
AND has contributed to at least one positive-delta episode
(`last_improved_tick.is_some()`, the M ≥ 1 form) earns the edge
`R(<id>, ESTABLISHED_MARKER)`. Demotion piggybacks on the
existing `retract_pattern` cascade — added as step (7) right
after the PATTERN_MARKER registry edge.

Implementation seam:
- `pub const ESTABLISHED_MARKER: &str = "__established__"` in
  `src/lib.rs`. Exposed via `collect_meta_ids` like every other
  registry marker.
- `RSet::retract_pattern` step (7) removes
  `R(pattern_id, ESTABLISHED_MARKER)` if present. No-op when the
  pattern was never promoted.
- `ActionKind::Declarativize` and `FrontierKind::EstablishedPromotion`
  added (and round-tripped through the action-kind text codec).
- `PromotionConfig` lives next to `StalenessConfig`; `Frontier`
  gains a `promotion: PromotionConfig` field.
- `Frontier::refresh_established_promotions(&rset, &history, tick)`
  runs alongside `refresh` / `refresh_stale_prune` under the same
  `dirty` gate. Idempotent.
- Scheduler picks `EstablishedPromotion` in Consolidate mode
  alongside Prune / TheoryNeedsRelations. Dispatch runs
  `rset.add(R::new(id, ESTABLISHED_MARKER))`.

Notes from implementation:
- The C0 slice deliberately uses M ≥ 1 — `last_improved_tick.is_some()`
  is a binary signal already in `ObjectHistory`. ADR 0053's open
  question 1 (cumulative episode scan vs. cheap path) is
  resolved in favor of the cheap path; tightening to "M ≥ N"
  needs a new counter, deferred.
- ADR 0053's "Where the gate runs" originally proposed a
  Reflect-entry hook. The Frontier / scheduler path proved
  cleaner — reuses the mode-thrash gate, budget, and episode
  log without special casing. ADR updated in the same commit.
- B3 / negative-cv interaction confirmed: a promoted pattern
  whose counterfactual value is negative (or that goes stale)
  gets pruned in subsequent Consolidate ticks; the ESTABLISHED
  edge cascades via the new `retract_pattern` step (7).
  Verified by `c0_b3_interaction_promote_then_prune_cascade`
  end-to-end.

Tests (9 new C0):
- Age below the 100-tick floor → no item.
- Aged enough but `last_improved_tick = None` → no item (M ≥ 1
  not met).
- Both conditions met, not yet promoted → item with
  `prune_p_good_<tick>`-style id and Pattern target.
- Already promoted (`R(id, ESTABLISHED_MARKER)` in rset) → no
  item.
- Pattern dropped from rset between history snapshot and
  refresh → no item.
- Two consecutive `refresh_established_promotions` calls → same
  count (idempotent).
- End-to-end Declarativize: 2-tick run on a planted history
  emits the edge and records an `ActionKind::Declarativize`
  episode.
- Cascade: bare named pattern + manual ESTABLISHED →
  `retract_pattern` removes both.
- B3 interaction (full ADR test): promote on tick 152, then
  the negative-cv prune fires on tick 153 and cascades the
  ESTABLISHED edge along with the pattern.

Tests: 358 → 367 (+9). ADR 0053 status: Proposed → Accepted
(Phase C0 implemented). Phase C1 (theories) and C2 (shared
axioms) remain sketched; Phase B0/B1/B1+/B2/B3/C0 complete.

### Phase C1 — theory promotion (M1, slice 2)
Lifts C0's pattern gate into a parallel branch for named
theories. Same `ESTABLISHED_MARKER`, same M ≥ 1 cheap path; only
the age knob differs — `PromotionConfig.min_theory_age_for_promotion`
defaults to **200 ticks** (per ADR sketch — theories are larger
investments).

Implementation:
- `Frontier::refresh_established_promotions` extracted shared
  helpers (`passes_promotion_gate`, `make_promotion_item`) and
  now iterates both `history.patterns` and `history.theories`,
  applying each store's age threshold against the matching
  `rset.patterns()` / `rset.theories()` membership.
- `ActionKind::Declarativize` handler now also accepts
  `FrontierTarget::Theory(id)` — same `rset.add(R::new(id,
  ESTABLISHED_MARKER))` call.
- `RSet::retract_theory` gains a final cleanup step removing
  `R(theory_id, ESTABLISHED_MARKER)` symmetric to
  `retract_pattern`'s step (7).

Tests (7 new C1):
- Theory at age 150 (≥ pattern threshold but < theory threshold)
  → no item — confirms the theory-specific knob fires, not the
  pattern one.
- Aged theory but `last_improved_tick = None` → no item.
- Qualified theory → item with Theory target.
- Already promoted → no item.
- Theory dropped from rset → no item.
- `retract_theory` removes ESTABLISHED edge.
- Pattern + theory both qualify simultaneously → both items
  appear in one frontier pass.

Tests: 367 → 374 (+7). ADR 0053 status updated to "Accepted
(Phases C0 + C1 implemented)". Phase C2 (`SHARED_AXIOM_MARKER`)
remains sketched.

### Phase C2 — shared-axiom promotion (M1, slice 3)
The third meta-R class lands: a structural fact rather than an
experience-with one. `SHARED_AXIOM_MARKER` ("__shared_axiom__")
is emitted as `R(<axiom_id>, SHARED_AXIOM_MARKER)` whenever
`theories_containing(axiom_id).len() >= 2`. No `ObjectHistory`
lookup; the gate is fully derivable from rset state.

Implementation:
- `SHARED_AXIOM_MARKER` constant in `src/lib.rs`, exposed via
  `collect_meta_ids` for marker hygiene.
- `RSet::retract_theory` gains a final cascade step: for each
  member axiom, if `theories_containing(member).len() < 2`,
  remove the `R(member, SHARED_AXIOM_MARKER)` edge. Multi-share
  axioms (≥ 3 theories) keep the marker through any single
  retraction.
- `FrontierTarget::Axiom(String)` variant added; codec
  (`target_to_pair` / `pair_to_target` / `check_no_tab_or_newline`)
  updated. Round-trip preserved.
- `Frontier::refresh_shared_axiom_promotions(&rset, tick)` is the
  new dirty-pass step. Unlike C0/C1's history-driven gate, this
  iterates `rset.axioms()`, counts owning theories, and proposes
  an `EstablishedPromotion` item with `FrontierTarget::Axiom(id)`
  for each shared-but-unmarked axiom.
- The `ActionKind::Declarativize` handler now branches on target
  type to pick the marker: Pattern/Theory → ESTABLISHED, Axiom
  → SHARED_AXIOM. One action, two markers, target-driven
  semantics — keeps `ActionKind` lean.

Notes:
- C0/C1/C2 share the `EstablishedPromotion` FrontierKind for
  scheduler-pick simplicity. The kind name is now slightly
  generous (also covers shared-axiom), but introducing a second
  kind would force every dispatch site to fork without changing
  observable behavior.
- C2's drift detection is single-direction: promotion fires
  whenever the gate triggers, demotion only fires through
  `retract_theory`. That's enough because the only way an axiom
  loses theory-membership is theory retraction (axioms can't be
  silently unbound).

Tests (7 new C2):
- Axiom in only 1 theory → no item.
- Axiom in 2 theories → item with Axiom target.
- Already-marked axiom → no item.
- Idempotent across two consecutive refreshes.
- E2E `Declarativize` dispatch with Axiom target writes
  SHARED_AXIOM and NOT ESTABLISHED.
- Demotion: 2-theory share → mark → retract one → marker
  cascades.
- 3-theory share → retract one → 2 remain → marker stays.

Tests: 374 → 381 (+7). ADR 0053 status updated to "Accepted
(Phases C0 + C1 + C2 implemented)". The full ADR 0053 program
is done; M1 now covers patterns (experience-with), theories
(experience-with), and shared axioms (structural). Three meta-R
classes coexist with the original kind-of class.

### ADR 0054 (Proposed) — meta-meta-pattern discovery (Phase D)
Phase C produced three M1 marker classes; nothing currently reads
them. ADR 0054 is the design for the read side — making M1 a
subject of downstream discovery, which is the explicit promise
ADR 0052 / ADR 0053 made but neither one delivered.

Phase D0 (smallest viable slice): a new
`DiscoveryConfig::meta_subset_filter: Option<HashSet<String>>`
that lets `discover_motifs` see data + a *targeted* subset of
meta (specifically the ESTABLISHED / SHARED_AXIOM subgraph),
rather than the binary all-or-nothing of
`include_meta_in_discovery`. Surfaced through a new
`ActionKind::DiscoverMetaMetaPatterns` and a
`FrontierKind::MetaMetaCandidate` that the scheduler picks in
Expand mode once ≥ 5 M1 edges exist. Loop-closure: any
meta-meta-pattern named this way enters `ObjectHistory.patterns`
and is itself eligible for C0 promotion later.

D1 / D2 sketched (priority bias from M1 evidence; closed-loop
falsifiability via counterfactual ablation); both deferred.

ADR carries 6 verification tests (filter scope x3, action gate
x2, loop-closure smoke), 4 alternatives explicitly rejected
(skip D; ephemeral sub-rset; separate MetaRSet; ESTABLISHED-only
filter), and 4 open questions (strict vs lax marker matching;
cooldown counter sharing with DiscoverPatterns; checkpoint
persistence; termination cap on ESTABLISHED→meta-meta cycles).
Status: **Proposed**. No code yet.

Tests: unchanged (381).

### Phase-A verification rerun (post-B/C era)
Recaptured `examples/phase_a_verification` output into
`logs/2026-04-26_phase_a_verification.log`. The B0 → C2 wiring
between 2026-04-25 and 2026-04-26 changes scheduler decisions
in subtle ways; this log is the new reference.

Diff vs `2026-04-25` log: **one line.** The 8-case rigorous
battery is byte-identical (every `match=OK` with the same axiom
counts and members). Only the drip-feed scenario shifts:

```
- drip_feed: tick=40 lifecycle=Sleeping theories=3 is_poset=true
+ drip_feed: tick=40 lifecycle=Sleeping theories=2 is_poset=true
```

The named theory `t_0` is identical (10-axiom set, including
reflexivity + transitivity templates), and `is_poset=true`
holds. The drop from 3 → 2 reflects fewer intermediate
"settled" namings during edge ingestion — most likely the
B1+ DiscoverPatterns hit-rate cooldown suppressing one
intermediate plateau, or B3 stale-pruning a transient theory
that didn't accumulate enough recent improvements before its
cooldown floor. Verification #3 / #5 still pass (the `runtime::
tests::a_verification_*` test suite was green throughout
B/C/D-design landings).

This is the expected kind of behavioral drift from history-aware
rules: more conservative naming, no loss of the load-bearing
theory. To regenerate: `cargo run --example phase_a_verification`.

### Phase D0 — meta-meta discovery wiring (ADR 0054, slice 1)
First wiring of M1 markers as input to discovery, not just output
of the runtime. ADR 0054's smallest slice: prove the mechanism
works end-to-end before committing to the loop-closure naming
pipeline.

Implementation:
- `RSet::discover_motifs_with_meta_subset(config, subset)` — new
  public entrypoint. Internally selects edges via a new private
  `edges_with_meta_subset_sorted(subset)` helper (data edges are
  always included; meta edges are included iff at least one
  endpoint is in `subset`). Existing `discover_motifs` was
  refactored to share a `discover_motifs_from_edges` inner
  helper.
- `ActionKind::DiscoverMetaMetaPatterns` and
  `FrontierKind::MetaMetaCandidate` variants added (codec
  updated for both).
- `MetaMetaConfig` struct lives next to `StalenessConfig` and
  `PromotionConfig` on `Frontier`. Default thresholds:
  `min_m1_edges_for_meta_meta = 5`, `markers = [ESTABLISHED,
  SHARED_AXIOM]`, `target_size = 3`, `sample_count = 200`,
  `top_m = 10`, `rng_seed = 2026`.
- `Frontier::refresh_meta_meta_candidates(&rset, tick)` is the
  new dirty-pass step. Single item if the gate triggers; no
  per-marker enumeration.
- Scheduler dispatch: `MetaMetaCandidate` joins
  `TheoryCandidate` / `PatternCandidate` in Expand mode at
  priority 1.0 (loses ties to high-value theory work, which
  matches "exploratory" semantics).
- `execute_action` for `DiscoverMetaMetaPatterns` builds the
  subset (markers + their right-of subjects), calls
  `discover_motifs_with_meta_subset`, and discards the result.
  Episode is recorded with delta = 0 (no rset mutation).

Divergences from ADR 0054 sketch (recorded in the ADR):
- The filter is NOT a field of `DiscoveryConfig`; it's a separate
  rset method. Avoids touching 20+ existing literal-struct
  construction sites.
- Loop closure (find_instances over the filtered view, then
  name_pattern_instances) is deferred to a follow-on slice.
  `find_instances_of` and `is_clean_subgraph` both hard-code
  `data_edges_sorted` / data-only restrictions, so the loop
  closure requires extending those too. D0 proves the dispatch
  wiring; the next slice will prove the loop closes.

Tests (8 new D0):
- Filter includes data + ESTABLISHED edges + the registry edges
  for those subjects, and excludes unrelated meta (verified by
  adding an unrelated `AXIOM_MARKER` edge that stays excluded).
- No M1 in rset → filter returns data-only.
- Pure M1 (no data substrate) → discovery still runs without
  panic.
- 4 ESTABLISHED edges (< threshold) → no `MetaMetaCandidate`
  item.
- 5 ESTABLISHED edges (= threshold) → item appears with
  `MetaMetaCandidate` kind and `WholeRSet` target.
- Mixed M1 (3 ESTABLISHED + 2 SHARED_AXIOM = 5) → item appears.
- Two consecutive `refresh_meta_meta_candidates` → 1 item
  (idempotent).
- E2E runtime: 5 ESTABLISHED edges + a tiny data substrate →
  `DiscoverMetaMetaPatterns` episode appears within 3 ticks.

Tests: 381 → 389 (+8). ADR 0054 status: Proposed → Accepted
(Phase D0 implemented; naming pipeline deferred). The runtime
now has its first feedback loop where its own M1 markers
*influence what discovery does*, even if the closure
back-around to "rediscovered meta-meta-pattern named in rset →
eligible for C0" still needs the next slice.

### Phase D0+ — loop closure (ADR 0054, slice 2)
The deferred naming pipeline lands. `RSet::find_instances_of` and
`is_clean_subgraph` gained meta-subset siblings
(`find_instances_of_with_meta_subset`,
`is_clean_subgraph_with_meta_subset`) that walk the same filter
view as `discover_motifs_with_meta_subset`. A new
`RSet::meta_meta_subset(&[markers])` helper centralises the
"markers + their right-of subjects" set construction so the
runtime, `find_instances`, and tests all use the same filter
shape.

The `DiscoverMetaMetaPatterns` action now takes the top novel
candidate, finds its instances under the M1 view, and records
them via `name_pattern_instances_with_policy` with the
**Intensional** policy. The policy choice is deliberate:
Intensional writes Layer A (registry + roles + structural
edges among roles) but skips Layer B (the instance-bound
`R(<inst>, <participant>)` edges). With Layer B off, ESTABLISHED
or SHARED_AXIOM never get pinned as literal participants of the
freshly-named meta-meta-pattern — keeping marker semantics
clean and avoiding the kind of drift ADR 0054's open question
#4 (termination) flagged.

Loop-closure verified: a runtime that starts with 5 named
patterns each carrying ESTABLISHED produces a *new* pattern
within ≤ 8 ticks whose canonical lives in the M1 hypothesis
space.

Tests (3 new D0+):
- `find_instances_of_with_meta_subset` returns ≥ 10 clean
  instances of the canonical "3 edges with shared endpoint"
  when 5 ESTABLISHED edges are present (the WL-1 canonical
  collapses fan-in and fan-out at this size, doubling the
  raw-count expectation; documented inline). Every returned
  instance passes the meta-subset cleanness check.
- E2E: 5 ESTABLISHED-marked patterns + RuleBasedScheduler →
  `pattern_count` strictly grows after `run_bounded(8)`, AND a
  `DiscoverMetaMetaPatterns` episode appears in the log.
- Intensional policy invariant: after the loop closure, no
  `R(p_*_i_*, ESTABLISHED_MARKER)` edges exist (no Layer B
  pinning of the marker as a literal instance participant).

Tests: 389 → 392 (+3). ADR 0054 status: Phase D0 implemented →
Phase D0 + D0+ implemented. The "M1 → discovery → named
meta-meta-pattern" half of the closure now works; the
second half ("named meta-meta-pattern grows old enough → C0
promotes it back to ESTABLISHED → next D0 round sees it")
follows automatically through the existing C0 / B-line
infrastructure once the meta-meta-pattern accumulates
`first_seen_tick` age.

### Phase C0+ — `times_contributed_positive` counter
ADR 0053's open question #1 ("Counter source for M references")
was the last thing keeping C0/C1 on the cheap M ≥ 1 path. C0+
upgrades the gate to a real M ≥ N check by adding a dedicated
counter to `ObjectHistory` and threading it through the
checkpoint format and the promotion gate.

Implementation:
- New field `ObjectHistory.times_contributed_positive: u32`,
  defaulting to 0.
- Increments inside `execute_and_record` whenever a positive-
  delta episode lands and the object is present in
  `patterns_after` / `theories_after`. Distinct from
  `times_selected_as_focus`, which only increments when the
  object is the explicit `plan.target` (mostly Prune-side).
- B2 checkpoint format **gains a 9th column** on every
  `[object_history_*]` line: `<contributed>` after
  `<stability>`. Parser updated to require 9 fields. Format
  string version stayed `v1` because there are no out-of-tree
  consumers; the change is backward-incompatible only for
  hand-rolled checkpoint TSV.
- `PromotionConfig` gains `min_pattern_use_for_promotion: u32`
  (default 3) and `min_theory_use_for_promotion: u32` (default
  3). The `passes_promotion_gate` helper now checks
  `times_contributed_positive >= min_use` instead of the M ≥ 1
  proxy `last_improved_tick.is_some()`.

The default M = 3 reproduces ADR 0053's original sketch
("Suggest K = 200, M = 3"), now that the counter exists to
enforce it.

Test fixture migration: `history_with_pattern` and
`history_with_theory` helpers were updated to set
`times_contributed_positive = 3` whenever `last_improved` is
`Some(...)` and 0 otherwise. This preserves the prior test
semantic (M ≥ 1 ↔ last_improved.is_some()) while making each
test exercise the real M ≥ 3 path. Three inline literal
constructions in C0/C1 promote-success tests were similarly
bumped from 0 to 3.

Tests (4 new C0+):
- Age clears, last_improved set, but counter = 2 → no item
  (M ≥ 3 not met).
- Counter exactly = 3 → item appears.
- Non-zero counter round-trips through `checkpoint_text` /
  `from_checkpoint_text`.
- E2E: 20-tick run on the diamond poset produces at least one
  named pattern or theory with `times_contributed_positive > 0`,
  confirming the increment site fires under realistic dispatch.

Tests: 392 → 396 (+4). ADR 0053 open question #1 marked
resolved. C0/C1 are now on a real M ≥ N gate; the M ≥ 1 cheap
path documented in earlier C0/C1 commits is no longer the load-
bearing logic — it remains as a fast-path early-exit when
`last_improved_tick.is_none()` (which implies counter = 0).

### Phase D0+ — end-to-end demo + log capture
A standalone demo `examples/phase_d_demo.rs` that prints the loop
closure end-to-end on a hand-crafted seed (5 ESTABLISHED-marked
patterns + a tiny disconnected data substrate). Output captured
to `logs/2026-04-26_phase_d_demo.log`.

What the demo shows on the captured run:

```
named patterns: 5 → 6 (newly named: p_5)
DiscoverMetaMetaPatterns episodes: 3
  [0] tick=2 delta=-1.2000   ← naming pass; rset grew, score dropped
  [1] tick=3 delta=+0.0000   ← canonical now matches an existing pattern
  [2] tick=4 delta=+0.0000   ← same
new pattern p_5 intension:
  roles: [p_5_role_0..3]
  structural edges: 3 fan-out edges from role_0 to {role_1, role_2, role_3}
```

Reading the intension: the named meta-meta-pattern is the
"3 edges fan out from a single source" shape. The first sampled
instance was the PATTERN_MARKER fan-out (PATTERN_MARKER →
{p_a, p_b, p_c}), not the ESTABLISHED fan-in (which is its
WL-1 isomorphism partner — same canonical, opposite direction).
The runtime picked PATTERN_MARKER first because its lex-sorted
position (`__established__` < `__pattern__`) puts it second in
the participant order, but the first-instance index landed on
the fan-out branch.

This is consistent with the WL-1 fan-in/fan-out collapse the
D0+ tests already documented inline. The loop closure works;
the resulting meta-meta-pattern is structurally meaningful (a
"3-fan" shape that recurs across both PATTERN and ESTABLISHED
markers); it's just less *semantically* M1-anchored than the
ADR's idealised "what do all established patterns share?"
example. A WL-2 backend (deferred) would distinguish the two
directions and let the runtime name them as separate
meta-meta-patterns.

To regenerate: `cargo run --example phase_d_demo`. No new
unit tests — the existing `d0plus_loop_closure_names_meta_meta_pattern`
suite already covers the load-bearing assertions; the demo's
job is human readability and capture.

Tests: unchanged (396).

### ADR 0055 (Proposed) — direction-distinguishing canonical
The Phase D0+ demo log surfaced the WL-1 fan-in/fan-out
collapse concretely: the named meta-meta-pattern reflected a
fan-out shape even though the seeded M1 facts were fan-in. ADR
0055 isolates the bug — not in WL refinement, but in the
projection-to-canonical step (`rank_labels` collapses unique
signatures to local indices, discarding direction-sensitive
content the WL signatures already carried).

Phase E0 (smallest viable slice): replace `rank_labels` with a
**global hash of the converged WL signature** in the final
canonical projection. `CanonicalForm`'s inner type widens from
`(u32, u32)` to `(u64, u64)`. Five-line lib.rs change plus
type-width updates at every canonical-comparing callsite (the
compiler will enumerate them). Strongly regular and other
classical WL-1 counterexamples remain undistinguished — out of
scope for E0.

Phase E1 sketches a deferred WL-2 / individualisation-refinement
upgrade for any future failure beyond the fan-in/fan-out case.

ADR carries 4 verification items (existing tests pass after the
type change; new regression test asserting fan-in ≠ fan-out
canonicals; demo log re-capture; literal-canonical audit), 4
alternatives rejected, 4 open questions logged. Status:
**Proposed**. No code yet.

Tests: unchanged (396).

### Phase E0 — direction-distinguishing canonical (impl)
ADR 0055's smallest viable slice. Replaced the post-WL projection
in `Subgraph::canonicalize` with a hand-rolled FNV-1a hash over
the converged signature `(label, sorted_outs, sorted_ins)`.
`CanonicalForm` widens from `Vec<(u32, u32)>` to
`Vec<(u64, u64)>`; the WL refinement loop is unchanged.

The hand-rolled FNV-1a is intentional — `std::collections::hash_map::DefaultHasher`'s
seed regime is not part of Rust's stability guarantee, and the
canonical form is going to feed eventual cross-process diffs
(e.g., the Phase-D demo log). Hand-rolled = stable across Rust
versions and platforms.

Migration:
- 9 literal-pin sites in tests (e.g., `vec![(1,2), (2,0)]` for
  the 2-chain canonical) replaced with
  `Subgraph::from_edges([…]).canonicalize()` over a reference
  shape. The test contract becomes "the discovered canonical
  matches the canonical of a known reference subgraph" — same
  semantic, immune to label widening.
- D0+ runtime action `DiscoverMetaMetaPatterns` no longer takes
  the single top candidate. With sharper canonicals, the
  highest-frequency candidate is now more likely to encode a Y-
  or path-shape that crosses markers and fails
  `is_clean_subgraph_with_meta_subset`. The action now walks the
  top-`top_m` candidates by frequency and names the first novel
  one with at least one clean instance.

Verification:
- New regression test
  `canonicalize_distinguishes_fan_in_from_fan_out` asserts the
  two shapes that collapsed pre-fix now produce distinct
  canonicals.
- Demo log
  `logs/2026-04-26_phase_d_demo.log` re-captured. Diff vs.
  pre-E0:
  - **2 new patterns named** (`p_5` + `p_6`) where pre-E0 named
    only one — exactly ADR 0055's prediction. The runtime now
    sees fan-in and fan-out as separate hypotheses and names
    each.
  - **Two non-zero delta episodes** (both naming events)
    instead of one.
  - `p_5`'s intension is now **fan-IN** (`role_X → role_0`).
    Pre-E0 it was fan-out (`role_0 → role_X`). The two
    directions are no longer canonically equivalent.
- All 396 prior tests still pass after the type change and
  literal migration; +1 new fan-in/fan-out distinguisher test.

What this does NOT solve: strongly regular and other classical
WL-1 limits remain. Phase E1 (full WL-2 / individualisation-
refinement) is the deferred follow-on.

Tests: 396 → 397 (+1). ADR 0055 status: Proposed → Accepted
(Phase E0 implemented).

### ADR 0054 OQ #2 — independent meta-meta cooldown
The B1+ pattern-cooldown story now has a sibling for meta-meta.
`RuleBasedScheduler` gains `min_meta_meta_hit_rate` (default
0.05 — half of pattern's 0.1, since meta-meta is exploratory
and should fail more before being cooled) and
`min_meta_meta_attempts_before_cooldown` (default 5, same as
pattern). New helper `meta_meta_cooldown_active` mirrors
`pattern_cooldown_active`; both delegate to a shared
`action_kind_cooldown_active(stats, kind, min_attempts,
min_hit_rate)` so the gating logic lives once.

Wiring:
- Expand mode pick filter skips `MetaMetaCandidate` when cooled,
  alongside the existing `PatternCandidate` skip.
- `has_expand_work` likewise.
- `policy_stats.action_counts[DiscoverMetaMetaPatterns]` and
  `action_positive_delta_counts[DiscoverMetaMetaPatterns]` were
  already accumulating (B0 wired stats keyed by ActionKind);
  this commit just consults them.

Tests (5 new):
- Inactive when attempts < threshold.
- Active when ≥ 5 attempts AND hit rate < 5%.
- Inactive on healthy hit rate (≥ 5%).
- Independence: pattern-cooldown active does NOT activate
  meta-meta cooldown, and vice versa. Different counters.
- End-to-end scheduler pick: a cooled `MetaMetaCandidate` at
  synthetic priority 999 gets skipped; the scheduler picks
  `TheoryCandidate` instead.

Tests: 397 → 402 (+5). ADR 0054 open question #2 marked
resolved.

### ADR 0054 OQ #4 — termination empirics (CONVERGED)
Long-run experiment to answer "if ESTABLISHED →
meta-meta-pattern → C0 promotion → ESTABLISHED is a real
loop, does it terminate, oscillate, or grow without bound?"

`examples/phase_d_termination.rs` runs 500 ticks against a NoOp
environment with five seeded ESTABLISHED patterns + a tiny
disconnected data substrate. Snapshot every 50 ticks. Log
captured to `logs/2026-04-26_phase_d_termination.log`.

Trajectory:
```
  tick patterns theories estab.edges shared.ax mm.tries mm.hits ep lifecycle
     0        5        0           5         0        0       0  0 Running
    50        7        1           5         0        3       0  4 Sleeping
   100        7        1           5         0        3       0  4 Sleeping
   ...
   500        7        1           5         0        3       0  4 Sleeping
```

What happened:
- Ticks 0–50: runtime named **two meta-meta-patterns** (p_5 +
  p_6, the fan-in and fan-out from the M1 view, matching the
  Phase-E0 demo) and named one theory off the data side. Total
  4 episodes recorded.
- Tick ~50: scheduler hit "no expand work" with the cooldown
  gate active (0 hits / 3 attempts, eventually 0% < 5% floor
  once attempts ≥ 5 ... actually attempts plateaued at 3 here,
  so the cooldown isn't the proximate cause — but the
  scheduler still found no productive frontier item and went
  to Sleep).
- Ticks 50–500: stayed Sleeping. NoOp environment never wakes
  the runtime. No new patterns, no new episodes, no new M1
  edges.

**Verdict: CONVERGED.** The "infinite hypothesis explosion"
hypothesised in OQ #4 does not occur on this seed. The
combination of (a) the OQ #2 cooldown gate, (b) the scheduler's
mode-thrash gate (B1), and (c) the existing "no expand work →
Sleep" path produces a clean fixed point. No hard cap on
ESTABLISHED → meta-meta cycles is needed; the soft caps
suffice in practice.

Caveat: this only verifies termination for the bounded /
NoOp-environment case. A long-lived synthetic-stream environment
that keeps waking the runtime could still in principle support
a divergent loop. Re-test if that becomes load-bearing.

To regenerate: `cargo run --example phase_d_termination`.

ADR 0054 open question #4 marked resolved (empirically).
Tests: unchanged (402).

### ADR 0056 (Proposed) — Phase D verification battery
Two artefacts (`phase_d_demo`, `phase_d_termination`) both
exercise the same single seed: 5 fan-shaped synthetic patterns
around ESTABLISHED. Phase D's mechanism works, but
*systematic confidence across diverse shapes* doesn't exist —
which is what Phase A's 8-case rigorous battery (ADR 0027)
gives Phase A.

Phase F0 (smallest viable slice): a new
`examples/phase_d_battery.rs` that runs 6 seeds against
`RuleBasedScheduler::default()` for HORIZON=300 ticks each,
printing a per-seed trajectory + verdict (CONVERGED / STILL
GROWING / ANOMALOUS) plus a battery summary. Captured to
`logs/<date>_phase_d_battery.log` analogous to the Phase-A
verification log.

Initial seed set:
- `fan_only` — today's demo case (baseline).
- `diamond_poset` — A-battery's diamond, runs theory
  discovery first, then enables promotion.
- `bipartite` — `K_{2,3}`, different fan structure.
- `star` — single-centre data fan-out + ESTABLISHED on
  resulting patterns. Tests data-side / M1-side fan-out
  interference.
- `equivalence_classes` — A-battery's equivalence-3-classes,
  exercises full theory→C0→D pipeline.
- `disconnected_islands` — three disjoint 3-cycles, no
  cross-cluster signal.

Phase F1 sketches richer D-path scheduling state beyond
cooldown (cadence control via `last_meta_meta_tick`, separate
budget bucket, state-aware bias). Deferred until F0 surfaces
evidence that the OQ #2 cooldown counter alone is insufficient.

ADR carries 4 verification items, 4 alternatives rejected, 4
open questions. Status: **Proposed**. No code yet.

Tests: unchanged (402).

### Phase F0 — D-battery captured
ADR 0056 implemented as `examples/phase_d_battery.rs`. 6 seeds
× HORIZON=300 ticks each; per-seed snapshot every 50 ticks +
verdict; battery summary at end. Captured to
`logs/2026-04-26_phase_d_battery.log`.

Battery summary on the captured run:
```
                  seed         verdict   new patterns
              fan_only       CONVERGED              2
         diamond_poset       CONVERGED              0
         bipartite_2_3       CONVERGED              2
                star_5       CONVERGED              1
 equivalence_3_classes       CONVERGED              0
  disconnected_islands       CONVERGED              0
```

**All 6 seeds CONVERGE within 50 ticks**, every one transitioning
to `Sleeping` and staying there for the remaining 250 ticks.
No seed exhibits sustained growth, oscillation, or anomaly.

This is **strong empirical confirmation of the
compression-saturation diagnosis** — independent of topology
(synthetic-fan, poset, bipartite, star, equivalence-class,
disconnected-islands), the runtime reaches a fixed point
quickly. The "right" amount of self-extension is bounded by the
intrinsic-drive ceiling; richer topology buys more discoveries
along the way to that ceiling, but doesn't push past it.

Two specific findings:
- **Pattern discovery requires recurrent subgraphs.**
  `diamond_poset`, `equivalence_3_classes`,
  `disconnected_islands` produce 0 named patterns despite
  finding theories. The axioms cover the structure but no
  subgraph repeats often enough to clear `min_instances`.
- **Loop closure works on non-fan topologies**.
  `bipartite_2_3` and `star_5` both produce ≥ 1 new pattern,
  confirming the meta-meta path isn't tied to the specific
  fan-shape used in the original demo.

ADR 0056 status: Proposed → Accepted (Phase F0 implemented).
Tests: unchanged (402). The battery is a diagnostic example,
not a unit test.

### ADR 0057 (Proposed) — anomaly-coverage drive (Phase G0)
First **outward-facing** drive added to the runtime. Triggered
by F0's empirical confirmation of compression-saturation: 6
seeds, 0 STILL GROWING. Compression alone always terminates
fast.

Phase G0 = anomaly-coverage drive (the cheapest of three
candidate "outward" mechanisms — anomaly-priority,
prediction-error, curiosity). Key idea: define
`RSet::uncovered_data_edges()` = data edges not in any named
pattern's Layer B instance binding. When this set is non-empty,
the runtime has unexplained data and shouldn't sleep yet.

Two narrow scheduler hooks:
- **Cooldown relaxation under pressure**: B1+ pattern-cooldown
  hit-rate floor drops from 10% to 5% (default
  `anomaly_relaxation = 0.5`) when `uncovered.len() >= 3`
  (default `anomaly_pressure_threshold`).
- **Sleep suppression under pressure**: when the scheduler
  would otherwise return `Sleep` and `uncovered.len() > 0`,
  return `SwitchMode(Expand)` instead. Bounded by
  `max_mode_oscillations` so it can't loop forever.

What G0 does NOT do:
- No prediction. That's G1 / ADR 0058 (forward-application
  semantics) + ADR 0059 (prediction-error drive).
- No novelty reward. Curiosity is its own thing.
- No ActionKind / FrontierKind / Memory schema changes.
  G0 is purely on-demand computation off the current rset.

Verification plan: 3 new unit tests + F0 battery re-run after
G0 lands. The expected diff in the new battery log: most seeds
still CONVERGED (no fresh data → coverage trivially stable),
but at least one of `bipartite_2_3` / `star_5` /
`equivalence_3_classes` shows higher pattern count or extended
runtime (anomaly hooks firing on existing uncovered data).

ADR carries 4 alternatives rejected, 4 open questions logged.
Status: **Proposed**. No code yet.

Tests: unchanged (402).

### Phase G0 — anomaly-coverage drive (implementation + finding)
ADR 0057 implemented:
- `RSet::uncovered_data_edges() -> HashSet<R>` returns data
  edges where neither endpoint is a participant of any named
  pattern's Layer B instance binding.
- `RuleBasedScheduler` gains `anomaly_pressure_threshold` (3)
  and `anomaly_relaxation` (0.5).
- Pattern-cooldown floor is multiplied by `anomaly_relaxation`
  when uncovered ≥ threshold (10% → 5% effective floor).
- Reflect → Sleep transition is replaced with
  Reflect → Expand when uncovered > 0 AND the pair hasn't
  already thrashed. Mode-thrash gate still wins ties.

Tests (6 new G0):
- `uncovered_data_edges` excludes Layer B-covered edges.
- Empty rset → empty uncovered.
- Intensional-only patterns don't cover anything.
- Relaxed cooldown picks pattern under pressure
  (1/20 not < 5%).
- Sleep suppressed under pressure; sleeps without pressure.
- Sleep suppression bounded by thrash gate.

**Finding (commit message):** F0 battery re-run after G0 is
**byte-identical** to pre-G0. All 6 seeds still CONVERGE
within 50 ticks. Diagnosis: the existing mode-thrash gate
(max_mode_oscillations = 4) bounds the sleep-suppression hook
before any new pattern discoveries can happen. The hook fires
a few times but Reflect↔Expand quickly hits 4 oscillations
and the thrash gate forces Sleep regardless of pressure.

This is a **real architectural finding**, not a bug. G0's
local mechanisms work (6 unit tests verify); the system-level
ceiling is set by the thrash gate, not by the cooldown
hit-rate floor. Conclusion: G0 alone is not enough. The
saturation problem genuinely needs G1's finer success signal
— successful prediction can produce positive-delta episodes
without naming a new pattern, sustaining runtime activity
where G0 alone gets thrash-bounded.

Tests: 402 → 408 (+6). ADR 0057 status: Proposed → Accepted
(with caveat — implementation correct, system-level effect
bounded by thrash gate; G1 needed for full outward-drive
thesis to manifest).

### ADR 0058 (Proposed) — axiom forward-application semantics
Phase G1's prerequisite. Defines `RSet::forward_apply_axiom(id)`
and `RSet::forward_apply_all()` — pure-read operators that
take a named axiom and return the set of conclusion-edge
instances the axiom predicts under every valid premise binding
over data identifiers.

Concretely:
```
forward_apply(axiom, rset) =
  { R(σ(c.x), σ(c.y))
    | σ : 0..num_vars → data_ids(rset)
    , for every p in axiom.premise:
        R(σ(p.x), σ(p.y)) ∈ rset }
```

Key semantic choices:
- Substitution domain = data identifiers only (commitment 3:
  meta-R is not subject to axiomatic prediction).
- Output is the *raw* set; caller decides whether to subtract
  existing rset edges (G1.0 stays raw; G1's drive picks).
- One-shot, one-axiom-at-a-time. No recursive closure. No
  fixpoint.

Phase G1.0 = standard premise forward-apply (this ADR's
primary slice). G1.1 = equality constraints (ADR 0044). G1.2 =
disjunctive premises. All sketched, none implemented.

Performance analysis: O(N^num_vars) where N = data id count;
v2's typical num_vars ≤ 3, so ≤ 2.7e7 candidate substitutions
on a 300-node rset. Acceptable per-tick. β-scale (1000+ ids)
needs ADR 0043-style sampling, deferred to G1.X.

Independent of the drive, forward-apply is useful as a
debugging operator: "what does this axiom actually claim?"
The runtime today only evaluates axioms post-hoc against
existing rset state (`evaluate_axiom_template`); ADR 0058
fills the obvious gap.

ADR 0059 (prediction-error drive, TBD) will consume the output
of forward-apply to define and wire prediction error into the
scheduler. ADR 0058 is the mechanism; ADR 0059 will be the
drive.

Tests: unchanged (408). 4 alternatives rejected, 4 open
questions logged. Status: **Proposed**. No code yet.

### Phase G1.0 — axiom forward-application (impl)
ADR 0058 implemented. Two new public methods on `RSet`:

- `forward_apply_axiom(axiom_id) -> HashSet<R>`: takes a named
  template axiom, enumerates every variable substitution
  σ : 0..num_vars → data_ids that satisfies every premise
  edge, returns the set of conclusion edges under σ.
- `forward_apply_all() -> HashSet<R>`: union of
  `forward_apply_axiom` over `self.axioms()`.

Both follow ADR 0058's semantic decisions:
- Substitution domain = data identifiers only (commitment 3).
- Returns the *raw* predicted set; caller decides whether to
  subtract `self.instances` to keep "predictions not yet
  observed."
- One-shot, no recursive closure / fixpoint.

Implementation pattern mirrors the existing
`evaluate_template_recursive`. Predicate axioms (`AX_REFLEXIVITY`
/ `AX_ANTISYMMETRY` / `AX_TOTALITY`) and ADR 0044's equality /
disjunctive premise extensions all bypass the template-based
path — for those, `reconstruct_axiom_template` returns `None`
and forward-apply produces an empty set. G1.1 / G1.2 will
extend coverage.

Tests (5 new G1.0):
- Unknown axiom id → empty.
- Predicate axiom (`AX_REFLEXIVITY`) → empty (template-based
  path doesn't reconstruct).
- Template axiom on a transitive-closure substrate (5 nodes,
  10 edges, total-order shape) → non-empty prediction set
  including at least one re-derived closure edge.
- No named axioms → empty.
- Meta identifiers excluded from substitution domain.

Tests: 408 → 413 (+5). ADR 0058 status: Proposed → Accepted
(Phase G1.0 implemented).

This is the **prerequisite** for ADR 0059 (prediction-error
drive). With forward-apply landed, the runtime can now compute
"what does my axiom set claim should hold?" — the missing piece
between ADR 0057's anomaly-coverage drive and a real
prediction-error signal that decouples runtime activity from
mode-transition counters.

### ADR 0059 (Proposed) — prediction-error drive (Phase G1)
The drive design that consumes ADR 0058's mechanism. Three
sub-slices ordered by ambition:

**G1.3 — PredictionState + error tracking.** New runtime
field `PredictionState` holding `last_predicted` (the snapshot
from end of last tick) plus per-axiom prediction +
verified counters. Snapshot at end of tick;
verify-against-actual at start of next tick. Hit rate per
axiom = verified / total once total ≥ 5. Pure accounting,
no scheduler decisions yet. Round-trips through B2-style
checkpoint.

**G1.4 — Wire into anomaly drive.** Replace ADR 0057's
`uncovered_data_edges()` with
`unexplained_data_edges() = data_edges - layer_b_covered -
forward_apply_all()`. Same scheduler hooks, tighter signal.
Edges that NO pattern's Layer B covers AND NO axiom's
forward-apply predicts.

**G1.5 — Positive delta from prediction improvement** (the
load-bearing change). New `ActionKind::EvaluatePredictions`
fires during Reflect mode. Re-runs forward-apply, compares
per-axiom hit rate with previous Reflect's rate, records an
Episode with delta = sum of hit-rate improvements. Positive
delta possible WITHOUT rset mutation. This breaks the
"sustained activity = mode transitions = thrash counter" loop
that ADR 0057's Finding identified as the empirical null
cause.

Why G1.5 matters most: it's the architectural move that lets
the runtime stay productively active past compression
equilibrium. G1.3/G1.4 are mechanism + incremental
tightening; G1.5 is the new degree of freedom.

Expected F0 battery diff after G1.5:
- `fan_only` / `disconnected_islands` (no axioms) → still
  CONVERGED.
- `diamond_poset` / `equivalence_3_classes` → STILL GROWING
  (axioms predict, hit rates accumulate, Reflect ticks earn
  positive delta).

Stream-based seeds (ADR 0056's `stream_diamond` sketch) become
prerequisite for G1.5 verification — without ongoing
environmental events, predictions never get the chance to
verify or fail.

ADR 0059 carries 5 alternatives rejected, 5 open questions
logged. Status: **Proposed**. No code yet.

Tests: unchanged (413).

### Phase G1.3 — PredictionState + accounting (impl)
ADR 0059 G1.3 lands. Key additions:
- `PredictionState` struct on `Memory` with `last_predicted_at_tick`,
  `last_predicted_per_axiom`, `total_predictions_per_axiom`,
  `verified_predictions_per_axiom`, plus
  `last_reflect_hit_rate_per_axiom` (G1.5 — landed alongside).
- `AutonomousRuntime::snapshot_predictions` runs at end of each
  Running tick: `forward_apply_axiom(ax)` per named axiom, stores
  by id.
- `AutonomousRuntime::verify_predictions` runs at start of each
  tick (after env events applied): for each (axiom, predicted),
  intersect with current data edges, increment counters.
- `PredictionState::hit_rate(ax, min_total)` returns
  `verified / total` if `total >= min_total`, else `None`.
- Round-trips through B2 checkpoint as
  `prediction_state: PredictionState::default()` on restore
  (counters not yet serialized — deferred).

Tests (5 G1.3): no axioms / closure substrate (100% hit rate
on every recorded axiom) / hit_rate gating below min_total /
unknown axiom returns None / sleeping skips snapshot.

### Phase G1.4 — anomaly signal tightening (impl)
`RSet::unexplained_data_edges() = uncovered_data_edges -
forward_apply_all()`. Strictly tighter than the G0 metric: an
edge counts as unexplained iff (1) it's data, (2) no named
pattern's Layer B covers it, AND (3) no axiom's forward-apply
predicts it. The two G0 scheduler hooks (cooldown relaxation,
sleep suppression) now consume the new signal.

Tests (3 G1.4): equals uncovered when no axioms / strictly
smaller than uncovered when axioms predict data / total-order
substrate produces unexplained < uncovered.

### Phase G1.5 — EvaluatePredictions + delta override (impl)
The load-bearing change. New `ActionKind::EvaluatePredictions`
fires at the **top of `choose`** when `zero_streak >=
max_zero_streak` (the global stagnation gate would otherwise
force Sleep) AND `predictions_have_pending_delta(ctx)` is true.
Anti-stagnation placement, not Reflect-only, because the global
gate runs before mode dispatch.

Action handler computes per-axiom hit-rate delta vs. previous
Reflect snapshot, returns `Some(delta_sum)` overriding the
abstraction-score diff. `execute_and_record` honors the
override.

`recent_positive_discovers` now also counts EP episodes with
positive delta — feeds `min_recent_gains` for
Expand→Consolidate transitions, decoupling sustained activity
from mode-transition counters.

Tests (4 G1.5): `any_axiom_has_hit_rate` with/without data /
Reflect picks EP under stagnation + axiom data / Reflect sleeps
without data / E2E test deferred (interaction with
sleep-suppression hook makes precise reproduction brittle —
unit tests cover the load-bearing semantics).

**F0 battery diff vs. pre-G1**:
```
                  seed   ep pre  ep post
              fan_only        0        0  (no axioms)
         diamond_poset        4        7  (+3 EP eps)
         bipartite_2_3        0        0
                star_5        0        0
 equivalence_3_classes        4        7  (+3 EP eps)
  disconnected_islands        4        7  (+3 EP eps)
        stream_diamond        5       64  (DRAMATIC: +59 eps)
```

`stream_diamond` is the breakthrough: 5 → 64 episodes; theories
2 → 3; mm.tries 3 → 6; mm.hits 0 → 5. The streaming environment
keeps generating new R, the prediction-error signal keeps
fluctuating, EP keeps firing during stagnation gaps. Final
verdict still CONVERGED (eventually predictions stabilize and
runtime sleeps), but the runtime's qualitative *amount of
work* during the active phase is dramatically richer — exactly
the architectural change Phase G1 was designed to produce.

The static-substrate seeds get a smaller bump (+3 EP eps each):
once theory + axioms are named, the first EP after stagnation
records the initial 0% → 100% hit-rate jump as positive delta;
subsequent EP attempts find no pending delta and Sleep.

ADR 0059 status: Proposed → Accepted (Phases G1.3 + G1.4 + G1.5
implemented). Tests: 413 → 426 (+13).

### F0 battery — `stream_diamond` seed added
ADR 0056's deferred `stream_diamond` seed lands. Drip-feeds a
diamond poset over the first 24 ticks via
`SyntheticStreamEnvironment`. Captured to refreshed
`logs/2026-04-26_phase_d_battery.log` (replaces the prior
6-seed version).

Battery summary (now 7 seeds):
```
                  seed         verdict   new patterns
              fan_only       CONVERGED              2
         diamond_poset       CONVERGED              0
         bipartite_2_3       CONVERGED              2
                star_5       CONVERGED              1
 equivalence_3_classes       CONVERGED              0
  disconnected_islands       CONVERGED              0
        stream_diamond       CONVERGED              0
```

`stream_diamond` produces **2 theories** (vs 1 for the static
`diamond_poset`) and **3 mm.tries** (vs 0 for static) —
streaming did exercise more of the runtime's loop than the
static seed. But once the stream ends at tick 24, the runtime
sleeps. Verdict CONVERGED on 300-tick HORIZON.

This is the expected pre-G1.5 outcome and the baseline for
future G1.5 verification: with G1.5's prediction-improvement
positive-delta source, `stream_diamond` should be the first
seed to flip from CONVERGED to STILL GROWING (the 2 theories
generate ongoing predictions; their hit rates against the
stream events would accumulate into Reflect-tick deltas).

Tests: unchanged (413).

### G1.3 — checkpoint serialization (impl)
PredictionState cumulative counters now round-trip through
checkpoint via new `[prediction_state]` section. Transient
`last_predicted_per_axiom` snapshot intentionally NOT persisted
— regenerates on first post-restore Running tick. Closes the
G1.3 deferred item.

Tests: 426 → 427 (+1 round-trip).

### stream_diamond → STILL GROWING (the load-bearing verification)
Two changes together flip the F0 battery's stream-substrate
seed from CONVERGED to STILL GROWING:

1. **Fresh forward-apply gating.**
   `predictions_have_pending_delta` and the EP action handler
   now use a fresh `RSet::forward_apply_axiom(ax)` against
   current rset state, NOT the cumulative counters. The
   counters update only on verify-against-snapshot, which is
   no-op while sleeping; environmental events arriving during
   sleep wouldn't shift the gate. Fresh forward-apply makes
   the gate respond to any rset change immediately at wake-
   time — exactly what an outward drive needs.

2. **Multi-phase stream_diamond.** Three disjoint diamond
   posets drip across the 300-tick window (phases at ticks
   1-24 / 100-123 / 200-223). Each phase wakes the sleeping
   runtime, EP fires, theory/pattern discovery re-engages,
   eventually re-quiesces; next phase repeats.

F0 battery diff:
```
                  seed   pre verdict  post verdict
        stream_diamond     CONVERGED   STILL GROWING (!)
```

`stream_diamond` final state:
- patterns: 0 → **3** (discovery picks up the diamond shape
  isomorphism across phases)
- ESTABLISHED edges: 0 → **3** (C0 promotion)
- mm.tries: 6, mm.hits: 5
- 53 episodes total, lifecycle alternates Sleeping↔Running
  across phases

**The first STILL GROWING verdict in v2's history.** The
architectural analysis predicted: with outward drive +
streaming environment, the runtime would no longer terminate
at compression equilibrium. Reproducible.

Tests: unchanged (427).

### ADR 0060 (Proposed) — Phase H meta-mechanism
With prediction-error drive in place, the runtime now has a
*standard for evaluating its own decisions*. EP delta lets it
compare scheduler configurations, action sequences, even drive
mixes — a capability the compression-only drive could never
ground. Phase H opens the door to genuine self-extension.

Three sub-slices, ordered by ambition:

**H0 — Parameterized scheduler with prediction-error feedback**
(smallest viable). Wraps `RuleBasedScheduler` in an A/B
controller: two candidate configs alternate across
"evaluation windows" (default 50 episodes); end of window,
mean EP delta picks the winner; loser gets one knob mutated
within bounds. Pure parameter-space self-tuning. No new
ActionKinds, no new R relations.

**H1 — ActionKind composition discovery** (sketched, deferred).
Mine the episode log for action sequences correlated with EP
delta improvements; promote those sequences to first-class
composite ActionKinds minted at runtime. Genuinely
self-extending: the runtime's *action space* grows. Requires
sequence-mining, dispatch routing, and an identity story for
new ActionKinds.

**H2 — Self-modifying drive** (most speculative). Use EP
trajectories to evaluate whether the current drive mix
(compression + prediction-error) is the right optimization
target; potentially introduce curiosity / novelty / long-
horizon objectives.

H0 design specifics:
- New `MetaSchedulerConfig` wraps two
  `RuleBasedSchedulerConfig` candidates plus window state.
- `RuleBasedSchedulerConfig` factored out of the scheduler
  struct itself.
- Mutation: pick a knob, scale by ×0.8 / ×1.25, clamp to
  declared bounds.
- New `[meta_scheduler]` section in B2 checkpoint.
- New F0 seed `h0_drift_test` for empirics.

ADR carries 4 alternatives rejected, 6 open questions
(window size, mutation magnitude, candidate count,
multi-objective, regression reset, checkpoint compat).
Status: **Proposed**. No code yet.

Tests: unchanged (427).

### Phase H0 — meta-scheduler A/B implementation (impl)
ADR 0060 H0 lands. New `MetaScheduler` struct implements the
`Scheduler` trait, owning two `RuleBasedScheduler` candidates
plus A/B state machine:

```text
state: TestingA → TestingB → (compare means → mutate loser) → TestingA → …
```

Per-stage stats are computed lazily by scanning
`ctx.memory.episodes[stage_start..]` for
`EvaluatePredictions` action_kind and averaging deltas — no
side-channel state on Memory needed. Mutation picks one of six
tunable knobs (`min_pattern_hit_rate`,
`min_pattern_attempts_before_cooldown`, `max_zero_streak`,
`recent_window`, `min_recent_gains`, `max_mode_oscillations`)
and scales by ×0.8 or ×1.25, clamped to per-knob bounds.

Window size 50 episodes, mutation step 0.8/1.25, deterministic
PRNG seeded at construction. Single A/B pair (no tournament).
State NOT persisted across checkpoint — by design; A/B progress
restarts on each `run_bounded` invocation. (Caller can manually
reconstruct with the prior winner's config to continue.)

Tests (5 new H0):
- Initial state is `TestingA` with empty A-mean snapshot.
- Window completion (5 EP episodes) advances A → B and stores
  A's mean correctly (within 1e-12 of arithmetic mean).
- B-window completion with worse mean (0.1 < A's 0.5) mutates
  B and leaves A untouched, returning to TestingA.
- 2000-iteration mutation fuzz: every knob stays within
  declared bounds.
- Delegation test: `MetaScheduler::choose` returns
  `Execute(_)` shape from the active candidate's logic.

Tests: 427 → 432 (+5).

ADR 0060 status: Proposed → Accepted (Phase H0 implemented).
The runtime can now A/B-test scheduler configurations under
the prediction-error drive — first move toward genuine
self-extension. H1 (composite ActionKind discovery) and H2
(self-modifying drive) sketched and deferred.

### stream_diamond — sustained STILL GROWING through full HORIZON
Two changes that together produce sustained outward-drive
activity across the full 300-tick HORIZON:

1. **Six phases (was three)** at 50-tick intervals — phases at
   ticks 1/50/100/150/200/250 with 4-node disjoint diamonds.
   Every snapshot interval (50 ticks) catches at least one
   phase's activity.

2. **Episode-count-based verdict logic.** The F0 battery's
   `consecutive_idle` metric was using `pattern_count`
   stability, but patterns sometimes prune-cycle across phases
   (C0 promotes them, B3 stale-prunes later) — that's not
   idle, that's churn. Episode count monotonically grows while
   the runtime works, so it's the right activity proxy.

stream_diamond final state at tick 300:
- episodes: 0 → **89** (monotonic across all 6 snapshots)
- theories: 3 (stable after phase 1)
- ESTABLISHED edges: 3
- Verdict: **STILL GROWING** (sustained throughout 300 ticks)

This is the cleaner empirical demonstration of the
prediction-error drive's purpose: with a continuously-active
streaming environment, the runtime stays productively engaged
without drifting toward sleep. Both the mechanism (G1.5
prediction-error drive) and the verification harness (F0
battery + multi-phase stream seed) are now solid enough to
serve as the regression baseline for Phase H1+.

### ADR 0061 (Proposed) — action-sequence mining (Phase H1)
Phase H0 lets the runtime tune scheduler **parameters**.
Phase H1's harder ambition: mine the episode log for
**recurring action sequences** that correlate with positive EP
delta improvement, and promote those sequences to first-class
composite ActionKinds — genuinely growing the action space at
runtime, not just tuning thresholds.

Three sub-slices:

**H1.0** (smallest viable, mechanism-only): new `SequenceStats`
on `Memory` tracks pair-counts and post-EP-delta correlations
across consecutive episodes. Updated as a side-effect of
`execute_and_record`. No scheduler change. Round-trips through
checkpoint via new `[sequence_stats]` section.

**H1.1**: promote high-correlation pairs to meta-R via
`R(__action_seq__, seq_N)` chains; scheduler biases priority
toward sequence-suffix actions when the prefix matches the
prior episode.

**H1.2**: full composite ActionKind dispatch. Either extend
`ActionKind` with a `Composite(seq_id)` variant or introduce a
parallel `ScheduledAction` enum. Promoted sequences gain
genuine compound execution semantics, single-episode
bookkeeping. **The deepest constitutional move Phase H raises:
ActionKinds are no longer a compile-time constant.**

ADR carries 4 alternatives rejected, 6 open questions
(pair-vs-N-gram, lookahead window K, promotion sample
threshold, demotion semantics, composite identity, episode
bookkeeping granularity).

Status: **Proposed**. No code yet. H1.0 is the next viable
implementation slice; H1.1 / H1.2 wait for H1.0's empirics.

### Phase H1.0 — sequence-stats accounting (impl)
ADR 0061 H1.0 lands. New `SequenceStats` field on `Memory`:

```rust
pub struct SequenceStats {
    pub pair_counts: HashMap<(ActionKind, ActionKind), u64>,
    pub pair_post_ep_count: HashMap<(ActionKind, ActionKind), u64>,
    pub pair_post_ep_delta_sum: HashMap<(ActionKind, ActionKind), f64>,
}
```

Updated as a side-effect of `Memory::record`:
- Pair count: `(prev.action_kind, current.action_kind)` increment
  whenever a new episode arrives and a previous one exists.
- Post-EP-delta credit: when the new episode is `EvaluatePredictions`
  with `delta > 0`, look back at the last
  `H1_LOOKAHEAD_K` (= 5) episode pairs preceding the EP and
  credit each pair-occurrence with the EP's delta. Per-pair
  mean delta = `sum / count`; usable for H1.1's promotion gate.

Pure observation: scheduler decisions unchanged. Round-trips
through the B2-style checkpoint via new `[sequence_stats]`
section, mirror of the `[prediction_state]` shape (rows
`<a_kind>\t<b_kind>\t<count>\t<post_ep_count>\t<post_ep_delta_sum>`).

Tests (6 new H1.0):
- Pair counts increment correctly across 4 sequential episodes
  with mixed kinds.
- First episode creates no pair (no `prev`).
- Post-EP credit for the most recent pair within K-window.
- Negative EP delta does not credit any pair.
- Multiple pair-of-same-kind occurrences accumulate count
  correctly (per-occurrence credit, not per-pair-type).
- Non-empty SequenceStats round-trips through checkpoint —
  pair_counts / post_ep_count / post_ep_delta_sum all preserved.

Tests: 432 → 438 (+6).

ADR 0061 status: Proposed → Accepted (Phase H1.0 implemented).
The signal is now live; H1.1 (promotion to meta-R + scheduler
priority bias) and H1.2 (composite ActionKind dispatch) wait
for empirical evidence from F0-battery sequence dumps.

### Phase H1.0 — F0 sequence-stats diagnostic
New `examples/phase_h1_sequence_dump.rs` runs all 7 F0 seeds
(HORIZON=300), dumps per-seed `SequenceStats`, previews H1.1
promotion gate at strict (count≥10 mean>0.1) and relaxed
(count≥5 mean>0.05) thresholds.

Captured to `logs/2026-04-26_phase_h1_sequence_dump.log`.
Empirical findings:
- Strict thresholds: **0 promotable pairs** across all 7 seeds.
- Relaxed thresholds: **1 pair** on stream_diamond:
  `(Declarativize, Declarativize)` count=5 mean=2.46 — twice
  declarativizing in quick succession correlates with EP-delta
  improvement.
- Static-substrate seeds don't accumulate enough samples for
  any threshold.

Implication: H1.1 should use relaxed thresholds at minimum;
firing primarily on streaming substrates is expected.

### Phase H1.1 — promotion + scheduler bias (impl)
ADR 0061 H1.1 lands. Three pieces:

**1. Meta-R chain for action sequences.** New
`ACTION_SEQ_MARKER` constant in `lib.rs`. RSet gains
`action_sequence_pairs() / has_action_sequence_pair /
name_action_sequence_pair` methods. Each named pair is a
5-edge chain:
```
R(ACTION_SEQ_MARKER, seq_N)
R(seq_N, seq_N_step_0)
R(seq_N, seq_N_step_1)
R(seq_N_step_0, "<ActionKind A name>")
R(seq_N_step_1, "<ActionKind B name>")
```
`collect_meta_ids` tracks all seq + step ids under the
marker for meta-R hygiene.

**2. Auto-promotion sweep.** `execute_and_record` calls
`maybe_promote_action_sequences` after each episode. Iterates
`Memory::sequence_stats.pair_counts`; for any pair with
`count >= 5` AND `mean_post_ep_delta > 0.05`, idempotently
writes the meta-R chain. Thresholds tuned to the H1.0 dump's
empirical reality.

**3. Scheduler priority bias.** New `pick_top_biased(ctx,
accept, bonus_kinds)` returns the highest-priority item
where items whose action_kind ∈ bonus_kinds get +1.0 added.
Used in Expand mode dispatch. `h1_1_bonus_kinds(ctx)` reads
named pairs from rset, builds the suffix set whose prefix
matches the last episode's action_kind.

Empty bonus set → falls back to `pick_top` (no extra cost).

Tests (6 new H1.1):
- Idempotent name_action_sequence_pair.
- action_sequence_pairs returns all named pairs.
- Auto-promote fires at threshold (5 occurrences mean 0.5).
- Auto-promote skips below threshold (3 < 5 floor).
- bonus_kinds empty without prev; correct suffix set with
  matching prev.
- pick_top_biased correctly applies +1.0 bonus
  (Theory@5.0 vs Pattern@4.5+bonus=5.5 → Pattern wins).

Tests: 438 → 444 (+6).

ADR 0061 status: H1.0 implemented → H1.0 + H1.1 implemented.

**This is the first time the runtime encodes learned
operational knowledge as first-class meta-R facts.** The
promoted action-sequence pair sits in rset under
ACTION_SEQ_MARKER, exactly the same form as a named pattern
or theory. Scheduler reads it back at choose-time and biases
decisions accordingly — closing a loop: runtime activity →
episode log → sequence stats → auto-promotion to meta-R →
scheduler bias → runtime activity.

### Phase H1.2 — composite ActionKind dispatch (impl)
The deepest constitutional move ADR 0061 anticipated:
**ActionKind is no longer a compile-time constant**. Promoted
sequences gain genuine compound execution semantics via a new
runtime dispatch path.

Three pieces:

**1. New variants.** `ActionKind::ExecuteComposite` (no
payload, preserves Copy), `FrontierTarget::ActionSequence(
String)` (carries the seq_N id), `FrontierKind::CompositeCandidate`
(scheduler dispatch tag). Codec entries (action_kind_to_str /
parse_action_kind / target_to_pair / pair_to_target /
check_no_tab_or_newline) updated for round-trip integrity.

**2. Frontier::refresh_composite_candidates.** Runs after
all other refresh steps (it depends on what they produced).
For each named (prefix, suffix) pair in
`rset.action_sequence_pairs()`, if the current frontier has
items producing BOTH the prefix and suffix ActionKinds,
inject a CompositeCandidate item carrying
`FrontierTarget::ActionSequence(seq_id)`. Idempotent. Mid-tier
priority 1.5 (above stale-prune, below typical negative-cv
prune). The H1.1 priority bias still stacks on top via
`pick_top_biased`.

**3. execute_action ExecuteComposite arm.** Looks up the
seq_id's (prefix, suffix) ActionKinds in rset, finds matching
frontier items for each step (using their existing targets,
not synthetic defaults), runs them in order via recursive
`execute_action` calls (which don't record sub-episodes —
the composite wraps both as one), and returns the abstraction-
score delta from before-composite to after-composite as the
episode's delta.

The recursive structure means sub-actions can themselves
mutate rset (DiscoverTheory names a theory; DiscoverPatterns
names a pattern; etc.) and the composite captures the
combined effect.

Tests (8 new H1.2):
- `ActionKind::ExecuteComposite` codec round-trip.
- `FrontierTarget::ActionSequence` codec round-trip.
- `execute_for_kind` maps `CompositeCandidate` →
  `ExecuteComposite`.
- `refresh_composite_candidates` skips when no named seq
  exists.
- `refresh_composite_candidates` injects a candidate when
  named seq + matching kinds both present.
- `refresh_composite_candidates` skips when seq named but
  kinds absent from frontier.
- `refresh_composite_candidates` idempotent (no double inject).
- E2E: name a (DT, DP) pair on diamond_poset, run runtime,
  verify the composite either surfaces in the frontier or
  fires as an ExecuteComposite episode.

Tests: 444 → 452 (+8).

ADR 0061 status: H1.0 + H1.1 → H1.0 + H1.1 + H1.2 implemented.

**v2 has crossed the architectural Rubicon.** The action space
is now a function of runtime experience, not a compile-time
enumeration. A streaming substrate that produces certain
correlations will mint dispatch units the runtime author
didn't anticipate. The composite's individual steps come from
existing primitives (no new ActionKinds invented from whole
cloth), but the *combinations* are runtime-discovered. This
is the genuine self-extension move v2's goal-statement
implied.

Open questions remaining (deferred to ADR 0062 or beyond):
- Demotion: when does a promoted sequence get retracted?
- Trigram (length-3) sequences and beyond.
- Composite of composites (recursive composition).
- Cross-checkpoint persistence of the H1.x state machine.

### Phase H1.x post-impl empirical refresh
Re-running `phase_h1_sequence_dump` after H1.2 lands shows
the closing meta-loop in motion. stream_diamond diff:

```
                pre-H1.2    post-H1.2
  episodes        89          48     (composites bundle steps)
  theories         3           4
  patterns         0           4     (composite-dispatched naming)
  promotable
    (strict)       0           1     (EP-EP count=17 mean=0.19)
    (relaxed)      1           1
```

Captured to
`logs/2026-04-27_phase_h1_sequence_dump.log`.

The runtime is now simultaneously:
1. Auto-promoting (Declarativize, Declarativize) and similar
   pairs to meta-R via H1.1.
2. Dispatching them as composites via H1.2 — fewer episodes,
   richer rset state.
3. Mining new sequence stats from this richer behaviour —
   `(EvaluatePredictions, EvaluatePredictions)` now crosses
   the *strict* H1.1 threshold (count≥10 mean>0.1) where
   pre-H1.2 it didn't.

Closing meta-loop: H1.0 mines → H1.1 promotes → H1.2
dispatches → richer behaviour → H1.0 mines new pairs.

### ADR 0062 (Proposed) — sequence demotion + N-grams
With H1.x landed, two structural gaps surface:

**1. Promotion is one-way.** No mechanism retracts a
named pair when its correlation later degrades. Stale
operational knowledge accumulates.

**2. Pair-only is shallow.** Triples and longer sequences
encode richer operational patterns; pair stats see them
only in fragments.

Two sub-slices:

**H1.3 — Sequence demotion.** New
`pair_recent_post_ep_count` / `_delta_sum` fields on
`SequenceStats` track a rolling window (default 50 ticks).
When a named pair's recent mean drops below
`MIN_RECENT_MEAN_FOR_RETENTION` (0.02 — half the
promotion floor) for ≥ 3 occurrences, demotion sweep
retracts the meta-R chain via new
`RSet::retract_action_sequence_pair`. Asymmetric
thresholds (promote 0.05 / demote 0.02) provide hysteresis
to avoid promote/demote oscillation.

**H1.4 — Trigram support.** Extend `SequenceStats` to
track triples; new `name_action_sequence_triple` mints a
7-edge meta-R chain. Composite dispatch (H1.2) loops over
N steps so it scales naturally. Trigram thresholds tighter
than pair (count ≥ 3 / mean > 0.10) — trigrams accumulate
slower but each occurrence carries stronger signal.

H1.3 prioritised over H1.4 (correctness > expressivity).
H1.4's design includes a backward-compat plan: keep
`action_sequence_pairs` alongside a new
`action_sequences() -> Vec<(seq_id, Vec<String>)>` to ease
migration.

ADR 0062 carries 4 alternatives rejected, 5 open
questions. Status: **Proposed**. No code yet.

H1.3 / H1.4 will close the operational-self-extension
loop further: not just *grow* the action space (H1.2 does
that), but also *prune* stale entries (H1.3) and *expand
expressivity* (H1.4).

### Phase H1.3 — sequence demotion (impl)
ADR 0062 H1.3 lands. Three pieces:

**1. Recent-window stats.** `SequenceStats` gains
`pair_recent_post_ep_count` / `_delta_sum` /
`last_recent_reset_tick`. Updated in lockstep with the
cumulative counters in `Memory::record`. After the new
episode records its credit, if `current_tick >=
last_recent_reset_tick + 50`, recent counters clear (the
just-recorded credit survives one tick) and
`last_recent_reset_tick` advances.

**2. Demotion via rset retract.** New
`RSet::retract_action_sequence_pair(prefix, suffix)`
removes the 5-edge meta-R chain. Returns count of edges
removed (0 if not named). Idempotent.

**3. Auto-demotion sweep.** New
`maybe_demote_action_sequences` runs alongside
`maybe_promote_action_sequences` after each episode.
Iterates `rset.action_sequence_pairs()`; for each named
pair whose recent count ≥ 3 AND recent mean < 0.02,
retracts the chain.

**Asymmetric thresholds**: promote 0.05 / demote 0.02.
This 2× hysteresis prevents promote/demote oscillation —
a pair whose mean drifts in the dead zone (0.02–0.05)
keeps its meta-R status until evidence clearly degrades.

`pair_recent_mean_post_ep_delta(pair)` is the new
accessor; mirrors the cumulative version.

Tests (7 new H1.3):
- Recent-window auto-resets when current tick crosses
  `last_recent_reset_tick + 50` boundary.
- `pair_recent_mean_post_ep_delta` returns the right mean
  for a single-occurrence-single-EP setup.
- Demote retracts a pair with low recent mean (recent
  count=3 mean=0.01).
- Demote skips a pair with healthy recent mean (recent
  count=5 mean=0.5).
- Demote skips when recent count is below 3 floor.
- `retract_action_sequence_pair` removes 5 edges; second
  call removes 0 (idempotent).
- Hysteresis: a pair just promoted at mean 0.10 with
  recent mean drifting to 0.04 (dead zone) does NOT get
  demoted.

Tests: 452 → 459 (+7).

ADR 0062 status: Proposed → Accepted (H1.3 implemented;
H1.4 sketched).

**Recent-window stats are NOT yet round-tripped through
checkpoint** — deferred for now. The cumulative counters
do persist; demotion candidacy resets on restore but
promotions survive. Acceptable for the current
streaming-substrate workload; if longer-cycle restart
patterns surface, revisit. Trigram support (H1.4) remains
the next major slice.

### Phase H1.4 — trigram support (impl)
ADR 0062 H1.4 lands. Three pieces:

**1. Triple stats.** `SequenceStats` gains
`triple_counts` / `triple_post_ep_count` /
`triple_post_ep_delta_sum` keyed on
`(ActionKind, ActionKind, ActionKind)`. `Memory::record`
increments triple_counts when a third predecessor exists
(after push, `len >= 3`); EP credits triples in the
K-lookahead window same way it credits pairs. New
`triple_mean_post_ep_delta` accessor.

**2. Triple meta-R chain.** `RSet` gains
`action_sequence_triples()`,
`has_action_sequence_triple(a, b, c)`,
`name_action_sequence_triple(a, b, c)` (idempotent),
`retract_action_sequence_triple(a, b, c)`. The chain is
7 edges:
```
R(ACTION_SEQ_MARKER, seq_N)
R(seq_N, seq_N_step_0)
R(seq_N, seq_N_step_1)
R(seq_N, seq_N_step_2)
R(seq_N_step_0, "<a>")
R(seq_N_step_1, "<b>")
R(seq_N_step_2, "<c>")
```

`action_sequence_pairs()` filters out anything with a
`step_2` so pair and triple APIs are disjoint. Existing
H1.1/H1.2/H1.3 callers that walked pairs continue to do
so; triples are accessed via the parallel API per ADR
suggestion.

**3. N-step composite dispatch.**
`Frontier::refresh_composite_candidates` now also walks
`action_sequence_triples()` and surfaces a CompositeCandidate
for any triple whose three step kinds are all currently
represented in the frontier.
`execute_action::ExecuteComposite` collects the step
ActionKinds (length 2 OR 3 depending on which API
matches), snapshots targets up front, and runs each step
in order via recursive `execute_action`. The episode's
delta is the abstraction-score change across the whole
composite (same semantic as H1.2 for pairs).

`maybe_promote_action_sequences` now extends with a triple
sweep at TIGHTER thresholds: `count >= 3` AND
`mean > 0.10` (vs pair's 5 / 0.05). Triples accumulate
slower; each occurrence carries stronger signal.

Tests (7 new H1.4):
- `triple_counts` increments correctly across 5 episodes
  producing 3 triples.
- Post-EP credit reaches the triple immediately preceding
  a positive-delta EP.
- `name_action_sequence_triple` idempotent; distinct
  triples get distinct seq_ids.
- `action_sequence_pairs` and `action_sequence_triples`
  return disjoint sets (pair filters out anything with
  step_2; triple requires all three).
- `retract_action_sequence_triple` removes 7 edges;
  idempotent.
- Auto-promote triple fires at count=3 mean=0.5.
- `refresh_composite_candidates` surfaces a triple
  CompositeCandidate when a triple is named AND all three
  step kinds are represented in the frontier.

Tests: 459 → 466 (+7).

ADR 0062 status: H1.3 → H1.3 + H1.4 implemented.

The H1 suite is now feature-complete: H1.0 mines pairs +
triples, H1.1 promotes both to meta-R, H1.2 dispatches
both as N-step composites, H1.3 demotes (pairs only — triple
demotion follows the same pattern but isn't load-bearing
yet at v2 scale; defer).

### Phase H1.x — long-run empirics (HORIZON=2000)

The 2026-04-27 retrospective named "long-run empirical
cycle" as the #1 next direction. Built a multi-regime
streaming environment (4 regimes × 500 ticks):

- A (1–490): diamond posets, 5 phases.
- B (501–990): bipartite 2×3 injections, 5 phases.
- C (1001–1490): clique families (equivalence classes), 5
  phases.
- D (1501–1990): diamonds + interleaved
  `R(PATTERN_MARKER, x)` injections, 5 phases.

Snapshots every 200 ticks; per-snapshot diff against the
prior named-pair / named-triple sets surfaces promotions
and demotions as discrete events.

Captured `examples/phase_h1_long_run.rs` →
`logs/2026-04-27_phase_h1_long_run.log`.

#### Empirical findings

The full-window trajectory:

```
 tick  pat  thy   est  shAx   epis     ep   pairs  tri  lifecycle
    0    0    0     0     0      0      0       0    0  Running
  200    3    4     0     7     39     15       1    1  Sleeping
  400    4    4     0     7     48     23       0    1  Sleeping  -pair
  600    4    5     0     7     49     23       1    1  Sleeping  +pair
  800    4    5     0     7     49     23       1    1  Sleeping
 ...    (idle through ticks 800–1400) ...
 1600    5    5     1     7     49     23       1    1  Sleeping
 2000    9    5     5     7     49     23       1    1  Sleeping
```

Three observations:

1. **First empirical demotion fire on real substrate.** At
   tick 400 the (EvaluatePredictions, EvaluatePredictions)
   pair was demoted by H1.3 — recent-window mean degraded
   below 0.02 floor as runtime quiesced post-regime-A.
   Tick 600 re-promotion: regime-B's first edges produced
   enough fresh EP activity to flip the pair back over the
   promotion threshold. This is hysteresis (promote 0.05 /
   demote 0.02) working as designed in production.

2. **Composite dispatch never fires.** `comp` column stays 0
   throughout, despite (EP, EP) being named. The
   `CompositeCandidate` frontier requires both step-kinds
   to be represented in the live frontier when refresh runs;
   under the prediction-error gate, EP is the only fired
   action during the dormant phases, so the candidate never
   gets surfaced when the runtime is awake enough to dispatch.
   Empirical reality: composite dispatch needs the runtime
   to *also* be doing other things alongside EP, and these
   substrates didn't produce that overlap.

3. **Regime B/C inert.** The runtime sleeps from ~tick 200
   through ~tick 1500. Bipartite + clique injections don't
   tickle the prediction-error wake gate because (a) they
   don't establish patterns/axioms that change forward-apply
   output, and (b) the G1.5 outward drive measures EP-delta
   on existing axioms, of which there are few applicable to
   bare bipartite edges. The runtime only re-stirs in regime
   D when `R(PATTERN_MARKER, d_X)` edges directly mutate
   `rset.patterns()`, which then triggers establishment-
   chain accounting.

#### Significance

Finding #1 is the core empirical confirmation of H1.3 — the
demotion machinery does what the ADR claimed on a real
substrate, not just synthetic tests.

Finding #2 is a real gap: composite dispatch is implemented
but the substrates that exercise H1.x's promotion path do
not naturally produce frontiers where composite candidates
are eligible to fire. Either (a) the F0 substrates don't
produce enough action variety, or (b) the
`refresh_composite_candidates` eligibility check is over-
strict. Worth a closer look in a future slice — but not a
blocker.

Finding #3 reframes a known issue: the prediction-error
drive is gated on changes to forward-apply output, which
many edge classes don't trigger. Long-run stability
across regime shifts is bounded by what wakes the runtime
in the first place. The non-stationarity of the test
environment was real but the runtime didn't engage with it,
because the new edges weren't predictively interesting under
the current axiom store.

#### Verdict

Long-run does what the retrospective wanted: it surfaces
empirical findings the F0 battery (300-tick HORIZON) was
too short to expose. H1.3 demotion is verified live (good).
Composite dispatch and regime-shift wake behaviour are
identified gaps (useful empirical input for future slices).

No code changes from this run. Captured as a one-shot
empirical report.

Commits to follow.

### Phase H1.x — triple demotion (ADR 0062 retro #2)

The 2026-04-27 retrospective noted that H1.3 demoted pairs
only — triple demotion was deferred as "same mechanism with
the larger map". Followed up with the small slice.

Changes (extending ADR 0062):

- `SequenceStats` gains `triple_recent_post_ep_count` and
  `triple_recent_post_ep_delta_sum` mirrors of the pair
  recent-window fields.
- `SequenceStats::triple_recent_mean_post_ep_delta` accessor.
- `reset_recent_window` clears both pair and triple recent
  counters on the same `H1_3_RECENT_WINDOW_TICKS` (50) tick
  boundary.
- `Memory::record` triple-credit loop now writes both
  cumulative AND recent counters (mirrors what the pair
  loop does for H1.3).
- `maybe_demote_action_sequences` extended: after the pair
  pass, iterate `rset.action_sequence_triples()` and apply
  the same gate (recent count ≥ 3 AND recent mean < 0.02);
  retract via `RSet::retract_action_sequence_triple`.

No new constants; reuses `MIN_RECENT_COUNT_FOR_DEMOTE = 3`
and `MIN_RECENT_MEAN_FOR_RETENTION = 0.02` from H1.3 —
identical hysteresis curve for both pair and triple
demotion (asymmetric vs. promotion's mean > 0.05 floor).

Tests: 466 → 470 (+4):
- `h1_3_triple_demote_retracts_named_triple_with_low_recent_mean`
- `h1_3_triple_demote_skips_with_healthy_recent_mean`
- `h1_3_triple_demote_skips_when_recent_count_below_floor`
- `h1_3_reset_recent_window_clears_triple_counters`

ADR 0062 status: H1.3 + H1.4 + triple-demotion follow-up
implemented. The H1 suite is now strictly feature-complete
across both promotion and demotion for both pair and triple
sequences.

### Phase H1.x — composite dispatch EP gap fix (ADR 0062 retro #3)

The 2026-04-27 long-run flagged composite dispatch firing
0 times across the whole 2000-tick window despite (EP, EP)
being named. Diagnosed root cause:

`EvaluatePredictions` is dispatched outside the frontier —
the scheduler's special anti-stagnation path
([mod.rs:583](../src/runtime/mod.rs)) fires EP when
`zero_streak >= max_zero_streak` AND axioms exist AND
predictions have pending delta. **No `FrontierKind` maps
to `EvaluatePredictions`** via `execute_for_kind` — the 7
existing FrontierKind variants cover the other 7
ActionKinds, but EP is structurally non-frontier.

Two consequences:

1. `Frontier::refresh_composite_candidates` builds
   `kinds_present` by mapping each frontier item's kind
   through `execute_for_kind`. EP is never in that set,
   so any pair containing EP fails the eligibility gate
   (`kinds.iter().any(|k| !kinds_present.contains(k))`
   always true). On stream-shaped substrates, EP-EP is
   the dominant promoted pair; CompositeCandidate never
   surfaces.
2. Even if it did surface, the `ExecuteComposite` arm
   collects step targets via
   `find(|it| execute_for_kind_static(it.kind) == k)`,
   which returns `None` for EP steps → step skipped →
   composite is a no-op for any EP-containing pair.

Fix (small, targeted):

- `refresh_composite_candidates` inserts
  `EvaluatePredictions` into `kinds_present` unconditionally
  (universally dispatchable; the scheduler's own EP gating
  controls when the synthetic step actually fires).
- `ExecuteComposite` arm short-circuits the frontier-item
  lookup when the step kind is EP and synthesizes
  `FrontierTarget::WholeRSet` directly.

Tests: 470 → 472 (+2):
- `retro3_composite_candidate_surfaces_for_ep_pair`
- `retro3_execute_composite_dispatches_ep_via_whole_rset`

Empirical impact — long-run rerun
([logs/2026-04-27_phase_h1_long_run_postfix.log](../logs/2026-04-27_phase_h1_long_run_postfix.log)):

| metric | pre-fix | post-fix | Δ |
|---|---|---|---|
| episodes | 49 | 268 | +5.5× |
| EP attempts | 23 | 129 | +5.6× |
| composite attempts | 0 | 1 | first fire |
| pairs currently named | 1 | 4 | +3 |
| triples currently named | 1 | 8 | +7 |
| pair demotions ever | 0 | 1 | first non-(EP,EP) demote |
| triple demotions ever | 0 | 1 | first triple demote on real substrate |

Notable: post-fix the runtime discovers diverse non-EP
sequences — `(Declarativize, Declarativize, Declarativize)`,
`(PruneLowValueObjects, EvaluatePredictions, PruneLowValueObjects)`,
etc. These were inaccessible pre-fix because the runtime
quiesced before stringing more than two action kinds
together. The EP-frontier-eligibility gap was load-bearing —
it was preventing the H1.x machinery from engaging at scale,
not the substrate.

This is the kind of finding the long-run was meant to
surface. ADR 0062 status augmented with retrospective
finding #3 implemented.

### Phase H1.x — long-run finding #3 diagnosis (no fix needed)

The 2026-04-27 long-run also flagged "regime B/C inert" —
the runtime appearing to sleep through ticks 200–1500
despite scheduled AddEdge events firing throughout. After
the finding-#2 fix, this dissolves: it was a downstream
artifact, not a wake-gate problem.

`should_wake` ([mod.rs:921](../src/runtime/mod.rs))
returns true for any `AddEdge` / `RemoveEdge` event — wake
gate itself is intact. The pre-fix observation was caused
by finding #2: the runtime would wake on regime-B/C edges,
but with no productive composite dispatch available and
sequence-mining stuck on (EP, EP) only, mode-thrash
penalty kicked in and the runtime returned to sleep
quickly enough that snapshot intervals (200 ticks) caught
it sleeping.

Empirical evidence — episodes-per-200-tick interval, post
finding-#2 fix:

| interval | regime | post-fix Δepisodes |
|---|---|---|
| 200–400 | A end | +14 |
| 400–600 | A→B | +33 |
| 600–800 | B mid | +12 |
| 800–1000 | B end | +12 |
| 1000–1200 | C start | +32 |
| 1200–1400 | C→D | +34 |
| 1400–1600 | D | +32 |
| 1600–1800 | D end | +30 |
| 1800–2000 | post-inj | +30 |

(Pre-fix, intervals 200–1400 totaled +1 episode.)

Conclusion: regime B/C now engage normally. No wake-gate
or prediction-error-drive change required. Long-run
finding #3 is **closed without code change** — the fix for
finding #2 covers it.

The composition of two findings into one fix is itself
an instructive trace. Long-run #2 surfaced as "composite
dispatch never fires"; long-run #3 surfaced as "regimes B/C
appear inert." Both pointed at the same architectural
defect (EP not in frontier). The diagnostic value of the
2000-tick window vs the 300-tick F0 battery: longer
substrates expose the *secondary* consequences of an
architectural gap, not just the gap itself.

### Phase H2.0 step 1 — Drive trait + 3 baseline impls (impl)

ADR 0063 status: Proposed → Accepted (step 1).

Phased implementation strategy: H2.0 in this ADR is
substantial (trait + impls + DriveMix A/B + checkpoint +
wake-gate refactor). The retrospective explicitly flagged
H2 as the highest-risk phase. Splitting into steps to
contain integration risk:

- **Step 1 (this commit)** — `Drive` trait + 3 baseline
  impls (`CompressionDrive`, `PredictionErrorDrive`,
  `ModeThrashPenalty`). Each impl computes a scalar from
  the same observables the existing scheduler consults.
  Shadow-only: nothing currently reads these values.
- **Step 2 (next slice)** — `DriveMix` struct with weight
  storage + A/B mutation cycle + checkpoint round-trip +
  episode-recording hook.
- **Step 3 (deferred)** — wake/mode/sleep gate refactor
  to read from DriveMix.combined_signal. The riskiest
  step; gated on step 2 empirics.

#### Step 1 changes

`Drive` trait:

```rust
pub trait Drive {
    fn id(&self) -> &'static str;
    fn evaluate(
        &self,
        rset: &RSet,
        memory: &Memory,
        tick: u64,
    ) -> f64;
}
```

Three impls (read-only; no internal state):

- `CompressionDrive` (id `"compression"`) — mean of
  positive-delta episodes over the last K=10. Saturates as
  rset compresses; mirror of the implicit signal that gates
  the existing productive-vs-stagnant decision.
- `PredictionErrorDrive` (id `"prediction_error"`) — sum of
  `|hit_rate_now - hit_rate_prev|` across named axioms.
  Scalar version of `predictions_have_pending_delta` (the
  G1.5 boolean gate).
- `ModeThrashPenalty` (id `"mode_thrash"`) — count of
  mode transitions in the last K=20 (caller weighs
  negatively in the blend).

#### What step 1 does NOT do

- Does NOT add `DriveMix` (deferred to step 2).
- Does NOT change wake/mode/sleep behaviour (deferred to
  step 3).
- Does NOT touch the checkpoint (no new fields to round-
  trip yet).
- Does NOT extend `Memory::record` (no DriveMix counters
  to update yet).

In other words: shadow code that's invocable but not
invoked. This makes step 1 risk-free against the existing
479-test suite.

#### Tests: 472 → 479 (+7)

- `h2_0_compression_drive_returns_zero_with_empty_memory`
- `h2_0_compression_drive_averages_recent_positive_deltas`
- `h2_0_compression_drive_ignores_negative_deltas`
- `h2_0_prediction_error_drive_returns_zero_with_no_axioms`
- `h2_0_prediction_error_drive_returns_positive_with_pending_delta`
- `h2_0_mode_thrash_penalty_counts_recent_transitions`
- `h2_0_drive_ids_are_stable_and_distinct`

The drive-id stability test is load-bearing: ids
(`"compression"`, `"prediction_error"`, `"mode_thrash"`)
will become checkpoint keys in step 2; renaming them later
would silently break round-trip on existing checkpoints.
The test pins them at step 1.

#### Constitutional check

H2.0 step 1 introduces:
- A trait (`Drive`) and 3 unit-struct impls — no R
  relations.
- No new meta-R class — drives are compile-time Rust
  constructs, not registered in rset.
- No new identifiers or relations on rset.

All five v2 commitments PASS by construction (same as the
ADR 0063 H2.0 review predicted: drive-as-type is deferred
to H2.1).



### ADR 0063 (Proposed) — drive self-modification (Phase H2)

The 2026-04-27 retrospective listed "H2 ADR drafting" as
the #4 priority direction with the framing: H2 has more
potential for getting wrong than any prior phase, so a
careful design ahead of any code is the load-bearing move.

Drafted [`0063-drive-self-modification.md`](decisions/0063-drive-self-modification.md).
Status: Proposed; no code yet.

#### Three slices, ordered by ambition

- **H2.0** — multi-drive blend (compression / prediction-
  error / mode-thrash) with weights mutated under EP-delta
  feedback. Mirror of H0's MetaScheduler design, scaled to
  weight space. Drive function bodies remain compile-time;
  only weights mutate. Smallest viable slice.
- **H2.1** — drives registered as meta-R objects under a
  new `DRIVE_MARKER` class, with the existing ESTABLISHED-
  promotion / demotion lifecycle applied to the drive set
  (same shape as PATTERN, AXIOM, THEORY chains). Drive set
  becomes self-managing.
- **H2.2** — drive synthesis from a small grammar over
  primitive metrics (mean / variance / ratio / lag-diff).
  Synthesized candidates enter the H2.1 lifecycle; the
  ESTABLISHED-via-EP-delta gate decides which actually
  contribute. Research-mode; out of scope for this ADR's
  commit.

#### Constitutional review

The ADR scores all five v2 commitments against H2.0
explicitly:

- Commitments 1, 2, 4, 5: PASS by construction (no new R
  class, no new relations, drive ids are compile-time
  string constants, no similarity claim made).
- Commitment 3 (types are meta-R instances): H2.0
  *constrains itself* to compile-time drive identities,
  deferring drive-as-type to H2.1. PASS — but called out
  as the load-bearing constraint.

For H2.1, commitment 3 becomes the hinge: drives must
register under DRIVE_MARKER as `R(DRIVE_MARKER, drive_X)`
(same shape as PATTERN_MARKER chains). Constitution-
compatible by construction.

For H2.2, commitments 3 and 4 both get tested. Synthesized
drives need (a) deterministic, structural identifiers
(commitment 4 — proposed: hash of the composition
expression), (b) registration as meta-R (commitment 3).
Neither is broken if the synthesizer follows these
constraints. The ADR flags this as the highest-risk
constitutional surface and recommends careful identifier
design before any H2.2 code.

#### Why H2.0 is the recommended starting slice

Per the ADR's "alternatives considered":

- Smallest implementation surface (`Drive` trait + 3
  baseline impls + `DriveMix` parallel to `MetaScheduler`).
- Reuses H0's A/B-mutation pattern unchanged — the
  empirical risk profile is well-understood.
- Constitutionally clean (deferred drive-as-type via
  commitment 3).
- H2.0 produces validation data that H2.1 needs (does
  weight-tuning even matter empirically? if not, H2.1's
  promotion machinery has nothing to bite on).

#### Open questions raised in the ADR

1. Initial weights — hand-tuned baseline vs. all-equal.
2. Window size sensitivity (50 episodes/window from H0;
   may need shorter for drive responsiveness).
3. Mutation step magnitude (×0.8 / ×1.25 from H0 vs.
   additive ±0.1 — drive weights live in [0, 1] so
   different scale).
4. Negative drives (penalties as negative `evaluate`
   return values vs. negative weights).
5. Interaction with H0's MetaScheduler — two A/B loops
   on the same EP-delta signal need phase-shifted
   windows to avoid stepping on each other.

These are all empirical questions; the ADR recommends
deciding on adoption rather than ahead of code.

#### Status

ADR 0063 is **Proposed**, not Accepted. The retrospective
called for "a careful ADR before any implementation"; this
file is that ADR. Acceptance + H2.0 implementation is a
future commit when the user signals readiness.

