# OQ#2 long-horizon equilibrium observation (post ADR 0079)

**Status**: ✓ done (2026-05-08); honest characterization of partial fix
**Log**: [`logs/2026-05-08_oq2_equilibrium_15k.log`](../../logs/2026-05-08_oq2_equilibrium_15k.log)
**Example**: [`examples/phase_emergence_oq2_equilibrium.rs`](../../examples/phase_emergence_oq2_equilibrium.rs)
**Predecessor**: [`phase_emergence_drive_integration.md`](phase_emergence_drive_integration.md)

## Goal

Per the user's request after ADR 0079 ship: with runtime now
auto-minting on OQ#2, **what does the long-horizon
mint-and-trim equilibrium actually look like?** Is it ongoing
balanced cycle? Single bigger initialization burst? Something
else?

## Method

Focused observation on OQ#2 (the only substrate where ADR 0079
changes behaviour; OQ#1-clade has silent drive). Horizon 15000
ticks, snapshots every 250 ticks. Tracks per-snapshot deltas
in patterns / pattern_instances / episodes plus drive
unexplained count and bucket count.

## Result — three distinct phases

```
phase                  tick range   patterns   eps   drive_unex   wakes
1. Active mint         0–750        2 → 7      10→24    0 → 4      0 → 18
2. Wake without disp.  750–2000     7 stuck    24       4 → 52     18→100
3. Frozen equilibrium  2000–15000   7          24       0→124cap   100 cap
```

### Phase 1: Active mint (0–750)

The initialization phase from before ADR 0079 (which would
have ended at tick 250 with 2 patterns) extends to tick 750
with 7 patterns minted. This is what ADR 0079 was supposed
to deliver, and it does.

- DP dispatches: 5 → 13 (8 new dispatches due to drive)
- DP positive: 2 → 7 (5 new mints)
- Pattern instances: 12 → 60 (5x growth)
- Wake-on-drive transitions: 0 → 18

By tick 750 the runtime has minted the same 7 patterns that
manual `autonomous_pass` reaches. Drive is briefly low (4
unexplained, 1 bucket).

### Phase 2: Wake without dispatch (750–2000)

Runtime continues to wake on drive every 25 ticks (`wake`
column climbs 18 → 44 → 68 → 78 → 94 → 100), but **no new
dispatches occur**. DP count frozen at 13. Patterns frozen
at 7. Episodes frozen at 24.

Meanwhile drive **grows**: 4 → 20 → 34 → 43 → 52 unexplained,
buckets 1 → 4. Stream is still feeding edges (OQ#2 stream ends
at tick 4209), but axioms can't cover them so unexplained
accumulates.

The runtime sees this — wake-on-drive triggers — but cannot
act on it.

### Phase 3: Frozen equilibrium (2000–15000)

By tick 2000, wake-on-drive transitions plateau at exactly 100.
Drive plateaus at 124 unexplained / 5 buckets after tick 4250
(post-stream-end). The runtime stays in this state for the
remaining 11000 ticks.

Across the second half of the horizon (ticks 7750–15000):
- 0 episodes added
- 0 pattern instances added
- 0 new patterns

## Diagnosis: ADR 0079 is a partial fix

ADR 0079 successfully removed the single-shot ceiling
(2→7 patterns) but did not enable truly sustained
mint-and-trim. The runtime now has three distinct sleep
states:

1. **Pre-ADR 0079**: deep sleep on frontier-empty, never wakes
2. **Post-ADR 0079, drive accessible**: wake-on-drive →
   dispatch → mint → repeat (Phase 1 above; ~750 ticks
   active)
3. **Post-ADR 0079, drive observable but unactionable**:
   wake-on-drive → scheduler returns Sleep without dispatch
   (Phase 3 above; permanent for the remaining run)

The transition from state 2 to state 3 happens around tick
2000 on OQ#2. Wake count caps at exactly 100 — strong
signal that some gate counts wake events and trips at 100.

