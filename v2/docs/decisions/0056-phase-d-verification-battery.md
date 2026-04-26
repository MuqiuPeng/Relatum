# 0056: Phase D verification battery

Status: Proposed
Date: 2026-04-26

## Context

Phase D0 + D0+ + E0 are implemented end-to-end. The runtime can
discover meta-meta-patterns from its own M1 markers, name them,
and (per ADR 0054 OQ #4) terminate cleanly on a bounded
NoOp-environment seed. ADR 0055 added direction-distinguishing
canonicals.

What still doesn't exist: **systematic confidence that the loop
closure produces meaningful results across diverse seeds**.

The current verification surface is two artefacts:

- `examples/phase_d_demo.rs` — one hand-crafted seed (5
  ESTABLISHED patterns + tiny data substrate) showing the loop
  fires.
- `examples/phase_d_termination.rs` — same seed, longer run,
  shows convergence.

Both artefacts exercise the same "5 fan-shaped patterns around
a marker" topology. ADR 0055's WL-1 limitations may bite
differently on other shapes. Pattern-cooldown / stale-prune /
C0+ M-counter interaction with meta-meta is untested on
non-trivial substrate sizes. The Phase-A battery (ADR 0027
8-case rigorous) gives Phase A this kind of confidence; Phase
D doesn't have its analogue yet.

A second motivation: ADR 0054 OQ #2 (independent meta-meta
cooldown) and OQ #4 (termination) were both closed *on the same
seed*. The OQ #4 verdict ("CONVERGED") could be brittle if
other seeds expose oscillation or cooldown thrash. We don't
know.

## Decision

### Phase F0 — `phase_d_battery` example

A new example `examples/phase_d_battery.rs` that runs the Phase
D loop on a fixed set of seeds and reports a tabular trajectory
+ verdict per seed. Captured to `logs/<date>_phase_d_battery.log`,
analogous to `2026-04-25_phase_a_verification.log`.

#### Seed set (initial — 6 cases)

1. **`fan_only`** — 5 ESTABLISHED-marked synthetic patterns +
   1 disconnected data edge. Today's demo / termination case.
   Baseline.
2. **`diamond_poset`** — the same 4-node a/b/c/d diamond from
   the A-battery, with C0+ promotion **disabled** initially
   (set `min_pattern_age_for_promotion = u64::MAX` for the
   experiment). Runs the data side without any M1, then the
   battery harness flips the promotion gate back on after tick
   100 to see whether real-shape patterns can earn ESTABLISHED
   and feed D.
3. **`bipartite`** — `K_{2,3}` (two left nodes, three right
   nodes, all left→right edges). Different fan structure than
   the synthetic seed; tests whether the meta-meta-discovery
   sees only the "same-marker" anchored shapes or also picks
   up cross-marker patterns when present.
4. **`star`** — one centre node with five outgoing data edges.
   Degenerate fan-out shape on the data side, then six
   ESTABLISHED edges seeded on the resulting patterns. Tests
   whether the data-side and M1-side fan-outs interfere when
   they produce isomorphic meta-meta canonicals.
5. **`equivalence_classes`** — the A-battery's
   `equivalence_3_classes` case. Runs theory discovery first,
   then waits for C0 to promote stable theories, then watches
   D fire on the resulting M1 graph.
6. **`disconnected_islands`** — three independent 3-cycle
   components on disjoint nodes. Tests whether meta-meta sees
   something at all when the data substrate gives no
   cross-cluster signal.

Each seed runs against `RuleBasedScheduler::default()` for a
fixed `HORIZON = 300` ticks (enough to clear C0's age threshold
on the slow seeds without dragging the report into multi-page
territory).

#### Per-seed report shape

```
=== seed: fan_only ===
  tick patterns theories established shared.ax mm.tries mm.hits epis lifecycle
     0        5        0          5         0        0       0    0 Running
    50        7        1          5         0        3       0    4 Sleeping
   ...
   300        7        1          5         0        3       0    4 Sleeping
verdict: CONVERGED (final ticks idle: 5/6)
new patterns named: 2  (p_5, p_6)
final canonical hash counts: …
```

The "final canonical hash counts" line is deliberately vague:
F0 reports a *bag* of canonical hashes, not a literal pin (per
ADR 0055). Useful for diffing across seeds: "fan_only and
disconnected_islands produced the same meta-meta canonicals"
would be a real finding worth investigating.

#### Battery verdict

A summary line at the end:

```
battery summary:
  CONVERGED: fan_only, diamond_poset, ...
  STILL GROWING: ...
  ANOMALOUS (theories=0 but mm.tries > 0): ...
```

The "ANOMALOUS" bucket is deliberate — flags seeds where the
loop fired but produced no theory work. May indicate a real
issue or a feature; the battery is for *surfacing* such
patterns, not enforcing them.

### Phase F1 (sketch, deferred) — D-path scheduling state beyond cooldown

