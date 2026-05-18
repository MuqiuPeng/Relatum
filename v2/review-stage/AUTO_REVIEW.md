# AUTO_REVIEW — ADR 0081 Phase 1.D cross-substrate canonical claim

**Mode**: ARIS auto-review-loop Phase A (review only; no fix loop applied)
**Reviewer**: fresh-context Claude Code sub-agent (Agent tool, general-purpose)
**Reviewer-independence note**: this is NOT a true cross-model review. Reviewer is Claude, but spawned as a sub-agent with no access to the executor conversation history — sees only the v2 source files listed below. Stronger than in-context self-review (no sunk-cost narrative bias), weaker than ARIS's intended Codex/GPT-5 cross-model design (model-level blindspots still possible).
**Date**: 2026-05-11
**Claim under review**: "v2's pattern emergence produces substrate-distinct structural categories; Jaccard 0.26 (Lean vs OQ#2) consistent with prior 0.17, both in 0.15-0.30 'substrate-sensitive without over-fitting' range."

## Files supplied to reviewer (absolute paths)

1. `docs/constitution.md`
2. `docs/decisions/0081-vibe-proving-bridge.md`
3. `docs/results/bridge_lean_dep_probe_phase0.md`
4. `docs/results/bridge_cross_substrate_canonical.md`
5. `examples/bridge_lean_dep_probe.rs`
6. `examples/bridge_cross_substrate_canonical.rs`
7. `logs/2026-05-11_bridge_cross_substrate_canonical.log`
8. `docs/results/phase_emergence_canonical_form_diversity.md` (prior Jaccard 0.17)

## Round 1 — Reviewer output

# Review: ADR 0081 Phase 1.D — Cross-Substrate Canonical Comparison

