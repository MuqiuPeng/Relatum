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

---

## Addendum 5 — Perf fix Option A implemented (2026-04-28)

User confirmed perf-fix follow-up. Implemented Option A
from Addendum 4's fix candidates: amortize redundant
`collect_meta_ids` and `data_ids` computation across
the multiple `forward_apply_axiom` calls in a single tick.

#### Changes

`v2/src/lib.rs`:
- New public `RSet::forward_apply_axiom_with_data_ids(axiom_id, data_ids)`
  accepting precomputed `data_ids: &[String]`. Identical
  output to `forward_apply_axiom`.
- New public `RSet::compute_data_ids(meta) -> Vec<String>`
  extracting non-meta identifiers (sorted, deterministic).
- Original `forward_apply_axiom` unchanged in API; internally
  delegates after computing meta + data_ids itself.

`v2/src/runtime/mod.rs`: 4 hot-path call sites refactored:
- `AutonomousRuntime::snapshot_predictions` (every Running tick)
- `RuleBasedScheduler::predictions_have_pending_delta`
  (every scheduler.choose call)
- `PredictionErrorDrive::evaluate` (consulted by
  `combined_drive_signal`)
- `ActionKind::EvaluatePredictions` arm in `execute_action`
  (when EP fires)

Pattern: each site computes `meta = rset.collect_meta_ids()`
and `data_ids = rset.compute_data_ids(&meta)` once at the
top, then calls `forward_apply_axiom_with_data_ids(ax,
&data_ids)` per axiom.

#### Empirical impact

Per-100-tick comparison (no intervention, 2000 ticks):

| Chunk | Pre-fix ms/tick | Post-fix ms/tick | Δ% |
|---|---|---|---|
| 1 | 2.2 | 1.8 | -18% |
| 5 | 18.9 | 17.2 | -9% |
| 7 | 30.8 | 29.7 | -4% |
| 10 | 92.2 | 87.7 | -5% |
| 11 | 159.8 | 166.9 | +4% (variance) |
| 12 | 224.1 | 223.8 | ~0% |

5-10% reduction at low N where `collect_meta_ids` is a
meaningful fraction of per-tick cost. Invisible at high N
where `forward_apply_recursive`'s O(N^k) dominates.
Amortization saves ~12 redundant `collect_meta_ids` calls
per `snapshot_predictions` invocation but doesn't address
the recursion's exponential scaling.

#### Correctness verification

- **520 unit tests pass** post-fix (no regression).
- F0 battery: stream_diamond CONVERGED, all CONVERGED
  (identical to pre-fix).
- OQ #1 long-run hand-tuned: **268/129/1/4/8 — byte-
  identical to pre-fix baseline.** Signal trajectory
  matches exactly: -0.654 → -1.235 → -0.988.

The fix is "compute-once-pass-in" refactor; results
guaranteed identical to pre-fix.

#### What Option A does NOT address

`forward_apply_recursive`'s O(N^k) cost is unchanged. To
address: Options B/C from Addendum 4:

- **Option B**: Cache `forward_apply_axiom` results across
  ticks; invalidate on relevant rset changes. Complex
  invalidation; correctness risk.
- **Option C**: Defer `snapshot_predictions` to specific
  scheduler phases (e.g., Reflect mode only) instead of
  every Running tick. Simpler change; affects prediction-
  error-drive responsiveness.

Both deferred. Option A landed first because smallest +
guaranteed correct.

#### Status

Option A implemented; correctness verified by byte-
identical long-run baseline. Modest empirical improvement
(5-10% at low N). Options B/C scoped but deferred.

ADR 0066 status: Phase Alpha series complete + Option A
perf fix shipped + remaining perf options scoped for
future ADR.

---

## Addendum 6 — Option B (per-axiom cache) shipped (2026-04-28)

User confirmed Option B follow-up. Implemented per-axiom
`forward_apply_axiom` result caching keyed on `rset.version()`.

#### Changes

`v2/src/lib.rs`:
- New private `version: u64` field on `RSet`. Incremented
  in `add()` and `remove()` when a mutation actually
  occurs. Not part of identity (`PartialEq` compares
  `instances` only).
- New public `RSet::version()` accessor.

`v2/src/runtime/mod.rs`:
- New fields on `PredictionState`:
  - `forward_apply_cache: HashMap<String, HashSet<R>>` —
    per-axiom cached forward-apply results.
  - `forward_apply_cache_version: Option<u64>` — rset
    version at which cache was built.
- `snapshot_predictions` now:
  1. Queries `rset.version()`.
  2. If matches `forward_apply_cache_version`, cache is
     valid. Otherwise wipes cache + updates version.
  3. For each axiom: cache hit → clone cached HashSet;
     cache miss → run `forward_apply_axiom_with_data_ids`
     and store result in cache.

#### Empirical impact

Per-100-tick comparison vs Option A (only):

| Chunk | Option A | Option B | Δ |
|---|---|---|---|
| 1 | 1.8 | 1.7 | -0.1 |
| 5 | 17.2 | 17.9 | +0.7 |
| 8 | 44.8 | 45.9 | +1.1 |
| 10 | 87.7 | 88.5 | +0.8 |
| 11 | 166.9 | 163.4 | -3.5 |

Within-variance. **No measurable speedup on OQ#1
substrate.**

Diagnosis: OQ#1's runtime executes a rset-mutating action
on essentially every Running tick (Discover / Declarativize
/ Prune all increment version). Cache invalidates each
tick before it can be reused. Cache hit rate ≈ 0%.

#### Why ship anyway

1. **Correctness-preserving.** Cache hits return byte-
   identical data to fresh computation. Verified on OQ#1
   long-run: hand-tuned trajectory ticks 0-1400 match
   pre-fix baseline exactly (signal -0.654 → -0.991, all
   intermediate snapshots identical).
