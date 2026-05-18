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

---

## Round 1 — Phase B (triage decisions)

Selected mixed strategy: highest-leverage W3 (null baseline) + W5 (hash vs canonical equality) shipped as code; W1 (rename), W2/W6/W7 (overclaim narrowing) shipped as prose; W4 (multi-seed) deferred to a named follow-up.

| Weakness | Decision | Rationale |
|----------|----------|-----------|
| W1 | Fix (rename throughout) | Cheapest narrative fix; removes false Lean credibility framing. |
| W2 | Acknowledge in prose; full fix deferred | N=10+ multi-family substrate scan is its own multi-day project. Round 1 doc explicitly narrows "consistent" claim. |
| W3 | Fix (new null-baseline experiment) | Highest scientific leverage. Without this, cross 0.26 is uninterpretable per the reviewer's own falsifier F2. |
| W4 | Acknowledge in prose; deferred | Multi-seed scan is the natural next slice; flagged in result doc §11 follow-ups. |
| W5 | Fix (direct `CanonicalForm` set equality) | Cheap, technically correct, removes a real (if currently moot) bug. |
| W6 | Fix (prose narrowing of "merge motif" claim) | Cheap; removes another false-credibility inference. |
| W7 | Fix (prose narrowing of generalization verdict) | Cheap; removes the strongest overclaim. |

## Round 1 — Phase C (implementation, all shipped 2026-05-11)

### C.1 New experiment: `examples/bridge_null_baseline.rs`

Pre-registers H0 vs H1 with explicit thresholds **before** running:

```
H1 (substrate-sensitive emergence):
    Jaccard_within > 0.7  AND  Jaccard_cross < 0.4
    (gap > 0.3)
```

Measures three Jaccards on the same discovery pipeline:
1. `Jaccard_OQ#2_self` — same OQ#2 graph, different `autonomous_pass` RNG seeds.
2. `Jaccard_DAG_self` — two synth-DAG draws from the same family, different graph-generation seeds.
3. `Jaccard_cross` — OQ#2 vs synth-DAG (Phase 1.D reproduction).

Result (single seed, log [`logs/2026-05-11_bridge_null_baseline.log`](../logs/2026-05-11_bridge_null_baseline.log)):

```
Jaccard_OQ#2_self    = 1.0000
Jaccard_DAG_self     = 1.0000
Jaccard_cross        = 0.2632
gap (within - cross) = 0.7368
```

**H1 supported** under pre-registered thresholds. Honest caveat documented in result doc §4.1: within-family Jaccard 1.0 is suspiciously perfect; either discovery is saturating at sizes 2-3, or both substrate families really do have small invariant canonical vocabularies. Either way the conservative reading (deterministic structural fingerprint per substrate family, fingerprints differ between families) is supported. Multi-seed falsification is W4, deferred.

### C.2 Hash → direct `CanonicalForm` equality (W5 fix)

`examples/bridge_cross_substrate_canonical.rs` rewritten to use `HashSet<CanonicalForm>` (`= Vec<(u64,u64)>`) for set ops instead of truncated 64-bit hash tags. Numbers are unchanged (no collisions at this scale) but the technique is now correct. Verified by re-run log [`logs/2026-05-11_bridge_cross_substrate_canonical_v2.log`](../logs/2026-05-11_bridge_cross_substrate_canonical_v2.log).

### C.3 Substrate renamed (W1 + W2 + W6)

- `examples/bridge_cross_substrate_canonical.rs`: function `build_synthetic_lean_dep_graph` → `build_synthetic_layered_random_dag`; all "Lean" prose in source comments and `println!` rewritten to "synth-DAG"; explicit non-Lean disclaimer added in source header.
- `docs/results/bridge_cross_substrate_canonical.md`: full rewrite with Round 1 disclosure section, renames, and explicit §9 "What this slice does NOT support."
- `docs/decisions/0081-vibe-proving-bridge.md`: Round 1 disclosure note added at top.

### C.4 Overclaim narrowing (W7)

