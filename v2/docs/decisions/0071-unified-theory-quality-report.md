# ADR 0071 — Unified theory-quality report (2026-04-30)

## Status

**Accepted.** Single-step implementation shipped (commit pending
this draft). Lib tests now 576 (was 568); 8 new `adr0071_*`
tests cover Indeterminate / Signal / Noise / Mixed paths, family
memberships, sorted multi-theory reports, and unknown-theory
None return.

Builds on ADR 0070 (shape-family abstraction layer). Like 0070,
ADR 0071 is a **consolidation slice** — it does not introduce
new measurement, only collects the measurements already produced
by Alpha-3++/Alpha-7+/F.1/F.1.1/0049/0070 into one queryable
surface.

## Context

A "theory" in v2 (ADR 0030) is a conjunction of axioms named
under `THEORY_MARKER`. To decide whether a theory is doing well,
the codebase currently consults SEVEN signals from FIVE
different APIs:

| signal | API | introduced | dimension |
|---|---|---|---|
| primary hit rate (per-axiom) | `Memory::prediction_state.hit_rate(ax, min)` | G1.5 / ADR 0059 | observation-fit |
| cross-precision (per-axiom) | `RSet::axiom_cross_precision(ax, substrates)` | F.1 / Alpha-7+ | imagined-substrate fit |
| family quality (per-family) | `RSet::family_quality(id, substrates)` | ADR 0070 / F.1.1 | structural cluster fit |
| family quality class | `FamilyQuality::class()` | ADR 0070 | Signal/Noise/Uniform/Mixed |
| theory neighborhood | `RSet::theory_neighborhood(t)` | ADR 0049 | structural relation to other theories |
| pairwise theory relation | `RSet::classify_theory_pair(a, b)` | ADR 0049 | extends/parallel/independent |
| family-aware noise count | inline in B.2 example | B.2 / Beta-2 | how many noise-family axioms in this theory |

Every example that has needed to "judge a theory" — Alpha-3, Alpha-3+,
Alpha-3++++, Alpha-5, Beta-2, F.2, F.2.1, F.4, F.5 — has rolled its
own combination of these. The user's strategic critique
(2026-04-30) named this directly:

> 系统看到一个坏 theory 时，到底该选择哪种干预？
> 未来不应该继续手动选干预，而应该形成一个 intervention classifier
> ... 这会把 Phase Alpha 从"实验系列"升级成正式的 theory
> maintenance policy.

ADR 0072 will be the classifier (facts → recommended
intervention). ADR 0071 is the prerequisite — it produces the
**facts in a uniform structure** so 0072 has stable input.

The user's framing called this the "Level 1.5" report:

> Level 0: 只作为 example/log 诊断
> Level 1: shadow ranking，和 primary hit rate 并排输出
> **Level 1.5**: 统一报告 — primary + cross + family + neighborhood +
>            recommendation，**不自动执行**
> Level 2: 作为 demote / repair / family-level prune 的实际决策依据

ADR 0071 ships Level 1.5. ADR 0072 will be Level 2.

## Decision

Introduce `TheoryQualityReport` and `RSet::theory_quality_report(...)`
as the canonical theory-evaluation surface. The report aggregates
the seven scattered signals into a single read-only struct.

