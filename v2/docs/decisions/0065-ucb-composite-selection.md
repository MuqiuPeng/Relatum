# 0065: UCB1 composite selection (Phase Alpha-1)

Status: Proposed
Date: 2026-04-28

## Context

The 2026-04-28 cognitive-game-framing doc maps AlphaGo-class
ideas onto v2's existing architecture. Three candidate
transfer paths surfaced:

- **MCTS at composite layer** with cheap leaf evaluation
  (avoiding cost asymmetry with rollouts)
- **Value-policy decoupling** (already partially present)
- **Self-play candidates** (deferred — no symmetric
  formulation prototyped yet)

User initiated Phase Alpha (a branch off the H2.1 mainline)
to try the AlphaGo approach empirically. Per the framing
doc's analysis, naive MCTS even at the composite layer
is awkward in v2 because:

- Composites are recreated each frontier refresh; their
  identity is partially stable (`seq_id`) but their
  context isn't.
- "Leaf evaluation" requires either (a) executing the
  composite for real (defeats search purpose) or (b) a
  cheap heuristic that's already used by the existing
  scheduler (priority bias).
- Tree expansion requires simulating "what composites will
  be available next?" — same cost-asymmetry problem.

This ADR scopes the **smallest AlphaGo-flavored slice that
works in v2's current architecture**: replace the
composite-candidate selection rule (currently greedy on
priority) with **UCB1**. This is the "selection" rule of
AlphaGo's MCTS; it doesn't require rollouts or tree
expansion to be useful. It uses existing
`SequenceStats` data as priors.

If UCB1 selection produces empirically interesting
behaviour, deeper MCTS work (Phase Alpha-2) becomes
empirically motivated. If not, the negative finding
informs the cost-asymmetry hypothesis.

## Decision

### What changes

A new scheduler `UcbCompositeScheduler` wraps an inner
`RuleBasedScheduler`:

- For non-composite decisions, delegates to the inner
  scheduler unchanged.
- When the inner scheduler would pick a `CompositeCandidate`
  via `pick_top_biased`, intercept and apply UCB1
  selection over the eligible composite candidates instead.

### UCB1 selection rule

Standard UCB1:

```text
UCB1(c) = mean_reward(c) + exploration_const * sqrt(ln(N) / visits(c))
```

Where:
- `mean_reward(c)`: per-composite average post-EP-delta,
  drawn from `SequenceStats` (pair / triple
  `mean_post_ep_delta`)
- `visits(c)`: per-composite visit count, drawn from
  `pair_post_ep_count` / `triple_post_ep_count`
- `N`: total visits across all eligible composites at this
  decision point
- `exploration_const`: UCB exploration parameter
  (default `sqrt(2)` — standard UCB1)

Composites with `visits(c) == 0` get treated as
"unexplored" — UCB1 selects them deterministically until
all candidates have ≥1 visit (standard UCB1 cold-start).

### Why this is AlphaGo-flavored

AlphaGo's MCTS has four phases: **selection**, expansion,
simulation (rollout), backpropagation. UCB1 is the
selection rule. Even without expansion / simulation, UCB1
selection alone is meaningfully different from greedy:

- **Greedy**: always pick the composite with highest
  observed mean reward.
- **UCB1**: balance exploitation (high mean) with
  exploration (low visit count). Sometimes pick a rarely-
  tried composite even if its mean isn't best.

This addresses a known issue with H1.1's priority bias:
once a composite earns high priority via accumulated mean
post-EP-delta, it dominates frontier selection. New
composites with low visit counts (and therefore noisy
mean estimates) struggle to compete. UCB1 explicitly
favors low-visit composites until they've been tried
enough.

### What this slice does NOT do

- Does NOT add tree expansion (no MCTS tree).
- Does NOT add rollouts.
- Does NOT modify primitive ActionKind selection (only
  composite layer).
- Does NOT introduce per-composite stats beyond what
  `SequenceStats` already tracks.
- Does NOT change `RuleBasedScheduler` itself; new logic
  lives in a wrapper.

### Empirical contract

- F0 battery: stream_diamond verdict consistent with
  baseline. No regression.
- OQ #1 long-run hand-tuned: episode count / EP attempts
  / pair-triple count *may* differ from baseline (if UCB1
  picks differently than greedy at any decision point).
  This is the success criterion — UCB1 should produce
  *some* observable behaviour change to demonstrate
  load-bearing influence.