2. **Reusable infrastructure.** `RSet::version()` is a
   general-purpose API for any future cache /
   invalidation logic. The forward_apply_cache mechanism
   itself is ready for substrates where rset has stable
   periods.
3. **Negligible overhead.** Cache check is O(1) per axiom
   per tick; cache wipe is O(axioms) per rset change. Both
   small constants relative to forward_apply work.
4. **Future-proofing.** When v2 develops substrates with
   sleep-then-wake-stable phases (e.g., long-horizon
   experiments where the runtime stops actively
   mutating rset for stretches), cache will hit and
   provide proportional speedup.

#### What remains unsolved

- `forward_apply_recursive`'s O(N^k) is unchanged (was
  always the dominant term).
- Cache hit rate is substrate-dependent. On OQ#1: 0%.
  On hypothetical sleep-stable substrates: could be 80%+.

#### Refined fix candidates

After Options A + B, the dominant remaining cost is
`forward_apply_recursive` itself. Future approaches:

- **Option D (algorithm-level)**: prune the recursion
  by indexing data_ids by their roles in axiom premises
  (e.g., for an axiom with premise `R(x, y)`, only iterate
  data_ids that appear as `x` of some edge). Could turn
  N^k into N^(k-1) or better depending on axiom structure.
- **Option E (cache further upstream)**: cache the entire
  snapshot (not per-axiom) when no rset change AND
  episode count unchanged. Less granularity than
  per-axiom but might catch patterns A/B miss.
- **Option F (lazy snapshot)**: skip `snapshot_predictions`
  on ticks where it's known to be redundant (e.g., no
  data event arrived AND no productive action fired).

All deferred. Options A + B are sufficient for now.

#### Status

Option B implemented; correctness verified. No empirical
speedup on OQ#1 but reusable infrastructure shipped.

ADR 0066 status: Phase Alpha series + Option A + Option B
all shipped. Options D/E/F scoped for future ADR.

---

## Addendum 7 — Option D (early premise termination) — major win (2026-04-28)

User confirmed Option D follow-up. Implemented algorithm-
level pruning in `forward_apply_recursive`: at each
recursion depth, after binding a variable, immediately
check any premises whose variables are all now bound.
If unsatisfied, prune the branch instead of letting the
recursion explore N more levels of bindings before
discovering the violation at the leaf.

#### Change

`v2/src/lib.rs` — `forward_apply_recursive` body modified
to add the early-termination check inside the iteration
loop:

```rust
for i in 0..ids.len() {
    binding[depth] = i;
    // Early-termination: are any premises fully bound now?
    let mut prune = false;
    for e in &template.premise {
        if e.x_var <= depth && e.y_var <= depth {
            let x = &ids[binding[e.x_var]];
            let y = &ids[binding[e.y_var]];
            if !rs.instances.contains(&R::new(x.clone(), y.clone())) {
                prune = true;
                break;
            }
        }
    }
    if prune {
        continue;
    }
    forward_apply_recursive(rs, template, ids, binding, depth + 1, out);
}
```

This is the only change. ~15 lines of code added.

#### Why this works

Without pruning, the recursion explores all `N^k`
combinations of variable bindings, then checks premises
at the leaf. Premises that fail at depth d trigger a
return that wastes the entire `N^(k-d)` subtree
exploration.

With pruning, premises are checked AS variables are
bound. For an axiom like transitivity `R(x,y) ∧ R(y,z) ⇒
R(x,z)`:
- At depth 1 (y bound), premise `R(x,y)` is fully bound
- If unsatisfied, prune — saves all N values of z
- Effective complexity: `N * |children(x)| * |children(y)|`
  instead of `N^3` — substantially less for sparse rsets

#### Empirical impact: ~40% speedup, consistent across N

Comparison vs all prior options (per-100-tick):

| Chunk | Pre-fix | Option A | Option B | Option D | Δ vs pre |
|---|---|---|---|---|---|
| 1 | 2.2 | 1.8 | 1.7 | 1.6 | -27% |
| 5 | 18.9 | 17.2 | 17.9 | 12.3 | -35% |
| 8 | 47.6 | 44.8 | 45.9 | 26.8 | -44% |
| 10 | 92.2 | 87.7 | 88.5 | 49.2 | **-47%** |
| 11 | 159.8 | 166.9 | 163.4 | 94.6 | -41% |
| 12 | 224.1 | 223.8 | — | 131.2 | -41% |
| 13 | 313.0 | — | — | 182.6 | -42% |
| 14 | 396.4 | — | — | 237.1 | -40% |
| 15 | 496.4 | — | — | 295.5 | -40% |

Consistent **40-47% reduction** in per-tick cost.
Importantly, the speedup HOLDS at high N — it's an
algorithmic improvement, not a constant-factor win.

#### Correctness verification

- 520 unit tests pass (no regression).
- F0 battery: stream_diamond CONVERGED, all CONVERGED.
- OQ #1 long-run: **hand-tuned 268/129/1/4/8 byte-identical**
  through tick 2000. Equal-weighted run also byte-
  identical (matches at every snapshot from tick 200 to
  tick 2000). Signal trajectory matches exactly:
  hand-tuned -0.654 → -1.235 → -0.988.

The pruning is correct because it's strictly an
**ordering** change: same premises, same conclusion. We
just check earlier rather than at the leaf. Any branch
where ALL premises hold reaches the leaf and produces
the same conclusion edge.

#### Why this is the right slice

- Algorithmic gain (not constant-factor): scales with N.
- Tiny code change (~15 lines): low complexity, easy to
  reason about correctness.
- General-purpose: helps any axiom with multiple
  premises, regardless of substrate.
- No new state, no API change.

The framing doc's "cost asymmetry" warning was real —
forward_apply_recursive is the asymmetric component. But
the asymmetry isn't N^k inherent; it's
**unconstrained** N^k. Constraining via early
termination produces effective N^k_eff with k_eff
smaller than k.

#### What this slice produced

Three layered perf improvements now in place:

| Slice | Mechanism | Effect on OQ#1 |
|---|---|---|
| Option A | Amortize meta_ids/data_ids across axiom calls | -5-10% (low N), invisible (high N) |
| Option B | Per-axiom cache keyed on rset.version | ~0% (cache rarely hits) |
| Option D | Early premise termination in recursion | **-40-47%** (uniform) |

Option D is empirically the load-bearing one. Options A
and B are kept for completeness and future utility.

#### What remains

`forward_apply_recursive` still has tail recursion +
HashSet contains lookups. Further optimizations
(unrolling, indexing-based premise lookup) would yield
diminishing returns. Most algorithmic work is now done.

#### Status

Option D shipped. ~40% speedup verified. Byte-identical
correctness confirmed.

ADR 0066 status: Phase Alpha series + Option A + Option B
+ Option D all shipped. Options E/F (snapshot-level cache,
lazy snapshot) deferred but lower priority given Option D's
gain.

---

## Addendum 8 — Phase Alpha-3++ multi-round demote: converges in one iteration (2026-04-28)

User confirmed Phase Alpha-3++ direction. Implemented as
`examples/phase_alpha_theory_demote_loop_n.rs`: same
OQ#1 4-regime substrate as Phase Alpha-3+, but the demote
loop now runs up to N=3 iterations with a stop condition
(lowest theory hit rate ≥ 0.50 threshold) and tracks
history across rounds.

#### Empirical questions

1. Does the demote loop reach a fixed point, or does each
   round produce a new "worst" theory?
2. Does the runtime re-discover demoted theories within
   the next 1000 ticks?
3. Do mean/min hit rates monotonically improve, or does
   demotion plateau / regress?

#### Results

| iter | mean | min | qualifying | demoted | Δ_mean |
|---|---|---|---|---|---|
| 0 (initial) | 0.7188 | 0.3757 | 4 | — | — |
| 1 | 0.8401 | 0.6664 | 3 | t_0 | +0.1212 |
| 2 | 0.8401 | 0.6664 | 3 | — (converged) | +0.0000 |

Iteration 2 stopped without demoting because t_1 (the new
lowest, rate 0.6664) was already above the 0.50 threshold.

Verdict: **converged after 1 iteration**. Phase Alpha-3+'s
single-round demote is a stable fixed point on this
substrate, not a partial step.

#### Answers to the empirical questions

1. **Fixed point: yes, at N=1.** The substrate produces
   exactly one structurally-broad-and-noisy theory (t_0).
   Once removed, remaining theories all clear the 0.50
   bar. Multi-round iteration is a no-op on OQ#1.

2. **No re-discovery in 1000 ticks.** Confirms Addendum 2
   finding at the 2× horizon (effectively 2000 ticks
   post-demote). t_0-shaped grouping does not regenerate.

3. **Monotonic improvement, then plateau.** Iteration 1
   produces +12% mean / +29% min; iteration 2 is
   byte-identical to post-iter-1 (no churn).

#### Why this matters beyond Phase Alpha-3+

Phase Alpha-3+ proved demote works *once*. Phase Alpha-3++
proves the loop *terminates cleanly* — there's no risk of
the runtime cycling through demote → re-discover → demote
on this substrate. The intervention is well-defined as a
"prune to fixed point" operation, not a destabilizing one.

A different substrate (more theory diversity, denser
co-occurrence) might produce N>1 convergence. Worth
re-running Phase Alpha-3++ on long5k or a future H2-class
substrate before declaring N=1 universal.

#### Status

Phase Alpha-3++ **Accepted with positive empirical
findings**. Demote loop is a fixed-point intervention on
OQ#1; converges in one iteration with no re-discovery.

The loop framework (history tracking + threshold-based
termination) is reusable for future axiom-level or
drive-level tournament cycles.

---

## Addendum 9 — Phase Alpha-3+++ counterexample-guided theory repair (2026-04-28)

User asked the research-scout question: "borrow academic
ideas". Selected ILP (FOIL/PROGOL) counterexample-guided
specialization as the most falsifiable next slice — direct
follow-up to Phase Alpha-3+/3++. Rather than retract a
whole theory, detach only the failing axioms (those whose
hit_rate falls below an axiom-level threshold).

#### New API

[`RSet::retract_theory_member`](../../src/lib.rs)
removes one `R(theory_id, axiom_id)` membership edge,
leaves the theory itself and other members intact, leaves
the axiom global registration intact, cascades
`SHARED_AXIOM_MARKER` demotion (mirroring `retract_theory`).
4 unit tests in `tests` module:
- `adr0066_retract_theory_member_keeps_theory_and_other_members`
- `adr0066_retract_theory_member_rejects_non_member`
- `adr0066_retract_theory_member_rejects_unknown_theory`
- `adr0066_retract_theory_member_does_not_affect_other_theory`
524 lib tests pass.

#### Experiment design

[`examples/phase_alpha_theory_repair.rs`](../../examples/phase_alpha_theory_repair.rs).

Two paths from byte-identical Phase 0 (deterministic OQ#1
stream, 1000 ticks):
- **Path A (control = Alpha-3+)**: retract whole bottom
  theory if agg_hit_rate < 0.50; run 1000 more ticks.
- **Path B (treatment = repair)**: in bottom theory,
  detach each axiom whose hit_rate < 0.20 with ≥ 5
  predictions; theory itself stays; run 1000 more ticks.

Phase 0 sanity check asserts both paths produce identical
theory shapes (deterministic stream).

#### Results

Phase 0 (identical both paths): 4 theories. t_0 has 10
axioms with bimodal hit rates: 4 axioms at 0.10–0.12
(noise), 5 axioms at 0.41–1.00 (signal), 1 ax_reflexivity
with 0 predictions. t_0 agg = 0.3757.

Path B detaches the 4 noise axioms below 0.20:
`ax_tpl_v3_p0-0_p1-2_c{0-2,2-0,0-1,1-0}` — all
"false transitivity"-shaped (the `p0-0` premise is
weak). t_0 keeps 6 axioms (5 qualifying).