**Reviewer**: External (NeurIPS/ICML/TMLR-style critical review)
**Subject**: The claim "v2's pattern emergence machinery produces substrate-distinct structural categories" supported by Jaccard(OQ#2, Lean) = 0.26 + prior Jaccard 0.17 (ADR 0075 piece 3).

---

## 1. Score: **3/10**

## 2. Verdict: **not ready**

The experiment is internally executed correctly but the inferential leap from procedure to claim is severely overstated. The result is at best a sanity check that two graphs with different generative processes produce different size-3 subgraph distributions — a near-tautology that the framing then dresses up as evidence of "substrate-sensitive structural abstraction." This is not publishable in any peer-reviewed venue without substantial methodological retooling.

---

## 3. Specific Weaknesses

### W1 — "Synthetic Lean" is not Lean; the claim's external-validity vocabulary is unearned

`bridge_lean_dep_probe.rs:30-86` (and duplicated at `bridge_cross_substrate_canonical.rs:94-140`) generates edges via xorshift-seeded random draws with uniform target picking (`(next() as usize) % i`), light layering, and 5-node random cliques. There is no Lean-specific structure: no axiom-vs-theorem distinction, no actual proof DAG dependency, no realistic out-degree distribution (real Mathlib follows a heavy-tailed power law; this code does `% 2 + 1`, `% 3 + 2`, `% 4 + 2`, i.e. nearly uniform low integers). The "synthetic-first" rationale in ADR 0081 lines 113-124 acknowledges this, but the result doc (`bridge_cross_substrate_canonical.md:106-113`, line 184) still concludes "the bridge produces substrate-distinct structural emergence" and "first empirical evidence that v2's pattern path generalizes to natural-data structural categories." **There is no natural data in this experiment.** What was tested is "v2 on one random DAG vs v2 on OQ#2's regime-mixed stream." Calling it Lean is a marketing label.
**Fix**: Rename throughout to "synthetic layered random DAG." Defer the Lean-generalization claim until real Mathlib (or at minimum a graph with documented degree distribution matching empirical Mathlib statistics) is ingested. The current claim is unfalsifiable as stated because no property of Lean was tested.

### W2 — N=2 is not "consistent"; the inference is statistical theater

Result doc lines 132-150 frame two Jaccard values (0.17 and 0.26) as "both in the 0.15-0.30 range" implying convergent evidence. Two data points cannot establish a "range" — they trivially fit any range that contains both. With no variance estimate (no seed sweep, no bootstrap, no alternative substrate pairs), there is no basis to call 0.26 "consistent with" 0.17. They could equally well be drawn from a distribution centred at 0.50 — we cannot tell from N=2. The phrase "v2's pattern emergence machinery consistently produces canonical sets that are both substrate-sensitive and partially-overlapping" (line 144-146) requires consistency to be measured, and it wasn't.
**Fix**: Run ≥10 random-substrate pairs across distinct generative families (Erdős–Rényi, preferential attachment, planted-partition, real Mathlib slice, real citation slice, etc.). Report Jaccard distribution mean ± std. Compare against a null where both substrates are drawn from the same family.

### W3 — No null baseline; the result is uninterpretable without one

There is no control condition. What is the Jaccard between two *independent draws of OQ#2 itself* (different RNG seeds)? Between two independent draws of the synthetic Lean graph? Between two random graphs of the same density? Without these, Jaccard 0.26 is a number floating in space. If two independent OQ#2 draws yielded Jaccard 0.85, then 0.26 would indeed be distinctive. If two independent OQ#2 draws yielded Jaccard 0.30, then 0.26 means *nothing*. The example file (`bridge_cross_substrate_canonical.rs:30`) uses `const RNG_SEED: u64 = 0xC0FFEE;` — a single seed for everything. No within-substrate replication exists.
**Fix**: Add a same-substrate baseline (Jaccard(OQ#2_seed_A, OQ#2_seed_B)) and a fully-random baseline (Jaccard(ER(n,p), ER(n,p))) using the same `autonomous_pass` pipeline. Only then can 0.26 be assigned an interpretable position.

### W4 — Sampling regime confounds the comparison

`bridge_cross_substrate_canonical.rs:46-51` and 65-71 hard-codes `sample_count: 400, top_m: 20` for both substrates regardless of substrate size/density. OQ#2's autonomous pass (line 159) runs after `rt_oq2.run_bounded(1000)` whereas the Lean graph runs only `autonomous_pass` on a static RSet — different runtime histories. Furthermore, the Lean graph has 270 edges (per Phase 0 result line 21) while OQ#2's runtime-built RSet edge count is not even reported in the log. **The canonicals are functions of the edge multiset that was sampled, not of "the substrate."** A larger / denser graph naturally produces more distinct size-3 induced subgraphs by pure combinatorics — not necessarily by "richer structural diversity" (Phase 0 line 47).
**Fix**: Match graphs on edge count, density, and node count before any cross-substrate diversity claim. Report `|R|`, mean degree, and edge-distribution statistics for both substrates. Equalize `sample_count` per unit edge.

### W5 — Canonical-form hash collisions are unaccounted for

`bridge_cross_substrate_canonical.rs:34-40` uses Rust's `DefaultHasher` (currently SipHash-1-3) truncated to 12 hex chars = 48 bits. Birthday collision probability at ~24 canonicals is negligible *for these data*, but the result doc treats hash-tag equality as canonical-form equality without verifying the underlying `CanonicalForm` structs match. With more substrates this becomes a real bug. More importantly, the result doc lines 88-103 claim "3 different star variants" and "5 different 4-node-3-edge variants" — these are *informal labels* applied by `format_pattern_shape` (called at line 240, displaying only `lines().next()`), and the doc does not show that the underlying canonical forms are graph-isomorphism-distinct. They could be the *same* shape distinguished only by node ordering inside the canonicalizer.
**Fix**: Verify canonical equality via `==` on the `CanonicalForm` struct, not hash equality. For shape-labelled groupings ("star variant 1/2/3"), display the full canonical form (edge tuple list) or perform an explicit isomorphism check to confirm they are not artifacts of an incomplete canonicalizer.

### W6 — The "GO signal" precedent contaminates this result's framing

Phase 0 (`bridge_lean_dep_probe_phase0.md:38-67`) already declared GO based on 15 patterns vs OQ#2's 7. This experiment then "follows up" by computing Jaccard. But the Phase 0 conclusion is itself confounded by W4 (sample regime, edge count mismatch). The 1.D doc inherits and amplifies Phase 0's framing without correcting it. Result doc line 184-187: "produces 2× more patterns of which 67% are substrate-novel" — both ratios (2× and 67%) depend on uncorrected confounds.
**Fix**: Re-run Phase 0 with matched-density controls before drawing 1.D's stronger conclusions on top of it.

### W7 — Confusion of substrate effect and runtime-pipeline effect

OQ#2 is processed through `AutonomousRuntime::run_bounded(1000)` + manual `autonomous_pass`; the Lean graph receives only `autonomous_pass` on a static RSet. These are *different pipelines*. Any difference in minted canonicals could be attributable to the scheduler/environment difference, not to the substrate. The comment at line 156-158 says "1000 is well past OQ#2's Phase 0 maturity" — but the Lean side has no analogous maturity step.
**Fix**: Process both substrates through identical pipelines. Either run the Lean graph through a synthetic-stream environment, or run OQ#2 through static-RSet autonomous_pass only.

---

## 4. Adversarial Check — three falsifiers and whether they were ruled out

**F1**: "Any two graphs with different densities give Jaccard < 0.30 under v2's pipeline." — **Not ruled out.** No density-controlled comparison performed.

**F2**: "Two independent draws of OQ#2 itself yield Jaccard ≈ 0.26." — **Not ruled out.** No within-substrate replication.

**F3**: "The 10 'Lean-only' canonicals are sampling artifacts; with `sample_count` set higher, OQ#2 would produce them too." — **Not ruled out.** No sample-budget convergence test; `sample_count=400` was used unreflectively for both.

All three are alive. The claim is not yet supported against any of them.

---

## 5. Statistical / Methodological Concerns with N=2

Two Jaccard values do not constitute evidence of a *consistent* phenomenon. A single sample is a data point; two samples are also essentially a single data point about the relationship between them. The phrasing "Both Jaccards in the 0.15-0.30 range" is post-hoc range fitting: the range was chosen *after* seeing the values. The methodologically sound statement would be: "We observed Jaccard 0.17 in one experiment and 0.26 in another; we lack the replications needed to estimate whether v2's Jaccard is bounded, what its mean is, or how its variance compares to a null model." That's a much weaker claim than the doc makes.

---

## 6. Conceptual Concern — "individual motifs are graph-theoretic, but combinations are informative"

The result doc at lines 124-129 hedges: each canonical (3-cycle, star, fork, chain, bidirectional pair) is a graph-theoretic fundamental, but the *combination minted per substrate* is supposedly the informative content v2 produces.

This is partly valid and partly rhetorical. **Valid**: which subgraphs appear at significant frequency in a graph is a non-trivial structural property — degree distribution, triangle density, etc., genuinely differ across graph families. **Rhetorical**: v2 is doing nothing more than `find_instances_of(size_k_subgraph)` on different graphs; the "combination is informative" framing dresses up subgraph census as cognitive abstraction. A 1970s graph-theory paper that counted size-3 induced subgraphs on two random graphs would arrive at the same Jaccard difference without claiming "substrate-sensitive structural abstraction." The constitution's commitment 5 (similarity is structural) is satisfied trivially; the question of whether v2 does anything *beyond* subgraph census is not answered by this experiment.

---

## 7. Counterfactual — Is the claim falsifiable?

If v2 had returned Jaccard = 1.0, the author would almost certainly have written: "v2 produces a universal motif vocabulary stable across substrates — encouraging evidence of *substrate-invariant* structural abstraction." (See the framing in `phase_emergence_canonical_form_diversity.md:226-232`, which casts OQ#2's earlier Jaccard 0.17 as positive evidence.)

If v2 had returned Jaccard = 0.0, the author would have written: "v2's canonicalizer is sharply substrate-discriminative — different inputs produce wholly different vocabularies."

Both extreme outcomes would be claimed as evidence for the system working. The middle is also claimed as evidence ("not over-fitted, not insensitive" — line 148-150). **Every possible Jaccard supports the claim.** This is not a falsifiable test; it is a Rorschach inkblot with predetermined interpretation.
**Fix**: Pre-register, *before* running, an expected Jaccard range under H1 (v2 is substrate-sensitive) and an expected range under H0 (v2 is just doing subgraph census). Only if observed Jaccard falls outside the H0 range does the experiment support H1.

---

## 8. What the Experiment Actually Proves (Strictest Reading)

Under the strictest reading: running `RSet::autonomous_pass` with `sample_count=400, target_size=2..=3` once with seed `0xC0FFEE` on a single OQ#2 runtime instance and once on a single xorshift-generated 80-node 270-edge random layered+clustered DAG yields canonical-form sets whose intersection-over-union is 0.2632, with 5 forms shared and 14 forms non-shared. The shared forms are common small directed-graph motifs (bidirectional pair, 3-cycle, star, fork, chain). That is the entire empirical content. It supports the conditional statement *"under this one configuration, these two specific graphs differ in their size-2-and-3 induced-subgraph canonical census."* It does **not** support: (a) substrate-sensitivity as a general property of v2 (only one comparison done with no controls), (b) generalization to natural data (no natural data tested), (c) absence of over-fitting (no over-fitting test specified or run), (d) "consistent" behavior across substrates (N=2 cannot establish consistency), or (e) discovery of "real structural novelty" (the alleged novel motifs were not validated against a same-substrate-resample baseline).

---

## Bottom Line

The experiment is honest in its code, but the result doc's prose runs far ahead of what the procedure can underwrite. The core problem is the asymmetry: a careful, contained engineering test ("compute Jaccard between two pattern sets") is being asked to carry an ambitious cognitive-architecture claim ("v2 produces substrate-distinct structural emergence beyond hand-crafted tests"). The gap between procedure and claim is too large. The path forward is straightforward and known to the author (per "What this slice did not address" lines 152-162): real Mathlib, more substrates, more seeds, controls. Until then, this is a pilot study — not a publishable result.

---

## Next-step status (Phase B candidates)

Per ARIS auto-review-loop, weaknesses W1–W7 are now the input to Phase B (decide which to fix) and Phase C (implement). This Phase A output is recorded here; the user has not yet authorized iterative fix loop. Options:

- **B.1 — Implement fixes in priority order**: W3 (null baseline) and W4 (sampling/density match) are the highest-leverage corrections; W1 (rename "synthetic Lean" → "synthetic layered random DAG") is the cheapest narrative fix. W2 (N=10+ substrates) and W7 (pipeline match) are the biggest scope.
- **B.2 — Revise result doc prose only**: keep experiment, drop overclaims. Cheap; preserves the engineering record while removing inferential overreach.
- **B.3 — Mark Phase 1.D as superseded by Phase 1.E (controlled re-run) when produced**: pending a follow-up experiment.
- **B.4 — Stop here, file review as honest counter-evidence**: don't fix, keep as part of v2's self-critique trail. ARIS-philosophy-aligned but leaves the public claim unfixed.
