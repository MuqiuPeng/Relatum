# 0077: Pattern quality framework + intervention recommendations

Status: Proposed
Date: 2026-05-06

Parents:
- [0071 — Unified theory-quality report](0071-unified-theory-quality-report.md)
- [0072 — Intervention policy classifier](0072-intervention-policy-classifier.md)
- [0075 — Emergence kernel audit](0075-emergence-kernel-audit-and-runtime-integration.md)
- [0076 — Micro-agent reframing](0076-micro-agent-reframing.md)

## Context

ADR 0072 introduced the `TheoryQualityReport` + `recommend_
intervention` pair: a single classifier that, given a theory's
quality profile, recommends a specific structural intervention
(retract / merge / family-demote / etc.). It works on theories
and shape families.

Patterns (ADR 0010 / 0029) are now established as v2's
constitution-compliant emergent concepts (ADR 0075 audit). The
kernel audit + canonical-form-diversity slices showed patterns
have substrate-distinct behaviour and per-pattern statistics
(instance count, distinct participants) that the runtime already
records.

What's missing: a quality framework parallel to 0071/0072 that
operates on patterns. Without it, the runtime can mint patterns
but cannot decide which ones are valuable, redundant, or worth
retracting. The micro-agent audit (ADR 0076) hinted at this —
`PruneLowValueObjects` agents already exist but they target
patterns based on *counterfactual value*, a coarser signal than
the structured quality reports 0071 introduced for theories.

This ADR ships the missing layer: `PatternQualityReport` +
`recommend_pattern_intervention`, mirroring 0071 + 0072 in shape
but specialized to the pattern-extension semantics (instances,
participants, canonical structure).

## Decision

Define five types parallel to ADR 0071/0072:

```rust
pub enum PatternQualityClass {
    Signal,        // valuable, distinct, predictively useful
    Mixed,         // medium-quality; partial signals
    Redundant,     // overlaps heavily with another pattern
    Anomalous,     // singleton with no cross-substrate support
    Indeterminate, // not enough data
}

pub struct PatternQualityReport {
    pub pattern_id: String,
    pub canonical_size: usize,          // # edges in intension
    pub role_count: usize,              // # roles
    pub instance_count: usize,
    pub distinct_participants: usize,
    pub mdl_gain: usize,                // (n - 1) * k
    pub cross_substrate_match_count: Option<usize>,  // sum across substrates if any
    pub overlap_score: f64,             // [0, 1] — max participant-overlap with any other pattern
    pub overlap_partner: Option<String>, // pattern id with max overlap
    pub summary_class: PatternQualityClass,
}

pub enum RecommendedPatternIntervention {
    None,
    PatternRetract { reason: String },
    PatternMergeWith { partner: String, reason: String },
    Manual { reason: String },
    ShadowMonitor { reason: String },
}
```

### Quality classification rules

The classifier inspects four orthogonal signals:

1. **MDL gain** — `(n - 1) × k` where `n` = instance count, `k`
   = canonical size (number of edges). Reflects how much the
   pattern compresses the rset by being named.
2. **Overlap** — for every other pattern, `|participants(P) ∩
   participants(Q)| / |participants(P)|`. Take the max as
   `overlap_score`. ≥ 0.8 is treated as "redundant with Q".
3. **Cross-substrate match count** — when caller provides an
   `&[RSet]` of generated substrates (analog of ADR 0071's
   imagined-substrate validation), count `find_instances_of(canonical)`
   across each substrate. Zero matches with `instance_count = 1`
   marks the pattern as anomalous.
4. **Instance count** — primary measure of structural recurrence.

Decision tree (sequential; first match wins):

```text
if instance_count == 0:
    Indeterminate
elif overlap_score >= REDUNDANT_OVERLAP_FLOOR (0.8):
    Redundant
elif instance_count == 1
     AND cross_substrate_match_count.unwrap_or(0) == 0:
    Anomalous
elif mdl_gain >= SIGNAL_MDL_FLOOR (5)
     AND overlap_score < SIGNAL_OVERLAP_CEILING (0.3):
    Signal
else:
    Mixed
```

Initial threshold values:
- `REDUNDANT_OVERLAP_FLOOR = 0.8` — high enough that a
  near-subsumed pattern is flagged but a partial overlap isn't
- `SIGNAL_MDL_FLOOR = 5` — `(n-1)*k ≥ 5` covers e.g. 6 instances
  of a 1-edge pattern, 4 instances of a 2-edge pattern, 2
  instances of a 5-edge pattern; small enough that real signals
  pass, large enough to filter trivial 1-edge motifs
- `SIGNAL_OVERLAP_CEILING = 0.3` — Signal patterns must have
  reasonably distinct participant sets

These mirror ADR 0072's tuning style: hand-picked initial values
backed by the rationale above; empirically validated by the
threshold-scan example pattern (analog of phase 0072-B).

### Recommendation rules

```text
match summary_class:
  Signal:       None
  Indeterminate: ShadowMonitor("no instance evidence yet")
  Anomalous:    PatternRetract("singleton with no cross-substrate match")
  Redundant:    PatternMergeWith {
                  partner: overlap_partner.unwrap(),
                  reason: "overlap_score >= 0.8 with partner",
                }
  Mixed:        Manual("instance/overlap/MDL signals disagree")
```

Conservative-by-default: only `Anomalous` triggers automatic
retract; `Redundant` recommends merge but execution stays user-
gated (since v2 has no `merge_patterns` API yet — that's a
separate ADR if/when needed).

### Existing intervention reuse

