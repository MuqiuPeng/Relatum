# v2 retrospective — 2026-05-08

Two days after 2026-05-06's retrospective. The "stop and observe"
recommendation was ignored; instead, the user's strategic question
("stream干涸后怎么办") triggered a new mechanism arc: drive
metric (ADR 0078) → drive→scheduler integration (ADR 0079) →
thrash bypass fix (ADR 0079.1). Three ADRs over ~30 hours of work,
~50 lines of mechanism change, with the final state qualitatively
different from where we started.

## Recap from 2026-05-06

The 2026-05-06 retrospective closed the Phase Emergence
*creation* arc: v2 demonstrated constitution-compliant concept
emergence (ADR 0075 audit), micro-agent reframing (ADR 0076),
pattern quality framework (ADR 0077). It recommended:

> Stop and observe. With the runtime now auto-minting patterns,
> actual experiments using these emergent patterns become
> possible.

The user did exactly that, then asked the question that ended
the recommendation:

> stream 干涸的可能性或者要过多久才会干涸

## Four ADRs in 30 hours

### ADR 0078 (5/7) — Pattern-aware drive metric

The 2026-05-06 reflection on what's *not yet learned* surfaced
that v2 had a withdrawn drive metric draft (per-edge
`EdgeFingerprint` bucketing, forbidden by heavy reading).
ADR 0078 shipped a constitution-compliant rewrite:
`UnexplainedDriveSignal` groups unexplained R by connected-
component canonical form (subgraph-level, never per-token).

**Empirical finding**: OQ#2 leaves 91% of edges unexplained at
maturity, organized into 5 canonical buckets matching its
stream regimes. This was a *measurable gap* — runtime had
information about its own under-coverage that nothing
consumed.

### ADR 0079 (5/8) — Drive→scheduler integration

ADR 0079 wired drive into scheduler with three coordinated
changes:
- drive-driven `PatternCandidate` in `Frontier::refresh`
- drive-wake in `run_bounded` sleep short-circuit
- drive bypass in `RuleBasedScheduler` stagnation gate

**Empirical finding**: OQ#2 jumped from single-shot 2 patterns /
10 episodes to sustained-init 7 patterns / 24 episodes. v2
crossed reactive→proactive — but only for the first ~2000
ticks.

### Long-horizon observation (5/8) — Phase 3 freeze

15000-tick OQ#2 observation revealed three phases:
- Phase 1 (0-750): active mint, drive consumed
- Phase 2 (750-2000): wake-on-drive triggers, scheduler
  returns Sleep without dispatch
- Phase 3 (2000+): permanent freeze, drive plateau at 124
  unexplained / 5 buckets

Each wake-on-drive transition appeared in lifecycle log as
`Sleeping→Running@N, Running→Sleeping@N` same-tick — wake +
sleep in one tick. 100 wake events recorded but zero new
dispatches in second half.

The observation produced a more honest characterization than
the ADR 0079 commit message had claimed: "v2 has an extended
initialization phase driven by drive, after which it
stabilizes." Real but bounded improvement, not full
sustained cognition.

### ADR 0079.1 (5/8) — Drive-aware thrash bypass

Tracing the wake/sleep ping-pong revealed `would_thrash` gate
in `switch_or_sleep` — Reflect↔Expand mode oscillation count
exceeded `max_mode_oscillations=4`, so wake-on-drive→
SwitchMode(Expand)→Sleep within one tick.

5-line bypass mirrors ADR 0079's stagnation bypass: when
drive is alive on a mature rset, override thrash gate.

**Empirical finding** (800-tick verification):
- 2nd-half episodes +14 (was 0)
- 2nd-half pattern_instances +11 (was 0)
- Drive metric stays at 0 throughout (was 124 plateau) —
  thermostat behavior

Pattern count remains 7 (structural canonical ceiling). The
"7" isn't a freeze; it's the complete set of distinct
canonical forms in OQ#2's mature rset.

## What this arc revealed about v2's current motivation

v2's drive metric is **purely novelty-based**:
`unexplained_drive_signal` reports any R that no axiom
predicts and no pattern covers. There is no:

- **Learning progress** signal — drive doesn't know whether
  dispatching at a bucket *historically reduced* drive
- **Learnability filter** — drive doesn't know whether
  bucket's canonical is mintable or already-named
- **Competence weighting** — drive doesn't bias toward areas
  where existing capability has traction

