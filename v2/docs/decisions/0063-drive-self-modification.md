# 0063: Drive self-modification (Phase H2)

Status: Accepted (Phase H2.0 step 1 + step 2 + step 3a implemented; wake-gate integration deferred to step 3b)
Date: 2026-04-27

## Context

ADR 0060 sketched Phase H as three slices ordered by
ambition: H0 (parameter-tuning), H1 (action-space self-
extension), H2 (drive self-modification). H0 and H1 are
landed:

- H0 — `MetaScheduler` A/B-tests two `RuleBasedSchedulerConfig`s
  using mean EP delta as the evaluation standard.
- H1 — `SequenceStats` mines pair / triple correlations,
  `ACTION_SEQ_MARKER` chains promote/demote them, and
  `ActionKind::ExecuteComposite` dispatches them as
  bundled units.

H2 was deliberately left as "research direction" pending
empirical evidence from H0/H1. With H1 feature-complete and
the 2026-04-27 long-run empirics in hand, the design space
for H2 is concrete enough to commit to a careful spec ahead
of implementation.

The 2026-04-27 retrospective phrased the H2 question:

> The runtime currently can't, e.g., notice "my prediction-
> error metric isn't capturing what matters here" and adapt.
> Is there a meaningful design space short of hand-crafted
> alternative drives?

This ADR scopes the answer.

## What "drive" means today

Two coupled signals jointly govern wake / sleep / mode
behaviour:

1. **Compression drive** — `abstraction_score` delta from
   action selection. Saturates as the rset reaches
   compression equilibrium (the original G0 problem).
2. **Outward drive** — fresh `forward_apply_axiom` results
   per tick that change the prediction footprint, gated
   through `EvaluatePredictions` and credited to per-axiom
   hit rates (ADR 0059, G1.5).

Together they produce the current drive curve: compression
keeps the runtime productive while novel structure exists;
outward drive keeps it productive past compression
equilibrium (the load-bearing fix that turned
`stream_diamond` `CONVERGED → STILL GROWING`).

**Both are hard-wired.** The runtime has no mechanism for
asking "is compression+prediction-error the right
optimization target *for this substrate*?" — much less for
synthesizing a new target if the current one fails.

## What H2 is asking for

Three increasingly ambitious framings:

1. **Tune which drive matters more** in the current mix.
   Closest to H0; just shift the weight between compression
   and outward signals based on outcome. Trivial in
   principle.
2. **Add or remove drives from a fixed set of candidates**
   (compression, prediction-error, anomaly-coverage, novelty,
   recency). The set is hand-curated; what changes is which
   ones the runtime actively listens to.
3. **Mint a new drive** from existing metrics. E.g.,
   "EP-delta variance" (reward stability over volatility)
   could emerge as a derived metric. The runtime authors a
   new evaluation standard the runtime author never wrote.

(1) is incremental engineering. (2) is non-trivial but
clean. (3) is the deep move that mirrors H1's "self-
extending action space" along the *evaluation* axis. It is
also where the constitutional risk concentrates.

## Decision

### Three sub-slices, ordered by ambition

**H2.0 — Multi-drive blend with feedback-tuned weights.**
The smallest viable slice. The runtime exposes a
`DriveMix` struct holding non-negative weights over an
explicit set of candidate drives (initially: compression,
prediction-error, mode-thrash penalty). The wake / sleep
gate evaluates `Σ_i w_i · drive_i(rset, memory)` instead
of the current hand-coded combinator. An evaluation window
similar to H0's MetaScheduler tracks mean EP delta per
weight configuration; mutate weights at window boundaries.

This is **A/B-testing the drive mix**, not authoring new
drives. Drive identities are still compile-time. What
changes is weight allocation across them.

H2.0 produces a self-tuning *evaluation*. It does NOT
extend the drive space; only its parameterization.

**H2.1 — Drives as meta-R objects (`DRIVE_MARKER` chain).**
Each drive registers as an R fact:

```text
R(DRIVE_MARKER, drive_compression)
R(DRIVE_MARKER, drive_prediction_error)
R(DRIVE_MARKER, drive_mode_thrash)
R(drive_compression, drive_<id>_weight)  // currently active weight
R(drive_compression, drive_<id>_score)   // window-mean delta
```

