# 0079.1: Drive-aware mode-thrash bypass

Status: Proposed
Date: 2026-05-08

Parent: [0079 — Drive-driven frontier candidate](0079-drive-driven-frontier-candidate.md)

## Context

ADR 0079 shipped drive→scheduler integration in three pieces:
drive-driven `PatternCandidate`, drive-wake from sleep, and
stagnation-gate drive bypass. The 15000-tick OQ#2 equilibrium
observation (`docs/results/phase_emergence_oq2_equilibrium.md`)
revealed three distinct phases:

```
phase                    tick range     patterns   eps   wakes
1. Active mint           0–750          2 → 7      10→24  0 → 18
2. Wake without disp.    750–2000       7 stuck    24     18→100
3. Frozen equilibrium    2000–15000     7          24     100 cap
```

Phase 1 is what ADR 0079 promised. Phase 3 reveals a second
gate the ADR didn't address: even with stagnation-gate bypass,
the runtime stops dispatching ~tick 2000 despite drive
remaining non-empty (eventually plateauing at 124 unexplained
edges / 5 buckets).

The remaining gate is **`would_thrash`**: in `RuleBasedScheduler::
switch_or_sleep`, when the cumulative count of forward+back
mode transitions between two modes reaches `max_mode_
oscillations` (default 4), the scheduler returns `Sleep`
instead of `SwitchMode(target)`.

The trace is:

1. `wake_on_drive` brings runtime out of `Sleeping`
2. `frontier.dirty=true` triggers refresh → drive-driven
   `PatternCandidate` appears
3. `scheduler.choose()` enters with `mode=Reflect`
4. Reflect arm: `has_expand_work(ctx)` returns true (drive
   item present)
5. `switch_or_sleep(ctx, Expand)` called
6. `would_thrash` counts Reflect↔Expand transitions:
   accumulated from Phase 0 + Phase 1 → ≥ 4
7. Returns `Sleep` → lifecycle Running → Sleeping same tick

Drive-aware stagnation bypass let scheduler reach the mode
arm; drive-aware thrash bypass is needed to let it switch
into the mode where dispatch can happen.

## Decision

Add a **drive-aware bypass** to `switch_or_sleep`, mirroring
the stagnation-gate bypass added in ADR 0079:

```rust
fn switch_or_sleep(&self, ctx: &SchedulerContext<'_>, target: RuntimeMode) -> SchedulerDecision {
    if self.would_thrash(ctx, ctx.mode, target) {
        // ADR 0079.1 — drive-aware thrash bypass.
        // When drive is alive on a mature rset, mode oscillation
        // is justified by structural unexplored work, not by
        // policy thrashing. Override the thrash gate.
        const MATURE_DATA_EDGE_FLOOR: usize = 100;
        let drive_alive = !ctx.rset.axioms().is_empty()
            && ctx.rset.iter().count() >= MATURE_DATA_EDGE_FLOOR
            && ctx.rset.unexplained_drive_signal().has_signal();
        if drive_alive {
            return SchedulerDecision::SwitchMode(target);
        }
        return SchedulerDecision::Sleep;
    }
    SchedulerDecision::SwitchMode(target)
}
```

### Why path A (and not B or C)

The 5/8 result doc proposed three options. Per-option analysis:

**Option A — drive-aware thrash bypass (this ADR)**

- Pros:
  - Mirrors the existing stagnation bypass exactly (consistency)
  - Single-file change (~5 lines)
  - Pattern-cooldown gate remains the safety net: if DP keeps
    failing, cooldown blocks PatternCandidate selection,
    `has_expand_work` returns false on Reflect, no more
    switch attempts → loop terminates naturally
  - Episode log integrity preserved (all dispatches go through
    normal scheduler path)
- Cons:
  - In principle, can produce more mode transitions in
    pathological cases. Mitigated by pattern-cooldown gate.

**Option B — direct dispatch path bypassing scheduler**

- Pros:
  - Decouples drive from scheduler ecology entirely
