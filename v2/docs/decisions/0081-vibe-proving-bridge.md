# 0081: vibe-proving bridge — external substrate ingestion

Status: Closed by [0084](0084-direction-housekeeping.md) (Phase 0 ran on synthetic data; headline claim retracted 2026-05-19; Phase 1 folded into N1 real Mathlib — reopen requires explicit user commitment to N1)
Date: 2026-05-11

Parent proposal: [`proposal-vibe-proving-bridge-2026-05-11.md`](../proposal-vibe-proving-bridge-2026-05-11.md)

## 2026-05-11 Round 1 ARIS auto-review-loop disclosure

The original Phase 0/1 work in this ADR used the label "synthetic
Lean dep" to refer to a synthetic layered random DAG with clustered
substructure. The ARIS auto-review-loop Round 1 reviewer correctly
flagged that the "Lean" label overstates what the substrate is
(W1+W2+W6). The substrate is renamed to "synthetic layered random
DAG" in Phase 1.D result docs and in the example source as of
2026-05-11. Any claim that v2 is "Lean-substrate-sensitive" requires
real Mathlib data — see Phase 1.E in the result doc's §11
follow-ups. The Phase 0 / Phase 1.D experiments establish v2's
substrate-sensitivity at the canonical-form level between
*synthetic substrate families*, NOT between OQ#2 and Lean.

See [`docs/results/bridge_cross_substrate_canonical.md`](../results/bridge_cross_substrate_canonical.md)
for the full revised finding and [`review-stage/AUTO_REVIEW.md`](../../review-stage/AUTO_REVIEW.md)
for the Round 1 review preserved verbatim.

## Context

The 5/11 proposal flagged a one-way ETL bridge from
vibe-proving-math's already-structured math corpora
(arXiv citation graphs, Lean dependency graphs, proof-step
DAGs) into v2's RSet TSV format. The proposal was placed in
backlog 5/11 pending "preparation" — i.e., ADR 0079 perf
caching (5/11 done) + ADR 0080 learning-progress drive
(5/11 mechanism shipped).

User direction: after preparation, introduce. Both
preparation items are now shipped (ADR 0079 caching commit
45e6878, ADR 0080 commit dda4a2e). This ADR promotes the
proposal to a Phase 0 experiment.

## Decision

**Phase 0**: single bridge example reading a small Lean
dependency-style graph (synthetic for first run, since no
local Mathlib checkout is available in this slice). Pipeline:

1. Construct a synthetic Lean-dep-flavored graph (named
   lemma nodes + dep edges, sized ~10² nodes / ~10³ edges)
2. Anonymize names to `lem_NNNN` tokens
3. Write TSV per ADR 0038 format
4. Load via `RSet::from_text`
5. Run `autonomous_pass` size 2-5
6. Run `discover_axioms_minimal`
7. Run `discover_theory`
8. Report patterns / axioms / theories discovered

Synthetic data substitutes for real Mathlib until a
checkout is available — the *bridge mechanism* is what
gets tested in Phase 0; the data origin is interchangeable.

## What this ADR commits to

- **Phase 0 example**: `bridge_lean_dep_probe.rs`. Single
  Rust file, self-contained. Generates synthetic Lean-style
  graph in-process; no external dependencies.
- **No bridge-to-actual-vibe-proving in Phase 0**: per
  proposal's non-goals, the bridge is unidirectional and
  external. Phase 0 doesn't call vibe-proving APIs.
- **Result doc compares**: discovered patterns / axioms /
  theories on synthetic-Lean substrate against the canonical
  v2 suite (OQ#1, OQ#2, etc.). Looking for: anything that
  emerges on Lean-style but doesn't on the canonical suite,
  or vice versa.

## What this ADR does NOT do (still per proposal)

- No real Mathlib dependency parsing. Synthetic substitute.
- No arXiv citation graph ingestion. Synthetic only.
- No vibe-proving runtime dependency. Bridge is one-way
  TSV.
- No new v2 mechanisms. Reuses existing autonomous_pass /
  discover_axioms_minimal / discover_theory.

## Constitution check

Re-checked per proposal §"Constitution check":

- C1 (R is singular): synthetic edges all same R ✓
- C2 (R is binary): lemma_X depends-on lemma_Y is binary ✓
- C3 (types are meta-R): no pre-loaded types ✓
- C4 (token-based identity): tokens are `lem_NNNN` opaque
  ids ✓
- C5 (similarity is structural): no similarity scores in
  bridge ✓

Per the strict reading (constitution heavy reading from
2026-05-06): the bridge's tokenization decision IS a semantic
act outside v2. Recorded with the dataset (synthetic
generation policy in the example file) so the choice is
traceable. No commitment violation, per proposal's analysis.

## Go / no-go criteria (per proposal)

**Go signal** (any of):
- A discovered pattern with positive MDL gain on synthetic-Lean
  substrate that has no analog in OQ#1-clade
- An axiom (strict or defeasible) holding on synthetic-Lean
- A theory fingerprint structurally distinguishable from
  OQ#1-clade theories

**No-go signal** (legitimately stop):
- All outputs degenerate (no patterns named, no axioms
  hold, theory fingerprint matches synthetic case identically)

**Abort signal**:
- Bridge tokenization policy turns out to do work that
  should have been v2's (changing the policy changes the
  discovered patterns substantially)
- Scale becomes bottleneck before semantic signal appears

## Implementation

Single example file `examples/bridge_lean_dep_probe.rs` (~150
LOC). Pipeline shown above. Result doc reports findings.

If Phase 0 shows signal, Phase 1 introduces real Mathlib
extraction (separate ADR).

## Rationale for synthetic-first

The proposal recommends starting with real Mathlib for cleanest
edge semantics. We instead start with synthetic Lean-style
because:
1. Synthetic data is in-process, no external dependencies
2. We can vary graph structure to test what v2 picks up
3. Phase 0 success criterion (any non-trivial output) doesn't
   require real-world signal — synthetic structure is fine
4. Phase 1 with real Mathlib is a natural follow-up if
   Phase 0 succeeds

The cost of synthetic-first is that "natural occurrence of math
structure" isn't directly tested. Phase 1 fixes that, if reached.

## Next

Implement Phase 0 example + result doc in this commit.
