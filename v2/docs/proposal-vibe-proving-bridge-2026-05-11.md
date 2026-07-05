# Proposal: vibe-proving as RSet data source (2026-05-11)

Status: **Closed 2026-07-06 per [ADR 0084](decisions/0084-direction-housekeeping.md).**
Promoted to [ADR 0081](decisions/0081-vibe-proving-bridge.md) on 2026-05-11; Phase 0
ran on a synthetic stand-in and the headline substrate-sensitivity claim was
retracted through the 8/9-round ARIS review arc (see retrospective-2026-05-19).
The surviving scientific content — v2 on naturally-occurring math graphs — is
exactly N1 (Phase 1.E real Mathlib) in forward-directions-2026-05-19, which needs
no vibe-proving integration (this proposal's own Option A). Reopen = explicit
user commitment to N1's multi-week scope; N1 inherits this proposal's
pre-registered go/no-go/abort criteria, including the tokenization-leak abort test.

Original text below, unchanged.

---

Status at time of writing: Proposed (not yet an ADR)
Author trigger: cross-project investigation — `E:/projects/vibe-proving-math`
analyzed 2026-05-11 for whether it intersects v2.

## TL;DR

vibe-proving-math is an LLM-driven math research assistant. Its底层 cannot
be replaced by Relatum (they point in opposite information-flow directions).
But vibe-proving touches several **already-structured math corpora** —
arXiv citation graphs via TheoremSearch, Lean dependency graphs via Aristotle,
proof-step DAGs via its own Generator output. With a thin extractor those
become **anonymized binary directed graphs** — exactly what v2 consumes.

Proposal: build a one-way ETL bridge (vibe-proving → RSet TSV per ADR 0038).
Run [`autonomous_pass`](README.md:64) + [`discover_axioms_minimal`](README.md:99)
+ [`discover_theory`](README.md:124) on the result. See whether the patterns,
axioms, and theories Relatum discovers on naturally-occurring math graphs
are different in kind from those it discovers on the canonical-mixed
synthetic graph the v2 suite has used to date.

This is a Phase 0 probe (per [practice 2](practices.md:20)), not a commitment.
Go/no-go after ~half-day of bridge code + one `autonomous_pass`.

## Premise

v2's empirical surface has been synthetic: target_size=3 mixed graphs,
8-case blind axiom batteries (equivalence / tolerance / total order / etc.),
hand-constructed posets. These are sufficient to verify mechanisms but
do not tell us what v2 **says** about a graph whose structure was not
designed by us.

vibe-proving-math is a convenient bridge to such graphs because:

- TheoremSearch ([app/core/theorem_search.py](../../vibe-proving-math/app/core/theorem_search.py))
  indexes ~9M theorems with citation/dependency metadata.
- The reviewer mode already parses citation patterns
  ([app/modes/research/reviewer.py:116](../../vibe-proving-math/app/modes/research/reviewer.py:116))
  out of LaTeX/PDF papers — citation edges are essentially free output.
- The Aristotle integration ([app/core/aristotle_client.py](../../vibe-proving-math/app/core/aristotle_client.py))
  speaks to Lean 4, which exposes per-lemma dependency edges as a first-class
  notion. No NLP required for that path.

The structural payload of these sources — strip the names, keep the edges —
is binary, directed, and free of external labels at the edge level. That is
what an RSet is.

## Hypotheses

Three independent things this probe might show, in decreasing order of
expected likelihood:

1. **H1 — Naturally-occurring math graphs have non-trivial structural
   density.** [`discover_motifs`](README.md:60) on a ~10³-edge citation
   subgraph finds asymmetric motifs that the symmetric `compound_class`
   heuristic misses, validating ADR 0016 on a non-synthetic substrate.

2. **H2 — `discover_axioms_minimal` finds a non-empty axiom bundle on
   citation graphs.** Plausible candidates: transitivity (if A cites B and
   B cites C, A tends to also cite C — "ancestor citation") or its
   defeasible form via ADR 0033. If found, the **rate** itself is
   data-of-interest: how transitive *is* mathematical citation in
   practice?

3. **H3 — Theories discovered on Lean dependency graphs are
   structurally distinguishable from theories on citation graphs.**
   [`discover_theory`](README.md:124) fingerprint comparison between
   two substrates of the same domain (e.g. group theory papers vs.
   `Mathlib.GroupTheory`) tests whether structural axiomatic content
   transfers across representation.

Each hypothesis has a clear null. H1 nulls if motifs are uniform random
or all symmetric; H2 nulls if no axiom in the template space holds at
strict rate or with defensible rate ≥ 0.5; H3 nulls if fingerprints
overlap.

## Constitution check

Per [practice 3](practices.md:30), each commitment, against this proposal:

- **C1 (R is singular).** Loaded edges are all the same R. The mapping
  table that says "edge in citation file means 'cites'" lives in the
  bridge tool, not in the RSet. ✓
- **C2 (R is binary).** Citation / dependency / proof-step relations
  are natively binary. ✓
- **C3 (types are meta-R).** No pre-loaded types. Any type Relatum
  emits comes from its own `consider_naming` / `name_theory` /
  `register_axiom_with_intension` paths. ✓
- **C4 (token-based identity).** Tokens are opaque IDs assigned by
  the bridge (`thm_00041`, `lem_mathlib_03128`). No structural
  deduplication at load time — two distinct arXiv IDs yielding
  isomorphic neighborhoods stay distinct in the RSet, as required.
  ✓
- **C5 (similarity is structural).** No TheoremSearch similarity
  scores cross the bridge. Only edges. ✓

Concern flagged for the strict reading at [constitution.md:79](constitution.md:79):
the **bridge's tokenization decision** — what counts as one node, what
counts as one edge — is itself a semantic act. It is performed *outside*
Relatum, before any token enters the RSet. Once tokens enter, they are
opaque. This matches the "scaffolding inside an atomic act" pattern: the
atomic act is "construct an RSet from corpus C with policy P", and P is
recorded with the dataset so the choice is traceable per
[practice 1](practices.md:8). No violation, but record P.

## Phase 0 — minimum experiment

**Scope**: one corpus, one policy, one autonomous_pass. No bridge into
vibe-proving's runtime. A throwaway extractor reading TheoremSearch's
JSON or a Mathlib dependency dump.

**Steps**:

1. Pick the substrate with the cleanest edge semantics:
   - Option A — **Mathlib dependency graph.** Lean 4 already records
     "lemma X's proof body refers to lemma Y" as a typed edge.
     Reachable from vibe-proving via its Aristotle integration but
     also extractable independently from a local Mathlib checkout.
   - Option B — **arXiv math citation graph for one
     subject class.** Needs vibe-proving's reviewer parser plus the
     TheoremSearch index. Higher noise, richer semantics.
   - **Recommended start: A.** Edge semantics are unambiguous,
     extraction is a `lake env` invocation, no LLM needed.

2. Subsample to ~10³ nodes / ~10⁴ edges. v2's tested scale is small
   (target_size=3 in the canonical suite); jumping to 10⁶ is a
   separate question.

3. Emit a TSV per ADR 0038 format:

   ```
   # mathlib.GroupTheory subgraph, depth-2 from `Group.mul_left_cancel`
   # extracted 2026-05-11 by relatum-bridge v0.1 policy=lemma_deps
   lem_001	lem_002
   lem_001	lem_017
   lem_002	lem_017
   …
   ```

4. Load via `RSet::from_text` (ADR 0038). Verify edge count matches
   extraction count (basic sanity).

5. Run `autonomous_pass` at target_size=3, then target_size=4.
   Record discovered patterns and their MDL gains.

6. Run `discover_axioms_minimal` (strict mode), then
   `discover_axioms_minimal_compositional` (ADR 0037).
   Record any axiom that holds.

7. Run `discover_theory`. Record theory fingerprint.

8. Write log entry to `logs/2026-MM-DD_vibe-proving-mathlib-probe.log`
   per [practice 1](practices.md:8).

**Go signal**: any of H1 / H2 / H3 produces a non-trivial result that
would not have appeared on the canonical mixed graph. Then promote
to ADR and Phase 1.

**No-go signal**: outputs are degenerate (no patterns named, no
axioms hold, theory fingerprint matches a synthetic case identically).
Then write a result doc explaining why and stop.

## Phase 1 — if Phase 0 has signal

Three independent expansions, each its own ADR:

- **P1.a — Cross-substrate fingerprint comparison.** Repeat Phase 0
  on Option B (arXiv citation graph for one math arXiv subject
  class, e.g. `math.GR`). Compare theory fingerprints across A and B.
  Does the same mathematical area look structurally different in
  citation vs. formal-dependency form?

- **P1.b — Counterfactual ranking on a natural graph.** Run
  [`rank_by_counterfactual`](README.md:174) (ADR 0035) on the named
  objects from Phase 0. Which discovered patterns / theories
  contribute most to `abstraction_score` on a natural-occurring graph?

- **P1.c — Defeasible axiom rate distribution.** Lower
  [`AxiomDiscoveryConfig::min_rate`](README.md:159) (ADR 0033) and
  scan rate ∈ {0.95, 0.9, 0.8, 0.5} on the Phase 0 substrate. Plot
  axiom count vs. min_rate. The shape of this curve characterizes
  the substrate.

None of these requires changes to v2 core — they are configuration
experiments on existing mechanisms applied to a new input.

## Success / failure criteria

This proposal **succeeds** if Phase 0 produces at least one of:

- A discovered pattern with positive MDL gain on the natural substrate
  that has no analog in the synthetic suite.
- An axiom (strict or defeasible) holding on the natural substrate.
- A theory fingerprint structurally distinguishable from any of the
  8 canonical battery cases.

It **fails** (legitimately, write up and stop) if all three are
degenerate. Failure is informative: it would suggest v2's discovery
machinery is calibrated to a class of structure that mathematical
citation/dependency graphs do not exemplify. That is a real fact
about v2's coverage worth knowing.

It **aborts** (revisit before continuing) if:

- The bridge's tokenization policy turns out to do work that should
  have been Relatum's. Symptom: changing the policy changes the
  discovered pattern set dramatically. If the choice of node
  granularity decides what gets discovered, the bridge is no longer
  scaffolding — it is leaking into the result.
- Scale becomes the bottleneck before semantic signal appears.
  Symptom: `autonomous_pass` takes hours on 10³ nodes. v2 may need
  the deferred sampling-based `find_instances_of` replacement
  (open direction in [README:204](README.md:204)) before this is
  practical.

## Open questions

- **Node granularity for proof-step DAGs.** vibe-proving's Generator
  produces natural-language proofs. Decomposing into "atomic steps"
  is a tokenization decision that may not be stable across runs of
  the same proof. Defer until Option A signal is known.
- **Cross-corpus token identity.** If both a Mathlib lemma and an
  arXiv theorem represent "the same" result, do we keep them as
  distinct tokens (per C4) or unify them via meta-R? C4 says distinct;
  the meta-R route would let Relatum *discover* the equivalence
  through a discovered pattern. Preferred answer: distinct, let
  Relatum find it. But this is testable.
- **What does Relatum write back?** None of Phase 0 / Phase 1 writes
  anything back to vibe-proving. The bridge is one-way. If discovered
  patterns are interesting, surfacing them to vibe-proving users is
  a separate question — and probably the wrong question, since
  vibe-proving's users want answers about theorems, not about graph
  motifs.

## Non-goals

- **Not replacing vibe-proving's底层.** Established as infeasible in
  the analysis that triggered this proposal. The bridge is unidirectional
  and stays that way.
- **Not making v2 dependent on vibe-proving.** The bridge tool lives
  outside the v2 crate. v2 only sees TSV files. If vibe-proving is
  unavailable or deleted, v2 continues working on its existing suite.
- **Not embedding LLM judgment into Relatum.** No similarity scores,
  no semantic labels, no LLM-generated tokens. The LLM is upstream
  of the bridge boundary, not downstream.
- **Not productizing.** This is an experimental probe to find out
  what v2 says about non-synthetic structure. If results are
  publishable as a research artifact, that is the deliverable.
  No "feature" for vibe-proving users is implied.

## Implementation sketch

Single-file Rust binary, ~200 LOC, in `v2/examples/bridge_mathlib_probe.rs`
(if Option A) or external repo (if scope grows). Pipeline:

```
mathlib checkout
  │
  ▼ (lake env --print-paths + custom Lean meta script)
[lemma_id → [dep_lemma_id]] JSON
  │
  ▼ (Rust extractor, anonymizes ids to lem_NNNNN)
mathlib_subgraph.rset (TSV per ADR 0038)
  │
  ▼ (existing v2 lib)
RSet::from_text → autonomous_pass → discover_axioms → discover_theory
  │
  ▼
logs/2026-MM-DD_vibe-proving-mathlib-probe.log
```

The extractor's policy file (node-granularity, depth, anonymization
seed) is committed alongside the .rset so Phase 1's reproductions
are deterministic per ADR 0038's "cross-machine reproduction"
property.

Estimated cost: half a day to first `autonomous_pass` output, given
a local Mathlib checkout. No new v2 mechanisms required for Phase 0.

## Traceability hooks (per practice 1)

- This file: `v2/docs/proposal-vibe-proving-bridge-2026-05-11.md`.
- If accepted: promote to `v2/docs/decisions/00NN-vibe-proving-bridge.md`
  with Status: Accepted; this file becomes the proposal trail.
- Phase 0 log: `v2/logs/YYYY-MM-DD_vibe-proving-mathlib-probe.log`.
- Phase 1 ADRs (if reached): one each for P1.a / P1.b / P1.c.
- Cross-reference back to vibe-proving:
  `E:/projects/vibe-proving-math/app/core/theorem_search.py`,
  `app/modes/research/reviewer.py:116`,
  `app/core/aristotle_client.py`.
