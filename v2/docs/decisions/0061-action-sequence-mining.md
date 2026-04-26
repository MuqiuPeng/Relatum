# 0061: Action-sequence mining (Phase H1)

Status: Accepted (Phases H1.0 + H1.1 implemented; H1.2 sketched)
Date: 2026-04-26

## Context

ADR 0060 / Phase H0 landed: a `MetaScheduler` A/B-tests two
`RuleBasedScheduler` configurations using `EvaluatePredictions`
delta as the selection metric. The runtime now self-tunes its
**parameter space**, but the **action space** is fixed —
`ActionKind` is a closed enum of seven variants
(`DiscoverPatterns`, `DiscoverTheory`,
`PruneLowValueObjects`, `UpdateTheoryRelations`,
`Declarativize`, `DiscoverMetaMetaPatterns`,
`EvaluatePredictions`).

H0's parameter-space tuning answers "*which threshold values
make the runtime work better?*". H1's harder question is
"*which sequences of existing actions produce more
prediction-error improvement than any individual action?*".

Phase H1's ambition: mine the episode log for *recurring action
patterns* that correlate with positive EP delta, then promote
those patterns to first-class **composite ActionKinds** the
scheduler can dispatch as units. This is the first phase where
the runtime's *action space* genuinely grows from its own
experience — a real self-extension move, not just
parameter-tuning.

## Decision

### Three sub-slices, ordered by ambition

**H1.0 — Sequence frequency tracking (mechanism only).**
The runtime accumulates frequency statistics over recent
episode subsequences without modifying dispatch. New struct
`SequenceStats` on `Memory`:

```text
struct SequenceStats {
    /// Count of each (ActionKind, ActionKind) pair observed
    /// across consecutive episodes.
    pair_counts: HashMap<(ActionKind, ActionKind), u64>,
    /// Mean EP delta of episodes that *immediately follow* the
    /// pair — the per-sequence outcome signal.
    pair_post_ep_delta_sum: HashMap<(ActionKind, ActionKind), f64>,
    pair_post_ep_count: HashMap<(ActionKind, ActionKind), u64>,
}
```

Updated as a side-effect of `execute_and_record`. Pair `(A, B)`
counts iff the previous episode's `action_kind` is A and the
current episode's `action_kind` is B. The "post-EP" signal
fires when a pair is followed within K (default 5) episodes by
an `EvaluatePredictions` episode whose delta > 0 — that pair's
mean-post-EP-delta accumulates the contribution.

H1.0 builds the signal. Scheduler decisions are unchanged.

**H1.1 — Sequence-aware promotion (one-step lookahead).**
Once sufficient samples accumulate for a pair (default
`min_pair_samples_for_promotion = 10`), compute its
post-EP-delta mean. If the mean exceeds a threshold (default
`min_pair_promotion_delta = 0.1`), the pair is **promoted** to
a meta-R fact:

```text
R(__action_seq__, seq_N)        // registry edge
R(seq_N, step_0)                // step ordering: step_0 = first
R(seq_N, step_1)                // step_1 = second
R(step_0, <ActionKind A name>)  // step_0 references A
R(step_1, <ActionKind B name>)  // step_1 references B
```

The scheduler reads named sequences from meta-R and, when a
matching prefix appears (`previous_episode.action_kind == A`),
biases its next pick toward B by raising the priority of
frontier items that produce action_kind B.

H1.1 produces a self-extending mechanism via priority bias —
the runtime learns *preferential next-action choices* from
its own history. No new ActionKind variants yet; just biased
scheduling over existing ones.

**H1.2 — Composite ActionKind dispatch.**
The most ambitious slice. Promoted sequences gain genuinely
*compound* execution semantics: dispatching `seq_N` runs
`step_0` then `step_1` (and possibly more) as a single
"composite action" with one bookkeeping episode covering the
whole sequence.

Requires:
- An `ActionKind::Composite(seq_id: String)` variant — or, to
  keep `ActionKind` a closed enum, a parallel
  `ScheduledAction` enum with `Single(ActionKind)` and
  `Composite(seq_id)`.
