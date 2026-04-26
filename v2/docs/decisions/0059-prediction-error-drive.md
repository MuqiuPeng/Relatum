# 0059: Prediction-error drive (Phase G1)

Status: Accepted (Phases G1.3 + G1.4 + G1.5 implemented)
Date: 2026-04-26

## Context

ADR 0057 / Phase G0 confirmed empirically that an anomaly-
coverage drive alone cannot push the runtime past compression
saturation: the mode-thrash gate dominates before the drive can
exercise itself. ADR 0058 / Phase G1.0 landed the prerequisite
mechanism — `RSet::forward_apply_axiom` /
`forward_apply_all` — so the runtime can now compute "what
does my current axiom set predict should hold?"

This ADR scopes how that mechanism becomes a **drive**: a
real, sustained outward signal that gives the runtime reason
to keep working past the compression equilibrium without
falling into thrash.

## Decision

The drive has three sub-slices, ordered by increasing
ambition. G1.3 is the mechanism for tracking error; G1.4 wires
it into the existing scheduler hooks; G1.5 closes the
"positive-delta-without-mutation" loop that the architectural
analysis identified as the root cause of G0's null result.

### Phase G1.3 — Prediction state + error tracking

A new runtime structure:

```text
struct PredictionState {
    last_predicted_at_tick: u64,
    last_predicted: HashSet<R>,
    /// Per-axiom total prediction count over runtime lifetime.
    total_predictions_per_axiom: HashMap<axiom_id, u64>,
    /// Per-axiom verified-prediction count (edge in `last_predicted`
    /// that was also in rset at the next snapshot).
    verified_predictions_per_axiom: HashMap<axiom_id, u64>,
}
```

Field on `AutonomousRuntime`. Round-tripped through B2-style
checkpoint format (one new section
`[prediction_state]`).

Lifecycle:

1. **Snapshot** runs as a side-effect of the existing tick loop,
   *after* the action executes and *before* the next tick's
   environment poll. The snapshot calls
   `rset.forward_apply_all()` and replaces `last_predicted`.
2. **Verify** runs at the *start* of the next tick, *after*
   environment events have been applied. For each axiom `a`:
   - `predicted_by_a = forward_apply_axiom(a)` at snapshot time
     (cached or re-computed; see open question 1)
   - `verified = predicted_by_a ∩ rset.data_edges()` at verify
     time
   - `total_predictions_per_axiom[a] += predicted_by_a.len()`
   - `verified_predictions_per_axiom[a] += verified.len()`
3. **Hit rate** for axiom `a` is
   `verified_per_axiom[a] / total_per_axiom[a]` once
   `total_per_axiom[a] >= min_predictions_for_assessment`
   (default 5).

This is purely accounting — no scheduler decisions yet. G1.3
just builds the signal.

### Phase G1.4 — Wire into anomaly drive

ADR 0057's `uncovered_data_edges` becomes one of two
"unexplained" signals:

```text
unexplained_data_edges() = data_edges
    - layer_b_covered (the existing Phase G0 metric)
    - forward_apply_all() (new)
```

Edges that no pattern's Layer B covers AND that no axiom's
forward-apply predicts. This is the strongest version of "what
does the runtime not yet explain?" — combining structural
coverage (G0) with axiomatic coverage (G1).

The G0 scheduler hooks (cooldown relaxation, sleep
suppression) consume `unexplained_data_edges()` instead of
`uncovered_data_edges()`. Same thresholds, same gates — only
the signal definition tightens.

This is still drive-level. It doesn't yet break the
"positive-delta-requires-mutation" coupling.

### Phase G1.5 — Positive delta from prediction improvement

The crux of G0's null result was: every Sleep-suppression
oscillation feeds the mode-thrash gate, and the runtime can
only earn positive-delta episodes from rset *mutations*.
G1.5 introduces a non-mutating positive-delta source:

When the scheduler enters Reflect mode, instead of always
returning `Sleep` or `SwitchMode(Expand)`, it can return a new
`SchedulerDecision::ReflectAndScore` (or the existing Execute
shape with a new ActionKind, see alternatives). The runtime:

1. Re-runs `forward_apply_all` against current rset state.
2. Compares the **per-axiom hit rate** with the rate from the
   previous Reflect.
3. Records an Episode with:
   - `action_kind = EvaluatePredictions` (new)
   - `delta = sum_over_axioms(hit_rate_now - hit_rate_prev)`
   - This delta can be positive, negative, or zero.
4. Updates `PredictionState`.