### Most likely cause

The wake-count cap at 100 plus the lack of new dispatches
suggests a **mode-thrash gate** (or analogous oscillation
limiter) is firing. With each wake-on-drive triggering a
mode transition (e.g., Reflect→Expand→Reflect) and the
runtime defaults having `max_mode_oscillations = 4`, the
gate likely refuses further mode transitions after some
threshold of toggles. Once gated, drive-driven candidates
in the frontier can't be picked because the scheduler can't
enter Expand mode to pick them.

Verification of this hypothesis would require tracing actual
mode transitions per tick — out of scope for this
observation slice but a clean follow-up question.

## What this slice reveals

1. **ADR 0079 part-way achieves its stated goal.** The headline
   "v2 crosses reactive→proactive" from the ADR 0079 commit
   message is technically true — for the first ~750 ticks
   post-Phase-0. After that, the runtime is back to a sleep
   state, just with a different shape.

2. **Mint-and-trim is not an ongoing balanced cycle.** It's
   "extended initialization" — drive lets the runtime do more
   pattern work during init, then the same gates that
   stopped pre-ADR 0079 stop the runtime here too, just
   delayed.

3. **Drive observation continues even when action stops.**
   In Phase 3, drive metric reports 124 unexplained / 5
   buckets — the runtime still *knows* what's left
   unexplored. It just can't *do* anything about it. This
   is information v2 has but doesn't act on.

4. **The honest characterization of v2's current cognitive
   substrate**: it is *more sustained than before*, but still
   ultimately *initialization-bounded*. Phase 1's burst is
   real and substantive (5 additional mints over Phase 0),
   but the second half of any run is identical to pre-ADR
   0079 behavior — frozen.

## What's needed for true sustained cognition

Two candidate fixes (ADR 0079.1 candidates, deferred):

**Option A: Drive-aware mode-thrash bypass.**  Mirror the
stagnation-gate bypass: when drive is alive on a mature rset,
skip the mode-oscillation cap. Risk: actual thrashing,
since mode-thrash exists to prevent endless oscillation.

**Option B: Drive-targeted dispatch path.**  Instead of going
through frontier+scheduler, have wake-on-drive directly
invoke `autonomous_pass(modal_canonical)` via a new dispatch
short-circuit. Skips all gates but creates a parallel
dispatch path that doesn't appear in episode log.

**Option C: Reset cooldown / oscillation counters on drive
events.** Each drive wake increments a "drive bypass count"
that decrements normal cooldown / thrash counters. Allows
sustained drive to override sustained cooldown.

None of these is obviously correct; each has failure modes.
The fact that ADR 0079 fixed one gate (stagnation) but not
all suggests the gate ecosystem is more interconnected than
ADR 0079 modeled. A full sustained-cognition solution may
require systematic gate audit rather than another piecewise
fix.

## Files

- `examples/phase_emergence_oq2_equilibrium.rs`
- `logs/2026-05-08_oq2_equilibrium_15k.log`
- This result doc

## Verdict

**ADR 0079 ships a real but bounded improvement.** v2's
proactive phase is now ~750 ticks instead of ~250 ticks (3x
extension), and the resulting state is qualitatively richer
(7 patterns vs 2, 60 instances vs 12). After that, the
runtime hits another set of gates that the ADR didn't address.

The retrospective from 2026-05-06 noted "stop and observe"
was the right next step. This observation surfaced the
follow-up: ADR 0079 is the first of perhaps several gates
to remove if true sustained cognition is the goal. The full
gate ecosystem audit is open work.

For now, v2 is **more proactive than before, but not fully
proactive**. The user-facing description should reflect this
— not "v2 sustains cognition" but "v2 has an extended
initialization phase driven by drive, after which it
stabilizes." This is still a substantive improvement, just
short of the ambition implied by "reactive→proactive."