ADR 0054 OQ #2 added an independent cooldown counter for
`DiscoverMetaMetaPatterns`. That covers "should we keep trying
meta-meta given how often it succeeds?" but leaves richer
per-action scheduling state unaddressed:

- `last_meta_meta_tick: Option<u64>` for *cadence* control
  (e.g., "don't run meta-meta more than once per 10 ticks").
- A separate budget bucket for meta-meta exploration so it
  doesn't compete with theory discovery on the same per-tick
  cap.
- An age-aware bias: prefer meta-meta when the system has been
  in Sleep recently (signal that data-side discovery has
  saturated).

These are speculative. F0 first, then revisit if the battery
shows the cooldown counter alone is insufficient.

## Alternatives considered

- **Skip F0 entirely; keep adding ad-hoc demos.** That's where
  Phase D verification started; it doesn't scale and doesn't
  produce a comparable artefact. F0 normalises the comparison.
- **Make F0 a unit-test suite, not an example.** Unit tests
  fail loudly when they should be diagnostic logs. The Phase-A
  pattern (separate example + captured log + complementary
  unit tests in `runtime::tests::a_verification_*`) is the
  right shape for diagnostic verification. Apply it here too.
- **Run all 6 seeds inside one process; share state.** Faster
  to write but conflates failures. Each seed gets its own
  `AutonomousRuntime` so a seed-N runtime crash doesn't
  contaminate seed-N+1's report.
- **Pin canonical hashes in the report.** Pre-ADR 0055 this
  was tempting — small u32 indices, easy to eyeball. Post-E0
  the hashes are u64s with no semantic meaning to a human
  reader. The report should print *bags* of canonical hashes,
  not literals. Consumers who want shape comparison can
  diff bags across seeds.

## Non-goals

- A pass/fail oracle. F0 reports trajectories and verdicts but
  the battery doesn't fail the build on "STILL GROWING" or
  "ANOMALOUS" — these are diagnostic flags, not regressions.
- Running the battery in CI. Captured logs are committed and
  the `cargo run --example phase_d_battery` invocation is
  manual, same as `phase_a_verification`. CI run-on-every-PR
  comes later if drift becomes a problem.
- Replacing existing Phase D unit tests. The
  `runtime::tests::d0plus_*` suite has the load-bearing
  assertions; the battery is for human-readable diagnostic.

## Verification plan

For Phase F0:

1. **Existing 402 tests pass after the example lands.** F0 is
   additive; touching only new files plus optionally the
   `phase_d_demo` re-export. No production-code edits.
2. **Battery runs cleanly on all 6 seeds.** Each seed must
   either CONVERGE or STILL GROWING (no panics, no crashes).
   Anomalous seeds are allowed and reported.
3. **Captured log committed.** `logs/<date>_phase_d_battery.log`
   becomes the reference output. Re-run to update.
4. **No new unit tests introduced.** Diagnostic example, not
   verification.

## Open questions

1. **HORIZON tuning.** 300 ticks is a guess. C0+'s age gate is
   100 ticks; doubling that should give cycles room to fire.
   But `equivalence_classes` may need longer to mature
   theories. Adjust per seed if needed.
2. **Disable promotion temporarily for some seeds.** The
   `diamond_poset` seed wants to first run theory discovery
   without M1, then enable promotion. Cleanest mechanism: a
   per-seed override of `PromotionConfig` thresholds. Plumbing
   this through the example is the only friction.
3. **What about `SyntheticStreamEnvironment`?** The current
   ADR 0054 OQ #4 verdict was on NoOp; the battery should
   include *one* synthetic-stream seed too, for the
   long-lived-events case. Add as `drip_feed_diamond` if scope
   allows; defer otherwise.
4. **Cross-seed canonical fingerprint diff.** The "final
   canonical hash counts" line invites cross-seed diffing.
   Should the battery produce a structured artefact (e.g., one
   line per (seed, canonical_hash) pair) for downstream tooling
   to consume? Premature. F0 keeps it as plain text; revisit
   if the battery actually grows consumers.

## Touched ADRs

- **ADR 0054** Phase D this verifies; OQ #4's "bounded NoOp"
  caveat narrows once F0 includes a synthetic-stream seed.
- **ADR 0027** the 8-case rigorous battery is the precedent
  this ADR's verification artefact follows.
- **ADR 0055** canonical hash labels — the battery's "shape
  fingerprint" report relies on the post-E0 stability of these
  hashes.

## Summary

Phase D has the mechanism (D0/D0+/E0/cooldown). What's missing
is *systematic evidence that the mechanism does the right thing
across seeds*. F0 fills that gap with a small additive example
+ a captured log, mirroring Phase A's verification pattern.

F1 (richer D-path scheduling state) sketched and deferred —
F0's evidence will tell us whether the cooldown counter alone
is sufficient or whether cadence / budget / state-aware biases
are warranted.

Status: **Proposed**.