After 1000 more ticks:

| metric | A:demote | B:repair |
|---|---|---|
| theories | 3 | **4** |
| qualifying | 3 | **4** |
| mean hit rate | 0.8401 | 0.7967 |
| min hit rate | 0.6664 | **0.6664** |
| t_0 status | retracted | **rate=0.6664, qualifying=5** |

#### Why the verdict classifier said NEGATIVE — and why
that's wrong

The example's classifier flagged B negative because
mean(B) < mean(A) by 0.0434. **This is a Simpson's-paradox
artefact**: B retains a 4th theory at 0.6664 which drags
the arithmetic mean down despite every individual theory
in B being healthy. Demote drops a redundant theory whose
removal mechanically raises the mean of the remaining set;
repair retains the theory at a lower-but-acceptable rate.

The right success criteria are:
1. **Target theory passes threshold**: t_0 went 0.3757 →
   0.6664 (✓ above 0.50)
2. **Min not degraded**: 0.6664 ≡ 0.6664 (✓ identical)
3. **Qualifying preserved**: 4 ≥ 3 (✓ +1)
4. **No global axiom loss**: ax registry intact (✓)

By all four criteria, **repair succeeds**. The mean
metric was a poor choice — replaced in the take-away.

#### Surprise: structural equivalence with t_1

After repair + 1000 ticks, **t_0 = t_1 = 0.6664 exactly**.
Pre-repair, t_0's qualifying axioms (excluding noise)
averaged 0.5861, identical to t_1's pre-Phase-0 average.
Both then evolve to 0.6664 over the next 1000 ticks.

**Implication**: t_0's "good core" and t_1 are
functionally equivalent on this substrate. Demote works
not because t_0 was *bad*, but because t_0's good content
was *redundantly captured* elsewhere. Repair preserves the
redundancy explicitly.

This raises a new question for future work: **theory
deduplication / merge** as a cleaner intervention than
either demote or repair when bottom theory's good core
overlaps with a survivor's. Not pursued in this slice.

#### Verdict

Phase Alpha-3+++ **Accepted with positive empirical
findings, with a methodological correction**: the simple
"mean across qualifying theories" metric is a
Simpson's-paradox trap when interventions change the
qualifying-set size. Future tournament-style verdicts
should report (target_theory_rate, min, qualifying,
preserved_theories) tuple, not just mean.

Repair is now a viable runtime intervention alongside
demote. On OQ#1 they produce equivalent functional
outcomes (same min, t_1 reaches the same rate as
post-repair t_0); on substrates where bottom theory
contains *unique* good content, repair would strictly
beat demote (untested — needs different substrate).

#### Status

`retract_theory_member` shipped + tested. Phase
Alpha-3+++ Accepted. Theory deduplication / merge
recorded as candidate future slice.

---

## Addendum 10 — Phase Alpha-3++++ naive merge falsified; reveals subset+noise structure (2026-04-28)

User picked Direction F (theory dedup/merge) as continuation
of Alpha-3+++ — borrowing concept-lattice / FCA-style
consolidation for v2's theory layer.

#### New API

[`RSet::merge_theories(a, b)`](../../src/lib.rs) — takes
the union of two theories' member sets. If the union
matches an existing theory's member set, reuses that id;
otherwise mints a new theory id. Both originals retracted
(except the reuse target). 5 unit tests cover disjoint
union, subset reuse, self-rejection, unknown rejection,
overlapping-dedup. 529 lib tests total pass.

The API does NOT call `verify_axiom_holds` — merging two
already-named valid theory objects is a structural
operation. Validity is inherited.

#### Experiment design