`PatternRetract` calls existing `RSet::retract_pattern` (ADR
0010). No new retraction mechanism. `PatternMergeWith` is
**aspirational** in this slice — no implementation, the
recommendation surfaces but the runtime doesn't auto-execute it
(matches ADR 0072's earliest behavior where Merge was advisory
before `merge_theories` was implemented).

### What this does NOT do

- No new dispatch action (no `PruneLowValueObjects`-style
  scheduler integration in this slice; that follows once the
  framework's recommendations have empirical backing)
- No `merge_patterns` API
- No automatic application of recommendations
- No new ontology entities; all data comes from existing
  `RSet::patterns()`, `instances_of()`, `participants_of()`,
  `pattern_structure()`, `mdl_gain()`

The framework is observational + advisory only.

## Alternatives considered

**Alt A — Reuse `TheoryQualityReport`** by treating each pattern
as a "theory of one axiom". Rejected: theories are unions of
axioms (extensional logic objects), patterns are subgraph
structural classes (intensional structural objects). Their
quality semantics differ — a "Signal theory" predicts well, a
"Signal pattern" recurs distinctively. Forcing the same struct
on both would obscure both meanings.

**Alt B — Skip the framework; rely on counterfactual value
(existing PruneLowValueObjects)**. Rejected: counterfactual
value is a single scalar with no structural breakdown. The
distinction between "this pattern is anomalous" vs "this
pattern is redundant with that one" is exactly what the user
needs to know to decide which intervention to apply. A scalar
hides this.

**Alt C — Build cross-precision-style validation only (skip
the full 0071/0072 mirror)**. Rejected: the per-pattern
quality breakdown (MDL gain, overlap, cross-substrate matches,
instance count) is more useful for diagnostic / audit purposes
than a single cross-precision number. The 0071/0072 pattern is
that the *report* is the persistent product, the
*recommendation* is the action surface. Mirror both.

## Consequences

**Now possible:**

- Audit each pattern's structural quality without resorting to
  counterfactual-value-based pruning
- Identify redundant pattern pairs (pattern X overlaps 90% with
  pattern Y) as candidates for merging or retraction
- Mark singleton-no-recurrence patterns (`Anomalous`) for
  cleanup
- Surface a unified surface for the runtime to read pattern
  status — analog to `theory_quality_report_all` for theories

**Now harder:**

- Integration with the runtime scheduler (deferred): a
  `RecommendPatternIntervention` action would need to be added
  to consume the report. That's a follow-up.
- Pattern merging (deferred indefinitely): even if the
  framework recommends merging two patterns, v2 lacks a
  `merge_patterns` mechanism analogous to `merge_theories`.
  The recommendation can be surfaced for user inspection but
  not auto-executed.

**Newly easy:**

- Audit-driven decisions about pattern population health: how
  many Signal vs Mixed vs Anomalous patterns does each
  substrate produce?
- Cross-substrate transfer claims: a pattern that scores
  `Signal` on OQ#1 but produces zero `cross_substrate_match`
  on OQ#2's substrates is a clade-bound pattern, not a
  universal one. Useful for the diversity-analysis line of
  work.

## Implementation sketch

New types in `lib.rs` (parallel to `TheoryQualityReport`):
- `PatternQualityClass` enum
- `PatternQualityReport` struct
- `RecommendedPatternIntervention` enum
- Constants: `REDUNDANT_OVERLAP_FLOOR`, `SIGNAL_MDL_FLOOR`,
  `SIGNAL_OVERLAP_CEILING`

New methods on `RSet`:
- `pattern_quality_report(pattern_id, substrates)` — single pattern
- `pattern_quality_report_all(substrates)` — all patterns
- `recommend_pattern_intervention(report, others)` — classifier

Tests (~6-8 unit):
- empty rset → Indeterminate
- 1-instance + no cross-substrate → Anomalous
- high MDL + low overlap → Signal
- high overlap → Redundant
- mid-range → Mixed
- recommendation-classifier tests for each branch
- overlap calculation correctness

Example: `phase_emergence_pattern_quality.rs` runs OQ#1 / OQ#2
through the standard runtime + manual `autonomous_pass` to mint
patterns, then audits each pattern's quality + recommended
intervention. Should reveal which OQ#2-only patterns are Signal
(per ADR 0075 piece 3 finding) vs which are Mixed/Anomalous.

Shipping target:
- ~250 lines lib + ~150 lines tests + 1 example
- Lib tests: 626 → ~633
- 0 regressions

## Open questions

- **Overlap denominator**: divide by `|participants(P)|` (own
  size) or by `|participants(P) ∪ participants(Q)|` (Jaccard)?
  ADR 0072 Addendum 2 used Jaccard for theory near-disjoint
  detection. For pattern overlap, asymmetric "P contained in Q"
  is likely more useful (a small pattern wholly inside a larger
  one's participant set is the redundancy case worth flagging).
  Initial implementation: divide by `|participants(P)|`. Revisit
  if empirics suggest Jaccard is better.
- **Cross-substrate handling for absent substrates**:
  `cross_substrate_match_count: Option<usize>` is `None` when
  caller passes empty `&[RSet]`. The classifier treats `None`
  as "not validated" — does NOT downgrade to `Anomalous` on its
  own. This matches ADR 0071's `Indeterminate` philosophy: data
  absence is not signal absence.
- **MDL floor calibration**: `5` is hand-picked. A
  threshold-scan analog of ADR 0072-B might tune it. Deferred.

These are decided in implementation; the deferral matches
ADR 0072's pattern (ship initial values, scan later when there's
enough data).

## Implementation

Pending. Initial implementation in next commit.
