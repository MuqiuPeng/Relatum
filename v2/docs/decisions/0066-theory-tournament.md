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
