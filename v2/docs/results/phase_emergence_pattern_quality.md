# ADR 0077 — Pattern quality + intervention audit

**Status**: ✓ shipped (2026-05-06)
**Log**: [`logs/2026-05-06_phase_emergence_pattern_quality.log`](../../logs/2026-05-06_phase_emergence_pattern_quality.log)
**Example**: [`examples/phase_emergence_pattern_quality.rs`](../../examples/phase_emergence_pattern_quality.rs)
**ADR**: [0077 — Pattern quality framework + intervention recommendations](../decisions/0077-pattern-quality-and-intervention.md)

## Goal

ADR 0072's `recommend_intervention` works on theories. Patterns
(ADR 0010 / 0029) are now established as v2's
constitution-compliant emergent concepts (ADR 0075 audit). The
missing piece: a parallel `PatternQualityReport` +
`recommend_pattern_intervention` framework so the runtime can
decide which patterns are valuable, redundant, or worth
retracting.

ADR 0077 specified the framework. This is its first audit on
real data.

## What shipped

### Library (`src/lib.rs`)

- `PatternQualityClass` enum (Signal / Mixed / Redundant /
  Anomalous / Indeterminate)
- `PatternQualityReport` struct (canonical_size, role_count,
  instance_count, distinct_participants, mdl_gain,
  cross_substrate_match_count, overlap_score, overlap_partner,
  summary_class)
- `RecommendedPatternIntervention` enum (None / ShadowMonitor /
  PatternRetract / PatternMergeWith / Manual)
- Constants: `REDUNDANT_OVERLAP_FLOOR = 0.8`,
  `SIGNAL_MDL_FLOOR = 5`, `SIGNAL_OVERLAP_CEILING = 0.3`
- `compute_pattern_summary_class` — pure classifier function
- `RSet::pattern_quality_report(pattern_id, &substrates)` —
  single report
- `RSet::pattern_quality_report_all(&substrates)` — all reports
- `RSet::recommend_pattern_intervention(report, others)` — classifier

### Tests

10 new ADR-0077 tests in `src/tests.rs`. Lib tests:
**626 → 636**, 0 regressions.

### Example

`phase_emergence_pattern_quality.rs` — runs the standard
runtime to maturity then manually invokes `autonomous_pass`
sizes 2-5 to populate the pattern set, then audits every
pattern with the new API.

### Cross-substrate validation: sampling integration (2026-05-06 update)

Original ADR 0077 ship deferred cross-substrate validation
because `find_instances_of` is exhaustive `O(data^k)` and hung
on size-4/5 canonicals over imagined substrates of ~100 nodes.

The 2026-05-06 follow-up wired up `sample_instances_of`
(ADR 0024). API change:

```rust
pub fn pattern_quality_report(
    &self,
    pattern_id: &str,
    substrates: &[RSet],
    sampling: Option<&SamplingMatchConfig>,  // NEW
) -> Option<PatternQualityReport>;
```

`Some(cfg)` runs sample-based matching with a bounded budget;
`None` keeps the exhaustive path for callers that need exact
counts. The example uses `sampling_budget = 200` and completes
in seconds.

Empirical impact (matched runtime substrates):
- OQ#1 p_3 (size 3, 25 instances) — `xsub = 5` from 3 imagined
  substrates: this pattern recurs on imagined-substrate
  generations of OQ#1's theories. Confirms p_3 is genuinely
  cross-substrate-portable; class shifted Redundant → Mixed
  due to mdl 72 + overlap 0.75 disagreement
- OQ#2 p_3 (84-instance 3-cycle, mdl 249) — `xsub = 1`,
  remains Signal
- OQ#2 p_1 (size 2, 3 instances) — `xsub = 2`, more
  cross-substrate matches than its primary instance count
  suggests structural-portable; remains Redundant due to high
  overlap with p_3
- Other patterns mostly `xsub = 0`: their canonicals are
  substrate-specific structural shapes, validating the
  `Anomalous` candidate path even though no current pattern
  triggered it (`xsub = 0` AND `instance_count == 1` was the
  Anomalous gate; current minted patterns mostly have
  `instance_count >= 5`)

The Anomalous classification path is now reachable in
principle; it would activate on any future singleton pattern
that fails cross-substrate validation.

## Result

### OQ#1 (7 patterns; size-2 to size-5 mints)

```
id     size role inst  part   mdl  xsub    ovr class
p_0       2    3   30    25    58     —   1.00 Redundant
p_1       2    3   15    25    28     —   1.00 Redundant
p_2       3    2   25    20    72     —   0.75 Mixed
p_3       3    4   10    25    27     —   1.00 Redundant
p_4       4    4   15    25    56     —   1.00 Redundant
p_5       5    3    5    15    20     —   1.00 Redundant
p_6       5    3    5    15    20     —   1.00 Redundant
```

Class distribution: 6 Redundant, 1 Mixed.

Recommendations:
- 6 patterns → `PatternMergeWith` (each Redundant pattern paired
  with a maximally-overlapping peer)
