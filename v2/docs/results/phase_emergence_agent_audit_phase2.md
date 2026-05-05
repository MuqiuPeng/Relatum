# ADR 0076 phase 2 — Episode-log enrichments

**Status**: ✓ done (2026-05-06)
**Log**: [`logs/2026-05-06_phase_emergence_agent_audit_phase2.log`](../../logs/2026-05-06_phase_emergence_agent_audit_phase2.log)
**Example**: [`examples/phase_emergence_agent_audit_phase2.rs`](../../examples/phase_emergence_agent_audit_phase2.rs)
**Predecessor**: [`phase_emergence_agent_audit.md`](phase_emergence_agent_audit.md) (phase 1)
**ADR**: [0076 — Micro-agent reframing](../decisions/0076-micro-agent-reframing.md)

## Goal

ADR 0076 phase 1 added agent-class summaries (counts, success
rate, temporal envelope, mean delta) — a coarse one-row-per-class
view. Phase 2 adds finer reading along three axes per ADR 0076's
roadmap:

1. **Outcome distribution** — per-class delta histogram
   (negative / zero / positive bucket counts; min / median / max)
2. **Temporal density** — equal-width tick windows showing when
   each class fired, plus the peak-density window
3. **Target overlap** — for id-bearing target types
   (Pattern / Theory / Axiom / etc.), which specific instances
   the class repeatedly acted on

All three are read-only queries over `Memory::episodes`. No new
state, no dispatch changes, no new agent ontology. Path C of
ADR 0076 fully preserved.

## What shipped

### Library (`src/runtime/agent_view.rs`)

Three new query helpers + accompanying public structs:

```rust
pub struct AgentOutcomeDistribution {
    pub episode_count: usize,
    pub negative_count: usize,
    pub zero_count: usize,
    pub positive_count: usize,
    pub min_delta: f64,
    pub max_delta: f64,
    pub mean_delta: f64,
    pub median_delta: f64,
}

pub fn agent_outcome_distribution<'a, I>(
    episodes: I,
    kind: ActionKind,
    target_label: &str,
) -> AgentOutcomeDistribution
where I: IntoIterator<Item = &'a Episode>;

pub struct AgentTemporalDensity {
    pub windows: Vec<(u64, u64, usize)>,
    pub peak_window_idx: Option<usize>,
    pub total_episodes: usize,
}

pub fn agent_temporal_density<'a, I>(
    episodes: I,
    kind: ActionKind,
    target_label: &str,
    n_windows: usize,
    runtime_horizon: u64,
) -> AgentTemporalDensity
where I: IntoIterator<Item = &'a Episode>;

pub struct AgentTargetOverlap {
    pub target_counts: Vec<(String, usize)>,
    pub distinct_targets: usize,
    pub modal_target: Option<String>,
    pub total_episodes: usize,
}

pub fn agent_target_overlap<'a, I>(
    episodes: I,
    kind: ActionKind,
) -> AgentTargetOverlap
where I: IntoIterator<Item = &'a Episode>;
```

### Tests

5 new ADR-0076 tests in `src/tests.rs`:

- `adr0076_outcome_distribution_buckets_by_sign`
- `adr0076_outcome_distribution_empty_when_no_match`
- `adr0076_temporal_density_finds_peak_window`
- `adr0076_temporal_density_empty_with_no_episodes`
- `adr0076_target_overlap_groups_by_specific_id`

Lib tests: 621 → **626**, 0 regressions.

### Example

`phase_emergence_agent_audit_phase2.rs` — runs OQ#1 and OQ#2
through the standard runtime, then prints each substrate's
agent ecosystem with the three new lenses.

## Result highlights

### OQ#1 (1000 ticks, 6 agent classes)

**Outcome distribution** reveals delta signs that the phase 1
mean-only stats hid:

```
agent class                       eps  neg  zer  pos   min Δ   med Δ   max Δ
EvaluatePredictions/WholeRSet      71   62    0    9  -0.487  -0.042   6.000
DiscoverPatterns/PatternSize(2)    18    0   18    0   0.000   0.000   0.000
Declarativize/Axiom                 7    7    0    0  -0.100  -0.100  -0.100
DiscoverTheory/WholeRSet            4    0    0    4   2.300   5.200  11.300
UpdateTheoryRelations/WholeRSet     3    2    0    1  -0.900  -0.600   0.700
Declarativize/Theory                3    3    0    0  -0.100  -0.100  -0.100
```

Reading the table:
- `EvaluatePredictions`: **62 of 71 dispatches were negative**.
  Phase 1's "12.7% success rate" obscured this — it now reads
  as "this agent mostly reports prediction *regressions*, with
  occasional large gains (max +6.0)". The mean +0.057 is misleading;
  the median is -0.042. This class produces noisy negative-delta
  episodes that may be over-counted as productive work.
