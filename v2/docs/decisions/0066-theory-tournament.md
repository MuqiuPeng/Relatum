# 0066: Theory self-play tournament (Phase Alpha-3)

Status: Proposed
Date: 2026-04-28

## Context

Phase Alpha-1 (UCB1 composite selection, ADR 0065) closed
with a negative empirical finding: low branching factor at
the composite layer makes selection-rule transfer from
AlphaGo silent on v2 substrates. The cognitive-game-framing
doc proposed three symmetric self-play candidates:

- **(a)** Internal theory competition — two co-existing
  theories predict overlapping streams; the better predictor
  survives.
- (b) Cross-substrate runtime clones — same runtime on
  different substrates; theories that transfer survive.
- (c) Mutual prediction — two runtime clones predict each
  other's predictions.

Phase Alpha-3 prototypes candidate (a). The hypothesis under
test: **does v2's runtime, on the OQ #1 substrate, discover
multiple theories whose prediction accuracy meaningfully
differs?** If yes, theory tournament is a real signal
that could feed ESTABLISHED-promotion / demotion decisions
in a future H2.1.1-extended slice. If no, theory-self-play
in this form is empirically silent on v2 substrates and
needs a richer setup (different substrate, or a different
self-play formulation).

This is **explicitly a different category** from selection-
rule transfer (Phase Alpha-1). Self-play in AlphaGo is
data-generation, not selection. The cognitive analogue is:
generate evaluation opportunities through theory
competition. The output of self-play is comparison data,
not action choices.

## Decision

### Smallest viable prototype: per-theory accuracy
ranking

Run an `AutonomousRuntime` to fixed HORIZON. At end:

1. Enumerate all theories: `rt.rset.theories()`.
2. For each theory T:
   - Get its constituent axioms: `rt.rset.theory_axioms(T)`.
   - For each axiom A: query
     `rt.memory.prediction_state.hit_rate(A, 5)` — the
     per-axiom accuracy already maintained by the
     prediction-error drive (ADR 0059 / G1.3).
   - Aggregate per-axiom hit rates into a per-theory score
     (mean across axioms, ignoring axioms with insufficient
     prediction count).
3. Sort theories by aggregated score.
4. Identify "winners" (above median) and "losers" (below).
5. Print tournament results.

### What this prototype does NOT do

- Does NOT modify ESTABLISHED status. (Future Phase Alpha-3+
  could feed demotion decisions; this prototype is
  observation-only.)
- Does NOT introduce new R relations / markers.
- Does NOT change runtime behaviour. Pure post-hoc analysis.
- Does NOT compare theories on their **shared** prediction
  territory specifically. The prototype uses each theory's
  total accuracy across its axioms; "shared territory"
  comparison is a follow-up if this prototype motivates
  one.
- Does NOT implement actual self-play data generation
  (where two-runtime clones learn from each other). That's
  candidates (b) and (c) territory.

### Why this is "self-play" and not just "ranking"

The framing-doc concern was preserving symmetry. In this
prototype:

- Both compared theories use the **same evaluation metric**
  (axiom hit rate via forward-apply).
- Both observe the **same substrate** (the OQ #1 stream).
- Both have the **same opportunity** to predict (their
  axioms are forward-applied identically).
- The **ranking is a relative comparison**, not an absolute
  threshold.

This is the symmetric-comparison structure self-play
preserves. It's degenerate in that there's no iterative
loop yet (compare once, observe, stop), but the symmetry
is intact.

Future Phase Alpha-3+ would add:
- Iterative loop: rank → demote losers → continue running →
  re-rank → repeat.
- Demotion via ESTABLISHED retraction (or theory-marker
  retraction).
- Possibly: re-discover theories from scratch after demotion;
  see if v2 converges to a stable theory set.

But for *this* slice, the symmetric comparison is the
interesting empirical question.

## Empirical contract

Two outcomes are interesting:

1. **Theories differentiate**: per-theory accuracy varies
   meaningfully (e.g., > 20% spread between best and worst).
   Means tournament has real signal; future iterative
   self-play has selection power.
2. **Theories don't differentiate**: per-theory accuracy is
   approximately uniform. Means either:
   - The substrate doesn't produce theories with materially
     different predictive value, OR
   - The current theories are all approximating the same
     underlying structure (degenerate tournament).
   Either way, this is information that constrains
   candidates (a)'s realistic scope.

A third outcome — **only 1 theory exists at end of run** — is
also informative (means tournament has no opponents and the
formulation needs a richer setup).

## Constitutional review

This prototype only **observes** the rset and prediction
state. It introduces no new R relations, no new marker
classes, no new identifiers. All 5 v2 commitments PASS by
construction.

If a future iterative-loop variant adds demotion via theory
retraction, that uses existing `RSet::retract_theory` which
is already constitution-compatible (ADR 0034 territory).

## Verification plan

- New example `examples/phase_alpha_theory_tournament.rs`.
- Run on OQ #1 substrate (HORIZON=2000, post-α + post-OQ-#4
  + post-H2.1.0+ baseline).
