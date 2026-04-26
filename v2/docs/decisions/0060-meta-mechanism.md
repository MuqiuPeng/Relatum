# 0060: Meta-mechanism — runtime self-tuning via prediction error (Phase H)

Status: Accepted (Phase H0 implemented; H1 / H2 sketched)
Date: 2026-04-26

## Context

Phase G1 closed the architectural gap that ADR 0057's empirical
null result identified: with a prediction-error drive
(forward-apply + EvaluatePredictions + fresh-rate gating), the
runtime stays productively active past compression equilibrium.
The F0 battery's `stream_diamond` seed now goes
`CONVERGED → STILL GROWING` — the first-ever STILL GROWING
verdict in v2.

That achievement creates a new opportunity: the runtime now has
a **standard for evaluating its own decisions**. Pre-G1, every
"did action X help?" reduced to abstraction_score delta — a
single compression metric that saturates. Post-G1, "did config
choice C produce more positive-delta EP episodes than config
C′?" is answerable. The runtime can compare its own behaviour
under different configs.

That capability is the prerequisite for **Phase H — meta-
mechanism**. The system can finally reason about *its own
operation*, not just its discoveries. v2's stated long-term
goal — under intrinsic drive, construct from R instances new
relations that explain new phenomena — implies eventually
constructing **new operational mechanisms**, not just new
patterns. Phase H is the first step toward that.

This ADR scopes Phase H. It is a research direction; specific
slices are speculative beyond H0.

## Decision

### Three slices, ordered by ambition

**H0 — Parameterized scheduler with prediction-error feedback.**
The smallest viable slice. The runtime keeps two
`RuleBasedScheduler` configurations (call them A and B) and
alternates between them across "evaluation windows." Each
window is bounded by a fixed number of episodes (default 50).
At end of window, compute the window's mean EP delta. The
config with higher mean wins; loser is mutated (one knob
randomly perturbed by a small amount) for the next window.
Repeat.

This is **A/B-testing the scheduler**, not designing a new
scheduler. The runtime doesn't invent ActionKinds or new modes.
It only varies thresholds: `min_pattern_hit_rate`,
`anomaly_pressure_threshold`, `max_zero_streak`, etc.

H0 produces a self-tuning runtime. It does NOT produce a
self-extending mechanism — the scheduler's *structure* is
fixed; only its parameters evolve.

**H1 — ActionKind composition discovery.**
The runtime observes that certain action sequences correlate
with EP delta improvements. E.g., "DiscoverTheory directly
followed by DiscoverPatterns size=3" produces higher EP delta
than the patterns alone. The runtime promotes such sequences
to first-class **composite actions** — new ActionKind variants
minted at runtime, not at compile time.

This requires:
- A **sequence-mining** mechanism over the episode log.
- A **dispatch routing** layer that can fire composite actions.
- An **identity** for new ActionKinds (string ids? hash-based?
  meta-R objects?).

H1 is genuinely a self-extending mechanism. The
runtime's action space grows.

**H2 — Self-modifying drive.**
Most speculative. The runtime evaluates whether its drive
itself (compression+prediction-error mix) is the right
optimization target. Could conceivably introduce
curiosity / novelty / long-horizon objectives by mining EP
trajectories.

H2 is genuinely a moving target — design depends on what H0/H1
empirics show. Don't commit to specifics yet.

### Why H0 is the right starting slice

Three reasons:

1. **Lowest implementation risk.** No new ActionKinds, no new
   identity story, no schema changes to meta-R. Just a wrapper
   around `RuleBasedScheduler` plus a window-counter and a
   simple A/B record.

2. **Biggest empirical-research yield.** H0 surfaces the
   *space of useful scheduler configurations* — how much do
   threshold choices matter? Are there cliff edges where a
   small change breaks behaviour? Does the runtime drift to
   sensible values, or thrash between extremes? These
   findings inform whether H1 is worth pursuing.

3. **Constitutionally clean.** H0 doesn't add new R relations,
   doesn't break the single-primitive commitment, doesn't
   require any new Memory schema. The "evaluation window" is
   purely a scheduler-internal concept.

### Phase H0 design

**`MetaSchedulerConfig`** structure replaces direct
`RuleBasedScheduler` use:

```text
struct MetaSchedulerConfig {
    candidate_a: RuleBasedSchedulerConfig,  // current "winner"
    candidate_b: RuleBasedSchedulerConfig,  // current "challenger"
    active: ABSlot,                          // which one is in use
    window_size: u64,                        // episodes per window
    window_episode_start: u64,               // counter at window start
    window_ep_delta_sum_a: f64,
    window_ep_delta_sum_b: f64,
    window_ep_count_a: u64,
    window_ep_count_b: u64,
    rng_seed: u64,                           // for mutation
}
```

`RuleBasedSchedulerConfig` is a new struct factoring the tunable
fields out of `RuleBasedScheduler` itself. The scheduler reads
its thresholds from the active config.

At each tick the scheduler returns its decision normally. The
window controller runs as a side-effect of episode recording: at
end of every window, compute `mean_a` and `mean_b`, swap roles,
mutate the loser.

**Mutation strategy**: pick a random tunable knob, scale by
×0.8 or ×1.25 (uniform draw). Clamp to predeclared bounds.
First-pass deterministic-seed mutation; randomized mutation is
follow-on.

