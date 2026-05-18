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

## What this slice did NOT solve initially — then fixed in follow-up

A separate observation from the LP-only run: **prune action entered a per-tick loop starting around tick 2000**. Episode count grew 1-per-tick (eps 248 at tick 2000 → 1000 at tick 3000); prune count grew 1-per-tick (193 → 1193). Step time grew from 32s → 94s.

This was NOT a LP-tuning issue (DP rate stayed cap'd at 39). The prune-loop turned out to have two distinct causes:

### Cause A — type mismatch in target routing

`RSet::rank_by_counterfactual` returns three kinds of ids: patterns, theories, and extension edges. The frontier's `LowValueObjectForPrune` proposal wrapped ALL of them as `FrontierTarget::Pattern(id)`. The action handler's `Pattern(id)` branch only calls `retract_pattern`, which fails silently for theory ids or extension-edge ids. The same id then keeps appearing in `rank_by_counterfactual` next refresh and gets re-proposed indefinitely.

**Fix (frontier.rs)**: route theory ids to `FrontierTarget::Theory(id)` and skip extension edges (no single-target prune handler for them; the WholeRSet branch handles them but isn't proposed here).

### Cause B — second proposal site (refresh_stale_prune) bypassing filter

The runtime calls `refresh_stale_prune` immediately after `refresh_with_episodes`. Stale-prune proposes `LowValueObjectForPrune` for any pattern whose `last_improved_tick` is too old. This is a SECOND proposal path that didn't apply the recently-pruned filter, so the same pattern got re-proposed every tick via the stale-prune route after its prune attempt failed.

**Fix (frontier.rs + frontier struct)**: `refresh_with_episodes` now computes a `recent_prune_targets` set and caches it on `self`. `refresh_stale_prune` reads `self.recent_prune_targets` and skips proposing for any id already there. Both proposal paths now apply the same rate-limiting filter.

### Final 3k OQ#2 wall-clock

After both fixes:

```
HORIZON=3000 ticks (post-LP tuning + prune-routing + stale-prune filter):

 tick=  250 | rset=  96 pats= 2 eps=  10 | DP=  5/2(40%) prune=1 | step  0.1s
 tick=  500 | rset= 323 pats= 5 eps=  21 | DP= 11/5(45%) prune=2 | step  5.0s
 tick=  750 | rset= 407 pats= 7 eps=  35 | DP= 21/8(38%) prune=3 | step 13.4s
 tick= 1000 | rset= 485 pats=10 eps=  50 | DP= 33/12(36%) prune=4 | step 21.4s
 tick= 1250 | rset= 527 pats=11 eps=  55 | DP= 38/13(34%) prune=4 | step 10.3s
 tick= 1500 | rset= 527 pats=11 eps=  55 | DP= 38/13(34%) prune=4 | step  0.0s
 tick= 1750 | rset= 552 pats=12 eps=  56 | DP= 39/14(35%) prune=4 | step  3.4s
 tick= 2000 | rset= 587 pats=12 eps=  74 | DP= 50/17(34%) prune=7 | step 29.7s
 tick= 2250 | rset= 660 pats=15 eps=  78 | DP= 54/19(35%) prune=7 | step 13.2s
 tick= 2500 | rset= 656 pats=14 eps=  85 | DP= 57/19(33%) prune=8 | step 12.6s
 tick= 2750 | rset= 665 pats=14 eps=  85 | DP= 57/19(33%) prune=8 | step  2.3s
 tick= 3000 | rset= 665 pats=14 eps=  85 | DP= 57/19(33%) prune=8 | step  0.0s

 Total wall-clock: 111.4s (1.9 min)
```

3000-tick OQ#2:
- pre-tuning: hangs indefinitely
- LP-tuning only: 6.2 min (1193 prune episodes from the prune-loop)
- LP-tuning + prune-routing + stale-prune filter: **1.9 min** (8 prune episodes total)

Bonus: with the prune loop gone, the LP gate sustained engagement properly, and the system **minted more patterns** (final pats=14 vs LP-only pats=9). The runtime now sleeps efficiently after canonical saturation but keeps drive-engaged when new structure arrives.

Log: [`logs/2026-05-11_oq2_3k_full_prune_fix.log`](../../logs/2026-05-11_oq2_3k_full_prune_fix.log).

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