- Print tournament: per-theory ranked list with axiom count
  and aggregated hit rate.
- Document findings.

No new unit tests — pure observational example.

## Alternatives considered

- **Compare theories on shared prediction territory only**.
  More principled (closer to "head-to-head") but requires
  computing prediction-set intersections which is more
  code. Defer to Alpha-3+ if first prototype shows
  differentiation.
- **Iterative self-play loop (rank-demote-rerun)**. Bigger
  scope; needs a way to demote theories at runtime
  (already exists via `retract_theory` but never invoked
  from a self-play context). Save for after this prototype.
- **Cross-substrate clones (candidate b)**. Larger
  engineering surface (need to clone runtime + maintain
  two parallel runs). Save for later if (a) results are
  interesting.

## Touched ADRs

- **ADR 0030** — theory naming (the theories this
  tournament ranks were created via that mechanism).
- **ADR 0059** — prediction-error drive provides the per-
  axiom hit-rate data the tournament aggregates.
- **ADR 0053** — ESTABLISHED-promotion lifecycle is the
  natural target for tournament results to feed (future
  work).
- **cognitive-game-framing.md** — candidate (a) of the
  three symmetric self-play candidates.

## Summary

Phase Alpha-3 = post-hoc theory ranking by aggregated axiom
hit rate. The smallest tractable prototype of self-play
candidate (a). Symmetric comparison preserved; iterative
loop deferred. Empirical question: does v2 produce
theories with materially different predictive accuracy on
its discovered substrates?

If yes → motivates Phase Alpha-3+ (iterative tournament with
demotion).
If no → candidate (a) is silent on current substrates;
needs richer setup or candidate (b)/(c).

Status: **Accepted (implemented; strong positive empirical finding)**. Theories differentiate with hit-rate spread 0.6095 on OQ #1 substrate. Self-play candidate (a) has real selection signal.

---

## Addendum 1 — Empirical result: theories differentiate strongly (2026-04-28)

Implemented per spec. Pure observational example —
`examples/phase_alpha_theory_tournament.rs` — runs the
runtime on the OQ #1 substrate, ranks theories by
aggregated per-axiom hit rate at end of run, prints
tournament results. No runtime changes; no demotion; no
new unit tests (observation only).

#### Tournament results

| rank | theory_id | axioms | qualifying | aggregated hit rate |
|---|---|---|---|---|
| 1 | t_2 | 3 | 1 | **0.9992** |
| 2 | t_3 | 4 | 3 | 0.8545 |
| 3 | t_1 | 6 | 5 | 0.6664 |
| 4 | t_0 | 10 | 9 | **0.3898** |

- **Hit-rate spread: 0.6095** (well above the 0.20
  differentiation threshold the ADR set as "real signal").
- Mean hit rate: 0.7275.
- All 4 theories have qualifying axioms — no degenerate
  tournament.

**Verdict: theories DIFFERENTIATE strongly.** Self-play
candidate (a) has real selection signal on this substrate.

#### Per-axiom breakdown — the structural insight

The interesting finding isn't that theories rank — it's
*why* they rank that way:

**t_2 (top-ranked, 0.9992)** — narrow but precise:
- Has 3 axioms total, only 1 qualifying
- That axiom `ax_tpl_v3_p0-1_p1-2_c0-2` is essentially
  perfect: 99.92% hit rate
- The 2 non-qualifying axioms (`ax_antisymmetry`,
  `ax_reflexivity`) lacked enough predictions to score.