This puts drives on the same constitutional footing as
patterns / theories / sequences: they are first-class
meta-R objects, observable to the runtime, and subject to
the existing ESTABLISHED-promotion / retraction lifecycle.

A drive can be ESTABLISHED if its window-mean contribution
to EP delta exceeds a threshold; demoted if it
consistently scores zero. The drive set becomes self-
managing the way the pattern set already is.

H2.1 is genuinely a self-extending mechanism along the
*evaluation* axis. The drive set grows / shrinks at runtime.
But — important — H2.1 still draws from a hand-curated
catalogue of drive function bodies (`drive_compression`,
`drive_prediction_error`, etc.). The mechanism authors
*activation*, not *function bodies*.

**H2.2 — Drive synthesis from existing metrics.**
Most speculative. The runtime composes new candidate
drive functions from existing primitive metrics (EP delta,
pattern count delta, axiom hit rate, etc.) via simple
combinators (mean, variance, ratio, lag-difference). A
synthesized candidate enters the H2.1 lifecycle: registered
under DRIVE_MARKER, evaluated for its window-mean
contribution, ESTABLISHED if it carries signal, demoted
otherwise.

Example synthesized candidate: "stability-of-improvement"
= -variance(recent EP deltas), rewarding consistent
positive movement over volatile spikes.

H2.2 is where "the runtime authors a new evaluation
standard the runtime author never wrote" becomes literal.
It is also where the constitutional-drift risk is highest.

### H2.0 design (concrete enough to start when chosen)

#### Drive trait

```text
trait Drive {
    fn id(&self) -> &'static str;
    fn evaluate(
        &self,
        rset: &RSet,
        memory: &Memory,
        tick: u64,
    ) -> f64;  // signal strength in [0, ∞)
}
```

`evaluate` returns a per-tick scalar; semantics are the
drive's own. The runtime's wake / mode-decision combinator
reads:

```text
let signal = drives
    .iter()
    .map(|d| mix.weight(d.id()) * d.evaluate(rset, memory, tick))
    .sum::<f64>();
```

Existing compression and outward-drive logic moves into
two impls (`CompressionDrive`, `PredictionErrorDrive`).
A third drive, `ModeThrashPenalty`, captures the existing
penalty term. Three baseline drives.

#### DriveMix and feedback

```text
struct DriveMix {
    weights: HashMap<&'static str, f64>,  // drive id → weight
    candidate_a: HashMap<&'static str, f64>,
    candidate_b: HashMap<&'static str, f64>,
    active: ABSlot,
    window_episode_start: u64,
    window_ep_delta_sum_a: f64,
    window_ep_delta_sum_b: f64,
    window_ep_count_a: u64,
    window_ep_count_b: u64,
    window_size: u64,        // default 50
    rng_seed: u64,
}
```

Mirrors H0's `MetaScheduler` design, scaled to weight
mutation. At window boundary, the higher-mean candidate
wins; loser is mutated by perturbing one randomly chosen
weight by ×0.8 or ×1.25 (clamped to [0, 1]).

#### Persistence

`DriveMix` round-trips through the B2 checkpoint as a
new `[drive_mix]` section. Cumulative window stats persist;
in-flight per-window counters reset on restore.

#### Verification

- Existing 470 tests continue to pass after the refactor
  that moves compression / outward into `Drive` impls.
- New unit test: `h2_0_weight_mutation_responds_to_ep_delta`.
  Plant two windows where weight-config B produces higher
  mean EP delta; assert that the post-window mix takes
  B as the winner and mutates A.
- New unit test:
  `h2_0_weight_clamp_holds_bounds_under_repeated_mutation`.
  Drive 100 mutations through the seed; assert all weights
  remain in [0, 1].
- F0 battery rerun: stream_diamond should converge to a
  weight mix that emphasizes prediction-error (since that's
  the load-bearing drive on streaming substrates).
- A new long-run extension to phase_h1_long_run.rs that
  logs DriveMix state across regime shifts. Hypothesis:
  weight on prediction-error grows in regime A; doesn't
  shift in B/C (no EP activity); slowly returns toward
  baseline if drive_mix stalemates.