**Persistence**: `MetaSchedulerConfig` round-trips through the
B2 checkpoint as a new `[meta_scheduler]` section.

**Verification**:
- A new F0 battery seed (`tunable_diamond` or similar) where
  H0's mutations should be observable in scheduler-config
  drift over time.
- Existing seeds should converge to roughly the same set of
  candidate values (within mutation noise).

### What H0 does NOT do

- Does not create new ActionKinds.
- Does not modify the scheduler's logical structure (only its
  parameters).
- Does not affect `Memory` or `RSet` schema beyond adding the
  `[meta_scheduler]` checkpoint section.
- Does not interact with C0/C1/C2 promotion gates — those are
  separate ESTABLISHED knobs, not scheduler thresholds.

## Alternatives considered

- **Skip H0; jump to H1 (composite action discovery).** Riskier,
  bigger surface, fewer load-bearing test points. Spec H0
  first; H1 only if H0's empirics suggest the action space is
  the bottleneck.
- **Use ESTABLISHED-promotion as the meta-mechanism.** ESTABLISHED
  promotes *named objects*, not *scheduler choices*. Different
  semantic; conflating them risks turning ESTABLISHED into a
  heterogeneous bag.
- **A genetic / evolutionary algorithm over scheduler space.**
  Bigger commitment than H0's two-candidate A/B. Faster
  convergence in principle but more code and more
  hyperparameters. The two-candidate A/B is the simplest
  feedback loop that actually qualifies as "self-tuning";
  scale up if needed.
- **Bayesian optimization over scheduler space.** Same
  argument — overkill for H0. Revisit if A/B proves slow.

## Non-goals

- A general AutoML / hyperparameter framework. Scope is the
  RuleBasedScheduler's thresholds, not arbitrary v2 knobs.
- Modifying `discover_axioms_minimal` or other library
  primitives. H0 only tunes the scheduler.
- Cross-session learning. Within a single `run_bounded` window,
  the runtime tunes its scheduler; no inter-process sharing.

## Verification plan

For Phase H0 (when implemented):

1. Existing 427 tests pass after introducing
   `RuleBasedSchedulerConfig` (purely a refactor — fields
   move from `RuleBasedScheduler` to `RuleBasedSchedulerConfig`,
   scheduler reads through it).
2. New unit test:
   `h0_winner_persists_across_window_with_higher_ep_delta`.
   Run two windows; the better-EP-delta config persists.
3. New unit test:
   `h0_mutation_changes_some_knob_within_bounds`. Construct an
   initial config; force a mutation; verify exactly one knob
   changed and stays within declared bounds.
4. F0 battery rerun with `MetaSchedulerConfig`-wrapped
   scheduler. Expect:
   - Static seeds (`fan_only`, `diamond_poset`, etc.) — no
     observable change in verdict; total episodes count may
     differ slightly due to scheduler-config drift.
   - `stream_diamond` — possibly more total episodes, possibly
     fewer; verdict should remain STILL GROWING.
5. New seed: `h0_drift_test`. Long-running stream where the
   right scheduler config genuinely matters (e.g., high pattern
   discovery rate vs low). Verify config drifts in the
   expected direction.

## Open questions

1. **Window size.** 50 episodes is a guess. Too short → noise
   dominates; too long → drift is glacial. Tune empirically.
2. **Mutation step size.** ×0.8 / ×1.25 are guesses. Larger
   steps → faster exploration, higher destabilization risk.
3. **Number of candidates.** Two (A/B) is minimum. Three+
   candidates allow tournament-style selection but add
   complexity. Defer.
4. **Multi-objective.** EP delta is one number; should H0
   also consider abstraction_score delta, episode-count, time
   to first promotion, etc.? For G1.5's logic, EP-delta-only
   is closest to the outward drive. Multi-objective is a
   future enhancement.
5. **Reset on regression.** If the active config produces
   *negative* EP delta over its window, should H0 reset to
   defaults rather than just mutating? Defensive but adds a
   special case. Defer.
6. **Interaction with checkpoint format versioning.** Adding a
   `[meta_scheduler]` section is similar to ADR 0053 / B2's
   prediction-state addition; same migration story.

## Touched ADRs

- **ADR 0052** RuleBasedScheduler — H0 wraps it but doesn't
  replace it.
- **ADR 0057** anomaly-coverage drive — H0 may tune
  `anomaly_pressure_threshold` over time.
- **ADR 0059** prediction-error drive — H0's evaluation metric
  IS EP delta. H0 cannot exist without G1.5.

## Summary

Phase G1 gave the runtime an outward drive. Phase H gives it
the ability to reason about *which scheduler configurations
produce more outward drive*. H0 is the smallest possible
implementation: A/B between two candidate configurations,
windowed by episode count, mutate the loser. No new
ActionKinds, no new R relations, no modification to library
primitives. Pure parameter-space self-tuning.

H0 is the first move toward genuine v2-style self-extension.
The runtime cannot yet *invent* mechanisms (that's H1). But it
can finally **choose between candidate mechanisms based on its
own outward-drive feedback**, which is a qualitative step
beyond every prior phase. v2's compression-only drive could
not have grounded this choice; ADR 0059's prediction-error
drive can.

Status: **Proposed**. No code yet. H1 / H2 sketched and
deferred until H0's empirics are in.