This produced the OQ#2 800-tick plateau pattern: drive
identifies unexplained edges → wakes runtime → dispatch
finds the canonical → outcome is *Existing* (canonical
already minted as one of the 7 patterns) → drive shrinks
locally (instances added), but new unexplained R may
have same canonical → repeat. Pattern count cap is
structural, but the *cycle* isn't insightful — it's the
runtime repeatedly hitting the same 7 known canonicals.

The arc closes here: v2 is sustained but its motivation is
flat (novelty-only). To get *qualitative* expansion past
the structural ceiling, v2 needs richer motivation
signals.

## What recent world model research suggests

A search of recent (2025-2026) literature on world models +
intrinsic motivation surfaces three patterns directly
applicable to v2's gap:

### 1. Learning Progress (Oudeyer / Schmidhuber lineage)

The classic Learning Progress (LP) framework: track
prediction error *over time* per region of state space.
Reward isn't novelty — it's the *rate of error reduction*.
A state where the agent is learning fast gets high
intrinsic reward; a state where it's already mastered
gets none; a state where it cannot reduce error (random,
pure noise) also gets none.

Mapped to v2:
- Per-canonical-bucket: track historical (dispatch_count,
  net_drive_reduction_per_dispatch) in episode log
- Drive priority for bucket = bucket_size × historical
  error-reduction rate
- Buckets with no learning progress (always Existing) get
  low priority; buckets where mint succeeds keep high
  priority

This is a constitution-compliant reframe of v2's drive —
all signals derived from existing episode log, no new
ontology.

### 2. Curiosity as Information Gain (CIG, 2026)

The CIG framework decomposes curiosity into:
- **Novelty Sensitivity** — has the system encountered
  this region before?
- **Learnability Filtering** — can the system actually
  reduce uncertainty here?
- **Competence-Weighted Priority** — focus where
  existing capability overlaps with novelty

v2 currently implements only Novelty Sensitivity (raw
unexplained count). The other two are absent. Adding
them would let drive distinguish "unexplained because
new" from "unexplained because unmintable" from
"unexplained because already explored but not yet
fully covered" — three categories that the current
unexplained_count conflates.

### 3. Curiosity ↔ Competence two-way dynamics (Mantiuk et al. 2025)

The 2025 paper *From Curiosity to Competence* showed
exploration and representation learning interact
bidirectionally. Better representations enable richer
exploration; richer exploration produces better
representations. v2's drive is one-directional
(structure → drive); a competence signal back into the
drive computation closes the loop.

v2 already has a competence signal: ADR 0076's agent
ecosystem statistics + ADR 0077's pattern quality. These
aren't yet wired into drive computation. Wiring them in
is the v2-equivalent of CIG's Competence-Weighted
Priority.

### 4. JEPA-style predictive latent (LeCun lineage, 2024-2025)

JEPA's core idea: predict future state in latent space,
treat prediction error as primary learning signal. v2's
analog: forward-apply axioms predict R, prediction-error
drive (ADR 0064) tracks how much new R surprises current
axioms. But v2's prediction-error drive isn't currently
weighted by recent improvement — it's just a snapshot.

Adding learning-progress weighting to prediction-error
drive parallels JEPA's "predictability is reward, not
absolute uncertainty."

## Proposed next direction: ADR 0080 — Learning-progress-aware drive

A v2-specific synthesis of (1) and (2):

```text
Per-canonical-bucket bookkeeping (in episode log, no new state):
  recent_dispatch_count (over last K episodes)
  net_drive_reduction_per_dispatch
  outcome_distribution (NewPattern / Existing / Skipped)

Drive priority for bucket =
    bucket_size                       (novelty)
  × max(0, learning_progress_rate)   (learnability)
  × competence_overlap                (where competence_overlap
                                      counts how many existing
                                      patterns share canonical
                                      shape with this bucket)
```

This is a constitution-compliant addition: bucket keys are
canonical forms (subgraph-level), all factors are derived
from existing data (rset + episode log + pattern registry).

Empirical hypothesis: with learning-progress weighting, the
OQ#2 long-horizon would spend less time spinning on
already-known canonicals and more time on canonicals
where dispatch actually mints. The pattern count ceiling
of 7 may not be exceeded (structural limit), but the
*time-to-7* should drop and the post-7 sustained activity
should target more useful categories of R coverage.

## Author's note — the iteration that the work demands

This arc had a different shape from the prior cycles.
2026-05-01 was accumulation→consolidation. 2026-05-06 was
philosophy→reframing. 2026-05-08 was *piecewise mechanism
fix*: ADR 0079 fixed one gate, the long-horizon
observation surfaced a second gate, ADR 0079.1 fixed it.

