# 0057: Anomaly-coverage drive (Phase G0)

Status: Accepted (Phase G0 implemented; system-level effect bounded
by thrash gate — see Finding below)
Date: 2026-04-26

## Context

ADR 0056 / Phase F0 captured the D-battery: 6 diverse seeds, all
CONVERGE within 50 ticks. Combined with the termination-property
empirics (ADR 0054 OQ #4), this confirms what the architectural
analysis predicted — **the runtime's intrinsic drive saturates
fast and the system terminates quickly regardless of input
topology**.

The diagnosis (recorded in the project conversation): v2's drive
is one-dimensional ("compression of existing R via
abstraction_score / counterfactual_value / MDL gain"). With only
this drive, once a fixed point in the compression frontier is
reached, the runtime has no reason to continue. Adding more
input doesn't help — input quantity changes how long it takes
to saturate, not whether it saturates.

The proposed remedy is **outward-facing drive**. Three flavours
were considered:

- (a) anomaly priority — give weight to "what doesn't yet have
  an explanation"
- (b) prediction-error — predict future R, observe, attribute
  error
- (c) curiosity — reward novelty per se

(a) is the cheapest path: it doesn't require axiom
forward-application machinery (which is what (b) needs and ADR
0058 will scope). (a) reuses the existing pattern-discovery
machinery and only changes *what the scheduler prioritises*.
Phase G0 is (a).

## Decision

### Anomaly = uncovered data edges

Define `RSet::uncovered_data_edges() -> HashSet<R>` as the set
of data edges (both endpoints non-meta) that **don't appear
in any named pattern's Layer B instance binding**.

Concretely: for each named pattern `p`, each of its instances
`R(p, inst_N)`, and each of `inst_N`'s participants
`R(inst_N, participant)`, the data edges connecting two
participants of the same instance are "covered". Everything
else is "uncovered."

This metric:

- Returns the empty set when every data edge is part of some
  pattern's instance — the system has fully explained itself
  in pattern terms.
- Is non-empty when fresh data arrives faster than the
  pattern-discovery machinery names it. Stream-driven runs
  should keep this signal high.
- Costs O(patterns × instance_size² × edge_lookups). At v2
  scale this is acceptable; at the β-edge-count scale we
  may need a sampled approximation but defer that.
- Is silent on Intensional-only patterns (no Layer B → no
  participants → those patterns don't cover anything by this
  definition). Acceptable; meta-meta-patterns named
  Intensionally don't represent specific data instances and
  shouldn't claim coverage.

### Phase G0 — wire the signal into the scheduler

Two minimal hooks. Both are inside `RuleBasedScheduler`:

**Hook 1 — pattern-cooldown relaxation under pressure.**

The B1+ pattern-cooldown gate (`min_pattern_hit_rate = 0.10`)
becomes adaptive: when `uncovered_data_edges().len() >=
anomaly_pressure_threshold` (default 3), the effective hit-rate
floor drops to `min_pattern_hit_rate * anomaly_relaxation`
(default 0.5 → effective 5%). Same `attempts` floor, same
counter, just a more-permissive rate threshold while there's
unexplained data sitting around.

**Hook 2 — sleep-suppression while pressure is high.**

When the scheduler would otherwise return `Sleep` (no expand
work, no consolidate work, no reflect work), check
`uncovered_data_edges().len() > 0`. If non-zero, return
`SwitchMode(Expand)` instead of `Sleep`, even if a tick was
just spent in Expand. Bounded by `max_mode_oscillations` so it
can't loop forever.

These two hooks together mean: as long as the rset has
uncovered data, the runtime keeps trying to discover patterns
that would cover it; if pattern discovery genuinely can't make
progress (cooldown attempts ≥ floor, hit rate at the relaxed
threshold), oscillation gate kicks in and the runtime
eventually sleeps anyway.

### Why this is "outward" enough to count

`uncovered_data_edges` measures how much of the rset is
*unexplained by named patterns*. That's not a compression
metric — it's a **coverage** metric. A pattern naming pass that
covers more edges raises the coverage; one that doesn't, doesn't.
Compression-only drive doesn't see this distinction (it only
sees the abstraction_score delta).

The anomaly signal naturally points the runtime at "things it
hasn't yet built up to explain". With a synthetic-stream
environment that keeps adding fresh edges, the signal stays
non-zero and the runtime stays alive — exactly the F0 verdict
we want to flip from CONVERGED to STILL GROWING (in a useful
sense, not a pathological one).

### What this does NOT do

- Does not give the runtime any way to *predict* future R.
  That's Phase G1 (ADR 0058 forward-application + ADR 0059
  prediction-error drive, both deferred).
