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

### B.5 — Runtime integration of shape-family discovery | pending
**Goal**: Add `DiscoverAxiomShapeFamilies` as a runtime `ActionKind`. Currently the API is invoked externally from examples. Make it part of the autonomous loop.

**Falsifiable**: runtime spontaneously mints shape families during normal Phase 0 runs without external intervention.

### B.6 — Family of families (nested abstraction) | pending
**Goal**: When multiple shape families themselves share structure (e.g., several "shared-premise" families with overlapping premise sets), promote that to a higher-order abstraction.

**Risk**: combinatorial. Probably needs constraint via cross-precision before pursuing.

## Phase C — Cross-cutting cleanup

### C.1 — Extract OQ#1 stream to shared library | ✓ done
17 stream-fn copies → 2 in lib (`src/test_substrates/{oq1,long5k}.rs`). 16 examples refactored via Python script + 1 manual; all 51 examples build, 545 lib tests pass. ~1300 lines net deletion. [Result](results/C.1_oq1_extraction.md).

### C.2 — Cross-precision validation on long5k | pending
**Goal**: Re-run Phase Alpha-7..9 (dream phase, cross-precision-driven demote, varying-T) on the long5k substrate. Does the structural-signal vs accumulative-signal gap hold on a more complex substrate?

**Falsifiable**: cross-precision still picks correct demote target on long5k AND still wins on threshold-crossing time.

### C.3 — Integer-direction prep work | pending
**Goal**: Implement the smallest mechanism that mints a new identifier via axiom application. E.g., a "successor closure" rule that, given seed `0` and rule `R(succ_marker, n) → R(succ_marker, S(n))`, generates `S(0), S(S(0)), ...`.

**Risk**: deep — requires ADR on identifier minting, constitution check on commitment 4 (deterministic minting must produce token-identical results to external input).

## Phase D — Dream phase follow-ups

### D.1 — Rejection-based dreaming | pending
**Goal**: Generate substrates that DELIBERATELY violate a theory's axioms (random noise on top of structure). Forward-apply theories on these "negative" substrates, see which axioms still predict consistently.

**Falsifiable**: axioms that hold on rejection-substrates are universally robust; axioms that fail are substrate-specific.

### D.2 — Predicate-axiom enforcement during substrate generation | pending
**Goal**: Currently `generate_substrate_from_theory` ignores predicate axioms (antisymmetry, totality). For theories containing antisymmetry, the random seed could violate it. Filter seeds to maintain predicate constraints.

**Falsifiable**: post-fix substrates verify all theory axioms (including predicate) at rate 1.0.

### D.3 — Composite scheduler signal | pending
**Goal**: Blend primary-rate + cross-precision-mean into a single tournament score. Test composite vs each alone on tournament decisions.

**Falsifiable**: composite outperforms either alone on a multi-substrate test.

### D.4 — Continuous dream loop in runtime | pending
**Goal**: Run dream phase every K ticks; demote whenever cross-precision drops below threshold. Tests stability and overhead.

**Risk**: dream phase has nontrivial cost (substrate generation). Needs careful K tuning.

## Phase E — Constitutional cleanup (drive layer)

### E.1 — Verify H2.1.0 drive-as-meta-R registration | pending
**Goal**: H2.1.0 was supposedly done (`register_drives_in_rset` exists). Verify the registrations are correct, queryable, and used by no path that breaks EP.

**Falsifiable**: query `rset.right_of(DRIVE_MARKER)` returns the registered drives.

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

(empty initially; appended as new directions emerge)
