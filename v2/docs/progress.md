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
