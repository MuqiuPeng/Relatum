# ADR 0080 — LP-threshold tuning (2026-05-11)

**Status**: ✓ done. 3000-tick OQ#2 (previously hung) now completes in 6.2 min.
**Mechanism**: [ADR 0080 — learning-progress-aware drive](../decisions/0080-learning-progress-aware-drive.md)
**Predecessor result**: [`phase_emergence_adr0080_partial.md`](phase_emergence_adr0080_partial.md) (mechanism ship + long-horizon hang observation)
**Example**: [`examples/oq2_long_horizon_lp_tuned.rs`](../../examples/oq2_long_horizon_lp_tuned.rs)
**Logs**:
- LP_WINDOW=10 LP_THRESHOLD=0.10 (still hangs at tick 1250): [`logs/2026-05-11_oq2_3k_lp_window10_th010.log`](../../logs/2026-05-11_oq2_3k_lp_window10_th010.log)
- LP_WINDOW=10 LP_THRESHOLD=0.20 (completes 6.2 min): [`logs/2026-05-11_oq2_3k_lp_window10_th020.log`](../../logs/2026-05-11_oq2_3k_lp_window10_th020.log)

## Problem

ADR 0080 shipped with LP_WINDOW=30, LP_DRIVE_THRESHOLD=0.05 — values chosen as starting guesses in the open-questions section of the ADR. Empirical observation in `phase_emergence_adr0080_partial`: long-horizon OQ#2 runs (3k+ ticks) hung at log header after ~5-minute monitor intervals.

Root cause: at LP_WINDOW=30 + LP_THRESHOLD=0.05, an OQ#2 substrate that exhausts its 7-canonical structural ceiling around tick ~750 still keeps drive-driven dispatches firing for ~5 minutes because:

1. Each DiscoverPatterns dispatch costs ~10s (multi-size fallback at sample_count=400).
2. LP_WINDOW=30 means **30 consecutive zero-mint dispatches** are needed before LP drops below 0.05.
3. 30 × 10s = 5 minutes during which dispatches still happen.

The mechanism was correct; the threshold was too lenient for the dispatch cost.

## Iteration

### LP_WINDOW=10, LP_THRESHOLD=0.10 (first attempt)

Expected: faster gate closure because window shrinks 3×. Predicted: ~100s gate-close time.

Actual: **still hangs**. At tick 1250, step time grew to **567s (9.5 min)** for 250 ticks. DP attempts jumped from 62 at tick 1000 to 303 at tick 1250 (+241 attempts in 250 ticks).

Diagnosis (mid-experiment):

> LP is computed per size. If size-3 has 30% mint rate (lucky) and size-2 has 0% mint rate (saturated), then **size-3's gate stays open even though size-2's closed**. The dispatch path uses multi-size fallback, so the runtime keeps trying all sizes 2-5. Aggregate DP hit rate is 9% but recent-window LP at the open size is above threshold.

Each individual size's LP needs to be **above 10%** to keep its gate open. With LP_THRESHOLD=0.10 a single mint in a 10-window = 10% LP, which barely stays above the threshold.

### LP_WINDOW=10, LP_THRESHOLD=0.20 (final)

Raise the threshold to 0.20: now a size needs **>20% mint rate** in the recent 10 attempts to keep its gate open. Single isolated successes can no longer keep gates indefinitely open.

Result: 3000-tick OQ#2 completes in **6.2 minutes**.

## Verification

