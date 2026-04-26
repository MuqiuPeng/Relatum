# v2 retrospective — 2026-04-26

After landing ADR 0061 / Phase H1.0, this is a snapshot of where
v2 stands and how it got here. Not a comprehensive history; a
focused reflection on the architectural arc.

## Starting position

v2 began with a constitutional reset (ADR 0001 split, then
v2 commitments):

- **R is singular.** One universal binary relation; no typed
  primitives.
- **R is binary.** No edge attributes, no n-ary tuples.
- **Types are meta-R instances.** Patterns, axioms, theories
  are R-encoded.
- **Identity is token-based.** String equality, no implicit
  dedup.
- **Similarity is structural.** Derivable from the graph
  alone.

The **goal** (recorded in `memory/MEMORY.md` and `v2/docs/
constitution.md`): "Under intrinsic drive, construct from R
instances new relations that explain new phenomena."

The **drive** at the start: `abstraction_score`,
`counterfactual_value`, MDL gain. All compression-flavoured.

## What landed

Each phase is one or more ADRs:

| Phase | ADR | What it added |
|---|---|---|
| A0–A3 | 0052 | Autonomous runtime: scheduler / mode machine / sleep-wake / checkpoint |
| B0–B3 | 0052 | History tracking + stats-driven scheduling rules |
| C0–C2 | 0053 | M1 declarativization: ESTABLISHED / SHARED_AXIOM markers |
| C0+ | 0053 | `times_contributed_positive` counter, real M ≥ N gate |
| D0 / D0+ | 0054 | Meta-meta-pattern discovery + loop closure |
| E0 | 0055 | Direction-distinguishing canonical form |
| F0 | 0056 | D-battery verification (6 seeds + stream_diamond) |
| G0 | 0057 | Anomaly-coverage drive (mechanism only — null result) |
| G1.0 | 0058 | Axiom forward-application |
| G1.3 / G1.4 / G1.5 | 0059 | Prediction-error drive (the load-bearing fix) |
| H0 | 0060 | Meta-scheduler A/B parameter tuning |
| H1.0 | 0061 | Action-sequence accounting (signal only) |

438 unit/integration tests pass. ~75 ADRs total in the project.

## Key turning points

### G0's null result (the diagnosis)

ADR 0057 added an anomaly-coverage drive — count of data edges
not yet covered by named patterns — and wired two scheduler
hooks. Implementation passed 6 unit tests. **F0 battery
re-run: byte-identical to pre-G0.**

The mode-thrash gate (`max_mode_oscillations = 4`) bounded the
sleep-suppression hook before any new pattern discoveries
could happen. The hook fired a few times, but Reflect↔Expand
quickly hit 4 oscillations and the thrash gate forced Sleep.

This was a **real architectural finding, not a bug**:
- G0's local mechanisms worked (unit tests prove).
- The system-level ceiling was set by the thrash gate, not by
  the cooldown floor.
- Anomaly pressure alone, without a richer success signal, is
  not enough to overcome thrash protection.

**The diagnosis sentence**: G0 needed a *finer success signal*
— one where individual ticks could have positive-delta
episodes *without* naming a new pattern (so the activity isn't
tied to mode transitions the thrash gate punishes).

### G1's prediction-error drive (the fix)

ADR 0058 + ADR 0059 implemented forward-applying axioms (taking
a named axiom and computing what R-instances it claims should
hold), then comparing predictions to observations to define
prediction error.

Three sub-slices:
- **G1.3** accumulates per-axiom hit-rate counters as a side-
  effect of the tick loop.
- **G1.4** tightens ADR 0057's anomaly signal: an edge counts
  as unexplained iff *no* named pattern's Layer B covers it
  AND *no* axiom's forward-apply predicts it.
- **G1.5** introduces `ActionKind::EvaluatePredictions`. The
  scheduler dispatches it at the top of `choose` when
  `zero_streak >= max_zero_streak` AND
  `predictions_have_pending_delta(ctx)` is true.
  EvaluatePredictions returns `Some(delta)` overriding the
  abstraction-score diff with the per-axiom hit-rate-improvement
  sum. Positive-delta episodes WITHOUT rset mutation —
  decoupling sustained activity from mode-transition counters.

### The verification flip

Before G1.5: F0 battery had 6 seeds, all CONVERGED within 50
ticks regardless of topology. Compression saturation, observable
empirically.

After G1.5 + multi-phase `stream_diamond` seed +
fresh-forward-apply gating + episode-count-based verdict logic:
**`stream_diamond` is STILL GROWING through the full 300-tick
HORIZON**. Episodes 0 → 89 monotonically. Theories 3, ESTABLISHED
edges 3, mm.tries 5. Lifecycle alternates Sleeping↔Running across
phases.

The first-ever STILL GROWING verdict in v2's history. The
architectural analysis (made explicit in user / assistant
back-and-forth before any G1 design): "compression-only drive
saturates regardless of input; needs outward drive". Predicted.
Then implemented. Then observed.

**This is the most significant empirical milestone of v2 to
date** — analysis → design → implementation → observation,
closed loop.

### H0 / H1.0 (the next move)

Once the runtime had a non-saturating drive, it could start
*choosing between mechanisms* using EP delta as the standard.

- **H0** (parameter-space self-tuning): MetaScheduler A/B-tests
  two RuleBasedScheduler configs, mutates the loser. Pure
  parameter drift; no new ActionKinds.