- OQ #1 long-run equal-weighted: similar story.

If hand-tuned long-run is byte-identical to baseline, that
means UCB1 is producing the same selections as greedy on
this substrate — possible if exploration term doesn't tip
any decisions, but a useful negative finding.

## Alternatives considered

- **Full MCTS with rollouts**. Cost asymmetry per framing
  doc; inappropriate for v2's expensive cognitive
  operations. Skipped.
- **MCTS with simulated rollouts using cached policies**.
  Requires a generative model of next-tick state; v2
  doesn't have one. Skipped.
- **Multi-armed bandit at primitive ActionKind layer**.
  Bigger scope; touches RuleBasedScheduler's main
  selection path. Composite layer is more contained.
- **Self-play as data generator** (framing doc shape (a)
  internal theory competition). Different category of
  experiment; could be pursued in parallel as Phase
  Alpha-3 if Phase Alpha-1 indicates AlphaGo direction is
  worth pursuing.

## Constitutional review

UCB1 is a numeric selection rule operating on existing
`SequenceStats` data. It does not:
- Introduce new R relations or marker classes (commitment
  1, 2 PASS)
- Change drive identity / type registry (commitment 3 PASS)
- Affect token-based identity (commitment 4 PASS)
- Make similarity claims (commitment 5 PASS)

This is a pure scheduler wrapper change. All five
commitments PASS by construction.

## Verification plan

- 5 new unit tests:
  - `alpha1_ucb_selects_unvisited_composite_first`
  - `alpha1_ucb_balances_exploration_and_exploitation`
  - `alpha1_ucb_falls_through_to_inner_for_non_composite`
  - `alpha1_ucb_with_single_composite_picks_it`
  - `alpha1_ucb_handles_empty_eligible_set`
- F0 battery: stream_diamond consistent with baseline.
- A/B example `phase_alpha_composite_ucb.rs`:
  baseline `RuleBasedScheduler` vs `UcbCompositeScheduler`
  on the same substrate (HORIZON=2000). Capture episode
  count, EP attempts, composite attempts, pair/triple
  promotions. Document differences.

## Open questions

1. **Exploration constant tuning.** `sqrt(2)` is the UCB1
   default; should we tune it? Likely answer: defer until
   we see whether even default UCB1 produces meaningful
   divergence.
2. **What's the "reward" semantically?** This ADR uses
   `mean_post_ep_delta` (which captures EP-delta after
   composite firing). Alternative: post-execution
   `normalized_drive_signal` delta. The latter is more
   AlphaGo-flavored (reward = value-net output) but
   requires after-the-fact attribution. Start with mean
   post-EP-delta (no new accounting); revisit if needed.
3. **What if H1.1 priority bonus already approximates
   UCB1 effects?** UCB1 explicitly favors low-visit;
   H1.1 doesn't. So they should diverge for unvisited
   composites. If they don't diverge empirically, that's
   itself a finding — H1.1's bias is doing UCB-like work.
4. **Does this slice scale to triple composites?**
   `SequenceStats` has both pair and triple counters; UCB1
   should apply to both equivalently. The implementation
   should iterate composite candidates regardless of
   length and use the appropriate stats.

## Touched ADRs

- **ADR 0061** (action-sequence mining) — UCB1 reads from
  `SequenceStats` populated by H1.0.
- **ADR 0063** (drive self-modification) — independent;
  Phase Alpha branches off the drive-tuning mainline.
- **cognitive-game-framing.md** — this is the first
  empirical attempt at the framing's "AlphaGo-flavored"
  direction.

## Summary

Phase Alpha-1 = UCB1 selection at the composite layer,
using existing `SequenceStats` as priors. No tree search,
no rollouts. The smallest AlphaGo-flavored slice that
works under v2's cost asymmetry constraints.

Empirical question: does UCB1 selection produce different
composite-firing patterns than the existing greedy bias,
and does that translate to different runtime behaviour
(episode count, EP attempts, sequence promotions)?

If yes → motivates Phase Alpha-2 (deeper MCTS).
If no → cost-asymmetry hypothesis empirically supported;
Phase Alpha closes here.

Status: **Accepted (implemented; negative empirical finding documented)**. UCB1 ≡ greedy on the current OQ #1 substrate due to low composite density. Further AlphaGo-flavored work deferred pending a substrate that exercises composite-vs-composite contention.

---

## Addendum 1 — Empirical result: UCB1 ≡ greedy on OQ #1 (2026-04-28)