### What H2.0 does NOT do

- **Does not author new drive function bodies.** Drives are
  compile-time impls of `Drive`. Only their weights are
  mutable.
- **Does not register drives as meta-R objects.** That's
  H2.1.
- **Does not interact with H1.x.** ActionKind composition
  and drive blending are orthogonal mechanisms; both feed
  into the same MetaScheduler / lifecycle accounting but
  don't overlap.
- **Does not change EP-delta semantics.** EP delta remains
  the canonical evaluation signal. H2.0 only changes how
  drive *signals* combine into the wake/mode gate; the
  *evaluation* of a window's outcome is unchanged from
  H0/H1.

### H2.1 design (sketch only)

When implemented, H2.1 reuses ADR 0033's defeasibility +
ADR 0053's ESTABLISHED-promotion machinery applied to the
new DRIVE_MARKER class. A drive's window-mean contribution
becomes its analogue of pattern-hit-rate. ESTABLISHED
drives have their weight floored above zero; demoted drives
have weight zeroed (effectively removed from the active
mix without losing the catalogue entry).

The DRIVE_MARKER chain is an extension of the meta-R class
hierarchy already housing PATTERN, AXIOM, THEORY,
ESTABLISHED, SHARED_AXIOM, ACTION_SEQ. Constitutionally
clean — same shape as the existing classes.

### H2.2 design (sketch only)

H2.2 introduces a small grammar over primitive metrics:

```text
metric := primitive | mean(metric) | variance(metric) |
          ratio(metric, metric) | lag_diff(metric, k)
```

Primitive metrics are the per-tick scalars currently
exposed (compression delta, EP delta, anomaly count, ...).
A drive synthesizer enumerates compositions up to a small
depth, registers each as a DRIVE_MARKER candidate with
weight 0 (passive), and lets the H2.1 lifecycle observe
its activation by chance — promoting to non-zero weight if
its window-mean correlates positively with overall EP
delta improvement.

This is gradient-descent-like search over drive space
(per the v2 search-mode practice — propose / score /
refine, not exhaustive enumeration).

H2.2 is **out of scope** for this ADR's commit; it's
mentioned only to bound the design vocabulary that H2.0
and H2.1 should not paint themselves into corners against.

## Alternatives considered

- **Skip H2 entirely; declare H1 the natural plateau.**
  Defensible. The 2026-04-27 retrospective lists "self-
  modifying drive" under "what still doesn't exist" — the
  trajectory toward v2's stated goal asks for at least an
  attempt. H2.0 is small enough to attempt without
  committing to H2.1/H2.2.
- **Bayesian optimization over drive weights.** Same
  argument as ADR 0060 against BO over scheduler space:
  overkill for the smallest viable slice. Revisit if A/B
  mutation proves too slow.
- **Genetic algorithm over drive weight + selection.**
  Bigger commitment, more hyperparameters. H2.0's two-
  candidate A/B is the lower-bound feedback loop that
  qualifies as drive self-modification.
- **Use ESTABLISHED-promotion directly** (skip H2.0,
  jump to H2.1). Risky — ESTABLISHED's weight semantics
  need empirical validation before they govern the wake
  gate. H2.0 produces the validation data.

## Constitutional review

Each of the five v2 commitments, scored against H2.0:

1. **R is singular.** H2.0 does NOT introduce a new
   R class — drives are compile-time runtime structures,
   not meta-R objects yet. PASS.
2. **R is binary.** No new relations introduced. PASS.
3. **Types are meta-R instances.** H2.0 does not register
   drives as meta-R; the catalogue is compile-time. This
   is a *constraint*, not a violation: H2.0 stays within
   commitment 3 by deferring drive-as-type to H2.1. PASS.
4. **Identity is token-based.** Drive ids are compile-time
   string constants (`"compression"`, `"prediction_error"`,
   `"mode_thrash"`). PASS.
5. **Similarity is structural.** No similarity claim made;
   drives compare via their numeric scores, not structure.
   PASS.

For H2.1, commitment 3 becomes the load-bearing one: drives
must register as `R(DRIVE_MARKER, drive_X)` to become
first-class types — same shape as PATTERN_MARKER /
THEORY_MARKER. H2.1's design is constitution-compatible by
construction.

