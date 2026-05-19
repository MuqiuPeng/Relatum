# 2026-05-19 retrospective — Phase 1.D arc closeout + Phase Emergence runtime tuning

Spans 13 commits across 2026-05-11→2026-05-19 (single extended autonomous session). Two parallel threads ran to completion:

1. **Phase 1.D substrate-sensitivity arc**: 8-round ARIS auto-review-loop + post-loop multi-family / structural-class / size-4 / top_m extensions.
2. **Phase Emergence runtime hole**: ADR 0080 LP-threshold tuning + prune-loop fix + capability demo refresh.

Plus minor build-warning cleanup.

## Commit map

| commit | thread | summary |
|--------|--------|---------|
| `aa21ded` | P1.D | ARIS Phase A review committed (3/10 not ready) |
| `86a7571` | P1.D | Rounds 1-3 (Phase A→B→C→D, loop exit 7/10 ready, claim retracted) |
| `c83e87f` | P1.D | Round 4 multi-seed (N>1 reinforces retraction) |
| `e6c7d5b` | P1.D | Round 5 multi-family (ER/SBM/DAG universal vocabulary) |
| `6e67137` | P1.D | Round 6 structural-class (BP × random distinguishable) |
| `6a5943c` | P1.D | Round 7 size 4 (BIPARTITE first H1-passing family) |
| `f6cbb04` | P1.D | Round 8 + retrospective (top_m=100 confirms BP H1) |
| `48c007d` | runtime | ADR 0080 LP-threshold tuning (30→10, 0.05→0.20) |
| `09a36de` | runtime | Prune-loop fix (type-routing + filter both proposal sites) |
| `af31c98` | runtime | ADR 0079.1 baseline validated post-fix |
| `4571c4f` | runtime | 6k OQ#2 stability confirms scaling |
| `d927c87` | runtime | Capability demo refresh (4500-tick OQ#2 in 4.6 min) |
| `03657d5` | hygiene | Build-warning cleanup (0 warnings, 650 tests pass) |

## Thread 1 — Phase 1.D arc

### Final state across 8 rounds

| Round | Outcome |
|-------|---------|
| 0 (original 2026-05-11) | "v2 substrate-distinct emergence; 67% novel; Phase 2 motivated" |
| 1 ARIS review | 3/10 not ready — surfaced W1-W7 |
| 2 baseline | 5/10 not ready — H1 disconfirmed; W4 surfaced via N=1 fragility |
| 3 framing tweaks | 7/10 ready — claim retracted; ARIS loop exits |
| 4 multi-seed N>1 | retraction reinforced |
| 5 multi-family | ER/SBM/synth-DAG produce identical canonical sets — universal vocabulary |
| 6 structural-class | BIPARTITE × random distinguishable; TREE × random overlap |
| 7 size 4 | BIPARTITE first H1-passing family (within 0.90, max cross 0.21) |
| 8 top_m=100 | BIPARTITE H1 robust to cap removal; max cross sharpens to 0.16 |

### The surviving narrow positive

> v2's canonicalization at sizes 2-4 reflects structural-class constraints. Substrates whose structure excludes motifs (BIPARTITE: no 3-cycle, no self-loop, no L→L) produce canonical-form sets sharply distinguishable from random-graph baselines. Within-random-class substrates (ER vs SBM vs synth-DAG) remain mutually indistinguishable at all sizes 2-4 tested.

This is classical subgraph census reflecting structural constraints — not "emergent substrate-sensitivity beyond classical motif census." Phase 1.E (real Mathlib) remains the gating experiment for any stronger claim.

### Methodological observation

The ARIS auto-review-loop substantively improved the work. Three independent sub-agent reviewers (Rounds 1, 2, 3) each caught real over-claims that the executor could not self-identify:
- Round 1 W1-W7: rename, null baseline, hash-vs-canonical equality, inferential overreach
- Round 2 N1-N4: methodologically empty within-baseline, saturation-vs-convergence ambiguity, retrospective "pre-registration" framing
- Round 3 M1-M3: residual framing residues post-retraction

Pre-registered H1 thresholds (within > 0.7 AND max cross < 0.4) gave each round a falsifiable structure. Round 7-8's H1-passing finding for BIPARTITE is correspondingly defensible — measured against the same explicit thresholds that disconfirmed the original claim.

## Thread 2 — Phase Emergence runtime hole

### ADR 0080 LP-threshold tuning

ADR 0080 shipped on 2026-05-11 with LP_WINDOW=30, LP_DRIVE_THRESHOLD=0.05 (starting guesses, marked in open questions). 3000-tick OQ#2 hung at log header because gates needed ~5 min to close.

Tuned: LP_WINDOW=10, LP_DRIVE_THRESHOLD=0.20. After two iterations (0.10 still hung due to per-size LP not matching multi-size fallback dispatch), final 0.20 cleanly closed gates. 3k OQ#2 completes in 6.2 min.

### Prune-loop fix

Same 3k log revealed a separate per-tick `PruneLowValueObjects` loop after LP gates closed. Two distinct causes:

1. **Type-mismatch in target routing**: `rank_by_counterfactual` returns patterns + theories + extension edges. Frontier wrapped ALL as `FrontierTarget::Pattern(id)`, but the action handler's `Pattern(id)` branch only calls `retract_pattern`, which fails silently for theory/extension ids. Fix: route theory ids to `FrontierTarget::Theory(id)`; skip extension edges from this proposal path.

2. **Second proposal site (`refresh_stale_prune`) bypassing filter**: a separate path proposed prune items based on `last_improved_tick` staleness. Fix: Frontier now caches `recent_prune_targets` on self during `refresh_with_episodes`; `refresh_stale_prune` reads this cache and applies the same filter.

After both fixes: 3k OQ#2 completes in **1.9 min** with prune=8 total (vs 1193 in LP-only run).

### Validation chain

- ADR 0079.1 sustained-mode baseline (800-tick OQ#2): preserved exactly — pats=7, second-half episodes +14, pat_instances +11.
- 6000-tick OQ#2 stability: 4.5 min wall-clock, prune cap'd at 8, runtime fully idle after tick 5000.
- Capability demo refresh (4500-tick OQ#2): 4.6 min wall-clock, all 9 sections populated, 15 patterns minted, 9 agent classes, 14% unexplained.

The 2026-05-11 retrospective's "ADR 0080 threshold tuning is open" and "capability demo refresh deferred" both resolved.

## What this session did NOT touch

The remaining work is multi-session-scale:

- **Phase 1.E** real Mathlib ingestion (1-3 weeks; gates any real substrate-sensitivity claim).
- **O1** recommendation execution loop (consolidate ADR 0070-0072 from diagnostic-only to runtime-executing).
- **O2** G-series autonomy bridge (drive triggers generative recipe minting; ADR 0080 closes the pattern-discovery side, G-series adds the identifier-creation side).
- **Multi-size LP gate** redesign (per-size gating doesn't match multi-size fallback dispatch; aggregate LP across sizes might be cleaner).
- **BA scaling fix** (Round 5 found `autonomous_pass` doesn't scale on power-law graphs at saturation budget at n=80).

## Methodological notes from this session

1. **ARIS auto-review-loop catches over-claims**. Three rounds of fresh-context sub-agent review identified 14 distinct weaknesses across two iterations (W1-W7, N1-N4, M1-M3). All were correct. None were self-identifiable inside the executor's context. The retraction this produced is the kind of save the framework is designed for.

2. **Pre-registered hypotheses + explicit thresholds enable both falsification and confirmation**. Same H1 (within > 0.7 AND max cross < 0.4) that disconfirmed the canonical-suite-vs-random comparison cleanly confirmed BIPARTITE × random at size 4. Symmetric measurement; symmetric verdict.

3. **Per-tick scaling observations matter**. The prune-loop didn't crash anything — it produced linear step-time growth. Without snapshot-by-snapshot timing in the verification example, this would have been invisible. Long-horizon runs need both timing and state telemetry.

4. **Type-correct dispatch paths**. The prune-loop's first cause was a simple type-routing bug: same target wrapper used for differently-typed ids. Static type-driven dispatch (encode "this is a theory id, not a pattern id") would have prevented this. Worth considering for future runtime evolution.

## Files of record

- `docs/results/bridge_cross_substrate_canonical.md` (Round 0 + revisions through Round 7)
- `docs/results/bridge_multi_seed_scan.md` (Round 4)
- `docs/results/bridge_multi_family_scan.md` (Round 5)
- `docs/results/bridge_structural_class_scan.md` (Round 6)
- `docs/results/bridge_size4_scan.md` (Rounds 7 + 8)
- `docs/results/phase_1d_retrospective.md` (8-round close-out)
- `docs/results/adr0080_lp_threshold_tuning.md` (LP tuning + prune fix combined)
- `review-stage/AUTO_REVIEW.md` (verbatim reviewer outputs, all rounds)
- `examples/` — 4 new bridge experiments (null_baseline, multi_seed_scan, multi_family_scan, structural_class_scan, size4_scan)
- `examples/oq2_long_horizon_lp_tuned.rs` (3k/6k LP-tuning verification)

## Closing

13 commits in one extended autonomous session. Two threads (Phase 1.D arc + Phase Emergence runtime tuning) both closed cleanly. The remaining work for v2 is now sized appropriately for explicit user direction — Phase 1.E or O-series operationalization. Capability demo runs end-to-end in under 5 minutes on the hardest substrate in v2's repertoire, with 0 lib warnings and 650 passing tests.
