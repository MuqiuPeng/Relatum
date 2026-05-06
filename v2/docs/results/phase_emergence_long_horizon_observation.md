# Phase Emergence — long-horizon observation

**Status**: ✓ done (2026-05-06); honest baseline diagnostic
**Log**: [`logs/2026-05-06_phase_emergence_long_horizon.log`](../../logs/2026-05-06_phase_emergence_long_horizon.log)
**Example**: [`examples/phase_emergence_long_horizon_observation.rs`](../../examples/phase_emergence_long_horizon_observation.rs)
**Predecessor**: [`phase_emergence_scheduler_integration_v2.md`](phase_emergence_scheduler_integration_v2.md)

## Goal

The 2026-05-06 retrospective recommended observation as the
first follow-up after Phase Emergence's mechanism work. With
runtime auto-mint enabled (ADR 0075 piece 2 revisited),
patterns are now first-class outputs. The question this slice
answers: **does the mint-and-trim cycle reach stable
equilibrium, oscillate, or grow unboundedly over a long
horizon?**

Method: snapshot each substrate's state every 250 ticks for
6000 ticks, well past stream-end on all substrates. Track
axiom / theory / pattern / episode counts plus DP and prune
dispatch totals.

(long5k was excluded after pilot runs — its persistent stream
injection keeps the runtime continuously active and exceeds
observation budget. Per Phase 0072-A's RSet-isomorphism
finding, OQ#1's pattern dynamics generalize to long5k.)

## Result

### OQ#1 (6000 ticks)

```
tick   axs  ths  pat  eps  DP  DPp  prune
250     11   3    1    22   3    3      3
500     11   3    1    22   3    3      3
1000    11   3    1    22   3    3      3
1500    11   3    1    22   3    3      3
1750    11   3    4↑   22   3    3      3
2000    11   3    6↑   22   3    3      3
2500    11   3    6    22   3    3      3
6000    11   3    6    22   3    3      3
```

Discovery completes at tick 250 (axioms=11, theories=3,
patterns=1, episodes=22). The runtime then sleeps. Patterns
jump 1→6 between tick 1500-2000 — **but episodes don't
change**. This jump is OQ#1's D regime stream directly
injecting `R(PATTERN_MARKER, x)` events; the runtime processes
them passively but doesn't mint anything new. After tick 2000,
state is fully static for the remaining 4000 ticks.

### narrow_a (6000 ticks)

```
tick   axs  ths  pat  eps  DP  DPp  prune
250     11   3    1    22   3    3      3
6000    11   3    1    22   3    3      3
```

Same discovery shape as OQ#1 (RSet-isomorphic at maturity per
Phase 0072-A). narrow_a's stream has no `PATTERN_MARKER`
injection, so patterns stay at 1 throughout. **All 5750 ticks
after the initial discovery phase are pure sleep** — zero
episodes, zero dispatches.

### OQ#2 (6000 ticks)

```
tick   axs  ths  pat  eps  DP  DPp  prune
250     2    2    2    10   5    2      1
6000    2    2    2    10   5    2      1
```

Sparse RSet (2 axioms, 2 theories, 2 patterns), same
single-phase discovery. Pattern=2 throughout. Episodes=10,
also static.

### Cross-substrate summary

```
Summary           OQ#1   narrow_a   OQ#2
final patterns      6        1        2
peak                6        1        2
trough              1        1        2
final episodes     22       22       10
2nd half episodes   0        0        0
```

## Headline finding

**v2's mint-and-trim cycle is single-shot, not ongoing.**

- All discovery happens within the first ~250 ticks of any run
- After discovery, the runtime sleeps forever (or until new
  stream events arrive; on narrow_a / OQ#2 no further events
  exist after stream-end and the sleep is permanent)
- Episode count in the second half of every run is **0**
- The OQ#1 pattern jump from 1→6 is a stream-driven artifact
  (D regime injects 5 `R(PATTERN_MARKER, x)` events directly),
  not emergent activity