### 1. The TheoryQualityReport struct

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct TheoryQualityReport {
    /// The theory under evaluation.
    pub theory_id: String,

    /// Member axioms (sorted, ADR 0030 convention).
    pub axiom_ids: Vec<String>,
    pub axiom_count: usize,

    // ── Observation-fit dimension (primary hit rate) ─────────
    /// Mean primary hit rate across axioms with sufficient data.
    /// `None` when no axiom passes `min_predictions`.
    pub primary_rate_mean: Option<f64>,
    /// Min observed primary hit rate.
    pub primary_rate_min: Option<f64>,
    /// Number of axioms that contributed to the primary stats.
    pub primary_rate_qualifying: usize,

    // ── Imagined-substrate-fit dimension (cross-precision) ───
    /// Mean cross-precision across axioms (F.1).
    pub cross_precision_mean: Option<f64>,
    pub cross_precision_min: Option<f64>,
    pub cross_precision_max: Option<f64>,
    pub cross_precision_qualifying: usize,

    // ── Structural-cluster dimension (shape families) ────────
    /// One entry per family that contains at least one of this
    /// theory's axioms. Each entry carries layer/kind/quality
    /// classification (ADR 0070 §4.3).
    pub family_memberships: Vec<TheoryFamilyMembership>,
    /// Count of this theory's axioms that appear in any family
    /// classified as `Noise` or `Uniform` (B.2 signature).
    pub noise_family_axiom_count: usize,
    /// Count of this theory's axioms that appear in any `Signal`
    /// family.
    pub signal_family_axiom_count: usize,

    // ── Structural-relation dimension (neighborhood) ─────────
    /// ADR 0049 — how this theory relates to every other named
    /// theory (subset, parallel, independent, etc.). `None` if
    /// the theory id is not registered.
    pub neighborhood: Option<TheoryNeighborhood>,

    // ── Composite summary ─────────────────────────────────────
    /// Theory-level quality class derived from the dimensions
    /// above. See §3 for the composition rule.
    pub summary_class: TheoryQualityClass,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TheoryFamilyMembership {
    pub family_id: String,
    pub layer: FamilyLayer,
    pub kind: Option<&'static str>,         // KIND_PREMISE_SHARED, etc.
    pub quality: Option<FamilyQuality>,     // None if no substrate data
    pub class: Option<FamilyQualityClass>,
    /// How many of this theory's axioms are in this family.
    pub members_in_theory: usize,
    /// How many total members the family has.
    pub family_total_members: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TheoryQualityClass {
    /// Both observation and imagination signals strongly positive,
    /// no noise-family contamination.
    Signal,
    /// Observation OR imagination signal failing; OR noise-family
    /// contamination present.
    Mixed,
    /// Both signals failing AND/OR theory dominated by noise-family
    /// members.
    Noise,
    /// Insufficient data on multiple dimensions to classify.
    Indeterminate,
}
```

### 2. The lib API

```rust
impl RSet {
    /// ADR 0071 — Build a unified theory-quality report.
    ///
    /// Per-axiom primary hit rates are caller-supplied because
    /// they live in `Memory::prediction_state`, outside RSet.
    /// Pass `&HashMap<String, f64>` mapping axiom ids to their
    /// observed hit rates; axioms missing from the map contribute
    /// `None` to primary stats.
    ///
    /// `substrates` provides the imagined-substrate set for
    /// cross-precision (per F.1). Pass empty slice to skip
    /// cross-precision (the report's cross_precision_* fields
    /// will be `None`).
    ///
    /// Returns `None` if `theory_id` is not a registered theory.
    pub fn theory_quality_report(
        &self,
        theory_id: &str,
        substrates: &[RSet],
        primary_rates: &HashMap<String, f64>,
    ) -> Option<TheoryQualityReport>;

    /// Convenience: build reports for all registered theories.
    /// Sorted by theory id for deterministic output.
    pub fn theory_quality_report_all(
        &self,
        substrates: &[RSet],
        primary_rates: &HashMap<String, f64>,
    ) -> Vec<TheoryQualityReport>;
}
```

### 3. The summary_class composition rule

The four-class summary is derived from the dimensions per the
following rules (precedence top-to-bottom; first match wins):

```text
if primary_rate_mean.is_none() AND cross_precision_mean.is_none()
                                 AND noise_family_axiom_count == 0:
    Indeterminate

if noise_family_axiom_count >= axiom_count / 2 (theory dominated by noise):
    Noise

if cross_precision_mean.unwrap_or(0.0) < 0.50
   AND primary_rate_mean.unwrap_or(0.0) < 0.50:
    Noise

if cross_precision_mean.unwrap_or(0.0) >= 0.80
   AND primary_rate_mean.unwrap_or(0.0) >= 0.80
   AND noise_family_axiom_count == 0:
    Signal

else:
    Mixed
```

These thresholds are inherited from ADR 0070 (FamilyQualityClass)
+ Alpha-3+ (0.50 demote threshold) + F.4 (0.80 signal floor). They
are constants, reviewable in a future ADR. **No threshold tuning
is part of this slice.**

### 4. What 0071 does NOT do

- **Does not specify a recommendation**. The user's "recommended
  intervention，但不自动执行" phrasing is correct in spirit but
  better implemented in ADR 0072 — keeping the recommendation
  algorithm OUT of 0071 keeps 0071 a pure facts surface. 0072
  will introduce `RecommendedIntervention` enum and a function
  `recommend_intervention(report) -> RecommendedIntervention`.
- **Does not auto-trigger anything**. The report is read-only.
  Even when `summary_class == Noise`, no demote / repair / merge
  fires.
- **Does not introduce primary-rate measurement**. Primary rate
  is sourced from caller (`Memory::prediction_state`); 0071
  doesn't redefine it.
- **Does not redefine cross-precision**. F.1's
  `axiom_cross_precision` is the source; 0071 only aggregates.
- **Does not change ADR 0070's family-level computation**. Family
  quality comes from `family_quality()`; 0071 attaches it to the
  theory as one of the membership entries.
- **Does not standardize the substrate set**. Caller decides
  what substrates feed cross-precision. Conventionally per-theory
  generated substrates (DreamCoder-style); ADR 0071 does not
  enforce this.

### 5. Constitutional review

| commitment | status | argument |
|---|---|---|
| C1 R singular | PASS | Report is a Rust data type; no R class introduced |
| C2 R binary | PASS | No relations introduced |
| C3 types as meta-R | NEUTRAL | Report is a runtime data type, NOT a meta-R class. This is correct: the report is *evaluation infrastructure*, not the *runtime ontology*. Future ADRs could promote `TheoryQualityReport` to meta-R if a use case demands it (analog of how H1 promoted action sequences). For now, runtime-only is honest. |
| C4 token identity | PASS | All theory / axiom / family ids in the report are byte-equal references to existing meta-R tokens |
| C5 similarity is structural | PASS | All quality dimensions are derived from rset structure + observation memory; no external metric |

### 6. Honest note on signal coverage

Two signals listed in §1 NOT YET in the proposed struct:
- **Theory-level cross-precision aggregate** (Alpha-7+'s
  per-substrate column means). The proposal aggregates per-axiom
  cross-precision into the theory's `cross_precision_*` fields.
  This is ALMOST the same as Alpha-7's column mean, but not
  identical (axiom-mean vs substrate-mean). Document the
  divergence: 0071 uses axiom-mean; if substrate-mean is needed
  in the future, add a separate field.
- **Pairwise theory similarity / Jaccard** (Alpha-5 / F.4). NOT
  in 0071. This is binary (per-pair), not unary (per-theory).
  ADR 0072 will need pairwise data when it considers merge
  candidacy; 0071 stays per-theory.

These are documented so a future ADR doesn't have to rediscover
them.

## Alternatives considered

### A. Recommendation field included in the report

User's original framing put "recommended intervention" as one of
the report fields. Clean for callers (one struct, full picture).
**Rejected for 0071** because it would couple 0071 to 0072's
classifier logic — 0072 would have to either supersede 0071 or
mutate the same struct.

The compromise: 0071 ships facts; 0072 ships
`recommend_intervention(&TheoryQualityReport) -> RecommendedIntervention`
as a separate function. Callers that want both build a tuple
`(report, recommend(&report))`. Same ergonomics, cleaner ADR
boundary.

### B. Method on `AutonomousRuntime` instead of `RSet`

The runtime owns both rset and memory; a single method there
could pull primary rates internally. **Rejected** because it
would couple `RSet` to runtime state. Keeping the method on
`RSet` with caller-supplied primary rates is constitutionally
cleaner — `RSet` has no opinion on Memory.

A wrapper method on `AutonomousRuntime` is fine to add later as a
convenience.

### C. Builder pattern (`TheoryQualityReportBuilder`)

Allows incremental construction (start with axiom list, add
substrates, add primary rates, build). **Rejected** as overkill
for the call sites we have. Plain function + struct is easier to
follow.

### D. Separate methods per dimension

`primary_stats(t)`, `cross_stats(t, subs)`, `family_memberships(t)`,
`neighborhood(t)`. Caller assembles. **Rejected** because that's
EXACTLY the status quo (signals scattered) — the entire point of
0071 is to ship one assembled output.

(`theory_neighborhood` already exists and is preserved; it's just
ALSO included in the unified report.)

## Consequences

### What becomes easy

- Every callsite that needs theory quality calls one method. The
  9 examples that currently roll their own combinations can
  migrate to `theory_quality_report` and reduce ~40 lines each
  to ~5.
- ADR 0072's recommendation classifier consumes a single struct
  rather than juggling 5 APIs.
- Future logging / dashboarding has a stable schema.
- Cross-substrate testing (I.1, I.2): the report can be computed
  against multiple substrate sets and the results compared
  apples-to-apples.

### What becomes harder

- Adding a NEW quality dimension (e.g., counterfactual value from
  ADR 0035, or theory age) is now ADR-gated: a follow-up ADR has
  to add the field with documented thresholds. This is the right
  friction.
- Changing the `summary_class` rule is a behavior change; the
  rule lives in 0071's text and lib code, so any change requires
  ADR amendment + migration.

### Deferred

- **Recommendation logic** → ADR 0072.
- **Pairwise theory quality** (for merge candidates) → ADR 0072
  or its successor.
- **Primary-rate-source-as-parameter trait** (allowing prediction
  state to be replaced by alternative observation backends).
  Currently `&HashMap<String, f64>` is sufficient.
- **`TheoryQualityReport` as meta-R** (commitment-3 promotion).
  Not motivated yet.

## Implementation

This ADR ships in **one step** — unlike 0070's three-phase
rollout, 0071 is purely additive and small enough to land
together.

### Lib changes

- `TheoryQualityReport`, `TheoryFamilyMembership`,
  `TheoryQualityClass` types in `lib.rs`
- `RSet::theory_quality_report(theory_id, substrates, primary_rates)`
- `RSet::theory_quality_report_all(substrates, primary_rates)`
- `summary_class` composition rule per §3 implemented as a
  private helper `compute_summary_class(...)` consumed by the
  builder

### Test changes

- 4-6 unit tests:
  - Indeterminate case (no data)
  - Signal case (high primary + cross + no noise)
  - Noise case (low both + noise families)
  - Mixed case (one dim high, one low)
  - Family memberships listed correctly
  - Unknown theory returns None

### No example migration in 0071

Migrating the 9 callsites currently rolling their own quality
logic is a separate cleanup PR — it's not gated on 0071's
correctness. They'll migrate when 0072 lands the classifier
(natural pairing point).

### Verification

- 568 lib tests pass + 4-6 new = ~572-574 total
- No behavior change in any existing example
- New `TheoryQualityReport` type round-trip (when persistence is
  needed, deferred — currently runtime-only)

## Touched ADRs

- **ADR 0030** (theory objects) — 0071 consumes theory_axioms,
  is_theory
- **ADR 0049** (theory neighborhood) — 0071 includes neighborhood
  as a field
- **ADR 0059** (prediction-error drive) — caller supplies primary
  rates from `Memory::prediction_state`, sourced from G1.5's
  per-axiom hit rate machinery
- **ADR 0066** (theory tournament) — Alpha-7+'s cross-precision
  is the cross-precision dimension's source via F.1
- **ADR 0070** (shape-family layer) — 0071 reads
  `family_quality`, `family_layer`, `family_kind`,
  `FamilyQualityClass`

## Future ADRs gated on this one

- **ADR 0072** — Intervention policy classifier. Consumes
  `TheoryQualityReport` and a pairwise quality view; produces
  `RecommendedIntervention` enum.

## Status

Proposed. Awaiting implementation.

---

*Author's note: ADR 0071 is the second consolidation ADR after
0070. The pattern is the same — the layer's facts are ALREADY
in the codebase; we're naming them as a layer. With 0071 landed,
the "what's the quality of this theory?" question has one stable
answer instead of seven scattered ones.*