- Does not reward novelty. Two patterns that cover the same
  edges are equally valued; this drive can't distinguish "I've
  seen this shape before" from "this shape is fresh."
- Does not change `abstraction_score`, `counterfactual_value`,
  or MDL gain. The compression drive remains; G0 adds an
  outward flavour alongside.
- Does not change `ObjectHistory` or `PolicyStats` schema. All
  computation is on-demand from the current rset.

## Phase G0+ (sketch, deferred)

If F0 re-runs after G0 still show CONVERGED on
non-stream seeds, that's expected — without an environment
producing fresh edges, coverage trivially saturates. Stream-
driven seeds are the real test. Add a `stream_diamond` seed to
the F0 battery that drips a diamond poset over 200 ticks to
exercise the anomaly drive against ongoing input.

## Alternatives considered

- **Boost DiscoverPatterns priority directly.** Equivalent in
  effect to the cooldown-relaxation hook, but harder to
  predict because frontier item priorities interact with the
  Theory / Pattern picking logic in subtle ways. Cooldown
  relaxation is more localized.
- **Define anomaly as "data edges not covered by any axiom's
  forward application."** This is the right answer for Phase G1
  but requires the forward-application machinery that doesn't
  yet exist. G0 stays Layer-B-only.
- **Make `uncovered_data_edges` a delta signal (recently
  uncovered) rather than absolute.** More predictive, less
  stable. Defer until streaming use surfaces a need.
- **Block Sleep entirely while pressure is high.** Removes the
  termination guarantee. Keep the oscillation cap.

## Non-goals

- A new ActionKind. G0 reuses `DiscoverPatterns`.
- A new FrontierKind. G0 changes scheduler dispatch but not
  the frontier item taxonomy.
- A new Memory or PolicyStats field. The signal is computed
  from rset state on demand.
- Changes to the existing 397 tests. G0 is additive.

## Verification plan

For Phase G0:

1. New unit test:
   `rset_uncovered_data_edges_excludes_layer_b_covered`. Build
   an rset with two named patterns, one with Layer B
   bindings, one Intensional. Assert the Layer-B-covered edges
   are excluded; the Intensional pattern's structural edges
   stay uncovered.
2. New scheduler test:
   `g0_relaxed_cooldown_picks_pattern_under_anomaly_pressure`.
   Memory has `DiscoverPatterns` attempts ≥ 5 with hit rate
   between the relaxed (5%) and base (10%) thresholds. Without
   pressure: pattern_cooldown_active = true. With pressure
   (uncovered ≥ 3): false.
3. New scheduler test: `g0_sleep_suppressed_under_pressure`.
   Frontier empty, all modes idle, but uncovered > 0 → returns
   `SwitchMode(Expand)` not `Sleep`. With uncovered = 0,
   returns `Sleep`.
4. F0 battery re-run after G0: capture the new log. Compare
   with `2026-04-26_phase_d_battery.log`. Expect:
   - Most seeds still CONVERGED (no fresh data, coverage
     trivially stable).
   - At least one seed (probably bipartite_2_3 or star_5)
     shows higher pattern count or extended runtime — the
     hooks fire on the existing uncovered data.
   - The `equivalence_3_classes` seed may stop sleeping early —
     its discovered theory doesn't produce coverage in the
     Layer B sense, so anomaly pressure stays high.