For H2.2, commitments 3 and 4 both get tested. Synthesized
drives need an identity (commitment 4 — propose hash of
their composition expression) and registration as meta-R
(commitment 3). The synthesis grammar must produce
deterministic, structural identifiers; neither commitment
is broken if it does.

## Non-goals

- **Cross-runtime drive transfer.** Within a single
  `run_bounded`, drives self-tune. No inter-process
  sharing of drive weights or synthesized drives.
- **Unbounded drive synthesis depth.** H2.2 must cap
  compositional depth (proposed: depth 3) to avoid
  combinatorial blowup. H2.0/H2.1 do not deal with this.
- **Replacing EP delta as the evaluation standard.** EP
  delta remains the meta-evaluation. Drives propose
  *what to attend to per tick*; EP delta judges *which
  drive proposals were good*.
- **Modifying the lifecycle state machine.** Wake / sleep
  / mode transitions stay structurally identical; only the
  scalar that governs them shifts.

## Verification plan

(For H2.0, when implemented.)

1. Existing 470 tests pass after the `Drive` trait refactor.
2. New unit tests:
   - `h2_0_weight_mutation_responds_to_ep_delta`.
   - `h2_0_weight_clamp_holds_bounds_under_repeated_mutation`.
   - `h2_0_drive_mix_round_trip_through_checkpoint`.
   - `h2_0_compression_drive_signal_matches_existing_logic`.
   - `h2_0_prediction_error_drive_signal_matches_existing_logic`.
3. F0 battery rerun: stream_diamond verdict remains STILL
   GROWING; final DriveMix has prediction-error weight
   visibly above its starting baseline.
4. Long-run extension: log DriveMix state at each snapshot
   in `phase_h1_long_run.rs`. Capture
   `logs/<date>_phase_h2_0_long_run.log`.

## Open questions

1. **Initial weights.** All-equal (0.33 / 0.33 / 0.33), or
   reproduce the hand-tuned current mix (compression 0.5 /
   pred-err 0.4 / mode-thrash 0.1)? Hand-tuned is the
   honest baseline — it's where H0/H1 already work — but
   then H2.0's self-tuning has less room to demonstrate
   itself. Likely answer: hand-tuned for empirics; equal
   for verification tests.
2. **Window size sensitivity.** H0 uses 50 episodes per
   window. Drive mutations affect every tick, so a
   smaller window (e.g., 20) might be more responsive but
   noisier. F0 empirics should inform.
3. **Mutation step magnitude.** ×0.8 / ×1.25 is the H0
   precedent. Drive weights have different scale (in
   [0, 1]) than scheduler thresholds; a different
   perturbation magnitude (e.g., ±0.1 additive) may be
   more appropriate.
4. **Negative drives.** Some signals (mode-thrash) act as
   penalties — should they be modeled as negative-weight
   drives or as separate "penalty" mechanisms? H2.0's
   default: keep weights non-negative; encode penalties as
   drives whose `evaluate` returns negative scalars. Cleaner
   than allowing negative weights.
5. **Interaction with H0's MetaScheduler.** Two A/B loops
   running simultaneously (scheduler thresholds and drive
   weights). They evaluate the same EP delta; can they
   step on each other? Likely yes if windows align —
   simplest defense is to phase-shift their windows so
   only one mutates per N-tick block.

## Touched ADRs

- **ADR 0060** (meta-mechanism Phase H) — H2 was sketched;
  this ADR makes H2.0/H2.1/H2.2 concrete and accepts
  H2.0 as the next implementation slice.
- **ADR 0059** (prediction-error drive) — `evaluate` for
  `PredictionErrorDrive` reuses the G1.5 fresh-rate logic
  unchanged.
- **ADR 0053** (selective declarativization) — H2.1 reuses
  the ESTABLISHED-promotion lifecycle for drive promotion.
- **ADR 0061** (action-sequence mining) — orthogonal to
  H2; both feed MetaScheduler signals but don't overlap.

## Summary