- New `FrontierKind::CompositeCandidate { seq_id }`.
- `execute_action` recognises composite, looks up the meta-R
  sequence, dispatches each step internally with combined
  delta and a single Episode tag.
- A naming policy for composites — should they be allowed to
  contain composites recursively? (Suggest no, for the first
  pass — depth-1 only.)

H1.2 is the move that genuinely **grows the action space**.
The runtime's total ActionKind count can exceed compile-time
hard-coded variants.

### Why H1.0 is the right starting slice

- Builds the **signal** without changing **behaviour**. Drift
  is observable in `SequenceStats` snapshots — the runtime's
  experiential statistics — without risking destabilization
  by altering what actions are dispatched.
- Keeps the architectural commitments intact. No new
  ActionKind variants, no new R relations, no schema changes
  beyond the new `Memory` substructure. Round-trippable
  through the existing checkpoint pattern (similar to how
  `PolicyStats` rounds-trips).
- Provides empirical input for H1.1 / H1.2 design. Without
  H1.0's data, we can't tell whether sequence promotion is
  actually useful — maybe pair correlations are too weak to
  promote, in which case H1.1 / H1.2 are not worth the
  cost.

### What H1.0 does NOT do

- Does not change scheduler decisions. `MetaScheduler` and
  `RuleBasedScheduler` are unchanged.
- Does not introduce new R relations (`__action_seq__` is
  proposed for H1.1, deferred from H1.0).
- Does not extend `ActionKind`.
- Does not interact with C0/C1/C2 promotion gates.

## Phase H1.0 design

`SequenceStats` field on `Memory`. Update in
`execute_and_record` after the episode is added:

```text
on episode_added(ep):
    if let Some(prev) = memory.episodes[-2]:
        pair = (prev.action_kind, ep.action_kind)
        sequence_stats.pair_counts[pair] += 1

        // Track post-EP-delta when an EP episode follows within
        // K of the pair's later step.
        for ep' in episodes since pair:
            if ep'.action_kind == EvaluatePredictions:
                sequence_stats.pair_post_ep_delta_sum[pair] += ep'.delta
                sequence_stats.pair_post_ep_count[pair] += 1
                break
```

Round-trip: new `[sequence_stats]` checkpoint section, mirror
of `[policy_stats_action_counts]` shape. Per-pair rows
`<a_kind>\t<b_kind>\t<count>\t<post_ep_sum>\t<post_ep_count>`.

Verification:
- Synthetic-episode test: append episodes
  `[DiscoverTheory, DiscoverPatterns, EvaluatePredictions(δ=0.5),
  DiscoverTheory, DiscoverPatterns, EvaluatePredictions(δ=0.3)]`.
  Assert `pair_counts[(DiscoverTheory, DiscoverPatterns)] = 2`,
  `pair_post_ep_delta_sum[(DiscoverTheory, DiscoverPatterns)] =
  0.5 + 0.3 = 0.8`, `pair_post_ep_count = 2`.
- Round-trip test: serialize a non-empty `SequenceStats`
  through checkpoint and back.
- Empty-pair test: with no prior episode, no pair recorded.
- F0 battery: re-run; verify nothing changes (signal-only).

## Phase H1.1+ (sketch, deferred)

Once H1.0 surfaces real correlations, H1.1 introduces:

- Promotion gate: pair count ≥ 10 AND mean post-EP delta ≥ 0.1.
- Meta-R writes: `R(__action_seq__, seq_N)` + step chain.
- Scheduler bias: when last episode's action_kind matches a
  promoted pair's prefix, frontier items producing the
  pair's suffix get +1.0 priority bonus.
- Demotion: if a promoted sequence's post-EP delta drops
  below the threshold over a recent window, retract its
  meta-R chain (mirror of ADR 0053 demotion).

H1.2 (composite dispatch) is sketched in the
"Three sub-slices" section above; design specifics
intentionally deferred — H1.0/H1.1 empirics will inform.

## Alternatives considered

- **N-gram (length 3+) instead of pairs.** More expressive but
  needs much more data to populate. Pairs first; longer
  sequences if pair-correlations prove insufficient.