## Open questions

1. **Layer B coverage definition for sub-pattern overlap.**
   If pattern A's instance includes edges {(u,v), (v,w)} and
   pattern B's instance includes {(v,w), (w,x)}, the edge
   (v,w) is covered by both. Is double-coverage a special
   case? Suggest no — coverage is set membership, not count.
2. **Thresholds.** `anomaly_pressure_threshold = 3` and
   `anomaly_relaxation = 0.5` are guesses. F0 re-run informs.
3. **Performance at scale.** O(patterns × instance² × lookups)
   is fine for hundreds of edges. At thousands, a sampled
   approximation may be needed. Defer.
4. **Interaction with B3 stale-prune.** Stale-prune retracts
   patterns; that *increases* uncovered count, raising
   anomaly pressure, encouraging more pattern discovery. This
   could be virtuous (replacing stale patterns with fresh
   ones) or vicious (perpetual prune-rediscover loop). The
   F0 re-run should reveal which. The mode-thrash gate
   should bound the loop in either case.

## Touched ADRs

- **ADR 0029** Layer B / Intensional naming — defines what
  "covered" means for this drive.
- **ADR 0052 / B1+** pattern-cooldown gate — G0 makes its hit-
  rate threshold adaptive.
- **ADR 0056** Phase F0 — battery is the verification baseline.
- **ADR 0058** axiom forward-application semantics — Phase G1
  prerequisite, designed in parallel with this ADR.

## Finding (post-implementation)

After G0 landed, an F0 battery re-run produced a log
**byte-identical** to the pre-G0 run. All 6 seeds still
CONVERGE within 50 ticks; no seed shows extended runtime.

Diagnosis: the existing **mode-thrash gate**
(`max_mode_oscillations = 4`) bounds the new sleep-suppression
hook before any new pattern discoveries can happen. The hook
fires a few times — `SwitchMode(Expand)` instead of `Sleep` —
but the Reflect↔Expand pair quickly hits 4 oscillations and
the thrash gate forces Sleep regardless of pressure.

This is a **real architectural finding**, not a bug:
- G0's local mechanisms work (6 unit tests verify them).
- The system-level ceiling is set by the thrash gate, not by
  the cooldown hit-rate floor.
- Anomaly pressure alone, without a richer success signal, is
  not enough to overcome thrash protection.

What this means for Phase G1 (prediction-error drive,
ADR 0058 / 0059):
- G1 will provide a *finer* success signal — successful
  prediction reduces error even when no new pattern is named,
  so individual ticks can have positive delta without naming.
- Positive-delta episodes don't feed thrash counters the way
  mode switches do, so G1 can sustain runtime activity that
  G0 can't.
- Conclusion: G0 is necessary but not sufficient. The
  saturation problem really does need G1.

The G0 mechanisms remain useful: they correctly bias the
scheduler when uncovered data is present, and the unit tests
guarantee no regression. Stream-driven seeds (deferred) may
exercise G0 differently than the static F0 battery — uncovered
count would be replenished by environment events, potentially
keeping pressure high even after a few thrashes.

## Summary

Phase G0 introduces the runtime's first **outward-facing**
drive: count of data edges not yet explained by any named
pattern's Layer B. The signal feeds two narrow scheduler hooks
(cooldown relaxation, sleep suppression) without touching
abstraction_score, the action taxonomy, or memory schema.

System-level impact on the F0 static-seed battery is null
because the mode-thrash gate dominates. This is the empirical
case for Phase G1 (prediction-error drive). Stream-driven
verification will need either ADR 0056's `stream_diamond` seed
or the next ADR's full forward-application machinery.

Status: **Accepted (Phase G0 implemented; awaiting G1 to fully
exercise the outward-drive thesis).**