- Cons:
  - Episode log no longer reflects dispatches caused by drive
  - Two parallel dispatch paths (scheduler + drive) is
    architectural divergence
  - ADR 0076 micro-agent reframing depends on episode-log
    being the single source of agent activity; option B
    breaks that

**Option C — drive events decrement cooldown/thrash counters**

- Pros:
  - Self-balancing: drive activity reduces gate counters,
    counters increment as drive is consumed
- Cons:
  - More complex (which counters? how much decrement?)
  - Risk of pathological tuning (counters bounce around 0)
  - Harder to reason about — gates are no longer monotonic

Option A is the smallest, most consistent, lowest-risk choice.
The cons of A (pathological mode oscillation) are bounded by
the pattern-cooldown gate that already exists — drive doesn't
bypass cooldown, so a permanently-failing DP eventually stops
the cycle.

## Alternatives considered

**Alt: Reset `mode_transition_counts` periodically.** Adding
a decay or reset would let the thrash gate forgive old
oscillations. Rejected: changes the semantics of
`mode_transition_counts` for all consumers (not just drive),
and the right decay rate isn't obvious. Drive bypass is
scoped to the specific case where it helps.

**Alt: Make `mode_transition_counts` drive-aware (don't
increment when drive caused the switch).** Cleaner than
post-hoc reset but requires propagating "drive caused this"
through all transition recording sites. Heavier change for
similar effect.

**Alt: Defer fix until a longer empirical observation
demonstrates Option A's risk profile.** Tempting but the
current observed problem (Phase 3 frozen) is concrete; option
A's risk (theoretical mode thrash) is hypothetical. Ship A
and observe.

## Consequences

**Now possible:**

- Phase 3 frozen state on OQ#2 should disappear: drive's
  discovery of new unexplored canonicals will trigger
  dispatches throughout the run, not just the first ~2000
  ticks
- Pattern population may grow beyond 7 on OQ#2 if drive's
  canonical buckets exhaust subsequent canonicals
- The runtime's "extended initialization" framing from the
  prior result doc may be replaced by genuine sustained
  cognition

**Now harder:**

- If pattern-cooldown trips (DP hit rate falls below 10%
  after 30+ attempts) the runtime will stop dispatching
  even with drive alive. This is correct (cooldown protects
  against thrashing on hopeless patterns) but means
  "sustained cognition" is bounded by mintability of
  remaining drive buckets.
- Mode oscillation count may grow unboundedly in
  `policy_stats.mode_transition_counts`. This is a
  diagnostic concern only (the count is read by `would_
  thrash` which is now bypassed by drive); no behavior
  depends on it bounded.

**Newly easy:**

- The "v2 has reactive→proactive transition" claim from ADR
  0079's commit can be revised to "v2 sustains drive-driven
  cognition until DP cooldown or drive exhaustion." This is
  a more accurate characterization that doesn't require
  caveats about Phase 3 freeze.

## Implementation

Single-file change to `src/runtime/scheduler_rule.rs` in
`switch_or_sleep`. Add 1-2 unit tests covering:
- thrash bypass triggers when drive alive (mature rset +
  drive non-empty)
- thrash bypass does NOT trigger when drive empty (lifecycle
  test invariant preserved)

Re-run:
- Lib test suite (645 tests, 0 expected regressions —
  diamond_poset fixtures fail maturity gate)
- `phase_emergence_oq2_equilibrium` to verify Phase 3 freeze
  resolved
- `phase_emergence_capability_demo` for end-to-end check
- Updated result doc explaining the resolution

## Empirical verification target

Before fix (Phase 3 from current observation):
- patterns 7, episodes 24, drive plateaus at 124 unexplained,
  wake count 100 cap, second-half episodes added = 0

After fix expected:
- patterns > 7 (drive consumed → more mints)
- episodes > 24
- drive < 124 (consumed by new dispatches), eventually 0 or
  near-0
- second-half episodes added > 0

If "after" shows the same numbers as "before", path A's
hypothesis is wrong and the freeze comes from another gate
(pattern cooldown, EvaluatePredictions interaction, etc.) —
in which case ADR 0079.2 follows.
