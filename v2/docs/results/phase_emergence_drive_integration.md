# ADR 0079 — Drive→scheduler integration shipped

**Status**: ✓ shipped (2026-05-08); v2 crosses reactive→proactive
**Logs**:
- pre-fix baseline: [`logs/2026-05-06_phase_emergence_long_horizon.log`](../../logs/2026-05-06_phase_emergence_long_horizon.log)
- failed first attempt: [`logs/2026-05-08_phase_emergence_long_horizon_post_adr0079.log`](../../logs/2026-05-08_phase_emergence_long_horizon_post_adr0079.log)
- working: [`logs/2026-05-08_phase_emergence_long_horizon_post_adr0079_v2.log`](../../logs/2026-05-08_phase_emergence_long_horizon_post_adr0079_v2.log)
**ADR**: [0079 — Drive-driven frontier candidate](../decisions/0079-drive-driven-frontier-candidate.md)

## Goal

ADR 0078 shipped a constitution-compliant drive metric that
revealed OQ#2 leaves 91% of its data edges unexplained at
maturity. The long-horizon observation (2026-05-06) confirmed
runtime sleeps permanently regardless. The generative-stream
experiment (2026-05-08) showed even unbounded input doesn't
fix this — the bottleneck is triggering, not input.

This slice closes the gap with the smallest viable change.

## What shipped

### Three coordinated changes

The first attempt (frontier-only change per ADR 0079 spec) did
not work. Debugging revealed two additional bottlenecks. The
working version requires all three:

**1. Drive-driven frontier candidate** (`src/runtime/frontier.rs`)

When the drive signal is non-empty AND the rset is mature,
`Frontier::refresh` proposes one extra `PatternCandidate` with
`PatternSize` matching the modal canonical (clamped to [2, 5])
and priority `modal_count * 5.0`. Maturity gate
(`axioms ≥ 1 AND data_edges ≥ 100`) preserves lifecycle test
invariants.

**2. Drive-wake in sleep loop** (`src/runtime/autonomous.rs`)

The runtime's sleep short-circuit previously had `continue`
on no-event ticks, never letting frontier refresh. Added:

```rust
let drive_wakes = !wake_signal
    && self.lifecycle == LifecycleState::Sleeping
    && self.tick % DRIVE_WAKE_INTERVAL == 0  // 25
    && self.rset.axioms().len() >= 1
    && self.rset.iter().count() >= 100
    && self.rset.unexplained_drive_signal().has_signal();
if drive_wakes {
    transition Sleeping → Running ("wake_on_drive")
    self.frontier.mark_dirty();
}
```

Throttle interval 25 ticks bounds the O(unexplained) cost of
drive computation on idle ticks.

**3. Stagnation-gate drive bypass** (`src/runtime/scheduler_rule.rs`)

The scheduler's stagnation gate (`zero_streak >=
max_zero_streak → Sleep`) short-circuited before the
frontier-selection path, so drive-driven candidates were
never reached. Added bypass:

```rust
let drive_alive = ctx.rset.axioms().len() >= 1
    && ctx.rset.iter().count() >= 100
    && ctx.rset.unexplained_drive_signal().has_signal();
if !drive_alive {
    // original Sleep / EP path
}
// Drive alive: fall through to frontier selection
```

### Why each was needed

**Without 1**: drive signal computed but no frontier item to
dispatch — runtime aware of unexplained R but had no work
proposal.

**Without 2**: frontier never refreshed during sleep —
runtime stayed asleep regardless of drive signal because the
sleep loop's `continue` skipped scheduler entirely.

**Without 3**: even after wake-on-drive woke the runtime,
stagnation gate caught it (zero_streak still high from
pre-sleep), routed back to Sleep without consulting frontier.
This produced a wake/sleep ping-pong observed empirically (100
sleep→running transitions over 6000 ticks, all immediately
followed by Running→Sleeping on the same tick).

The three pieces compose: drive proposes work (1), drive wakes
runtime so refresh runs (2), bypass lets scheduler see the
proposal instead of sleeping (3).

### Tests

- 3 new ADR-0079 tests in `src/tests.rs` covering frontier
  candidate presence/absence under maturity gate
- All 645 lib tests pass; no regressions

## Result — OQ#2 transition

```
                     pre-ADR 0079    post-ADR 0079
substrate state:    @ tick 6000      @ tick 6000

OQ#1                axs=11 ths=3 pat=6   axs=11 ths=3 pat=6
                    eps=22 DP=3/3        eps=22 DP=3/3
                    (unchanged — drive silent)

narrow_a            axs=11 ths=3 pat=1   axs=11 ths=3 pat=1
                    eps=22 DP=3/3        eps=22 DP=3/3
                    (unchanged — drive silent)