H2 is the deep self-modification move. H2.0 is the smallest
slice that earns the H2 label: drive blending under feedback,
without authoring new drive bodies and without introducing a
new meta-R class. H2.1 generalizes it via DRIVE_MARKER and
reuses the ESTABLISHED-promotion lifecycle. H2.2 is research
territory — synthesis of new drives from primitive metrics —
and is out of scope for this ADR's commit.

The constitutional review shows H2.0 is commitment-clean by
construction; H2.1 is commitment-compatible by reuse of the
existing meta-R class machinery; H2.2 needs careful
identifier design before it's safe.

Recommended sequence on adoption: H2.0 → empirics →
(optional) H2.1 → empirics → (research-mode) H2.2. No
implementation work in this ADR — only the design space
mapped.

Status: **Proposed**. No code yet.

---

## Addendum — Step 2 empirics + step 3 prep (2026-04-27 late)

After step 2 landed, the long-run example was extended to
log DriveMix state per snapshot. Captured to
`logs/2026-04-27_phase_h2_0_long_run.log`.

#### Observed step-2 behaviour

Over 2000 ticks (5 windows × 50 EP-episode budget):

- A/B state cycled 5 times (TestingA → TestingB → A → B → A → B).
- Loser mutations recorded:
  - `candidate_a.mode_thrash`: 0.10 → 0.125 (×1.25).
  - `candidate_b.compression`: 0.50 → 0.40 (×0.8).
- All weights stayed in [0, 1].
- Episode count, theory count, named-sequence sets
  all byte-identical to the pre-step-2 post-fix run.
  Shadow-only property verified live.

The mutation feedback loop is responsive on real
substrates. The pre-condition for step 3 (wake-gate
refactor) is empirically established.

#### Step 3 design — what wires DriveMix into the gate

Three minimum changes:

1. **Combined signal**. Add `RuleBasedScheduler::combined_drive_signal(ctx, drives, drive_mix)` that returns
   `Σ_id (active_weights[id] * drive.evaluate(rset, memory, tick))`.
   Drives evaluated against the active candidate's
   weights, not both candidates.
2. **Replace zero-streak gate**. Today the wake/mode/sleep
   gate uses `zero_streak >= max_zero_streak` as the
   anti-stagnation trigger. Replace with
   `combined_drive_signal < threshold`. Threshold
   calibrated against the post-fix long-run baseline so
   the F0 battery's STILL GROWING / CONVERGED verdicts
   remain stable.
3. **Phase-shift DriveMix windows vs MetaScheduler
   windows**. ADR OQ #5: both A/B loops feed on EP delta.
   Phase shift ensures only one mutates per N-tick block.
   Simplest: MetaScheduler windows align to multiples of
   50 episodes (current); DriveMix windows align to
   multiples of 50 with a +25 episode offset. Each loop
   sees a different mean.

#### Step 3 risks

- F0 battery's STILL GROWING verdict on stream_diamond is
  load-bearing for ADR 0059 / G1.5. If step 3's threshold
  is too high, the runtime may quiesce earlier than today;
  too low and it may never sleep at all.
- Two-loop interaction: if MetaScheduler mutates a
  scheduler threshold that affects EP firing rate, that
  changes the EP delta DriveMix observes, and vice versa.
  Phase shift is necessary but may not be sufficient.
- Constitutional commitment 3 stays compatible at step 3
  (drives still compile-time); H2.1 is the slice that
  raises commitment 3 questions.

#### When to do step 3

When user signals readiness. Step 2's empirical
verification is sufficient evidence that DriveMix can
contribute load-bearing signal. Step 3 is a focused, well-
scoped change.

Status: step 3 design captured; implementation deferred.

---

## Addendum 2 — Step 3b refinement after OQ #1 (2026-04-27 late²)

OQ #1 experiment ran (`examples/phase_h2_0_oq1_experiment.rs`,
`logs/2026-04-27_phase_h2_0_oq1_experiment.log`):
hand-tuned (0.5/0.4/0.1) vs equal-weighted (0.333/0.333/0.333)
DriveMix init over the same 2000-tick substrate.

Three findings reshape the step 3b design:

#### Finding 1 — shadow-only property holds across both inits

Both runs produce byte-identical episode counts, EP
attempts, named pairs/triples. Step 2 + step 3a are
genuinely additive; nothing leaks back into runtime
behaviour through the new observability.

