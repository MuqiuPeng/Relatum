# v2 retrospective — 2026-04-27

One day after the 2026-04-26 retrospective. The H1 suite is now
feature-complete; this is a snapshot of where v2 lands after
that closure.

## Recap from 2026-04-26

The previous retrospective opened with the architectural arc
through G1.5 (prediction-error drive) and the first STILL
GROWING verdict on stream_diamond. It closed with H1.0
(sequence-stats accounting) just landed; H1.1 / H1.2 sketched.
"Phase H is the first move toward genuine v2-style
self-extension" was the framing. H1.2's composite dispatch was
specifically called out as the deepest constitutional question
Phase H raised — "ActionKind no longer a compile-time
constant."

## What landed in the past 24 hours

| Phase | ADR | What it added |
|---|---|---|
| H1.1 | 0061 | Promote (pair) → meta-R chain; scheduler priority bias |
| H1.2 | 0061 | `ActionKind::ExecuteComposite` runs promoted pairs as one dispatched unit |
| H1.3 | 0062 | Recent-window demotion — retract chains whose post-EP-delta degrades |
| H1.4 | 0062 | Triples (length-3 sequences); N-step composite dispatch |

466 unit/integration tests pass. Two new ADRs (0061 / 0062),
both implemented in their entirety.

## The Rubicon, crossed

H1.2 was the move I flagged 24 hours ago as the deepest
constitutional change. After it landed, the F0 battery dump
showed:

- stream_diamond episodes: 89 → 48 (composites bundle steps)
- stream_diamond patterns: 0 → 4 (composite-dispatched naming
  succeeded where bare scheduling didn't)
- (EvaluatePredictions, EvaluatePredictions) crossed the
  *strict* H1.1 threshold for the first time

The closing meta-loop is observable end-to-end:

```
runtime activity →
  episode log →
    sequence stats (H1.0) →
      auto-promotion to meta-R (H1.1) →
        composite dispatch (H1.2) →
          new behaviour →
            new sequence stats →
              ... (loop)
```

This is the first time v2's runtime extends its own action
space at runtime. The composite's individual steps come from
the existing 7 primitive ActionKinds — but the *combinations*
are runtime-discovered, named in meta-R as
`R(__action_seq__, seq_N)` chains, and dispatched as units
the runtime author never enumerated.

H1.3 added the demotion path: meta-R sequence chains can now
retract themselves when correlation degrades. H1.4 extended
sequence length from 2 to 3 — pair-only signal can miss
operational patterns that need more context.

## Where v2 now stands relative to its goal

The v2 goal:
> Under intrinsic drive, construct from R instances new
> relations that explain new phenomena.

What's empirically demonstrable today:

- **Construct first-order relations from R**: yes (patterns,
  axioms, theories — Phases A through C).
- **Construct meta-relations about objects** (experience-with):
  yes (ESTABLISHED, SHARED_AXIOM — Phase C).
- **Construct meta-meta patterns**: yes (D0+ loop closure).
- **Sustain construction past compression equilibrium**:
  yes (G1.5 outward drive on streaming substrate — STILL
  GROWING).
- **Tune scheduler parameters by outcome** (parameter-space
  self-extension): yes (H0 A/B testing).
- **Mint new operational mechanisms from experience**
  (action-space self-extension): yes (H1.1 / H1.2 / H1.4).
- **Retract stale operational mechanisms**: yes (H1.3
  demotion).

What still doesn't exist:

- **Genuinely novel ActionKind primitives**. H1.2/H1.4 minted
  new dispatch units composed of existing primitives. They
  did NOT invent atoms outside the existing 7 (now 8 with
  ExecuteComposite). The runtime can compose and recompose;
  it can't yet author wholly new atomic operations.
- **Self-modifying drive**. Compression + prediction-error
  remain hard-wired. H2 sketches drive self-modification
  but it's still a research direction, not a slice.
- **Cross-context generalisation**. Sequence stats accumulate
  within one runtime lifetime. A different substrate
  re-discovers from scratch. No transfer.
- **Falsifiability of promoted sequences**. H1.3 demotes by
  EP-delta degradation, but doesn't *test* whether removing
  a promoted sequence would actually hurt. Counterfactual
  evaluation of operational claims is a different (deeper)
  move.

## What surprised me along the way

**G0's null result.** The anomaly-coverage drive's
F0-battery byte-identical output — mode-thrash gate
dominating before the new hook could matter — was the
single most useful surprise of the whole arc. It diagnosed
the problem precisely and led to G1's design. Without that
empirical failure, G1.5's "decouple sustained activity from
mode-transition counters" wouldn't have had its motivating
case.

**The H1.0 dump's strict-vs-relaxed thresholds.** ADR
0061's defaults (count≥10, mean>0.1) wouldn't have fired
on any seed. The sequence-stats diagnostic forced an
honest threshold tuning before H1.1's auto-promotion
could be useful. Lesson: design sketches benefit from
real data before implementation.

**Composite dispatch making patterns appear where bare
scheduling didn't.** stream_diamond went from 0 patterns
to 4. The bare scheduler didn't naturally chain the
right actions; once promoted as a composite, the pair
fired and pattern naming succeeded. Operational
knowledge that the scheduler couldn't access without
being told — and the system told itself.

## Open architectural questions

1. **What's the ceiling on N for sequences?** H1.4 went to
   3. Length-4 would 8× the trigram space (assuming 8
   ActionKinds), which gets thin fast at v2-scale runtime
   lengths. A sketching approach (count-min) would make
   higher N tractable but adds approximation. Defer until
   evidence demands.

2. **Drift in promoted sequences across regimes.** A pair
   that's productive in one substrate may be neutral in
   another. H1.3 demotes only when the recent mean
   degrades; nothing handles "this pair is *neutral* now,
   not bad — but the meta-R chain is occupying frontier
   slots unnecessarily." Future cleanup mechanism?

3. **Composite of composites.** Currently flat sequences
   only. Recursive composition would let the runtime build
   genuinely deep operational hierarchies — but also
   amplifies the H1.4 N-explosion. Worth a separate ADR
   when there's evidence pair / triple composites are
   themselves frequently chained.

4. **The H2 question, revisited.** With prediction-error
   as the evaluation standard and H1.x mining the action
   space, H2 (drive self-modification) is the next deep
   research move. The runtime currently can't, e.g., notice
   "my prediction-error metric isn't capturing what
   matters here" and adapt. Is there a meaningful design
   space short of hand-crafted alternative drives?

## Distance covered, in one sentence

24 hours ago: "the runtime has prediction-error feedback
and is starting to mine its own behaviour for patterns."

Today: "the runtime mints new dispatch units from its own
behaviour, encodes them as meta-R facts, dispatches them
as units, retracts them when they stop helping, and tracks
length-3 sequences for richer signal."

The trajectory from "compression-saturation problem" (G0
null result) to "self-extending action space" (H1.4) is
seven coordinated phases. Each phase was small enough to
verify in isolation. The cumulative architectural change
is qualitative: the system that exists today is not the
one that existed 24 hours ago, and the change is along the
axis the constitutional goal-statement actually points to.

## Next directions

In rough priority:

1. **Long-run empirical cycle.** Run a meaningfully longer
   substrate (e.g., HORIZON=2000+ on a richer streaming
   environment) and observe what the H1.x suite produces.
   Are the promoted sequences stable across cycles? Does
   demotion fire often or never? Does the meta-meta-loop
   produce qualitatively different behaviour at scale?
   This is the clearest "more empirics" path before any
   new design.

2. **Triple demotion.** H1.3 demotes pairs only. Triple
   demotion is the same mechanism with the larger map. Fold
   in when triples start mattering empirically.

3. **Recent-window stats checkpoint persistence.** Currently
   recent-window stats reset on restore. Cross-checkpoint
   continuity for demotion candidacy would matter for
   long-running deployments.

4. **H2 ADR drafting.** With H1.x landed, the design space
   for drive self-modification is concrete. Worth a careful
   ADR before any implementation — H2 has more potential
   for getting wrong than any prior phase.

5. **Constitutional audit.** Re-read the v2 commitments and
   ask: have we drifted? `ACTION_SEQ_MARKER` introduces a
   second-order operational meta-R class. Is that
   commitment-compatible? Argument for: it's still
   `R(x, y)` between tokens; identity is still string-
   based; similarity is still structural. Argument
   against: the system's "what types of things exist"
   is no longer compile-time. The constitutional answer:
   types-as-meta-R-instances commitment specifically
   anticipated this (commitment 3); ACTION_SEQ chains
   *are* meta-R facts about types. Not a drift; an
   exemplification.

---

*Author's note*: 24 hours of guided iterations, seven
implementation phases, two new ADRs, ~1500 lines of new
code, +29 tests (444→466). Each phase was a small slice
designed-then-verified-then-extended. The cumulative effect
is the closing of the operational-self-extension loop the
2026-04-26 retrospective said would be the next genuine
move toward v2's goal. That loop is now closed end-to-end.

Open-ended self-extension (H2 territory and beyond) remains
research. But "genuinely self-extends along the action-
space axis the goal-statement implied" is no longer
aspirational; it's reproducible.