- `DiscoverPatterns/PatternSize(2)`: **all 18 dispatches at zero
  delta** — uniformly unproductive (Phase 0075 piece 2's known
  limit). The median = mean = max = min = 0.0 confirms pattern
  mining never moves the score.
- `DiscoverTheory`: median +5.2, max +11.3, all 4 positive — by
  far the highest-impact agent class. Its rarity (4 dispatches)
  is the bottleneck: if this class fired more often, the runtime
  would gain abstraction faster.
- `Declarativize/Axiom`: -0.1 every time. This is the structural
  cost of meta-R growth (each declarativization adds one edge,
  abstraction_score subtracts 0.1 per meta edge). Semantically
  the action is fine; arithmetic just looks negative.

**Temporal density** (5 windows of 200 ticks each):

```
class                            total    peak window  peak count
EvaluatePredictions/WholeRSet       71      201-400          18
DiscoverPatterns/PatternSize(2)     18        1-200           9
Declarativize/Axiom                  7        1-200           7
DiscoverTheory/WholeRSet             4        1-200           4
UpdateTheoryRelations/WholeRSet      3        1-200           3
Declarativize/Theory                 3      401-600           3
```

- All discovery / promotion happens in window 1-200 (early
  Phase 0)
- `EvaluatePredictions` peaks 201-400 — after axioms exist,
  prediction-evaluation kicks in
- `Declarativize/Theory` peaks late (401-600) — theories need
  to age before promotion

This temporal layering is exactly what one would expect from a
healthy cognitive substrate: discover → evaluate → promote.

**Target overlap** for `Declarativize`:

```
Declarativize: 10 episodes over 10 distinct targets
  modal: Axiom("ax_antisymmetry")
  top entries: each target appears exactly once
```

Each axiom gets declarativized exactly once. The "modal" target
is whichever id sorts first in case of ties — not informative
here, but the **uniform distribution** is informative: this
agent class is per-instance idempotent (it doesn't repeatedly
re-declarativize the same axiom).

### OQ#2 (4500 ticks, 5 agent classes)

```
agent class                       eps  neg  zer  pos   min Δ   med Δ   max Δ
DiscoverPatterns/PatternSize(2)     5    0    3    2   0.000   0.000   2.000
DiscoverTheory/WholeRSet            2    0    0    2   1.800   2.650   3.500
PruneLowValueObjects/Pattern        1    0    0    1   1.300   1.300   1.300
UpdateTheoryRelations/WholeRSet     1    0    0    1   0.700   0.700   0.700
Declarativize/Axiom                 1    1    0    0  -0.100  -0.100  -0.100
```

- `DiscoverPatterns` is **40% successful** on OQ#2 (2 of 5
  positive), vs 0% on OQ#1. Phase 1 had this number; phase 2
  shows 3 zero + 2 positive — there are no negative dispatches,
  i.e. every pattern mint produces non-zero gain when it
  succeeds at all.
- `DiscoverTheory`: 2 dispatches, both positive, max +3.5 (vs
  OQ#1's max +11.3). OQ#2's theories are smaller because OQ#2
  has only 2 axioms.
- `EvaluatePredictions` again completely absent.

All classes peak in the early window (1-900) — OQ#2's runtime
sleeps quickly because the stream is too sparse to keep
generating work.

## What this view enables

- **Reading delta signs, not just success rates.** The OQ#1
  EvaluatePredictions analysis above wasn't visible at phase 1.
  62 negative / 9 positive paints a different picture from
  "12.7% success rate".
- **Comparing agent class lifecycles across substrates.** OQ#1
  has clear discover→evaluate→promote temporal layering; OQ#2
  has a single early window because the substrate doesn't
  sustain dispatching.
- **Identifying repeat targets vs. one-shot targets.** Most
  current id-bearing targets fire exactly once per id (idempotent
  per-instance), but the helper makes this checkable rather than
  assumed.

## Constitution preservation

Path C of ADR 0076 is preserved without modification:

- No new ontology entities — everything is queries
- No agent state stored — distributions / densities / overlaps
  computed on demand
- No phantom typing — agent classes are still derived from
  `(ActionKind, target-kind)` pairs, target ids in
  `AgentTargetOverlap` are read off existing FrontierTargets
- All three structs are pure data carriers (no methods that
  mutate)

The reframing remains an interpretive layer over data the
runtime already records.

## Files

- `src/runtime/agent_view.rs` — 3 new helpers + 3 new structs
- `src/runtime/mod.rs` — re-exports
- `src/tests.rs` — 5 new tests (621 → 626)
- `examples/phase_emergence_agent_audit_phase2.rs`
- `logs/2026-05-06_phase_emergence_agent_audit_phase2.log`
- This result doc

## Next steps

Per ADR 0076's roadmap:

- **Phase 3** (path B if needed) — deferred indefinitely; this
  slice did not encounter limits that would force ontologization.

ADR 0076 phase 2 is closed. The next priorities (per user 5/6
selection): B (pattern quality framework) and C (ADR 0075
piece 2 deeper scheduler coordination).
