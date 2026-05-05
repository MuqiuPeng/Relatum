# ADR 0076 — Micro-agent audit of v2's existing dispatch log

**Status**: ✓ done (2026-05-06)
**Log**: [`logs/2026-05-06_phase_emergence_agent_audit.log`](../../logs/2026-05-06_phase_emergence_agent_audit.log)
**Example**: [`examples/phase_emergence_agent_audit.rs`](../../examples/phase_emergence_agent_audit.rs)
**ADR**: [0076 — Micro-agent reframing](../decisions/0076-micro-agent-reframing.md)

## Goal

ADR 0076 reframes v2's existing dispatch system as a transient
micro-agent population: each `(ActionKind, target-kind)` pair
defines an agent class, each `Episode` is one agent instance.
This audit produces the empirical demonstration that the
reframing is non-trivial — different substrates yield distinct
agent populations that map naturally onto cognitive labour.

No new ontology, no new state. Pure read-only queries over
`Memory::episodes` and `policy_stats`.

## Method

Run the standard `RuleBasedScheduler` runtime on each canonical
substrate to its Phase 0 horizon. Re-read `Memory::episodes`
through the new query helpers (`agent_classes`,
`agent_attention_share_recent`). Tabulate per agent class:
episode count, success count (positive delta), success rate,
first/last tick, mean delta.

## Result

### OQ#1 (1000 ticks, 106 episodes, 6 agent classes)

```
agent class                       eps    succ    succ%    first   last    mean Δ
EvaluatePredictions/WholeRSet      71      9    12.7%        7    911    +0.057
DiscoverPatterns/PatternSize       18      0     0.0%       29    406     0.000
Declarativize/Axiom                 7      0     0.0%        4    123    -0.100
DiscoverTheory/WholeRSet            4      4   100.0%        1    121    +6.000
UpdateTheoryRelations/WholeRSet     3      1    33.3%       21    124    -0.267
Declarativize/Theory                3      0     0.0%      409    411    -0.100
```

Dominant: `EvaluatePredictions` (67% of episodes, low per-call
gain). Most efficient: `DiscoverTheory` (4 dispatches, 100%
success, mean delta +6.0 — the highest per-call structural
gain in the run). Pattern mining is high-frequency but currently
unproductive (ADR 0075 piece 2's known limit).

Recent attention share (last 20 episodes):
- EvaluatePredictions: 100% — the runtime ends Phase 0 in a
  prediction-evaluation regime

### long5k (1500 ticks, 159 episodes, 7 agent classes)

```
EvaluatePredictions/WholeRSet     103     16    15.5%        7   1411    +0.042
DiscoverPatterns/PatternSize       30      0     0.0%       29    702     0.000
ExecuteComposite/ActionSequence     8      0     0.0%      703    904     0.000
Declarativize/Axiom                 7      0     0.0%        4    123    -0.100
DiscoverTheory/WholeRSet            4      4   100.0%        1    121    +6.000
Declarativize/Theory                4      0     0.0%      409    502    -0.100
UpdateTheoryRelations/WholeRSet     3      1    33.3%       21    124    -0.267
```

Same shape as OQ#1 plus an extra `ExecuteComposite` agent class
that activated in the tick 700–900 window (action-sequence
execution from H1.2 — 8 dispatches, 0 success).

Recent attention share (last 20): EvaluatePredictions 100%.

### narrow_a (500 ticks, 76 episodes, 6 agent classes)

```
EvaluatePredictions/WholeRSet      41      9    22.0%        7    424    +0.133
DiscoverPatterns/PatternSize       18      0     0.0%       29    406     0.000
Declarativize/Axiom                 7      0     0.0%        4    123    -0.100
DiscoverTheory/WholeRSet            4      4   100.0%        1    121    +6.000
Declarativize/Theory                3      0     0.0%      409    411    -0.100
UpdateTheoryRelations/WholeRSet     3      1    33.3%       21    124    -0.267
```

Same as OQ#1 with a different EvaluatePredictions success rate
(22% vs 12.7% — narrow_a's stream produces more discoverable
prediction improvements).

Recent attention share (last 20): EvaluatePredictions 55%,
DiscoverPatterns 30%.

### OQ#2 (4500 ticks, **10 episodes**, 5 agent classes)

```
DiscoverPatterns/PatternSize        5      2    40.0%        8     16    +0.600
DiscoverTheory/WholeRSet            2      2   100.0%        1      2    +2.650
PruneLowValueObjects/Pattern        1      1   100.0%       10     10    +1.300
Declarativize/Axiom                 1      0     0.0%        4      4    -0.100
UpdateTheoryRelations/WholeRSet     1      1   100.0%        5      5    +0.700
```

Strikingly different ecosystem:
- Only 10 episodes total despite 4500-tick budget (sparse stream
  → most ticks are pure data ingestion with no scheduler work)
- DiscoverPatterns at 40% success rate vs ~0% on OQ#1-clade —
  matches Phase 0075's diversity finding (OQ#2 is the
  pattern-rich substrate)