Result doc verdict §13 narrowed to claims actually supported by data:
- **Drops** "first empirical evidence that v2's pattern path generalizes to natural-data structural categories." (No natural data was tested.)
- **Drops** "67% of Lean canonicals are substrate-novel." Replaced with "67% of synth-DAG canonicals at sizes 2-3 do not appear in OQ#2's canonical set under this discovery budget."
- **Drops** "Phase 2 of the bridge is empirically motivated." Replaced with "this slice strengthens but does not complete the case for Phase 1.E."
- **Keeps**: cross 0.26, within 1.0, gap 0.74; substrate-sensitivity at canonical-form level between *synthetic* substrate families.

---

## Round 1 — Phase D (state persisted)

Files modified:
- `examples/bridge_cross_substrate_canonical.rs` — full rewrite
- `examples/bridge_null_baseline.rs` — new
- `docs/results/bridge_cross_substrate_canonical.md` — full rewrite
- `docs/decisions/0081-vibe-proving-bridge.md` — Round 1 disclosure
- `review-stage/AUTO_REVIEW.md` — this update
- `logs/2026-05-11_bridge_cross_substrate_canonical_v2.log` — new
- `logs/2026-05-11_bridge_null_baseline.log` — new

The original log `logs/2026-05-11_bridge_cross_substrate_canonical.log` is **preserved** to anchor the original (overclaimed) finding alongside its correction.

---

## Round 2 — Phase A (re-review)

After Phase C ships, a fresh sub-agent reviewer is spawned with access to the revised files. Round 2 review verbatim below.

### Round 2 reviewer output (verbatim)

**Score: 5/10**
**Verdict: not ready** (one rung below the 6/10 exit threshold)

The revision is substantively better than Round 1 — the framing has been pulled back in important ways, the W5 technical bug is correctly fixed, the renames are honest, and there is now a falsifiable pre-registered hypothesis. But the null-baseline experiment that does the heaviest scientific lifting is methodologically flawed in a way the §4.1 caveat acknowledges but does not actually neutralize. The headline "H1 supported" verdict therefore still overclaims relative to what the data underwrites.

Per-weakness status (Round 1 W1-W7): W1 addressed, W2 partially, W3 partially, W4 deferred (borderline), W5 addressed, W6 addressed, W7 partially.

New weaknesses identified:

- **N1** — The OQ#2 within-substrate baseline is methodologically empty. Both `rt_oq2_a` and `rt_oq2_b` run the same deterministic OQ#2 stream and only vary the sampler RNG. That measures sampler determinism, not substrate-family variance. The DAG-side IS a legitimate family-variance test (two different `build_synth_dag` seeds → different graphs); the OQ#2 side is not.

- **N2** — Discovery saturation hypothesis (§4.1 explanation 1) likely IS the dominant cause of Jaccard=1.0, and that bites the H1 verdict. Under saturation, both H0 and H1 predict Jaccard_within = 1.0 — the hypotheses are not actually well-separated by the experiment.

- **N3** — Pre-registered thresholds were not pre-registered in any binding sense. They appear only in source comments and result doc both written after the Phase 1.D run. There is no commit hash showing the thresholds in the tree before the null-baseline numbers were observed. Use "ex-post-explicit" rather than "pre-registered."

- **N4** — The "consistent with prior 0.17" claim in §10 silently re-imports the W2 problem. N=2 cannot establish consistency.

The path forward — fixing the OQ#2 baseline to actually vary the graph, adding a saturation probe via raised/lowered `sample_count` to test the H0/H1 separation, and removing the N=2 consistency claim — would likely take the work to 6+.

## Round 2 — Phase B (triage)

Selected fixes for all four new weaknesses, since they were code-level methodological flaws not narrative excesses.

