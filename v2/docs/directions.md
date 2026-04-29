# v2 Directions Backlog

**Living document.** Tracks feasible research directions in priority order. Updated as new directions are discovered during execution.

## Conventions

- **Status**: `pending` / `in-progress` / `done` / `skipped` / `blocked`
- **Result file**: `docs/results/<id>_<short_name>.md` after completion
- **Log file**: `logs/<date>_<short_name>.log` after execution
- Stop after every 10 completed for sync.

## Phase Beta — Structural vocabulary extension (priority cluster)

After 9 Phase Alpha phases polished meta-mechanisms without extending what kinds of structures the system can identify, Beta is the first track that genuinely grows v2's structural vocabulary.

### B.1 — Axiom shape families ✓ done
First runtime extension of structural vocabulary since H1. ADR 0068, commit `d1bf932`.

### B.2 — Family-level demote intervention | ✓ done
Family-level demote on `shape_premise_p0-0_p1-2` (variance-zero signature) retracts 4 noise axioms wholesale. Functionally close to Alpha-3+++ repair on aggregate metrics (mean 0.7647 vs 0.7967), with cleaner global state (4 axiom registrations gone vs orphaned). [Result](results/B.2_family_level_demote.md).
**Goal**: Test if Beta-1's family discoveries have runtime utility. When a shape family's mean cross-precision is uniformly low (e.g., < 0.50), retract all members in one operation.

**Falsifiable**: family-level demote on OQ#1's `shape_premise_p0-0_p1-2` should produce same end state as Alpha-3+ demote of t_0 (which retracted t_0 wholesale, including these noise axioms). If stronger, family-level intervention also retracts noise axioms not in t_0.

**Significance**: closes the loop — Beta-1 finds families, Beta-2 acts on them. Without B.2, Beta-1 produces inert observations.