#### Finding 2 — combined-signal magnitudes are weight-sensitive

Hand-tuned signal range: 1.0–1.3.
Equal-weighted signal range: 3.3–4.2.
Persistent ~2-3× gap.

The dominating factor is `mode_thrash` (typical evaluate
output ~10) interacting with its weight (0.1 vs 0.333).

**This rules out step 3b's original "absolute threshold"
design.** A threshold of `combined_drive_signal < 0.5`
would force frequent sleep transitions under hand-tuned
and almost never fire under equal-weighted.

#### Refined step 3b design

Replace "absolute threshold" with **normalized signal**:

```rust
fn normalized_drive_signal(rt: &AutonomousRuntime) -> f64 {
    let weight_sum: f64 = rt.drive_mix.active_weights().values().sum();
    if weight_sum < f64::EPSILON {
        return 0.0;
    }
    rt.combined_drive_signal() / weight_sum
}
```

`normalized_drive_signal` is invariant to absolute weight
magnitudes — it's a weighted average of drive evaluations.
Threshold calibration becomes a single-parameter exercise
against the post-fix long-run baseline, not a per-mix
recalibration.

#### Finding 3 — mutation trajectories are currently identical

Both runs mutated the same knobs in the same directions
(hand: a.mode_thrash ×1.25, b.compression ×0.8;
equal: identical pattern proportionally). Expected: at
step 3a, EP behaviour is identical → DriveMix sees
identical window means → mutation chooser draws the same
keys/directions.

This will diverge at step 3b: once normalized signal
gates the wake/mode/sleep machinery, EP firing rate
becomes mix-dependent → DriveMix's evidence becomes mix-
dependent → mutation paths diverge.

**Step 3b is the slice that makes self-tuning observable
in mutation-trajectory space.** Until step 3b, mutation
mechanics work but operate on degenerate evidence.

#### Updated step 3b design (3 changes)

1. **Add `normalized_drive_signal(&self) -> f64`**.
   Weight-invariant; ranges in same magnitude as drive
   evaluate outputs. Threshold calibration is one-shot.

2. **Replace zero-streak gate with normalized-signal gate**.
   Existing gate fires EP when `zero_streak >= max_zero_streak
   && axioms exist && pending_delta`. Replacement:
   `normalized_drive_signal < threshold && axioms exist
   && pending_delta`. Threshold initial: 0.3 (calibrated
   against post-fix long-run; current normalized values
   sit around 1.0–4.0).

3. **Phase-shift DriveMix windows vs MetaScheduler**.
   Add a 25-episode offset to DriveMix's window-start so
   the two A/B loops never close the same window
   simultaneously. Mitigates OQ #5's two-loop interaction
   risk.

#### Verification plan for step 3b

- `h2_0_step3b_normalized_signal_is_weight_invariant`:
  hand-tuned and equal-weighted DriveMix produce
  comparable normalized signals over identical inputs.
- `h2_0_step3b_low_signal_triggers_sleep`: synthesize a
  drive registry returning all-zeros; assert runtime
  sleeps even with non-empty axioms.
- `h2_0_step3b_high_signal_keeps_runtime_running`:
  synthesize a drive registry returning high values;
  assert runtime stays Running across full window.
- F0 battery rerun: stream_diamond's STILL GROWING
  verdict must hold post-step-3b. Other seeds' verdicts
  match pre-step-3b.
- Long-run rerun: mutation trajectories should diverge
  between hand-tuned and equal-weighted runs (counter
  to OQ #1's finding under step 3a).

#### Risks (re-stated for step 3b adoption)

- **F0 verdict stability**. The step 3b threshold must
  not regress stream_diamond from STILL GROWING.
  Recommended threshold (0.3) is below typical observed
  normalized signal (1.0+) so quiescence triggers should
  be rare; calibrate empirically before commit.
- **Two-loop drift**. Phase-shifted windows reduce but
  do not eliminate interaction. May need a unified
  feedback controller in a future slice if drift
  manifests.
- **Constitutional commitment 3 (drive-as-type)**. Still
  deferred at step 3b — drives remain compile-time. H2.1
  is the slice that opens commitment 3.

Status: step 3b design refined; implementation deferred
pending user signal.