| Weakness | Decision | Rationale |
|----------|----------|-----------|
| N1 | Fix (replace OQ#2-self with OQ#1 vs narrow_a) | OQ#1 and narrow_a are genuinely different graphs from the canonical v2 synthetic suite. This makes the "within-canonical-suite" baseline an actual family-variance test, not a sampler-determinism check. |
| N2 | Fix (add saturation probe at low budget) | Re-run all comparisons at `sample_count=50, top_m=5`. Under H1, within-family Jaccard should stay high even at low budget; under saturation (H0), within-family Jaccard collapses. This is the discriminator. |
| N3 | Fix (rename framing) | Replace "pre-registered" with "ex-post-explicit" in source and docs. Concedes the rigor claim was overstated. |
| N4 | Fix (delete "consistent with 0.17" framing) | Already done in Round 1 §13 cleanup; Round 2 §13.4 explicitly retracts it. |

## Round 2 — Phase C (implementation)

Round 2 `bridge_null_baseline.rs` rewritten to:

1. Build five RSets: OQ#1, narrow_a, OQ#2, DAG_A (seed_A), DAG_B (seed_B).
2. Run all five through `autonomous_pass` at **saturation budget** (sample_count=400, top_m=20).
3. Re-run all five at **low budget** (sample_count=50, top_m=5) — the saturation probe.
4. Report four Jaccards per budget:
   - `Within(OQ#1, narrow_a)` — within-canonical-suite (N1 fix)
   - `Within(DAG_A, DAG_B)` — within-DAG-family (Round 1 keeper)
   - `Cross(OQ#1, DAG_A)` — cross
   - `Cross(OQ#2, DAG_A)` — cross (Phase 1.D's original number)

### Round 2 outcome — empirical (log [`logs/2026-05-11_bridge_null_baseline_round2.log`](../logs/2026-05-11_bridge_null_baseline_round2.log))

```
Saturation budget:
  Within(OQ#1, narrow_a) = 0.2000   ← FAILS H1 threshold of > 0.7
  Within(DAG_A, DAG_B)   = 1.0000
  Cross(OQ#1, DAG_A)     = 0.1875
  Cross(OQ#2, DAG_A)     = 0.2632
  within_mean=0.6000 cross_mean=0.2253

Low budget:
  Within(OQ#1, narrow_a) = 0.0000   ← drops further
  Within(DAG_A, DAG_B)   = 0.7778   ← drops from 1.0
  Cross(OQ#1, DAG_A)     = 0.2500
  Cross(OQ#2, DAG_A)     = 0.1333
  within_mean=0.3889 cross_mean=0.1917
```

**H1 NOT SUPPORTED at either budget.** Within(OQ#1, narrow_a) = 0.20 ≈ Cross(OQ#1, DAG_A) = 0.19. Two members of v2's canonical synthetic family are structurally as different from each other as one of them is from a synth-DAG. The cross-substrate Jaccard 0.26 is therefore **not interpretable** as evidence of substrate-sensitivity — it is comparable to the within-canonical-suite Jaccard.

**Honest negative finding.** Phase 1.D's original substrate-sensitive verdict is **retracted** in result doc §13. The retraction is the substantive Round 2 outcome of the ARIS auto-review-loop:

- The experiment's procedure was correctly executed.
- The narrative ran far ahead of what the procedure could underwrite.
- A corrected within-baseline disconfirms the claim at its own pre-registered thresholds.
- Phase 1.E (real Mathlib) remains the next test; Phase 1.D is reclassified from "substantive finding" to "honest null-baseline check."

## Round 2 — Phase D (state persisted)

Files modified in Round 2:
- `examples/bridge_null_baseline.rs` — full rewrite (N1+N2 fix)
- `docs/results/bridge_cross_substrate_canonical.md` — §13 rewritten as Round 2 retraction; status banner updated to ⚠ Round 2 negative
- `review-stage/AUTO_REVIEW.md` — this update
- `logs/2026-05-11_bridge_null_baseline_round2.log` — new

Files preserved (engineering record kept alongside retraction):
- `logs/2026-05-11_bridge_null_baseline.log` (original Round 1, now flagged as methodologically empty for OQ#2)
- `logs/2026-05-11_bridge_cross_substrate_canonical_v2.log` (Round 1 W5-fix re-run)
- `logs/2026-05-11_bridge_cross_substrate_canonical.log` (original Phase 1.D)

## Round 3 — Phase A (re-review)

After Round 2 Phase C ships, a fresh sub-agent reviewer is spawned again on the revised + retracted artifact.

### Round 3 reviewer output (verbatim)

**Score: 7/10**
**Verdict: ready**

A negative result, honestly documented, with the substantive claim explicitly withdrawn. The scientific output (a pre-registered-ish discriminator that failed) is valid. The retraction is largely honest, though a few framing residues remain (M1, M2, M3). Above the 6/10 exit threshold because the document now claims roughly what the data supports and does *not* repackage the failure as a triumph.

Round 2 N1-N4 status: N1 addressed, N2 addressed, N3 partially addressed (label changed but threshold rhetoric still works rhetorical lifting), N4 addressed.

New weaknesses identified (all framing-level, not scientific):

- **M1** — §13.3 #2 "Within(DAG_A, DAG_B) = 1.0 is a real signal about the DAG generator, not about v2" frames a property-of-generator as a v2 measurement. Should explicitly say "the generator family has a small invariant motif census at sizes 2–3" and not dress it as a v2 capability.
- **M2** — §13.5 "still works in the engineering sense" listing after a retraction risks reading as a consolation prize. Should tighten to "the pipeline still runs end-to-end; that was never the contested claim."
- **M3** — Saturation probe is single-paired and single-seed. The Within(OQ#1, narrow_a) = 0.0 at low budget could be a small-canonical-set artifact. The conclusion is robust because saturation-budget Within(OQ#1, narrow_a) = 0.20 already fails the threshold, but should phrase as "fails the pre-registered threshold at this single configuration" rather than "the structural-distinctness claim collapses."

Should the document ship? **Yes.** It documents a real negative result against pre-set thresholds, retracts the prior verdict by name, preserves the original logs alongside the retraction, and identifies Phase 1.E (real Mathlib) as the next test rather than re-spinning the negative. The remaining issues (M1, M2, M3) are framing tweaks, not scientific problems.

## Round 3 — Phase C (framing tweaks)

All three M-tier issues applied to result doc §13:
- M1: §13.2 reframes Within(DAG_A, DAG_B) = 1.0 as a property of the generator's small-motif vocabulary, not a v2 capability.
- M2: §13.5 tightened to "the pipeline still runs end-to-end; that was never the contested claim."
- M3: §13.2 reframes "structural-distinctness claim collapses" → "fails the threshold at this single configuration," explicitly noting Round 3 M3.

## Round 3 — Phase D (state persisted; loop exit)

Score 7/10 ≥ POSITIVE_THRESHOLD 6. **ARIS auto-review-loop exits successfully.**

Final files:
- `docs/results/bridge_cross_substrate_canonical.md` — Round 2 retraction + Round 3 M1/M2/M3 framing tweaks
- `docs/decisions/0081-vibe-proving-bridge.md` — Round 1 disclosure
- `examples/bridge_cross_substrate_canonical.rs` — Round 1 W5+W1 rewrite
- `examples/bridge_null_baseline.rs` — Round 2 N1+N2 rewrite
- `review-stage/AUTO_REVIEW.md` — this complete record across three rounds
- `logs/2026-05-11_bridge_cross_substrate_canonical.log` — original (preserved)
- `logs/2026-05-11_bridge_cross_substrate_canonical_v2.log` — Round 1 W5 re-run (preserved)
- `logs/2026-05-11_bridge_null_baseline.log` — Round 1 baseline (preserved, methodologically flagged)
- `logs/2026-05-11_bridge_null_baseline_round2.log` — **authoritative Round 2 baseline**

## Loop outcome summary

| Round | Score | Verdict | Key outcome |
|-------|-------|---------|-------------|
| 1 | 3/10 | not ready | 7 weaknesses W1-W7 surfaced |
| 2 | 5/10 | not ready | 4 of 7 W's addressed; 4 new weaknesses N1-N4 surfaced |
| 3 | 7/10 | ready | N1-N4 addressed by experimental rewrite that **disconfirmed** the original H1 claim; document retracted Phase 1.D's "substrate-sensitive" verdict; 3 framing tweaks M1-M3 applied. |

**Net scientific result of the auto-review-loop**: an over-stated Phase 1.D claim was caught at the auto-review stage, corrected via a stronger experimental design (within-canonical-suite baseline + saturation probe), and retracted in the result doc. The bridge mechanism (ADR 0081 Phase 0) remains valid; the substrate-sensitivity narrative around it does not, pending Phase 1.E real Mathlib data.

This is exactly the kind of save the ARIS auto-review-loop is designed for: surface an over-claim, force it through a stricter experiment, and let the data overturn it rather than letting the original narrative ship.