### B.3 — Shared-conclusion family kind | ✓ done
Mechanism shipped (6 families total on OQ#1: 3 premise + 3 conclusion). **MIXED empirical**: conclusion families do not capture quality dimension on OQ#1 — all 3 have spread > 0.20. Premise structure determines axiom quality, not conclusion. [Result](results/B.3_conclusion_family_kind.md).

### B.4 — Family-aware template enumeration | ✓ done
Mechanism shipped: `enumerate_axiom_templates_filtered(config, blocked_premise_keys)` + `RSet::shape_premise_key` parser. Closes the feedback loop: Beta-1 discovers noise family → B.4 filters it out of future enumeration. 3 unit tests, 545 lib tests pass. Standalone runtime experiment deferred to B.5. [Result](results/B.4_family_aware_enumerate.md).
**Goal**: When `enumerate_axiom_templates` runs, skip premise shapes that have a low-cross-precision shape family — don't waste discovery cycles re-finding noise variants.

**Falsifiable**: with this filter, future axiom discovery on similar substrates skips the p0-0 noise family entirely.

### B.5 — Runtime integration of shape-family discovery | ✓ done (wiring only)
ActionKind::DiscoverAxiomShapeFamilies added; persistence round-trip + execute_action arm; execute_action made pub. Demo verifies dispatch mints 6 families on OQ#1, idempotent on re-dispatch (delta=0). Scheduler integration deferred to B.5.1. [Result](results/B.5_runtime_family_integration.md).

### B.6 — Family of families (nested abstraction) | ✓ done
META_SHAPE_FAMILY_MARKER + discover_nested_shape_families: groups premise families by shared individual premise edge. On OQ#1, mints 2 nested families (meta_premise_p0-1, meta_premise_p1-2). Recursive structural abstraction works — Layer 2 (meta-families) discovered from Layer 1 (families) discovered from axioms. 3 unit tests; 550 lib tests pass. [Result](results/B.6_nested_families.md).

## Phase C — Cross-cutting cleanup

### C.1 — Extract OQ#1 stream to shared library | ✓ done
17 stream-fn copies → 2 in lib (`src/test_substrates/{oq1,long5k}.rs`). 16 examples refactored via Python script + 1 manual; all 51 examples build, 545 lib tests pass. ~1300 lines net deletion. [Result](results/C.1_oq1_extraction.md).

### C.2 — Cross-precision validation on long5k | ✓ done
STRONGLY POSITIVE. long5k @ 1500 ticks discovers same 4 theories with same axiom counts as OQ#1. t_0 cross-precision column mean = 0.3248 (vs 0.3756 on OQ#1). Beta-1 mints **identical 6 shape families**. Cross-precision + shape-family signals generalize across substrates that share regime types. [Result](results/C.2_long5k_validation.md).

### C.3 — Integer-direction prep work | pending
**Goal**: Implement the smallest mechanism that mints a new identifier via axiom application. E.g., a "successor closure" rule that, given seed `0` and rule `R(succ_marker, n) → R(succ_marker, S(n))`, generates `S(0), S(S(0)), ...`.

**Risk**: deep — requires ADR on identifier minting, constitution check on commitment 4 (deterministic minting must produce token-identical results to external input).

## Phase D — Dream phase follow-ups

### D.1 — Rejection-based dreaming | pending
**Goal**: Generate substrates that DELIBERATELY violate a theory's axioms (random noise on top of structure). Forward-apply theories on these "negative" substrates, see which axioms still predict consistently.

**Falsifiable**: axioms that hold on rejection-substrates are universally robust; axioms that fail are substrate-specific.

### D.2 — Predicate-axiom enforcement during substrate generation | ✓ done
Soundness gap closed: antisymmetry enforced via DAG-restriction at seed time (saturation preserves DAG); totality enforced via post-saturation pair sweep. 2 unit tests, 547 lib tests pass. First-attempt naive "filter at seed only" failed because saturation under transitivity re-introduced violations — DAG construction is the right shape. [Result](results/D.2_predicate_enforcement.md).

### D.3 — Composite scheduler signal | ✓ done
α=0.5 blend mechanism shipped + 5-T sweep on OQ#1. Composite matches cross-precision speed (both cross 0.50 threshold at T=100, vs primary T=350). Mechanism POSITIVE; arbitration value not yet shown — OQ#1 has both signals always agreeing. Future D.3.1: construct a substrate where signals disagree. [Result](results/D.3_composite_signal.md).

### D.4 — Continuous dream loop in runtime | pending
**Goal**: Run dream phase every K ticks; demote whenever cross-precision drops below threshold. Tests stability and overhead.

**Risk**: dream phase has nontrivial cost (substrate generation). Needs careful K tuning.

## Phase E — Constitutional cleanup (drive layer)

### E.1 — Verify H2.1.0 drive-as-meta-R registration | ✓ done
4-check verification: drive count + penalty marker + EP path intact + trait/meta-R consistency. All POSITIVE on default runtime + OQ#1 stream. API note: `left_of(MARKER)` for `R(MARKER, ?)` patterns (initial attempt used `right_of`, got 0). Confirms H2.1.0 is intact. [Result](results/E.1_drive_meta_r_verify.md).

### E.2 — Drive query via meta-R (replace compile-time fast paths) | pending
**Goal**: Currently `combined_drive_signal` and `normalized_drive_signal` use `Drive::is_penalty()` compile-time method. Replace with rset query (`rset.right_of(PENALTY_MARKER)`).

**Risk**: H2 area is high-risk for breaking EP. Default to shadow-mode (compute new query path, compare to old, log discrepancy).

### E.3 — Drive synthesis (H2.2 proposed) | blocked
**Goal**: Compose new drive functions from existing ones at runtime.

**Status**: ADR 0063 H2.2 still proposed. Major design surface. Defer until E.1/E.2 ship.

## Phase A — ILP perf follow-ups

### A.1 — Premise reordering by selectivity | pending
**Goal**: ILP / Datalog optimization: order premise edges by selectivity (most-restrictive-first) so the candidate filter prunes earlier in recursion.

**Falsifiable**: measurable additional speedup over Alpha-6 indexed join on long5k.

**Likelihood low**: Alpha-6 found forward_apply is no longer the bottleneck. Probably small additional win.

## Newly discovered (added during execution)

### Round 2 — appended after first 10/10 sweep (2026-04-29)

### B.5.1 — Scheduler picks DiscoverAxiomShapeFamilies | ✓ done
New FrontierKind::ShapeFamilyDiscoveryCandidate + refresh_shape_family_candidates + scheduler routing in Expand mode. On OQ#1 scheduler autonomously fires 1 DiscoverAxiomShapeFamilies episode → 6 families. Trajectory changed (16 axioms / 5 theories vs prior 13/4) — runtime integration is real, not just plumbing. [Result](results/B.5.1_scheduler_shape_family.md).

### D.4 — Continuous dream-phase loop | ✓ done
6 phases × 300 ticks. Phase 0: t_0 cross-prec=0.3248 → demote. Phases 1-5: stable, no further demote (lowest = t_1 at 0.5273, just above 0.50 threshold). Loop converges in 1 demote; idempotent post-convergence (4 phases byte-identical). [Result](results/D.4_continuous_dream.md).

### D.3.1 — Signals-disagree substrate (composite arbitration test) | ✓ done
NULL on hypothesis: narrow_a substrate (regime A only) still has both signals picking t_0. Magnitudes differ (t_3: primary 0.9427 vs cross 0.7500) but ranking ties. **POSITIVE methodological finding**: signals correlate strongly because they measure the same property from different angles. Composite's value is robustness/smoothing, not arbitration. [Result](results/D.3.1_signals_disagree.md).

### F.1 — Per-axiom cross-precision API | ✓ done
`RSet::axiom_cross_precision(ax, substrates)` shipped + 2 unit tests. **Bonus latent-bug fix**: `collect_meta_ids` now includes SHAPE_FAMILY_MARKER + META_SHAPE_FAMILY_MARKER (without it, B.5.1 would have polluted data-id accounting). 552 lib tests pass. [Result](results/F.1_per_axiom_cross_precision.md).

### C.2.1 — OQ#2 with non-overlapping regimes | ✓ done
PARTIAL/STRUCTURAL-LIMIT. OQ#2 (tournament+lattice+star) produces 0 template axioms (transitivity violations break it) → 0 shape families, no cross-precision applicable. **Methodological**: cross-precision and shape-family signals require forward-applicable template axioms; don't apply to predicate-axiom-only theories. Qualifies C.2's "generalizes" claim. [Result](results/C.2.1_oq2_validation.md).

### B.7 — Layer-3 nested abstraction (meta-meta-families) | ✓ done
SUPER_META_SHAPE_FAMILY_MARKER + discover_super_meta_shape_families. Groups L3 nested families by shared L2 member. On OQ#1 mints 1 super-meta containing both nested families (they share shape_premise_p0-1_p1-2). 4-layer recursive structural abstraction: L0→L1→L2→L3→L4. 3 unit tests, 554 lib tests pass. [Result](results/B.7_super_meta_l4.md).

### E.2 — Drive query meta-R replacement (full) | ✓ done (verification only)
Audit complete: runtime decision paths already use `is_drive_penalty_via_meta_r`. Only the registration source-of-truth call site keeps `drive.is_penalty()` (structurally necessary — boot circular dependency). No code change required. [Result](results/E.2_drive_query_audit.md).

### F.2 — Family-aware merge candidate selector | ✓ done (signal)
Family-signature complementarity (1 - Jaccard of family-set per theory) computed pairwise. On OQ#1 picks (t_0, t_2) at 0.667 as most complementary — distinct from Alpha-5's (t_2, t_3) Jaccard pick. **Caveat**: signal needs combining with quality-floor to avoid diluting good theories. [Result](results/F.2_family_aware_merge.md).

### F.3 — Cross-precision-driven theory merge | ✓ done
STRONGLY POSITIVE convergent finding. (t_2, t_3) max_diff = **0.0000** — identical cross-precision column profiles. **Same pick as Alpha-5's smart-merge** (different signal, same answer) → high-confidence merge target. Method-of-method: convergent signals from independent metrics is stronger evidence than either alone. [Result](results/F.3_xprec_merge.md).

### A.1 — ILP premise reordering by selectivity | ✓ verified deferred
Audit confirms Alpha-6's diminishing-returns finding. forward_apply leaf-check already short-circuits. Catch-22: selectivity-aware reorder needs neighbor-set sizes, but computing those IS the work. Bottleneck has moved elsewhere per Alpha-6. **No code change**. Closes the last item from the original scout-framework backlog. [Result](results/A.1_premise_reorder.md).