**t_0 (bottom-ranked, 0.3898)** — broad but messy:
- Has 10 axioms, 9 qualifying
- Contains the SAME `ax_tpl_v3_p0-1_p1-2_c0-2` axiom
  (same 99.92% hit rate as t_2's load-bearing axiom)
- ALSO contains many low-quality axioms at 0.04-0.05
  hit rate (`ax_tpl_v3_p0-0_p1-2_c0-1` etc.)
- The low-quality axioms drag the average from 0.99 down
  to 0.39.

**Insight**: t_0 is a "broad theory" that includes the
right axiom but pollutes itself with bad ones. t_2 is a
"narrow theory" with just the right axiom plus
non-predicting structural axioms (antisymmetry,
reflexivity). The tournament correctly identifies which
theory has higher predictive density.

This is a real selection signal — demoting t_0 would not
lose `ax_tpl_v3_p0-1_p1-2_c0-2` (it's also in t_2), but
would remove t_0's many bad axioms.

#### Comparison to Phase Alpha-1 (UCB1) result

The contrast with Phase Alpha-1 is sharp:

- **Phase Alpha-1 (selection-rule transfer)**: zero
  divergence from baseline. Cost asymmetry + low branching
  factor → rule produces no observable difference.
- **Phase Alpha-3 (data-generation transfer)**: strong
  positive signal. Spread 0.6095. Tournament has selection
  power.

Both experiments are AlphaGo-flavored, but they target
*completely different* aspects of AlphaGo's contribution:

- AlphaGo's **selection rule (UCB1 / PUCT)** doesn't
  transfer to v2 because the substrate doesn't produce
  competing candidates.
- AlphaGo's **comparative-data-generation through
  symmetric play** *does* transfer because v2's theories
  *can* be ranked by a shared evaluation metric, and they
  differ meaningfully under that ranking.

This is informative for future framing: AlphaGo's value
isn't a single thing. Different aspects transfer
differently.

#### What this enables (Phase Alpha-3+)

With strong tournament signal established, the natural
next slice is **iterative tournament with demotion**:

1. Run runtime to baseline.
2. Rank theories.
3. Retract bottom-N theories from rset
   (`RSet::retract_theory` already exists).
4. Continue running.
5. Re-rank.
6. Repeat.

Empirical questions:
- Do the demoted theories' load-bearing axioms get
  re-attached to other theories?
- Does the runtime re-discover them?
- Does the system stabilize on a smaller, higher-quality
  theory set?

Phase Alpha-3+ is concrete enough to spec; just not in
this slice (this is observation-only).

#### What this does NOT yet show

- Whether dynamic demotion actually improves runtime
  long-term productivity.
- Whether the tournament metric (axiom hit rate
  aggregation) is the right way to score theories. There
  may be theories that score low under this metric but are
  load-bearing for other operations (e.g.,
  `theory_extension` or `theory_independence` relations).
- Whether this transfers to other substrates. Tested on OQ
  #1 only; runs on different streaming environments could
  show different patterns.

#### Constitutional implications

None for the observational prototype. Tournament reads from
existing rset / prediction-state. All 5 v2 commitments
PASS by construction.

For Phase Alpha-3+ (iterative demotion), `RSet::retract_theory`
is constitutionally sound (ADR 0034 territory). Adding a
"tournament-driven" caller doesn't introduce new commitment
concerns.

#### Status

Phase Alpha-3 prototype landed. **Strong positive empirical
finding** validates self-play candidate (a). Phase Alpha-3+
(iterative demotion) is the natural follow-up.

ADR 0066 status: **Accepted (with strong positive empirical
finding)**.

---

## Addendum 2 — Phase Alpha-3+ iterative demotion lands cleanly (2026-04-28 late)

User confirmed Phase Alpha-3+ direction. Implemented per
the Addendum 1 sketch: run 1000 ticks → tournament →
retract bottom-ranked theory via `RSet::retract_theory` →
run 1000 more ticks → re-rank.

#### Implementation

`examples/phase_alpha_theory_demote_loop.rs`. Pure example
— no runtime changes, no new tests. Demotion happens at
the example level via direct `rset.retract_theory()` call.

#### Results

| metric | Phase 1 (post-1000-ticks) | Phase 2 (post-demote+1000) | Δ |
|---|---|---|---|
| theories | 4 | 3 | -1 |
| qualifying axioms | 4 | 3 | -1 |
| mean hit rate | 0.7188 | 0.8401 | **+0.1212** |
| min  hit rate | 0.3757 | 0.6664 | **+0.2908** |
| episodes | 110 | 268 | +158 |

Demotion target was `t_0` (hit rate 0.3757, the broad-and-
noisy theory from Addendum 1).

Four specific empirical confirmations:

1. **All 10 axioms of t_0 survived demotion.** Per ADR 0030
   design: `retract_theory` removes membership edges +
   theory-marker registration, but does NOT remove the
   axioms themselves. 16 meta-R edges removed; axiom
   registrations intact.

2. **Load-bearing axiom preserved.**
   `ax_tpl_v3_p0-1_p1-2_c0-2` was 1.0000 hit rate in
   Phase 1 (it's the high-quality axiom that t_0 *also*
   contained); in Phase 2 it's 0.9992 — still essentially
   perfect. Survived because t_2 still references it.

3. **No re-discovery.** Over the 1000 ticks of Phase 2,
   the runtime did NOT recreate t_0 or any similar
   "broad" theory grouping. Demotion is empirically
   sticky on this substrate.

4. **Other theories unperturbed.** Phase 2's t_2/t_3/t_1
   hit rates (0.9992/0.8545/0.6664) match Phase Alpha-3's
   *baseline* values byte-identically. The demoted theory
   wasn't load-bearing for the others.

#### Significance

The empirical loop closes:

```
discover → rank → demote loser → continue → re-rank
```

All four steps now have working machinery in v2. The
intervention demonstrably:
- Improves measured aggregate theory quality (+12% mean,
  +29% min)
- Preserves load-bearing axioms (no information loss)
- Doesn't perturb productive theory structure
- Doesn't trigger compensatory re-discovery (the bad
  grouping stays gone)

#### Deeper question raised

The bad axioms (`ax_tpl_v3_p0-0_p1-2_c0-1` etc., hit rates
0.04-0.05) **still exist as rset registrations** after
t_0 demotion. They're just no longer grouped under any
theory. They:
- Don't hurt directly (predictions are evaluated, just
  rarely match)