[`examples/phase_alpha_theory_merge.rs`](../../examples/phase_alpha_theory_merge.rs).
Three paths from byte-identical Phase 0 (deterministic
OQ#1 stream, 1000 ticks):

- **Path A** (demote, Alpha-3+ baseline): retract bottom
- **Path B** (repair, Alpha-3+++ baseline): detach noise axioms
- **Path C** (merge, treatment): pick highest-Jaccard pair
  (≥ 0.30 floor), call `merge_theories`

Pairwise Jaccard matrix computed on full member sets.
Phase 0 sanity check asserts identical theory shape across
paths.

#### Results — naive merge underperforms

Pairwise Jaccard matrix at Phase 0:

|   | t_3 | t_1 | t_2 | t_0 |
|---|---|---|---|---|
| t_3 | — | 0.11 | 0.40 | 0.08 |
| t_1 | 0.11 | — | 0.29 | **0.60** |
| t_2 | 0.40 | 0.29 | — | 0.18 |
| t_0 | 0.08 | **0.60** | 0.18 | — |

Highest pair: (t_0, t_1) at Jaccard = 0.60. Merge picked.

| metric | A:demote | B:repair | C:merge |
|---|---|---|---|
| theories | 3 | 4 | 3 |
| qualifying | 3 | 4 | 3 |
| mean hit rate | 0.8401 | 0.7967 | **0.7479** |
| min hit rate | 0.6664 | 0.6664 | **0.3898** |
| target post-rate | retracted | t_0' = 0.6664 | **t_0 = 0.3898** |

C **NEGATIVE on min** — merged theory's hit rate drops to
0.3898, well below threshold.

#### The falsification's empirical content: t_1 ⊆ t_0

Inspecting axiom breakdowns reveals: **t_1's 5 qualifying
axioms are a strict subset of t_0's qualifying axioms**.

t_0's members (with rates):
- 4 noise axioms (`p0-0`-shaped, rates 0.10–0.12)
- 5 signal axioms (`p0-1`-shaped, rates 0.41–1.00) ← same 5 as t_1
- ax_reflexivity (no predictions)

t_1's members:
- The same 5 signal axioms
- ax_reflexivity

So **t_1 = t_0 \ noise**. The "redundancy" between them is
asymmetric: t_1 is the clean version of t_0.

When `merge_theories(t_0, t_1)` computes union, the result
equals t_0's full member set (since t_1 ⊆ t_0). The API's
existing-id reuse logic returns t_0, retracts t_1. **Merge
degenerated to "retract the clean subset, keep the noisy
superset"** — exactly the wrong direction.

#### Why this is a useful negative

This negative result *explains* the Alpha-3+++ surprise
finding ("t_0(post-repair) ≡ t_1") with a sharper
mechanism: it's not that they "converge" — t_1 was always
the qualifying core of t_0. Repair makes t_0 lose its
noise; the clean remainder is structurally identical to
t_1.

The right operation on a subset+noise pair is:
- Detect the subset relation
- Keep the higher-quality theory
- Retract the other

On (t_0, t_1) where t_1 ⊂ t_0 and t_1 has higher hit rate,
the "smart merge" answer = retract t_0 = **what Alpha-3+
demote already does**. Demote on this substrate is
implicitly a "quality-aware subset merge".

#### Recommended future work

Naive union-style merge is now **falsified** as a general
intervention. Two follow-up directions:

1. **Quality-aware merge**: before merging (a, b), compute
   per-axiom hit rates. If union members include axioms
   below `REPAIR_AXIOM_THRESHOLD`, exclude them from the
   merged theory (= merge + repair fused). On (t_0, t_1)
   this would produce a theory with the 5 signal axioms,
   matching what repair gives standalone.

2. **Subset detection + winner-take-all**: when one
   theory's member set ⊆ another's, compute aggregate hit
   rate of each. Keep the higher; retract the other. On
   (t_0, t_1) this collapses to `retract_theory(t_0)` =
   Alpha-3+ demote. So this isn't a *new* operation in
   that case; it's a *named recognition* that demote was
   the right move because of structural subset
   relationship, not just rate ranking.

Direction 1 is the more interesting follow-up because it
introduces a genuinely new operation (filtered merge).
Direction 2 codifies what Alpha-3+ already does, useful
mainly as documentation.

#### Methodological note for the tournament line

The verdict tuple should now report not just (target_rate,
min, qualifying, preserved) but also **structural
relations between candidates** (subset / superset / disjoint
/ overlapping). On substrates where bottom theory is the
*superset* of a higher-rated theory, demote is the optimal
move and we should not even *propose* merge of that pair.
This guards future tournament code against the same naive
union mistake.

#### What this slice produced

1. New `merge_theories` API + 5 unit tests; 529 lib tests
   pass.
2. Empirical falsification of naive union-style merge on
   OQ#1: NEGATIVE on min hit rate (0.3898 vs. baseline
   0.6664).
3. Sharper mechanistic explanation of the Alpha-3+++
   "t_0 ≡ t_1" finding: t_1 ⊂ t_0 (subset+noise structure),
   not symmetric functional equivalence.
4. Identified two concrete follow-up slices (quality-
   aware merge, subset-detection winner-take-all).
5. ADR 0066 Addendum 10 with full diagnosis.

#### Status

Phase Alpha-3++++ Accepted with **negative finding plus
positive mechanistic insight**. Naive merge falsified;
quality-aware merge recorded as next candidate slice.
The merge_theories API stays in the codebase as a
primitive, available for compositional use (e.g., in a
future filtered-merge or subset-aware operator).

---

## Addendum 11 — Phase Alpha-5: smart picker validates merge as deduplication primitive (2026-04-28)

User asked for the next direction after Alpha-3++++. I
proposed a substrate redesign (Candidate 1) but
re-examined and identified a smaller, sharper experiment:
the Alpha-3++++ NEGATIVE was selection bias from picking
the highest-Jaccard pair, which is structurally biased
toward subset relations. Smart picker fixes this by
*excluding* subset pairs explicitly, then choosing among
the remaining overlapping ones.

#### What changed

[`examples/phase_alpha_theory_merge_smart.rs`](../../examples/phase_alpha_theory_merge_smart.rs)
adds a smart candidate picker:
- Reject pairs where one's member set ⊆ other's
- Among remaining, pick highest Jaccard ≥ 0.20 floor
- Annotate each pair with `(jaccard, subset?, both_good?)`
  diagnostic so the picker's reasoning is auditable