The net effect: a Reflect tick that observes axiom hit rates
*improving* (because new edges arrived that confirm previous
predictions) earns positive delta. That delta:
- Counts toward `min_recent_gains` for the
  Expand→Consolidate transition gate.
- Resets `steps_since_last_gain` counter.
- Does NOT increment mode-transition counts (it's not a
  SwitchMode decision).

Result: the runtime can sustain "I'm watching my predictions
hold up" as a productive activity even when no new pattern is
named. Decouples sustained activity from mode-transition
counters, which was the architectural barrier identified in
ADR 0057's Finding section.

### What this does NOT do

- Does not give probability semantics to predictions. Hit rate
  is binary per edge per snapshot. Probabilistic / weighted
  predictions are a separate ADR.
- Does not handle ADR 0044 equality / disjunctive premises in
  the prediction signal — those still bypass forward-apply.
  G1.1 / G1.2 follow.
- Does not retract low-hit-rate axioms automatically. Hit
  rate becomes an input to retraction decisions but the
  retraction machinery (ADR 0040) doesn't change yet.
- Does not modify ESTABLISHED promotion gate. Tempting (use
  hit rate instead of `times_contributed_positive`) but
  cross-cuts ADR 0053 — defer to a separate decision once
  G1.5 stability is demonstrated.

## Phase G1.6+ (sketch, deferred)

Several rich follow-ons become natural once G1.3–G1.5 land:

- **Hit-rate-weighted ESTABLISHED promotion** (ADR 0053
  follow-on). An axiom only graduates to ESTABLISHED if its
  hit rate is ≥ some threshold. Currently graduation is by
  `times_contributed_positive` — which counts compression
  contribution, not prediction accuracy.
- **Per-axiom cooldown** analogous to ADR 0054 OQ #2's
  meta-meta cooldown. An axiom whose hit rate stays below a
  floor gets demoted from named status to `provisional`.
  Requires a new state in the axiom registry, hence its own
  ADR.
- **Sampling forward-apply** (ADR 0058 G1.X). At β-scale,
  exhaustive σ enumeration over all `data_ids^num_vars` is
  too expensive. Replace with sampled enumeration; trade
  determinism for tractability.
- **Cross-axiom error attribution.** When an edge is
  predicted by *multiple* axioms, who gets credit / blame?
  Current proposal: every axiom that predicted it gets a
  prediction-count increment. Refinements: weighted by
  axiom complexity (Occam-style), or only most-specific.

## Alternatives considered

- **Compute prediction error after every action, not just
  Reflect.** Cleaner data but expensive. Reflect-only keeps
  the cost bounded and matches the existing "Reflect is
  pure observation" semantics.
- **Use prediction error as the primary drive, replacing
  `abstraction_score`.** Bigger conceptual leap; risks
  invalidating the ESTABLISHED promotions already accumulated.
  Phased approach: prediction error supplements compression,
  doesn't replace.
- **Express "prediction" as an in-rset meta-R fact** (e.g.
  `R(axiom_id, PREDICTION_MARKER)` per predicted edge).
  Keeps everything in R but balloons the rset every tick.
  Keep predictions as runtime-only state (`HashSet<R>` in
  memory).
- **Tie prediction error to `counterfactual_value`.** ADR
  0035's counterfactual is "what abstraction_score change
  would removing this object cause?" — mixing in prediction
  error overloads the metric. Keep them separate.
- **`ReflectAndScore` as a new SchedulerDecision variant vs
  new ActionKind.** New ActionKind reuses the existing
  Execute path and the episode log; new SchedulerDecision
  needs more wiring. Go with new ActionKind
  (`EvaluatePredictions`).

## Non-goals

- A full Bayesian update / model-comparison framework. Hit
  rate is a count statistic, not a probability.
- Recursive prediction (predicted edges feeding new
  predictions in the same tick). One-step only — same as
  ADR 0058's forward-apply policy.
- Modifying `discover_axioms_minimal` to prefer high-hit-rate
  axioms. The discovery side stays unchanged for G1.

## Verification plan

For Phase G1.3:

1. **Existing 413 tests pass.**
2. New tests:
   - `g1_3_prediction_state_round_trips`: serialize +
     deserialize a non-empty PredictionState through B2
     checkpoint.
   - `g1_3_predictions_verified_against_actual`: build a
     transitive-closure rset, name axioms, run one tick,
     verify hit rate ≥ some threshold (the closure
     guarantees high accuracy).
   - `g1_3_predictions_unverified_when_data_diverges`: build
     an rset where predictions can't hold, run one tick,
     verify hit rate is low.