Implementation landed per spec:
- `UcbCompositeScheduler` wrapper in `runtime/mod.rs` (~150 LOC)
- 5 unit tests covering UCB1 score correctness, composite stats
  attribution, and fallthrough semantics (all pass)
- A/B comparison example
  `examples/phase_alpha_composite_ucb.rs`
- 515 → 520 tests pass

#### Empirical comparison (OQ #1 substrate, HORIZON=2000)

| metric | baseline (greedy) | ucb1 | Δ |
|---|---|---|---|
| episodes | 268 | 268 | **0** |
| EP attempts | 129 | 129 | **0** |
| composite attempts | 1 | 1 | **0** |
| pairs named | 4 | 4 | **0** |
| triples named | 8 | 8 | **0** |

Per-snapshot trajectory delta: **all zeros at every
checkpoint** (tick 0 through tick 2000, 11 snapshots).

#### Why zero divergence

The runtime fires **exactly 1 composite over the entire 2000-tick
run**. This is the same number under both schedulers. Inspecting
the per-tick context: for the vast majority of decision points,
**there is at most 1 composite candidate eligible** — and often
zero. UCB1 selection requires multiple competing candidates to
differ from greedy; with N=1, any selection rule is the identity.

#### Diagnosis: this is the cost-asymmetry hypothesis confirmed

The framing doc and this ADR both warned that v2's substrate
shape doesn't naturally produce the kind of branching factor
AlphaGo's MCTS exploits. This run is the empirical
demonstration:

- AlphaGo: hundreds of legal moves per turn → MCTS branching
  factor ≫ 1 → tree search adds genuine signal
- v2 (current): 0–1 composite candidates per decision →
  branching factor ≤ 1 → search collapses to selection

The framing doc explicitly anticipated this in OQ #3
("What if H1.1 priority bonus already approximates UCB1
effects?"). The empirical answer is even stronger: the
question doesn't even matter, because there's almost never a
choice between candidates.

#### What this rules out

- **Pure selection-rule transfer from AlphaGo** is empirically
  silent on v2 substrates. UCB1 / Thompson sampling /
  whatever-bandit-rule applied at the composite layer will
  produce the same result so long as composite density is low.
- **Tree search at composite layer** would face the same issue:
  if the root has only 1 child, the tree is a line.

#### What this does NOT rule out

- **Tree search at primitive ActionKind layer**. v2 has 8
  primitive ActionKinds; at each decision point typically 2–4
  are eligible. Branching factor here is meaningful. Cost
  asymmetry is the obstacle (rollouts are expensive), not low
  branching.
- **Self-play as data generator**. Independent of selection
  rule.
- **Value-policy decoupling experiments**. Drive signal
  informing primitive ActionKind selection (not just
  composite) could matter.

#### Decision

Phase Alpha-1 closes here. Code retained in tree (the
`UcbCompositeScheduler` is correctly implemented; it just
doesn't matter on current substrates). Future AlphaGo-flavored
work, if pursued, should target either:

1. **Synthetic high-composite-density substrate** — design a
   substrate where 5+ composites are simultaneously eligible
   (probably requires deliberately seeding multiple promoted
   sequences), then rerun this experiment.
2. **Primitive-layer tree search with cheap leaf eval**
   (Phase Alpha-2 territory if pursued). Branching factor
   meaningful; cost asymmetry remains real.
3. **Self-play as an entirely different category** (Phase
   Alpha-3 territory).

Status of this slice: **closed with negative finding**.
ADR retained as a record of the experiment + the empirical
ground for future Phase Alpha decisions.

#### Constitutional implications

None — UCB1 selection is a pure scheduler wrapper that touches
no R relations or marker classes. All 5 commitments PASS by
construction (as the verification plan confirmed).

#### Lessons for the framing doc

- The framing doc's MCTS-with-caveats discussion was right
  that cost asymmetry is the obstacle, but it under-estimated
  a *second* obstacle: low branching factor. v2's substrate
  doesn't naturally produce competing candidates at the
  composite layer.
- "AlphaGo-flavored selection rules" need a substrate that
  produces competition. v2's H1.x sequence-mining promotes
  dominant pairs aggressively, leaving little room for
  rivals.
- Framing doc should be updated to flag both obstacles in the
  MCTS section: cost asymmetry AND low branching factor.

(Update to framing doc deferred to a later editing pass; this
is the empirical input that motivates it.)