- DO consume `forward_apply_axiom` cycles each tick
- COULD be re-grouped into a new theory by future
  discovery (didn't happen in 1000 ticks)

Phase Alpha-4 candidate: **per-axiom tournament**. Rank
all axioms (regardless of theory membership) and
retract bottom-N axioms via `RSet::retract_axiom`. This
would be the finer-grained version of theory demotion —
strip noisy axioms while keeping good ones, even if they
share theory homes.

That's a separate slice; not committed in this addendum.

#### What this slice produced

1. Empirical confirmation that tournament-driven demotion
   is a *load-bearing* runtime intervention, not just an
   observational metric.
2. A reusable example pattern for "intervention then
   continue" — useful template for future Phase Alpha
   experiments.
3. Validation that v2's existing theory machinery
   (`retract_theory` + axiom-survival semantics) is
   correctly designed for tournament integration.

#### Status

ADR 0066 Phase Alpha-3 + Phase Alpha-3+ both **Accepted
with positive empirical findings**. Phase Alpha-4
(per-axiom tournament) recorded as natural next slice.

---

## Addendum 3 — Phase Alpha-4 lands with new performance finding (2026-04-28)

User confirmed Phase Alpha-4 direction. Implemented as a
combined Alpha-3+/Alpha-4 example: theory-level demote
followed by orphan-axiom retraction. The orphan filter is
load-bearing because `RSet::retract_axiom` fails on
theory-referenced axioms (per ADR 0030 design); only
axioms freshly orphaned by the preceding theory-level
demote can be retracted.

#### Implementation

`examples/phase_alpha_axiom_demote.rs`. Pure example —
no runtime changes. Pipeline:

1. Phase 1: discover (1000 ticks)
2. Step A: retract worst theory (Alpha-3+)
3. Step B: rank ALL axioms; for each orphan with
   hit_rate < 0.15 (calibrated), retract via `retract_axiom`
4. Phase 2: continue (200 ticks; longer hangs — see below)
5. Final tournament + comparison

#### Threshold calibration note

First attempt used 0.10 — caught 0 axioms (orphan rates
were 0.10–0.12 at 1000-tick horizon vs 0.04–0.05 at the
2000-tick Phase Alpha-3 horizon; less converged). Adjusted
to 0.15. Empirical lesson: hit-rate thresholds must be
calibrated to substrate convergence time. Percentile-based
selection (bottom 20%) would be more substrate-robust.

#### Results

| metric | Phase 1 | Phase 2 (200 ticks post-cleanup) | Δ |
|---|---|---|---|
| theories | 4 | 3 | -1 |
| **axioms** | **13** | **9** | **-4** |
| theory mean rate | 0.7188 | 0.8128 | +0.0939 |
| theory min rate | 0.3757 | 0.5829 | +0.2072 |

4 orphan axioms retracted (`ax_tpl_v3_p0-0_p1-2_c0-1` /
`_c1-0` / `_c2-0` / `_c0-2`, rates 0.109–0.116). Each
removed 19 meta-R edges. **76 meta-R edges cleaned in
total.** No retracted axiom resurrected.

#### New empirical finding: post-retract performance regression

Phase 2 ran **dramatically slower** than baseline. Phase
Alpha-3+'s 1000-tick Phase 2 finished in ~30 seconds.
Phase Alpha-4's 1000-tick Phase 2 attempt ran for 10+
minutes without finishing; reduced to 200 ticks completed
in ~5 minutes.

Likely cause: `RSet::retract_axiom` removes axiom +
variables + premise/conclusion edges correctly, but
runtime-side indices (`prediction_state.last_predicted_per_axiom`,
forward-apply caches, etc.) may not be incrementally
maintained on retract. Each subsequent tick may rebuild
or scan more than usual.

This is a **previously-unobserved performance
characteristic** of retract_axiom invoked on a live
runtime. ADR 0020 (pattern retraction) and H1.3 sequence
demotion don't trigger this — they operate on different
substructures with different runtime-side dependencies.

#### Findings summary

1. **Mechanism works**: orphan-axiom retraction succeeds
   at runtime; retracted axioms stay gone. Cleanup is
   real (13 → 9 axioms).
2. **Threshold calibration matters**: substrate
   convergence time governs which axioms can be reached.
3. **Performance regression discovered**: post-retract
   runtime is much slower per tick. Likely an
   index/cache invalidation gap. Worth investigating
   before deploying tournament-driven retraction in
   production paths.

#### Constitutional vs implementation gap

This is a real gap:

- **Constitutional layer**: axioms can be retracted at
  runtime (ADR 0030 / 0034 supports it; commitments 1-5
  pass).
- **Implementation layer**: retract-while-running is not
  an optimized path. Prediction state and frontier
  refreshes assume axioms are stable.

Phase Alpha-4 surfaces the gap; doesn't fix it. Future
fix candidates: lazy `prediction_state` cleanup, batch
retract during Reflect mode, etc.

#### Status

Phase Alpha-4 **Accepted with mixed findings**:
mechanism works, performance regression discovered,
underlying constitutional-vs-implementation gap noted.

ADR 0066 status: Phase Alpha-3 + Alpha-3+ + Alpha-4 all
implemented. Alpha-4 with caveat about runtime
performance.

---

## Addendum 4 — Phase Alpha-4 perf diagnosis: misdiagnosed (2026-04-28)

User confirmed Phase Alpha-4 perf investigation. Added per-
chunk timing to the example (`Instant`-based, 100-tick
granularity) and ran a control baseline (no intervention)
for direct comparison.

#### Method

- `examples/phase_alpha_axiom_demote.rs` instrumented with
  `Instant::now()` around `run_bounded(100)` calls.
- New `examples/phase_alpha_baseline_timed.rs`: same
  substrate, 2000 ticks, NO intervention. Provides per-
  chunk time series for the same tick range Alpha-4
  covered.
- Both runs in `--release` mode for fair comparison.

#### Headline correction

**My earlier diagnosis ("Phase 2 has retract-attributable
performance regression") was wrong.**

Direct comparison at the same tick range:

| Tick range | Baseline ms/tick | Alpha-4 ms/tick | Δ |
|---|---|---|---|
| 1001–1100 | 159.8 | 121.3 | **-38.5 (Alpha-4 faster)** |
| 1101–1200 | 224.1 | 166.6 | -57.5 (Alpha-4 faster) |
| 1201–1300 | 313.0 | 231.4 | -81.6 (Alpha-4 faster) |
| 1301–1400 | 396.4 | 296.4 | -100.0 (Alpha-4 faster) |

**Alpha-4 is consistently 25-30% faster per tick than
baseline at the same tick range.** This is exactly what
retracting axioms predicts: 9 axioms vs 13 means
proportionally less forward_apply_axiom work per tick.

#### What's actually slow: forward_apply_axiom O(N^k) scaling

Both runs (baseline + Alpha-4) exhibit linear growth in
ms/tick over time:

Baseline pattern:
- Tick 100: 2.2 ms/tick
- Tick 500: 18.9 ms/tick
- Tick 1000: 92.2 ms/tick
- Tick 1500: ~470 ms/tick (extrapolated)
- Tick 2000: ~900+ ms/tick (extrapolated)

This is **inherent** scaling of `forward_apply_axiom`'s
O(|data_ids|^|axiom_vars|) recursion. As the streaming
substrate ingests more identifiers, `data_ids` grows; for
3-variable axioms (which is the majority on this
substrate) per-axiom cost grows cubically. Multiplied
across all named axioms per tick (`snapshot_predictions`
calls `forward_apply_axiom` for each axiom every tick).

#### What I previously got wrong

Earlier observation: "Alpha-4 Phase 2 took 5+ minutes
while Alpha-3+ Phase 2 took ~30 seconds; therefore retract
caused regression."

Errors in this reasoning:

1. **Alpha-3+ Phase 2 timing was a guess, not a measurement.**
   The baseline-timed run shows that any 1000-tick run
   starting from tick 1000 will take ~6+ minutes due to
   inherent scaling, regardless of intervention.

2. **The "Phase 1 baseline 28ms/tick" was an average across
   ticks 0-1000.** Ticks 0-100 are 2ms/tick; ticks 900-1000
   are 90ms/tick. Comparing Phase 2's 121ms/tick (at tick
   1001-1100) to the Phase 1 average is comparing different
   parts of the curve.

3. **Apparent "post-retract jump" was just continuing the
   curve.** Baseline at tick 1100 is 159.8ms/tick;
   Alpha-4 at tick 1100 is 121.3ms/tick. The jump from
   28ms (Phase 1 avg) to 121ms is just the curve, not
   retract overhead.

#### Real finding

The actual finding is more interesting than "retract is
slow":

> **v2's `forward_apply_axiom` has O(N^k) per-axiom per-
> tick complexity, where N = data identifiers and k =
> axiom variable count. Long substrate runs (HORIZON ≥
> 2000) hit per-tick costs of 100-1000ms+, making them
> impractically slow for some experiments.**

This is an architectural observation that affects every
long-running experiment, not just Alpha-4. The framing
doc's "cost asymmetry" warning was right at a higher level
than I initially appreciated: forward_apply_axiom's cost
is the asymmetric component, and it's already biting on
2000-tick runs.

#### Fix candidates (now properly motivated)

The fix surface is clear:

1. **Cache `forward_apply_axiom` results across ticks.**
   Invalidate when rset changes for that axiom's
   premise / conclusion edges. Most ticks make small
   rset changes; cached results would only need partial
   recomputation.

2. **Restrict forward-apply to recent data identifiers.**
   New edges only need re-evaluation; old identifiers'
   forward-apply contribution is mostly stable.

3. **Defer forward_apply to specific scheduler phases**
   (e.g., Reflect mode only) instead of every tick.
   Currently `snapshot_predictions` runs every Running
   tick; making it conditional would cut the dominant cost.

4. **Index optimization in `RSet`**: data_ids set could be
   maintained incrementally rather than recomputed each
   call.

These are cleanly scoped engineering work. ADR 0066 doesn't
spec them; future ADRs can pick up.

#### Status correction

Phase Alpha-4 verdict: **mechanism works, retract is
correctly faster than baseline by axiom-count ratio**. The
"perf regression" narrative is withdrawn.

Real architectural finding: forward_apply_axiom is the
fundamental v2 long-run bottleneck. Worth its own
investigation but separate from Phase Alpha.

ADR 0066 status: Phase Alpha-3 + Alpha-3+ + Alpha-4 all
implemented; Alpha-4 verified correct (no regression);
underlying forward_apply_axiom scaling identified as
architectural concern for future work.