The pattern of "ship→observe→find next gate→ship" is the
opposite of the strategic-pivot pattern that produced
2026-05-01 and 2026-05-06. Both are productive. The
strategic-pivot pattern works when the work is in the
wrong frame; the iterate-on-gates pattern works when the
frame is right but the implementation has accumulated
piecewise constraints.

Today's arc is iterate-on-gates: each gate (stagnation,
thrash) had a legitimate purpose in some prior context,
and each gate's drive bypass is a small, targeted
exception. There may be more gates. A systematic gate
audit (mentioned but not done in 0079.1) would be the
strategic-pivot move that converts iterate-on-gates back
to a single coherent design.

But the world-model research suggests an even more
productive pivot: don't just fix gates, replace
novelty-based drive with learning-progress drive.
Learning-progress drive doesn't trigger as often (only
when there's evidence of recent learning at that
canonical), which natively reduces gate-bypass frequency.
Gates that exist for thrash protection become less
necessary because thrash itself becomes less common.

This is the natural next strategic pivot if the work
continues: not "audit and bypass gates per drive" but
"redesign drive so it doesn't fight gates."

## Numbers

Roughly:
- v2 ADRs: 77 → **80** (counting 0079.1 as a separate ADR
  for accounting; it has its own file)
- Lib tests: 636 → **645**
- Examples: ~93 → ~96
- Result documents: ~57 → ~62
- Reflection documents: 1 → 1
- Constitution amendments: 1 → 1
- Mechanism delta: ~50 lines across 4 files

Most lines were ADR / result / log content. The actual
runtime code change is small.

## Next directions

In priority order:

1. **Stop and observe (still).** ADR 0079.1 just shipped;
   the 800-tick verification confirmed the headline finding
   but a longer horizon observation under post-fix
   conditions was unaffordable due to drive_signal compute
   cost. Caching drive_signal (per-frontier-refresh or
   tick-bucketed) would unblock long-horizon analysis.
   Pure-perf change, no new mechanism.

2. **Drive_signal caching** — small, ungated. Unblocks
   future long-horizon observation runs.

3. **ADR 0080 — Learning-progress-aware drive**, as
   sketched above. The world-model research informs the
   design; v2's existing infrastructure provides all the
   raw data. Estimated S-M scope (drive computation
   adds learning-progress factor; 1-2 new tests; new audit).

4. **Capability demo refresh** under post-0079.1 +
   potentially post-0080 conditions. The 2026-05-07 demo
   showed the static capability surface; a post-fixes demo
   would show v2 *running* its sustained cognition rather
   than just listing it.

5. **ADR 0081+ — gate audit** (deferred). If learning
   progress weakens drive enough that thrash/cooldown
   bypasses become unnecessary, those bypasses can be
   removed. If it doesn't, a systematic audit produces
   either targeted fixes or evidence that v2's gate design
   needs rethinking.

## Closing observation

The arc 2026-05-01 → 2026-05-08 has been Phase Emergence
in two acts:

- **Act 1** (5/1 → 5/6): "what does v2 create?" → answer:
  the kernel was always there; new vocabulary needed
  (heavy reading, micro-agent reframing) more than new
  mechanism
- **Act 2** (5/6 → 5/8): "what sustains v2's creation?" →
  answer: drive metric + scheduler integration, with
  iterative gate-bypass corrections; novelty-only drive
  has structural limits that learning-progress framing
  could relax

A possible Act 3 (5/8 → ?): "how does v2 expand its
creative ceiling?" — which the structural canonical
limit on patterns brings into focus. World-model
research suggests this is the right next theoretical
question, if the current cycle's pattern of "gate fix +
empirical verification" eventually concludes.

The strategic-question rhythm continues to be the highest-
leverage input. 2026-05-01's "看到坏 theory 怎么选干预",
2026-05-06's "现在的问题是这个系统无法去进行新的概念的创造吧",
2026-05-08's "stream 干涸的可能性或者要过多久才会干涸" —
each opened a phase that took several days to close. The
next strategic question's timing is unpredictable; the
work between them, predictably, will be either
strategic-pivot or iterate-on-gates depending on whether
the current frame is wrong or right.

Sources for the world-model research summary:
- [From Curiosity to Competence (2025)](https://arxiv.org/abs/2507.08210) — world models + exploration dynamics
- [Curiosity as Information Gain (2026)](https://www.clawrxiv.io/abs/2603.00009) — three-component decomposition
- [Schmidhuber / Oudeyer Learning Progress](http://www.pyoudeyer.com/oudeyerGottliebLopesPBR16.pdf) — the foundational framework
- [JEPA for RL (2025)](https://arxiv.org/abs/2504.16591) — predictive latent + intrinsic reward