For Phase G1.4:

3. F0 battery re-run after G1.4 lands. Compare with
   `2026-04-26_phase_d_battery.log`. Expect:
   - Seeds with no axioms (`fan_only`, `disconnected_islands`):
     same as before — `forward_apply_all` returns empty,
     `unexplained ≡ uncovered`.
   - Seeds with named theories (`diamond_poset`,
     `equivalence_3_classes`): tighter `unexplained` because
     forward-apply now subtracts axiom-predicted edges.
     Should see *fewer* sleep-suppression triggers.

For Phase G1.5:

4. New scheduler test: `g1_5_evaluate_predictions_emits_episode`
   under a controlled setup where predictions improve
   between Reflect cycles.
5. F0 battery re-run after G1.5. Expect:
   - Seeds with named theories should now show **STILL
     GROWING** verdict on long enough HORIZON, because
     prediction-improvement episodes accumulate
     `min_recent_gains`.

The expected progression of F0 battery summaries across the
G1 sub-slices:

```
G0 (current):
  fan_only / diamond_poset / bipartite_2_3 / star_5 /
    equivalence_3_classes / disconnected_islands : all CONVERGED
G1.4 (anomaly-signal tightening):
  same — system-level effect bounded by thrash gate
G1.5 (positive delta from predictions):
  fan_only / disconnected_islands : still CONVERGED (no axioms)
  diamond_poset / equivalence_3_classes : STILL GROWING
  bipartite_2_3 / star_5 : depends on whether axioms get named
```

## Open questions

1. **Predicted-set caching.** `forward_apply_all` is O(N^k);
   recomputing it twice per tick (snapshot + verify) is
   wasteful. Cache strategy: the snapshot at end-of-tick T
   is the verify input at start-of-tick T+1. One computation
   per tick. Trivial. Doesn't apply to G1.5's Reflect
   re-evaluation (intentionally re-runs to compare against
   previous snapshot).
2. **Cross-axiom attribution policy.** As listed in G1.6+ —
   the simplest workable scheme is "all predicting axioms
   get equal credit." Defensible.
3. **Snapshot determinism.** `forward_apply_all` is
   deterministic given rset content. PredictionState
   round-trips through checkpoint, so snapshot/restore is
   safe. But: if checkpoint comes between snapshot and
   verify, the verify uses the deserialised snapshot — which
   might be stale by one tick. Acceptable error margin;
   document.
4. **Hit-rate granularity.** Per-axiom is the proposal. An
   alternative is per-axiom-per-binding (which σ produced
   each prediction), giving finer-grained attribution at
   higher memory cost. Defer.
5. **Stream-based seeds in F0.** The `stream_diamond` seed
   sketched in ADR 0056 becomes load-bearing for G1.5
   verification — without environmental events, predictions
   never get a chance to verify or fail. Building it is a
   prerequisite for G1.5's expected battery diff.

## Touched ADRs

- **ADR 0035** counterfactual_value — left unchanged; G1.5's
  delta is independent.
- **ADR 0040** Prune lane — receives prediction error as a
  *new* low-value signal in a future ADR; G1 itself doesn't
  modify the prune machinery.
- **ADR 0053** ESTABLISHED promotion — remains
  `times_contributed_positive`-based; G1.6+ may extend.
- **ADR 0056** D-battery — `stream_diamond` seed becomes
  prerequisite for G1.5 verification.
- **ADR 0057** anomaly-coverage drive — G1.4 tightens its
  signal; G1.5 fixes its empirical null result.
- **ADR 0058** forward-application — G1.3 consumes its
  output.

## Summary

ADR 0059 specifies the runtime's first **prediction-error
drive**, the mechanism the architectural analysis identified
as necessary to break the compression-saturation barrier.

Three slices, smallest first:
- **G1.3**: track per-axiom prediction hit rate.
- **G1.4**: tighten ADR 0057's anomaly signal by also
  subtracting forward-applied predictions.
- **G1.5**: emit positive-delta episodes from
  prediction-improvement during Reflect, decoupling sustained
  activity from mode-transition counters and breaking the
  empirical null result G0 produced.

G1.5 is the load-bearing change. G1.3 is mechanism; G1.4 is
incremental drive tightening; G1.5 is the new degree of
freedom that lets the runtime stay productively active past
compression equilibrium.

Stream-driven seeds become prerequisite-level for G1.5
verification — the `stream_diamond` already sketched in
ADR 0056 is the natural place to land that.

Status: **Proposed**. No code yet.