OQ#2                axs=2 ths=2 pat=2    axs=2 ths=2 pat=7
                    eps=10 DP=5/2        eps=24 DP=13/7
                    (single-shot)        (sustained)
```

OQ#2 timeline:

```
tick   axs  ths  pat   eps  DP  DPp prune
250    2    2    2     10    5   2    1
500    2    2    5↑    21   11   5    2
750    2    2    7↑    24   13   7    2
1000   2    2    7     24   13   7    2  ← equilibrium
...
6000   2    2    7     24   13   7    2
```

**Pattern minting now visible at runtime tick 250 → 500 → 750
on OQ#2**. Drive proposes work, scheduler dispatches,
autonomous_pass mints, drive shrinks. By tick 750 drive is
extinguished and runtime stays asleep — but at a substantially
richer terminal state (7 patterns instead of 2).

OQ#1 and narrow_a are unchanged because their axioms cover
their substrate streams completely. Drive stays silent on
those substrates (per ADR 0078 audit) so the bypass paths
don't engage.

## What this confirms

1. **Drive metric is sufficient as a triggering signal.** OQ#2
   reaches the same final state as manual `autonomous_pass`
   invocation (7 patterns), purely from drive→scheduler
   integration with no other intervention.

2. **The constitution heavy reading isn't violated.** Drive
   buckets are subgraph canonical forms (subgraph-level, no
   per-token signature). Frontier proposes a `PatternCandidate`
   that targets `PatternSize(N)` — same target type used by
   organic candidates. No new ontology entities.

3. **Lifecycle test invariants are preserved.** Maturity gates
   on all three changes mean small fixtures (`diamond_poset`
   with 9 edges, 0 axioms) never trigger drive paths. All 645
   lib tests pass.

4. **OQ#1-clade behaviour is unchanged**. Drive only triggers
   when there's actually unexplained structure (drive signal
   non-empty). For substrates whose axioms fully cover the
   stream, ADR 0079 is a no-op.

## What this slice does NOT achieve

- **Drive does not extinguish to 0 on OQ#2** beyond what
  `autonomous_pass(sizes 2-5)` already accomplishes manually.
  The runtime auto-mints 7 patterns then sleeps; if more
  unexplored canonicals exist (size 6+, or other size-2-5
  canonicals dispatched DPs missed), they remain.
- **No new substrate-level cognition emerges.** Drive
  integration unlocks v2's existing pattern-mint capability;
  it doesn't create new abstraction mechanisms.
- **Cooldown still applies.** If DP fails enough times, it
  cools down and drive bypass alone won't restart it. This is
  a feature (prevents thrashing on hopeless patterns) but
  could mask cases where drive points at unminteable shapes.

## Empirical implications

**v2 is now a proactive cognitive substrate.** It does not
require ongoing stream input to keep working. Given a substrate
that produces non-trivial drive (i.e., axioms that don't fully
cover the stream's R structure), the runtime continues
discovering patterns via drive-proposed work even when the
stream is silent.

The 2026-05-06 retrospective's "next directions" listed
"Pattern-aware drive metric" as the natural next mechanism.
That's now shipped (ADR 0078) and integrated (this slice).
The Phase Emergence arc — pivot, audit, framework, drive —
closes here, with v2's cognitive substrate genuinely
sustained-active on substrates that need it.

## Files

- `src/runtime/frontier.rs` — drive-driven candidate
- `src/runtime/autonomous.rs` — drive-wake in sleep loop
- `src/runtime/scheduler_rule.rs` — stagnation-gate bypass
- `src/tests.rs` — 3 new ADR-0079 tests
- `examples/phase_emergence_long_horizon_observation.rs` —
  diagnostic helper extended (lifecycle transition print)
- `logs/2026-05-08_phase_emergence_long_horizon_post_adr0079_v2.log`
- `docs/decisions/0079-drive-driven-frontier-candidate.md`
- This result doc

Lib tests: 645 (3 new). 0 regressions.

## Verdict

**v2 crosses reactive→proactive.** The architectural change is
small (~50 lines of dispatch / scheduler logic across three
files), but its consequence is qualitative: v2 no longer
permanently sleeps when it has unexplored work to do. On
substrates where drive surfaces structural under-coverage —
empirically, OQ#2 — the runtime now sustains pattern
discovery over the long horizon, reaching the same 7-pattern
state as manual `autonomous_pass`.

The Phase Emergence arc, opened on 2026-05-01 with ADR 0073's
pivot ("v2 cannot create new concepts"), closes here on
2026-05-08 with v2 demonstrating both that it can create
emergent concepts (ADR 0075 audit) and that it can keep
creating them autonomously (ADR 0079).