- **H1.0** (sequence-stats accounting): tracks pair-frequency
  + post-EP-delta correlations across consecutive episodes.
  Provides the signal H1.1 / H1.2 will eventually consume.
  No scheduler change yet.

Neither moves the F0 battery verdict by itself. Both build the
substrate for H1.1 (sequence promotion to meta-R) and H1.2
(composite ActionKind dispatch — the genuine "ActionKind is no
longer a compile-time constant" move).

## Distance to the original goal

The v2 goal:
> Under intrinsic drive, construct from R instances new
> relations that explain new phenomena.

What v2 *can* do today:

- **Construct new relations from R instances**: yes —
  patterns, axioms, theories all named via discovery, all
  encoded as R-edges in meta-R.
- **Construct meta-relations** (relations *about* relations):
  yes — theory-extends, theory-independence,
  theory-parallel; ESTABLISHED, SHARED_AXIOM markers.
- **Construct meta-meta-relations**: yes — Phase D0+ named a
  pattern over the M1 subgraph (a "fan" shape recurring across
  multiple ESTABLISHED-marked patterns).
- **Sustain construction past the compression equilibrium**:
  yes (G1.5 + streaming substrate). STILL GROWING for 300
  ticks.
- **Self-tune scheduler parameters by outcome**: yes (H0).
- **Track which action sequences correlate with success**: yes
  (H1.0). Not yet acted upon.

What v2 *cannot* do today:

- **Mint new ActionKinds**. The action space is fixed at 8
  variants. H1.2 (deferred) would be the move.
- **Generate new phenomena** rather than explain existing
  R-instances. The system is inherently re-active to its rset
  + environment; it doesn't author novel scenarios.
- **Self-modify the drive**. abstraction_score and
  prediction-error are hard-coded weighted into scheduler
  decisions. H2 (most speculative) sketches this.
- **Cross-process learning**. Each `run_bounded` invocation is
  independent; insights don't persist beyond a single
  checkpoint chain.

## Open architectural questions

1. **Will H1.2 actually be useful, or are pair-correlations
   too weak in practice?** H1.0's empirical output (the
   `[sequence_stats]` checkpoint section across diverse seeds)
   is the next data point. If pair correlations are
   uniformly low-signal, H1.2's composite dispatch is solving
   a non-problem.

2. **What's the right balance between compression drive and
   prediction-error drive?** Right now they coexist — both
   feed scheduler decisions but through different paths.
   ADR 0059 G1.5 deliberately *didn't* replace
   abstraction_score; it added prediction-error alongside.
   The interaction is implicit. If a future test scenario
   reveals that one drive is consistently winning, we may
   need explicit weighting.

3. **What does "stream substrate" generalisation look like?**
   stream_diamond drips known posets at known intervals. Real
   "outward" stimuli would be unstructured — a stream where
   the runtime can't trivially predict what's coming next.
   Building a richer environment substrate is a research
   direction in its own right.

4. **Can the runtime *retract* a drive component if it
   harms?** Currently drives are additive. If
   prediction-error drive is destabilizing for some
   substrates, there's no graceful degradation path; the
   drive just runs.

5. **The meta-mechanism layer's ceiling.** H0 tunes
   parameters. H1 mines sequences. H2 sketches drive
   self-modification. Beyond H2 — could the system invent
   *new evaluation metrics* (not just optimize over fixed
   ones)? At that point we're approaching the kind of
   open-ended self-extension v2's goal-statement implied.

## What this retrospective should *not* say

- It should not claim v2 has achieved its goal. It hasn't —
  the goal is open-ended, and v2's current capabilities are
  bounded by compile-time decisions in many ways.
- It should not say the prediction-error drive is "the
  answer." It's *an* outward-drive component. Real
  intelligence likely involves multiple drive types
  (curiosity, reward, social signals — none of which v2 has).
- It should not over-credit the runtime's autonomy. The
  runtime decides what to do *within a fixed action space*.
  H1.2 (deferred) is the first move that would change that.

## Next directions (deferred to action)

In rough priority:

1. **F0-battery sequence dump diagnostic**. Run F0 with H1.0
   active, dump the `[sequence_stats]` per seed, look for
   genuinely surprising correlations. This is the empirical
   case for / against H1.1 / H1.2.

2. **Phase H1.1**: scheduler bias from sequence stats, no new
   ActionKinds yet. Lower-risk move that could produce
   observable F0 verdict changes.

3. **Phase H1.2**: composite ActionKind dispatch. The deepest
   move — ActionKind is no longer compile-time. Gated on
   H1.1 evidence.

4. **Drive-mix retrospective**. Once H1.x has produced
   evidence about whether sequence-driven scheduling
   actually wins, audit what each drive contributes to F0
   verdicts. Possibly demote (or formally weight) drives
   that aren't pulling weight.

5. **Richer environment substrates**. Build at least one F0
   seed where the environment generates *unstructured* R —
   noise, partial structures, contradictions — to stress the
   prediction-error drive against genuinely novel phenomena.

6. **Phase H2 sketch**. Once the above are clear: a serious
   ADR for self-modifying drive. Not before.

---

*Author's note*: this retrospective was produced after a long
sequence of guided iterations. Each step was small enough to
verify in isolation; the cumulative effect is a runtime that
has crossed from "compression-saturating" to "stream-engaged"
behaviour, with a solid empirical handle on what mechanisms
matter and which architectural choices have load-bearing
consequences. Further moves toward genuine self-extension
(H1.2 onward) are research-grade — design only, then
empirics, then implement, then check predicted vs observed.
That cadence will continue.