- **Bayesian / contextual bandits over action choice.** Bigger
  conceptual leap; multi-armed bandit framing is fine but
  introduces significant new machinery. Stick with frequency
  counting until evidence demands more.
- **Mine episode log on demand instead of accumulating
  per-step.** Cleaner-looking but means the scheduler's
  read-side path runs an O(N) scan over episodes. Push
  computation to write-time (O(1) per episode-add).
- **Use composite ActionKind expansion at compile time
  (e.g., add `DiscoverTheoryThenPatterns` as a new variant).**
  Hard-codes priors and doesn't qualify as "self-extension".
  H1's whole point is runtime-discovered composites.

## Non-goals

- Cross-process learning. Sequence stats accumulate within a
  single runtime lifetime + checkpoint chain.
- Concurrent dispatch (composite actions executing in
  parallel). Serial only.
- Recursive composites (composite of composites). Defer to
  ever-future ADR.
- Replacing C0+'s `times_contributed_positive`-based ESTABLISHED
  promotion. ESTABLISHED is about object stability;
  H1 is about sequence outcome. Separate concerns.

## Verification plan

For Phase H1.0 (when implemented):

1. Existing 432 tests pass after introducing `SequenceStats`.
2. Synthetic-episode unit test (above).
3. Round-trip test (above).
4. F0 battery rerun: `[sequence_stats]` section appears in
   the captured log's checkpoint output (for any seed where
   the runtime takes ≥ 2 ticks). Verdicts unchanged.
5. New diagnostic example `phase_h1_sequence_dump`: runs the
   runtime on a complex seed, prints the top-K pairs by
   post-EP-delta-mean. Useful for human inspection of what
   correlations actually appear.

## Open questions

1. **Pair vs longer N-gram.** Pairs first; revisit if
   F0-battery diagnostic shows pair correlations are
   either too weak or too uniform to be useful.
2. **K (post-EP lookahead window).** Default 5; tunable.
   Empirics from F0 battery should inform.
3. **Sample threshold for promotion** (H1.1). 10 is a guess.
   Tradeoff: lower → faster promotion, more noise; higher →
   slower, stabler.
4. **Demotion semantics** (H1.1). Cool sequences whose
   post-EP delta degrades — but how much, over what window?
   Symmetric with promotion threshold? TBD.
5. **Identity for composite ActionKinds** (H1.2). String IDs
   (`seq_<n>`)? Hash of constituent steps? Meta-R is the
   single source of truth either way.
6. **Composite action episode bookkeeping** (H1.2). Single
   episode for whole composite, or one per inner step? The
   former simplifies sequence-stats; the latter preserves
   fine-grained action history for future H1.3+ analyses.

## Touched ADRs

- **ADR 0052** Memory schema — H1.0 adds `SequenceStats`
  field. Round-trips through B2 checkpoint pattern.
- **ADR 0059** prediction-error drive — H1's promotion
  signal is EP delta. H1 doesn't exist without G1.5.
- **ADR 0060** meta-mechanism — H1 is the next slice after
  H0's parameter-tuning. ADR 0060 explicitly listed H1 as a
  future direction.

## Summary

Phase H0 lets the runtime tune its scheduler thresholds. Phase
H1 is the qualitatively different next move: discover
**composite mechanisms** by mining the runtime's own action
sequences for correlations with prediction-error improvement.

H1.0 (smallest viable): track pair-frequency + post-EP-delta
correlations on `Memory`, no scheduler change. Pure
observation. Provides the signal H1.1 / H1.2 will eventually
consume.

H1.1: promote high-correlation pairs to meta-R; bias
scheduler priorities accordingly. Self-extending via
priorities, not via new dispatch types.

H1.2: full composite ActionKind dispatch. Genuinely grows
the action space at runtime.

H1.2 is the deepest constitutional question Phase H raises:
when the runtime can mint new ActionKinds, the system's
"set of things it can do" is no longer a compile-time
constant. That's the genuine self-extension move v2's
goal-statement promised. H1.0 and H1.1 are stepping stones
that surface the empirical case for whether H1.2 is worth
building.

Status: **Proposed**. No code yet. H1.0 is the next
implementation candidate; H1.1 / H1.2 wait for H1.0's
empirics.