- p_2 → Manual (overlap 0.75 below the 0.8 Redundant floor; mdl
  72 above the Signal floor; mid-range overlap means it doesn't
  pass Signal's 0.3 ceiling either)

The framework correctly identifies that OQ#1's pattern
population is **largely the same 25-token clique re-named
across different canonical sizes**. The diamond-poset stream
produces few distinct participating-token clusters; size 2-5
mints all draw from the same neighborhood. Merging recommendations
are advisory (no `merge_patterns` API yet).

### OQ#2 (7 patterns; same minting protocol)

```
id     size role inst  part   mdl  xsub    ovr class
p_0       2    3    9     6     0     —   1.00 Redundant
p_1       2    3    3     5    14     —   1.00 Redundant
p_2       2    2    1     2     0     —   1.00 Redundant
p_3       3    3   84    30   249     —   0.20 Signal
p_4       3    2   25    20    72     —   0.00 Signal
p_5       3    2   20    25    57     —   1.00 Redundant
p_6       5    3   30    25   145     —   1.00 Redundant
```

Class distribution: 5 Redundant, **2 Signal**.

Recommendations:
- 5 patterns → `PatternMergeWith` peers
- **p_3 → None** (healthy; the 84-instance 3-cycle from the
  kernel audit; **MDL gain 249 — the largest in the entire
  Phase Emergence study**)
- p_4 → None (healthy; completely distinct participants
  overlap = 0.0 with all other patterns)

The framework empirically validates the canonical-form
diversity finding (Phase 0075 piece 3): OQ#2's 84-instance
3-cycle is the highest-quality emergent pattern in v2 to date.
It scores Signal across all three dimensions — high MDL,
non-trivial participation breadth (30 distinct participants),
and a participant set 80% disjoint from any other pattern's.

The second OQ#2 Signal (p_4) is even more distinctive
structurally: 0% overlap with any other pattern, meaning its
20 participating tokens appear in no other minted pattern's
canonical instances.

## What this confirms

1. **The framework correctly grades the previously-identified
   substrate-distinct patterns.** Phase 0075 piece 3 found
   OQ#2's 84-instance 3-cycle was the largest emergent pattern.
   ADR 0077's classifier independently arrives at the same
   verdict via three orthogonal signals (MDL, overlap,
   instance count).

2. **OQ#1 vs OQ#2 substrate divergence shows up at the quality
   layer too.** OQ#1's pattern population is mostly Redundant
   (6 of 7); OQ#2 has two genuine Signals (5 of 7 still
   redundant, but the 2 Signal patterns carry the substrate's
   unique structural information). The Signal/Redundant ratio
   itself is now a substrate-level diagnostic.

3. **The Mixed class is rare and informative.** OQ#1's p_2
   (25 instances, mdl 72, overlap 0.75) doesn't quite cross
   either threshold. It's the kind of "interesting but not
   yet decisive" pattern that benefits from human review,
   exactly where ADR 0077's Manual recommendation routes.

## Caveats

- **Overlap = 1.0 is pervasive on dense rsets.** On OQ#1,
  almost every pair of patterns has 100% participant overlap
  because the diamond-poset substrate has a small
  identifier-cluster total. This is a feature not a bug —
  the framework correctly flags this as redundancy — but it
  may oversaturate Redundant on substrates with limited
  identifier diversity. Future work could add a "redundant
  but valuable" subclass for patterns that overlap on
  participants but differ on canonical form (which IS the
  case for all overlap=1.0 patterns observed here, since
  ADR 0010 guarantees canonical uniqueness).

- **No cross-substrate validation in this slice.** The
  classifier's `Anomalous` class requires
  `cross_substrate_match_count = 0` (singleton + no
  validation), which the current audit cannot trigger because
  no substrates are passed. With cross-substrate validation
  enabled (after `sample_instances_of` integration), some
  current Redundant patterns might shift to Anomalous if they
  lack substrate generality.

- **MDL gain alone is not perfect.** p_2 on OQ#1 has mdl=72
  (above Signal's 5 floor) but overlap 0.75 (above Signal's
  0.3 ceiling), so it's Mixed. This is correct — but the
  MDL value alone might suggest a stronger classification
  than overlap permits. The two-signal gate is the right
  design.

## Files

- `src/lib.rs` — types + 3 RSet methods + 1 helper function
- `src/tests.rs` — 10 new tests (626 → 636)
- `examples/phase_emergence_pattern_quality.rs`
- `logs/2026-05-06_phase_emergence_pattern_quality.log`
- `docs/decisions/0077-pattern-quality-and-intervention.md`
- This result doc

## Verdict

**ADR 0077's pattern quality framework is shipped, tested, and
empirically validated.** It produces structured per-pattern
quality assessments and routing recommendations that align
with prior diagnostic findings (Phase 0075 piece 3's
substrate-diversity observation). The 84-instance 3-cycle on
OQ#2 (the largest emergent pattern in v2) is the framework's
clearest Signal — empirically the highest-quality emergent
concept v2 has ever produced.

The next step from the user's roadmap is C: ADR 0075 piece 2's
deeper scheduler coordination — the partial-fix from 5/6's
session needs the multi-component re-engineering deferred at
that time. With pattern quality now first-class, the scheduler
work has a richer signal to consult: any future runtime
auto-execution of recommendations can read the quality reports.