- **EvaluatePredictions completely absent** — no axioms to
  evaluate predictions of, so the dominant OQ#1-clade agent
  class never instantiates here

Recent attention share (last 10): DiscoverPatterns 50%,
DiscoverTheory 20%. The pattern-mining agent is the
"floor-holder" on OQ#2.

## What this confirms about the reframing

Each substrate has a **substrate-specific agent population**
read off the same dispatch log:

- OQ#1-clade: dominated by long-running EvaluatePredictions
  ("prediction evaluator agents"), with high-impact-low-frequency
  DiscoverTheory ("theory builder agents") and high-frequency-
  low-impact DiscoverPatterns ("pattern miner agents")
- OQ#2: a sparser ecosystem where prediction evaluators don't
  instantiate at all, and pattern miners dominate by both
  attention share and success rate

This is what ADR 0076 said the reframing should produce: the
same data viewed under "many transient agents" instead of
"one runtime dispatching actions". The agent populations are
*real* in the sense that they're query results over actual
runtime data — not new state introduced by the reframing.

## What this empirically validates

1. **Path C is sufficient for substrate differentiation.** Even
   without per-agent persistent state, the agent populations
   are distinct enough across substrates that "OQ#2 has no
   prediction evaluator agents but lots of pattern mining
   agents" is a non-trivial true statement, not a relabelling.

2. **Confidence / specialization / attention are queryable.**
   Agent classes' success rates (`succ%`), temporal envelopes
   (`first/last`), and recent-window shares behave like
   meaningful agent attributes despite being computed on
   demand.

3. **The constitution heavy reading holds.** No agent token is
   registered in any RSet. No agent has private state. No
   phantom typing introduced. The agent ontology lives entirely
   in interpretation, exactly as ADR 0076 specifies.

## Surprises

- **DiscoverTheory is identical across OQ#1 / long5k / narrow_a**
  (4 episodes, 100% success, mean Δ +6.0, same temporal window
  tick 1-121). This is consistent with the RSet-collapse finding
  from Phase Emergence-1 substrate-diversity probe — those three
  substrates produce structurally identical RSets, so theory-
  building proceeds identically. OQ#2 deviates (2 episodes
  only) because OQ#2 produces 2 axioms total, so theory-naming
  has very few options.

- **Declarativize/Axiom always fires 7 times across OQ#1-clade**
  with mean Δ -0.1. This is a "promotion-bookkeeping agent"
  whose negative delta is just the meta-R growth penalty in
  abstraction_score; semantically it's working correctly but
  registers as zero-positive-delta.

- **OQ#2's complete absence of EvaluatePredictions** is a
  cleaner sign of substrate-specific cognition than any earlier
  diagnostic: an entire agent role is *missing* from OQ#2's
  ecosystem because the substrate fails to produce the inputs
  that role needs.

## What this doesn't address

- **No analysis of agent class evolution over time.** The
  audit reports a static end-of-run snapshot. A more elaborate
  query could show how agent populations shift across Phase 0
  → Phase 1 boundaries.
- **No cross-class interaction analysis.** Some agents likely
  enable others (e.g., DiscoverTheory enables EvaluatePredictions
  by providing axioms to evaluate). The current helpers don't
  surface inter-class dependencies; future work could do so via
  episode log windows.
- **No agent-by-agent identity over time.** Path C explicitly
  declines to track individual agents — this is the chosen
  trade-off documented in ADR 0076. If individual-agent
  continuity becomes empirically valuable, path B becomes
  necessary.

## Files

- `src/runtime/agent_view.rs` — query helpers (~110 lines)
- `src/runtime/mod.rs` — re-exports
- `src/tests.rs` — 4 new ADR-0076 tests
- `examples/phase_emergence_agent_audit.rs` — this audit
- `logs/2026-05-06_phase_emergence_agent_audit.log` — log
- This result doc

Lib tests: 617 → 621, 0 regressions.

## Verdict

**ADR 0076's path C produces non-trivial agent populations on
real runtime data.** v2 is now legitimately describable as a
multi-agent cognitive substrate where each substrate's agent
ecosystem reflects what cognitive work that substrate enables.
No new ontology was introduced, no commitment was relaxed; the
reframing is exactly the interpretive move ADR 0076 specified.

The next steps from ADR 0076's roadmap (phase 2 — episode-log
enrichments; phase 3 — path B if needed) remain deferred until
empirical demand surfaces.