The current cognitive substrate is **reactive** — it needs
stream events to wake and dispatch — not **proactive**. There
is no internal driver pushing the runtime to keep minting,
pruning, or evaluating after the rset has converged on its
initial axiom / theory / pattern set.

## Implications

### What this confirms about ADR 0075 piece 2 (revisited)

The runtime's auto-mint capability is real but bounded. Once
the maturity-gated multi-size fallback succeeds in minting a
pattern (or a few), and the rset stabilizes (no further
discovery work for the scheduler to find), the runtime sleeps.
The mint-and-trim balance from the v2 commit message
("DiscoverPatterns agents fire 3× minting 3 patterns;
PruneLowValueObjects agents fire 3× trimming 2") was an
**initialization-phase phenomenon**, not an ongoing
equilibrium.

Reading it through ADR 0076 micro-agent lens:
- DP agents fire 3× during initialization, then never again
- PruneLowValueObjects agents fire 3× during initialization,
  then never again
- All other agent classes also dispatch during initialization
  and then stop

Agent population goes from "active multi-class ecosystem" to
"frozen state" within ~250 ticks.

### What this implies about Phase Emergence's framing

The Phase Emergence narrative ("v2 is now a multi-agent
cognitive substrate where minter and pruner agents
collaborate") is technically accurate but should be read with
the qualifier: **the collaboration happens during a brief
initialization window**. In production runs against substrate
streams, the dynamic phase is bounded; the long-horizon state
is static.

This is not a regression from prior work — it's the same
behaviour the runtime always exhibited, now precisely
characterized. Auto-mint added pattern emergence to the
initialization phase but did not extend the dynamic phase
itself.

### What's needed for sustained dynamics

A **drive metric** that signals "there is unexplained / under-
explored structure to attend to" would push the runtime out
of sleep state in the absence of new stream events. ADR 0075's
withdrawn first form (unexplained-R drive) was attempting this
but used signature-based bucketing forbidden by the
constitution heavy reading. A constitution-compliant version
would compute drive over R uncovered by both axioms AND
patterns, with bucket-keys derived from subgraph canonical
forms (not per-token signatures).

If shipped, the runtime would continue dispatching
`DiscoverPatterns` (and other discovery actions) while
unexplained structure exists, generating a sustained
mint-and-trim cycle rather than the current single-shot one.

## What this slice does NOT claim

- Not a regression. The runtime has always slept after stream
  end on these substrates — the auto-mint addition didn't
  change that.
- Not a critique of ADR 0075 / 0076 / 0077. Those layers work
  as documented; this slice characterizes their bounded
  active window.
- Not evidence v2 lacks emergence — emergence happens, just
  in a brief initialization phase. The phase produces
  constitution-compliant emergent patterns including OQ#2's
  84-instance 3-cycle.

The slice is purely diagnostic: **measure the temporal extent
of v2's current cognitive activity to inform the next
phase's design priorities**.

## Files

- `examples/phase_emergence_long_horizon_observation.rs`
- `logs/2026-05-06_phase_emergence_long_horizon.log`
- This result doc

## Next steps (per retrospective ranking)

1. **Sample_instances_of integration** for ADR 0077's deferred
   cross-substrate validation. Smaller scope than drive
   metric; unblocks the Anomalous classification path.
2. **Pattern-aware drive metric** — the constitution-compliant
   version of ADR 0075's withdrawn first form. Larger scope
   but the natural fix for "runtime sleeps when stream
   silent". Would convert v2 from reactive to proactive.
3. **Pattern-aware intervention auto-execution** — gated
   behind the drive metric, since auto-execution without a
   drive signal would be premature.

The retrospective ranked observation first; this slice is
that observation. The honest baseline established here
informs the priority of #2 (drive metric): if sustained
cognitive dynamics is desirable, drive is the next concrete
step.
