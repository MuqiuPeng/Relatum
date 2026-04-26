# 0062: Sequence demotion + N-gram extension (Phase H1.3 / H1.4)

Status: Accepted (Phases H1.3 + H1.4 + triple-demotion + EP-composite-gap fix implemented)
Date: 2026-04-27

## Context

ADR 0061 / Phase H1 landed in three sub-slices:

- **H1.0** — `SequenceStats` accounting (pair counts +
  post-EP-delta correlations).
- **H1.1** — auto-promotion of high-correlation pairs to
  meta-R as `R(ACTION_SEQ_MARKER, seq_N)` chains; scheduler
  priority bias.
- **H1.2** — `ActionKind::ExecuteComposite` dispatches a
  promoted pair as one unit; composites bundle two actions
  into a single episode.

The post-H1.2 stream_diamond run (logged at
`logs/2026-04-27_phase_h1_sequence_dump.log`) showed the
closing meta-loop: H1.0 mines → H1.1 promotes → H1.2
dispatches → new behaviour → H1.0 mines new pair
correlations from the new behaviour.

Two structural gaps remain:

1. **Promotion is one-way.** Once a pair earns its
   meta-R chain, nothing retires it. If the (A, B) pair's
   correlation later degrades — e.g., because the rset has
   shifted into a regime where (A, B) no longer predicts
   improvement — the chain stays anyway, the
   CompositeCandidate keeps surfacing, the priority bias
   keeps stacking. Stale knowledge accumulates.

2. **Pair-only is shallow.** A 2-element sequence catches
   "consecutive A→B fires correlate with EP improvement",
   but real operational know-how often involves 3+ steps.
   The post-H1.2 dump shows several high-mean pairs whose
   counts are 1-2 because their natural occurrence is
   embedded in longer sequences (e.g., `(DT, Decl, EP)`
   would show as both `(DT, Decl)` and `(Decl, EP)`,
   diluting attribution).

This ADR scopes both extensions.

## Decision

### Two sub-slices

**H1.3 — Sequence demotion.**
Rolling window over recent post-EP-delta credits. When a
named pair's recent mean drops below a *demotion threshold*
(stricter than the promotion threshold) for a sustained
period, retract its meta-R chain. Mirror of ADR 0053's
ESTABLISHED demotion via retract cascade — cleanest is to
reuse the same lifecycle pattern.

**H1.4 — Trigram support.**
Extend `SequenceStats` to track triples `(A, B, C)`
alongside pairs. Promote triples whose stats clear a
trigram-specific threshold (likely *higher* count floor
than pairs because pair-promotion overlaps trigram
opportunities). `ActionKind::ExecuteComposite` already
handles arbitrary step counts via the `step_N` chain in
meta-R — the dispatch path scales naturally.

### H1.3 design

#### Recent-window stats

`SequenceStats` gains two fields per pair (parallels
deferred until empirics demand them):

```text
pair_recent_post_ep_count: HashMap<(ActionKind, ActionKind), u64>
pair_recent_post_ep_delta_sum: HashMap<(ActionKind, ActionKind), f64>
```

These are reset every `RECENT_WINDOW_TICKS` (default 50)
to a sliding window of fresh evidence. Cumulative counters
remain for long-run reporting; recent counters drive
promotion / demotion decisions.

When the new episode is `EvaluatePredictions(delta > 0)`,
each pair-occurrence in the lookahead window credits BOTH
the cumulative sum (existing H1.0) AND the recent sum.

At each window tick (every 50 ticks), iterate
`pair_recent_post_ep_count`; reset to zero. Pairs whose
recent mean drops below `MIN_RECENT_MEAN_FOR_RETENTION`
(default 0.02 — half of the promotion floor) AND whose
recent count was non-trivial (≥ 3) are flagged for
demotion.

#### Demotion path

`AutonomousRuntime::maybe_demote_action_sequences` runs
alongside the existing `maybe_promote_action_sequences`
sweep. For each named seq whose recent stats fall below
the retention threshold, retract its meta-R chain via a
new `RSet::retract_action_sequence_pair(prefix, suffix)`
method.

Retraction removes:
- `R(ACTION_SEQ_MARKER, seq_N)`
- `R(seq_N, seq_N_step_0)` and `R(seq_N, seq_N_step_1)`
- `R(seq_N_step_0, prefix_name)` and
  `R(seq_N_step_1, suffix_name)`

The frontier's `refresh_composite_candidates` will pick up
the absence on next refresh and stop injecting a
CompositeCandidate for the retracted seq.

#### Conservatism

Demotion is *more conservative* than promotion: the
runtime needs more evidence to demote than to promote. The
asymmetry reflects the cost: demoting a useful sequence is
worse than retaining a stale one for one extra cycle.
Specific defaults:
- Promote: count ≥ 5 AND mean > 0.05.
- Demote: recent count ≥ 5 AND recent mean < 0.02 (half
  the promotion floor).

### H1.4 design

#### Trigram stats

New `SequenceStats` field:

```text
trigram_counts: HashMap<(ActionKind, ActionKind, ActionKind), u64>
trigram_post_ep_count: HashMap<...>
trigram_post_ep_delta_sum: HashMap<...>
```

Updated as a side-effect of `Memory::record` parallel to the
pair updates. Episode triple = (episodes[-3].kind,
episodes[-2].kind, current.kind).

`SequenceStats::trigram_mean_post_ep_delta((A, B, C))`
mirrors the pair version.

#### Trigram promotion

A new `name_action_sequence_triple(a, b, c) -> seq_id`
method on RSet. Meta-R chain extends to 7 edges:

```text
R(ACTION_SEQ_MARKER, seq_N)
R(seq_N, seq_N_step_0)
R(seq_N, seq_N_step_1)
R(seq_N, seq_N_step_2)
R(seq_N_step_0, "<A name>")
R(seq_N_step_1, "<B name>")
R(seq_N_step_2, "<C name>")
```

`action_sequence_pairs` becomes
`action_sequences()` returning `Vec<(seq_id, Vec<String>)>`
where the inner Vec is the ordered step kinds. Existing
H1.1/H1.2 callers adapt to handle 2-step or 3-step
sequences uniformly.

Composite dispatch (H1.2) loops over `Vec<String>` of
steps instead of hard-coding two. ExecuteComposite arm
runs N steps, returns abstraction-score delta over the
whole composite.

#### Trigram thresholds

Trigrams accumulate slower than pairs (each occurrence
requires a specific 3-tuple in sequence). Defaults
tighter:
- count ≥ 3 (pairs use 5)
- mean > 0.10 (pairs use 0.05)

Reasoning: trigrams encode richer operational patterns;
fewer occurrences earn promotion, but each occurrence
must carry stronger correlation.

### What this does NOT do

- **No 4+-grams.** N-gram support extends to length 3 only
  here. Length-4+ requires either factor-of-N more memory
  or a sketching scheme (count-min, etc.) — out of scope.
- **No recursive composites.** ExecuteComposite of a
  composite is still forbidden; flat sequences only.
- **No cross-checkpoint demotion learning.** Recent-
  window stats reset on tick boundaries; demotion does
  not look at historical-but-old data.

## Alternatives considered

- **Skip H1.3; rely on permanent promotions.** Fine until
  the rset shifts regimes, then stale promotions
  contaminate priority bias and composite dispatch. The
  retrospective question is whether stream_diamond's regime
  is stable enough to never demote — but other substrates
  may not be.
- **Use cumulative stats for demotion.** Pair correlations
  decay slowly under cumulative averaging. Recent-window
  is more responsive.
- **Demote on first below-threshold cycle.** Too aggressive;
  flickers cause churn. Sustained-period requirement
  (recent count ≥ 3) damps that.
- **Track trigrams without pairs.** Triples are denser
  signal but also rarer occurrences. Pair stats catch
  patterns trigrams miss (e.g., (A, B) high-correlation
  pairs where the B->C suffix is irrelevant). Track both.

## Non-goals

- N-gram for N > 3.
- Probabilistic / contextual sequence models.
- Cross-runtime sequence transfer.

## Verification plan

For Phase H1.3 (when implemented):

1. Existing 452 tests pass.
2. New tests:
   - `h1_3_recent_window_resets_on_tick_boundary`: simulate
     50-tick window passing without new EP credits;
     assert recent counts hit zero.
   - `h1_3_demote_fires_below_threshold`: plant a named
     pair with low recent stats; demotion sweep retracts it.
   - `h1_3_demote_skipped_above_threshold`: plant a named
     pair with healthy recent stats; demotion sweep
     leaves it alone.
   - `h1_3_round_trip_recent_stats`: serialize/deserialize
     `SequenceStats` with non-empty recent fields.

For Phase H1.4 (when implemented):

5. New tests:
   - `h1_4_trigram_count_increments`: 5 sequential
     episodes [A, B, C, D, E] produce triples (A,B,C),
     (B,C,D), (C,D,E).
   - `h1_4_trigram_promote_fires_at_threshold`.
   - `h1_4_trigram_composite_runs_three_steps`: ExecuteComposite
     on a named triple dispatches 3 sub-actions.
   - `h1_4_trigram_round_trip_through_checkpoint`.
6. F0 battery rerun: stream_diamond should have new
   trigram-promoted sequences post-H1.4.

## Open questions

1. **Recent-window granularity.** 50 ticks is a guess;
   tunable. F0 empirics inform.
2. **Promotion-vs-demotion threshold gap.** Currently
   2× (0.05 vs 0.02). Hysteresis avoids
   promote/demote oscillation.
3. **Trigram occurrence counting.** When does the runtime
   "see" a triple? When the third member arrives. Same as
   pairs.
4. **Step-N dispatch generality.** Composite of arbitrary
   N is just iteration. Hard cap (e.g., max 5 steps) to
   avoid pathological cases?
5. **Backward-compat for action_sequence_pairs.** Existing
   callers expect Vec<(String, String, String)> for pairs.
   Either keep that signature alongside a new
   action_sequences() returning Vec<(String, Vec<String>)>,
   or adapt all callers. Suggest add new alongside,
   deprecate later.

## Touched ADRs

- **ADR 0061** (H1.0/H1.1/H1.2) is the parent. H1.3 / H1.4
  extend its mechanism.
- **ADR 0053** ESTABLISHED demotion is the precedent for
  retraction-via-rset-cascade (H1.3 follows the same
  pattern but doesn't reuse retract_pattern's machinery
  directly).

## Summary

H1 landed as a closing meta-loop: runtime experience
becomes meta-R fact via H1.1 promotion, then dispatches
back into runtime behaviour via H1.2 composites. H1.3
adds the missing demotion path; without it, the meta-R
sequence chain only grows. H1.4 extends sequence length
from 2 to 3, capturing operational patterns that pair
stats can only see in fragments.

H1.3 is the higher-priority slice (correctness — keeps
meta-R from accumulating stale entries). H1.4 is the
expressivity slice (richer signal, but pair-only is
already useful).

Status: **Proposed**. No code yet.