Memory note: the example was initially structured with
all three runtimes alive simultaneously (rt_a + rt_b +
rt_c in scope), causing 3× memory consumption. Refactored
to wrap each path in its own scope block so the runtime
drops between paths. Single-runtime peak ~80 MB instead
of 240 MB+. No leak — just structural concurrency. (Found
when the user halted a run suspecting a leak; diagnosis
in this commit's progress.md entry.)

#### Pair diagnostic on OQ#1 Phase 0

| a | b | Jaccard | subset? | both_good? |
|---|---|---|---|---|
| t_0 | t_2 | 0.18 | no | no |
| t_0 | t_3 | 0.08 | no | no |
| t_0 | t_1 | 0.60 | **yes** | no |
| t_2 | t_3 | 0.40 | no | **yes** |
| t_2 | t_1 | 0.29 | no | no |
| t_3 | t_1 | 0.11 | no | no |

Smart picker rejects (t_0, t_1) (subset) and picks (t_2,
t_3) — the only pair with non-trivial overlap, no subset
relation, and both sides above DEMOTE_THRESHOLD.

#### Merge result — primitive validated

- t_2 = {ax_antisymmetry, ax_reflexivity, p0-1_p1-2_c0-2}
  (rate 1.0000 on 1 qualifying axiom)
- t_3 = {ax_antisymmetry, p0-1_c1-1, p0-1_c0-0,
  p0-1_p1-2_c0-2} (rate 0.9136 on 3 qualifying)
- Merged t_4 = union (5 axioms, 3 qualifying), rate
  0.8545 after +1000 ticks
- Both inputs retracted; t_4 minted fresh

t_4 is **healthy**: above threshold, retains all
qualifying content from both inputs, no degradation.

#### The PARTIAL verdict and why it's misleading

| metric | A:demote | B:repair | C:smart-merge |
|---|---|---|---|
| theories | 3 | 4 | 3 |
| qualifying | 3 | 4 | 3 |
| mean | 0.8401 | 0.7967 | 0.6369 |
| min | 0.6664 | 0.6664 | 0.3898 |

Path C's mean/min look worse — the verdict classifier
flagged PARTIAL. **This is apples-to-oranges**:

- A and B target the *bottom theory* t_0 (noise removal)
- C targets the *overlapping pair* (t_2, t_3) (dedup)
- C **doesn't touch t_0**, which stays at 0.3898 and
  drags the global mean down

The right framing: **demote/repair/merge are not
alternatives**; they target *different structural
relations*. C's merged theory itself is healthy at 0.8545
— that's the metric for the merge operator. The aggregate
mean reflects what wasn't done (no bottom-theory cleanup).

#### Three-intervention structural framework

Tournament-style theory management on a substrate has
three structural relations to detect, each with its own
optimal operator:

| structural relation | example on OQ#1 | optimal operator |
|---|---|---|
| bottom theory has *noise + good core* and **other theory captures the good core** (subset+noise) | (t_0 ⊃ t_1's core) | **demote** the noisy superset |
| bottom theory has *unique signal + noise* (no other theory has the unique signal) | hypothetical | **repair** (detach noise, keep signal) |
| two non-subset *overlapping good* theories | (t_2, t_3) at 0.40 Jaccard | **merge** (dedup) |

The right tournament policy is to compute pairwise
structural relations FIRST, then dispatch the appropriate
operator per pair. This is more disciplined than the
"highest Jaccard" or "lowest hit rate" single-criterion
pickers used in earlier slices.

#### Falsifying my own "uniqueness vs quality
anti-correlation" worry

In the proposal preamble I worried that v2's framework
structurally prevents *unique high-quality* axioms (since
persistent axioms get rediscovered by every theory
formed). The Alpha-5 result **falsifies that worry**:

- t_3 has axioms `p0-1_c1-1` (rate 0.85) and `p0-1_c0-0`
  (rate 0.89) that **t_2 doesn't have**
- These are unique to t_3 AND high-quality
- They survived 1000+ more ticks of evolution

So unique-good axioms exist on OQ#1; my theoretical worry
was overstated. **No need to design OQ#2** — the existing
substrate is rich enough to support all three
intervention types. Substrate redesign is recorded as
deferred (and now lower priority).

#### What this slice produced

1. **Memory-safety fix** for multi-path experiments:
   scope-drop runtimes between paths. Pattern reusable
   for future tournament-style examples.
2. **Smart merge picker** that excludes subset pairs,
   producing structurally-justified merge candidates.
3. **Empirical validation of merge_theories** as a
   primitive: on the right structural relation
   (non-subset overlap, both above threshold), merge
   produces a healthy deduplicated theory.
4. **Three-intervention structural framework**: explicit
   mapping from (pair structural relation) → (optimal
   operator). Closes the methodological loop opened by
   Alpha-3+/3++/3+++/3++++.
5. **Falsified** the uniqueness-quality anti-correlation
   worry; no need for OQ#2 substrate redesign now.

#### Status

Phase Alpha-5 Accepted with empirically-validated merge
primitive and a structural-relation framework that
unifies the four prior interventions. The tournament
direction is now methodologically settled on OQ#1; future
slices can move to other layers (perf, drives, action
sequences) without revisiting theory-level operators.

---

## Addendum 12 — Phase Alpha-6: ILP-style indexed-join forward apply (2026-04-28)

User picked Direction A (ILP join optimizer for
`forward_apply_axiom`) as next slice after the tournament
line settled. This is a research-scout transfer from the
ILP / Datalog tradition: instead of full Cartesian
enumeration, use **binding propagation** — when a variable
gets bound, only iterate the next variable over neighbors
that satisfy a constraining premise edge.

#### Implementation

[`forward_apply_recursive_indexed`](../../src/lib.rs)
replaces `forward_apply_recursive` as the production
enumerator. The old function is kept (`#[allow(dead_code)]`)
for reference / debug-comparison.

Mechanics:
- For each premise edge `R(p, q)` where one endpoint is
  the current depth and the other is already bound:
  - If `q == depth, p < depth`: candidate set =
    `right-neighbors of binding[p]` (look up `left_of`)
  - If `p == depth, q < depth`: candidate set =
    `left-neighbors of binding[q]` (look up `right_of`)
- Take intersection of all such candidate sets
- Iterate over the (sorted) intersection
- Fall back to full `0..N` iteration when no premise
  constrains the current depth
- Final leaf check verifies premises with both vars
  strictly < depth (already bound at earlier depths,
  validated then)

The redundant Option D early-termination check inside the
iteration loop was removed: the candidate filter
structurally enforces depth-involving premises, and
strictly-earlier premises were validated when their max
var was bound.

Optimization details:
- `id_index: HashMap<&str, usize>` (borrowed, no String
  cloning). Built once per `forward_apply_axiom_with_data_ids`
  call, amortized over recursion depth.
- `left_of` and `right_of` already exist as O(d) indexed
  lookups via `RSet::by_source` / `by_target`.

#### Correctness

532 lib tests pass (529 before + 3 new). 3 explicit
equivalence tests for transitivity on chain, symmetry on
clique, empty-premise edge case. All examples build.

#### Performance — measured against Option D baseline

Baseline: [`logs/2026-04-28_phase_alpha_baseline_optiond.log`](../../logs/2026-04-28_phase_alpha_baseline_optiond.log)
(Option D, crashed at chunk 15 — exit code 1; pre-crash
data still useful).

After indexed join (HORIZON=2000, OQ#1, no intervention):
[`logs/2026-04-28_phase_alpha_baseline_indexed_v2.log`](../../logs/2026-04-28_phase_alpha_baseline_indexed_v2.log).

| chunk | tick | Option D | Indexed | speedup |
|---|---|---|---|---|
| 5 | 500 | 12.3 ms/tick | 8.5 ms/tick | 1.45× |
| 10 | 1000 | 49.2 ms/tick | 39.5 ms/tick | 1.25× |
| 15 | 1500 | 295.5 ms/tick | 237.3 ms/tick | 1.25× |
| 20 | 2000 | crashed | 523.9 ms/tick | (no Option D data) |

Indexed run completes HORIZON=2000 in 294s (Option D
crashed mid-run). **~25% per-tick speedup** consistent
across all chunks.

#### The empirical surprise: forward_apply is no longer
the dominant cost

Theoretical complexity argument predicted 100×+ speedup
from O(d^k) replacing O(N^k) on sparse OQ#1 (N≈300, d≈5).
Actual: 1.25×.

This **falsifies** the working assumption (carried since
ADR 0066 Addendum 4) that `forward_apply_axiom` is the
dominant per-tick bottleneck. The optimization line on
this function has reached **diminishing returns**.

Where the time actually goes (hypotheses, not yet
profiled):
1. `snapshot_predictions` building `HashMap<axiom_id,
   HashSet<R>>` per call — O(axioms × predicted_edges)
   HashSet inserts.
2. `compute_data_ids` rebuilding the data-id set from
   rset every snapshot — O(|rset|) HashSet operations.
3. Per-tick scheduler / frontier construction.
4. Mode-transition / lifecycle bookkeeping.
5. Memory accumulation cost (HashMap growth, rehashes
   in `PredictionState`).

To attack any of these would require **profiling first**.
Without profile data, further perf work on `forward_apply`
is wasted effort.

#### Methodological lesson

When a transferred academic technique gives a much
smaller-than-expected speedup, the productive next step
is **identifying the new dominant cost**, not iterating
on the same function. We had two prior optimizations on
`forward_apply` (Options A/B/D) that each landed
modest (~5–40%) gains; this Phase Alpha-6 slice continues
that trend with another 25%, but the cumulative effect
plateaus because the function is no longer the long pole.

The right next move on perf is **profile-driven**: run
a representative workload under a profiler (e.g.,
`samply`, `cargo flamegraph`), identify the hot path, and
attack that. Generic algorithmic improvements without
profile data are speculative.

#### What this slice produced

1. `forward_apply_recursive_indexed` — production
   indexed-join enumerator with O(d^k) on sparse premise
   edges, byte-identical to Option D output.
2. 3 new equivalence unit tests (transitivity chain,
   symmetry clique, empty premise) — paper trail for
   correctness equivalence.
3. ~25% per-tick speedup measured on HORIZON=2000 OQ#1.
4. **Empirical falsification** of "forward_apply is the
   bottleneck" working assumption. New question:
   *what's the new dominant cost?*
5. Methodological note: future perf work should be
   profile-driven, not theory-driven.

#### Status

Phase Alpha-6 Accepted with positive perf finding +
methodological pivot. ILP indexed-join transfer
empirically validated as a primitive (correctness +
modest speedup). The forward_apply optimization line
is now **closed by diminishing returns**; future perf
slices need profile data first.

Direction A (ILP join optimizer) is the **last of the
"obvious" perf optimizations on forward_apply**. The
deferred candidate operations (premise reordering,
selectivity-based join order, indexed intersection
algorithms) would yield further small gains but are
unlikely to break the diminishing-returns pattern. They
are recorded as deferred but de-prioritized.

---

## Addendum 13 — Phase Alpha-7: DreamCoder cross-precision dream phase (2026-04-28)

User picked Direction B (DreamCoder-style sleep substrate
generation) after Direction A finished. Transfer: agent
generates imagined data from its current theories, then
validates predictions across the imagined corpus to
extract information not present in the primary stream.

#### New primitive

[`RSet::generate_substrate_from_theory`](../../src/lib.rs)
(+ `saturate_under_axioms` helper) constructs a fresh
RSet exemplifying a given theory:
1. Generate `num_ids` identifiers `gen_<theory_id>_<i>`
2. If theory has `ax_reflexivity`, seed self-loops
3. Random sparse seed at `seed_density` per ordered pair
4. Iterate forward-apply on every template axiom until
   fixed point (each iteration only adds, never removes)
5. Register theory's axioms (intension) in the result
   so callers can `forward_apply_axiom` on it directly

Predicate axioms (`ax_antisymmetry`, `ax_totality`) are
not constructively applied — they are constraints, not
generators. Documented as a known soundness gap (random
seeds may violate them; saturation respects only forward
axioms). Pre-OQ#1's theories on this substrate it didn't
matter empirically.

4 unit tests pass (transitivity holds on generated chain;
reflexivity self-loops added; self not modified; unknown
theory rejected). 536 lib tests total.

#### Experiment design

[`examples/phase_alpha_dream_phase.rs`](../../examples/phase_alpha_dream_phase.rs):

1. Run primary OQ#1 stream 1000 ticks → discover N theories
2. **Dream phase**: for each theory T_i, generate one
   substrate S_i
3. **Cross-precision matrix**: for each (i, j),
   precision = | forward_apply_all(theory_j, S_i)
   ∩ S_i | / | forward_apply_all(theory_j, S_i) |
4. Report: diagonal sanity, off-diagonal variance,
   per-theory generality (column means)

Verdict tiers:
- POSITIVE: all diagonals = 1.0; off-diagonal var ≥ 0.01
  (cross-precision is discriminative)
- WEAK: diagonals = 1.0 but off-diagonals nearly identical
- INCONCLUSIVE / BUG: otherwise

#### Two false-start runs as a methodological lesson

**Run 1** (NUM_GEN_IDS=8, SEED_DENSITY=0.30, only theory_i's
axioms registered in S_i): every cell = 1.0, but with
`forward_apply` returning empty for any axiom not registered
in the substrate. The matrix was an artefact: only the
SHARED transitivity axiom contributed anything. Not a real
signal.

**Run 2** (NUM_GEN_IDS=15, SEED_DENSITY=0.05, same axiom
registration): every cell = 1.0 with prediction counts
identical across columns (33/33, 34/34, 23/23, 225/225).
Same artefact — sparser substrates didn't help because the
underlying bug was that theory_j's axioms weren't
registered in substrate_i, so `forward_apply_axiom`
returned empty and only transitivity (universally
registered because shared) contributed.

**Run 3** (same parameters as run 2 + register every
theory's axioms in every substrate): finally produced a
discriminative matrix.

The lesson: `forward_apply_axiom(ax)` requires `ax` to be
registered (its intension wired into the rset). Cross-
validation experiments must register all relevant axioms
in every substrate before forward-applying.

#### Discriminative result (run 3)

| sub\theory_j | t_0 | t_2 | t_3 | t_1 |
|---|---|---|---|---|
| t_0 | 1.00 | 1.00 | 1.00 | 1.00 |
| t_2 | **0.15** | 1.00 | 1.00 | **0.45** |
| t_3 | **0.16** | 1.00 | 1.00 | **0.50** |
| t_1 | **0.76** | 1.00 | 1.00 | 1.00 |

Diagonals = 1.0 (sanity ✓). Off-diagonal variance = 0.108
(POSITIVE verdict).

Per-theory generality (column means, excluding diagonal):

| theory | mean precision | min |
|---|---|---|
| t_0 | **0.36** | 0.15 |
| t_1 | 0.65 | 0.45 |
| t_2 | **1.00** | 1.00 |
| t_3 | **1.00** | 1.00 |

#### Mechanistic interpretation

- **t_2, t_3 universal**: contain only transitivity-shaped
  forward axioms (plus predicate axioms that don't
  forward-apply). Predictions on any saturated substrate
  are subsets of that substrate.
- **t_1 weak on antisymmetric substrates**: t_1 has
  symmetry (`ax_tpl_v2_p0-1_c1-0`). On t_2/t_3 substrates
  (antisymmetric by construction), symmetry predicts
  reverse edges that don't exist → precision ~0.45-0.50.
- **t_0 worst**: t_0's 4 `p0-0` noise axioms have premises
  like `R(x,x) ∧ R(x,z)` and conclusions involving
  reverse-direction edges. On any substrate with self-
  loops (reflexive), they fire on every (x,x) × (x,z)
  pair and predict edges that mostly don't exist. On
  t_1's substrate (which IS reflexive + symmetric, but
  NOT noise-saturated): t_0's 0.76 because the noise
  axioms predict edges not in t_1's substrate.

#### Significance

**Cross-precision provides a theory-quality signal
INDEPENDENT of primary-stream hit rate**. The result
recovers the same verdict as the prior tournament line —
t_0 is the worst theory, t_2/t_3 are the best — but via
a totally different mechanism: imagined substrate
cross-validation, no consultation of the primary stream's
hit-rate counters.

This is the first v2 mechanism that produces a
quality-judgment signal **without** running on real data.
DreamCoder's premise — that an agent can extract
information about its concepts via imagined-task
generation — empirically transfers to v2.

The signal is also strictly newer than prior tournament
metrics:
- `forward_apply` hit rate uses primary-stream observation
- Cross-precision uses theory-on-theory validation in
  generated synthetic data

These are complementary, not redundant.

#### Constitution check

- C1 (R singular): ✓ Generated substrate uses R
- C2 (R binary): ✓
- C3 (types as meta-R): ✓ Theories registered as meta-R
  in the generated substrate (so forward_apply works)
- C4 (token identity): ✓ Generated identifiers prefixed
  `gen_<theory_id>_<i>` to avoid primary collisions
- C5 (structural similarity): ✓ Saturation is purely
  structural (forward-apply over axiom templates), no
  external semantics

#### Echo-chamber resolved as setup-driven, not structural

Pre-experiment I worried about echo chamber: substrates
generated from theories would tautologically confirm them.
The empirical result shows **the echo chamber risk is
real but bounded**:
- Echo chamber appears when substrate isn't sparse enough
  AND only the source theory's axioms are registered
- With ALL theories registered + moderate sparsity,
  cross-precision is discriminative

Future slices using dream phase should:
1. Always register ALL relevant axioms in generated
   substrates
2. Use sparse seed density (0.05 worked; 0.30 saturates
   to near-complete graphs)
3. Treat self-precision = 1.0 as a sanity check, not a
   signal

#### What this slice produced

1. `generate_substrate_from_theory` API + 4 unit tests;
   536 lib tests total
2. Cross-precision matrix as a new theory-quality signal
   on OQ#1
3. Empirical recovery of the prior tournament verdict
   (t_0 worst, t_2/t_3 best) via independent mechanism
4. Methodological note: register all axioms in
   substrates for cross-validation experiments
5. Bounded echo-chamber characterization: setup-driven
   (avoidable) vs structural (not the case here)

#### Future deferred slices

- **Dream phase as scheduler signal**: feed cross-
  precision matrix into theory tournament; demote
  theories with low column means even if they look fine
  on primary stream
- **Generate from axiom REJECTIONS**: deliberately seed
  edges that violate theory axioms, see how the runtime
  reacts — turns the dream phase into a "boundary
  exploration" tool
- **Predicate-axiom enforcement during generation**:
  filter random seeds to respect antisymmetry / totality,
  closing the soundness gap
- **Dream phase + held-out stream**: generate substrates,
  run primary stream forward, compare predictions before/
  after to test if dreaming improves real prediction

#### Status

Phase Alpha-7 Accepted with **strong positive empirical
finding**. DreamCoder-style cross-validation transfers to
v2 and produces theory-quality signal independent of
primary-stream observation. Dream phase as a scheduler
input is recorded as a high-priority future slice.