```
HORIZON=3000 ticks, LP_WINDOW=10, LP_DRIVE_THRESHOLD=0.20:

 tick=  250 | rset=  96 ax=2 pats= 2 | DP=  5/2(40%) | step  0.1s   ← active minting
 tick=  500 | rset= 323 ax=2 pats= 5 | DP= 11/5(45%) | step  5.0s
 tick=  750 | rset= 407 ax=2 pats= 7 | DP= 21/8(38%) | step 13.6s   ← canonical ceiling reached
 tick= 1000 | rset= 485 ax=2 pats=10 | DP= 33/12(36%) | step 21.4s
 tick= 1250 | rset= 527 ax=2 pats=11 | DP= 38/13(34%) | step 10.4s   ← drive-driven slowing
 tick= 1500 | rset= 527 ax=2 pats=11 | DP= 38/13(34%) | step  0.0s   ← LP gate CLOSED, runtime idle
 tick= 1750 | rset= 552 ax=2 pats=12 | DP= 39/14(35%) | step  3.4s   ← brief productive wake
 tick= 2000 | rset= 492 ax=2 pats= 9 | DP= 39/14(35%) | step 32.0s   ← prune burst starts
 tick= 2250 | rset= 501 ax=2 pats= 9 | DP= 39/14(35%) | step 49.4s
 tick= 2500 | rset= 510 ax=2 pats= 9 | DP= 39/14(35%) | step 62.4s
 tick= 2750 | rset= 519 ax=2 pats= 9 | DP= 39/14(35%) | step 77.3s
 tick= 3000 | rset= 519 ax=2 pats= 9 | DP= 39/14(35%) | step 94.2s

 Total wall-clock: 369.0s (6.2 min)
```

Key observations:
- **DP attempts cap at 39** after tick 1750 (vs runaway 303 at tick 1250 under th=0.10). LP gate correctly closes on saturation.
- **DP hit rate stabilizes at 35%** (well above LP_DRIVE_THRESHOLD=0.20 floor when gate IS open, signaling productive engagement).
- **tick 1500 step=0.0s**: runtime correctly enters idle sleep when both LP and event signals are quiet.
- **Total wall-clock 6.2 min**: from "hangs indefinitely" to "completes in reasonable time."

## Implementation

Centralized two LP constants in `src/runtime/agent_view.rs`:

```rust
pub const LP_WINDOW: usize = 10;
pub const LP_DRIVE_THRESHOLD: f64 = 0.20;
```

Previously these were local constants duplicated across 4 sites (`frontier.rs`, `autonomous.rs`, `scheduler_rule.rs` × 2). Refactor: all 4 sites now reference the centralized constants. Future re-tuning is a one-line change.

## What this slice did NOT solve

A separate observation from the same run: **prune action enters a per-tick loop starting around tick 2000**. Episode count grows 1-per-tick (eps 248 at tick 2000 → 1000 at tick 3000); prune count grows 1-per-tick (193 → 1193). Step time grows from 32s → 94s linearly with tick.

This is **NOT a LP-tuning issue** — DP rate stays cap'd at 39, LP gate is closed. The runtime is correctly suppressing drive-driven DiscoverPatterns dispatches. The growth comes from a different action class (PruneLowValueObjects) firing every tick when the runtime is awake.

Likely cause: scheduler picks `PruneLowValueObjects` as the highest-priority frontier item when no DP candidate is alive (LP gate closed it). The prune action is not rate-limited. Each prune episode is cheap individually but accumulates linearly.

This is a separate scheduler tuning issue, not in scope for ADR 0080. Recorded as follow-up.

## Follow-ups

- **Prune-loop rate limiting**: scheduler should not fire `PruneLowValueObjects` every tick when nothing's changing. Likely fix: cooldown on prune action when previous prune episode produced no change. Separate from ADR 0080.
- **Multi-size LP gate**: current design gates per-size based on modal canonical. Could check aggregate LP across all sizes used in multi-size fallback. Mitigates the "size-3 LP open keeps size-2 dispatches running" issue. Worth ADR-grade discussion if it surfaces in another substrate.
- **Re-run 6000+ tick to confirm no further hangs**: long-horizon stability.
- **Re-validate ADR 0079.1 result**: that ADR's sustained-cognition demonstration used 800-tick horizon; verify post-tuning numbers don't regress.

## Constitution check

No structural changes. Only constants and call-site refactoring. Constitution unaffected.

## Verdict

**ADR 0080 LP-threshold tuning closed.** 3000-tick OQ#2 (the test case that motivated ADR 0080 in the first place) now completes in 6.2 minutes with correctly-closing LP gates. The mechanism shipped at LP_WINDOW=30/THRESHOLD=0.05 was correct in principle but the parameters were too lenient. LP_WINDOW=10/THRESHOLD=0.20 is the working configuration.

The 2026-05-11 retrospective's "ADR 0080 threshold tuning is open" item is now resolved.
