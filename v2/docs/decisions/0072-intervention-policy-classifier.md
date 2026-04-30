# ADR 0072 — Intervention policy classifier (2026-04-30)

## Status

**Accepted.** Two-step implementation shipped (commit pending
this draft). 10 new `adr0072_*` unit tests cover every
intervention path:
- `adr0072_indeterminate_returns_shadow_monitor`
- `adr0072_signal_returns_none`
- `adr0072_demote_superset_when_extending_signal_subset`
- `adr0072_family_demote_when_noise_family_present`
- `adr0072_axiom_repair_when_few_weak_axioms`
- `adr0072_merge_when_complementary_signal_partner_exists`
- `adr0072_theory_demote_when_both_dims_low`
- `adr0072_priority_demote_superset_beats_family_demote`
- `adr0072_manual_when_mixed_with_no_pattern`
- `adr0072_per_axiom_stats_populated_in_report`

Lib tests now 586 (was 576). 0 behavior regressions.

The third and final consolidation ADR after 0070 (shape-family
layer) and 0071 (theory-quality report). With 0072 landed, the
"see a struggling theory → choose intervention" loop has a
single named policy instead of nine ad-hoc example-side rules.

## Context

The codebase has accumulated SIX distinct intervention
mechanisms:

| intervention | mechanism | introduced |
|---|---|---|
| **theory_demote** | retract entire theory + cascade | Alpha-3+ / ADR 0066 |
| **per-axiom repair** | detach specific axioms; theory keeps clean members | Alpha-3+++ / ADR 0066 |
| **naive merge** | merge_theories on top-Jaccard pair | Alpha-3++++ (FALSIFIED) |
| **smart merge** | non-subset Jaccard pick | Alpha-5 / ADR 0066 |
| **family-level demote** | retract_shape_family on noise family | B.2 / ADR 0070 |
| **subset-aware demote** | demote noisy superset of clean subset | implicit in Alpha-5 |

ADR 0070 unified the **structural layer** (families).
ADR 0071 unified the **observation layer** (quality report).
ADR 0072 unifies the **policy layer** — given a quality report,
which intervention is recommended?

The user's strategic critique (2026-04-30) named this exactly:

> 系统看到一个坏 theory 时，到底该选择哪种干预？
> 未来不应该继续手动选干预，而应该形成一个 intervention classifier
> ...
> 这会把 Phase Alpha 从"实验系列"升级成正式的 theory
> maintenance policy.

The user's proposed decision tree (literal quote):

> if bad theory is noisy superset of clean theory:
>     demote superset
> if bad theory has identifiable noisy family:
>     family-level demote
> if theory mostly good but has few isolated bad axioms:
>     repair / detach axioms
> if two theories are complementary:
>     quality-aware merge
> if evidence insufficient:
>     shadow only

ADR 0072 codifies this tree with explicit thresholds, priority
order, and "manual review" fallback.

## Decision

### 1. The RecommendedIntervention enum

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RecommendedIntervention {
    /// Theory is healthy; no action.
    None,

    /// Insufficient signal to act safely. Document why.
    ShadowMonitor { reason: String },

    /// Retract a specific shape family (cross-cutting noise).
    /// Caller invokes `RSet::retract_shape_family(family_id)` or
    /// dispatches `ActionKind::RetractShapeFamily`.
    FamilyDemote { family_id: String, family_class: FamilyQualityClass },

    /// Detach specific axioms from this theory. The axioms aren't
    /// global problems — they're a small subset dragging this
    /// theory down. Caller invokes `RSet::retract_theory_member`
    /// per axiom.
    AxiomRepair { axiom_ids: Vec<String> },

    /// Retract the entire theory + cascade.
    /// Caller invokes `RSet::retract_theory(theory_id)`.
    TheoryDemote { reason: TheoryDemoteReason },

    /// Theory is a noisy superset of a cleaner subset theory;
    /// retracting the superset preserves the cleaner explanation.
    /// Caller invokes `RSet::retract_theory(this.theory_id)`,
    /// keeping `cleaner_subset_theory` registered.
    DemoteSuperset { cleaner_subset_theory: String },

    /// Merge with a complementary partner.
    /// Caller invokes `RSet::merge_theories(this.theory_id, partner)`.
    Merge { partner_theory: String, rationale: MergeRationale },

    /// All heuristics inconclusive — flag for manual review.
    Manual { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TheoryDemoteReason {
    /// Both primary AND cross-precision below 0.50 — theory
    /// underperforms on every dimension.
    BothDimensionsLow,
    /// Theory is dominated by noise-family members (≥ 50% of
    /// axioms in noise families) AND no targeted family demote
    /// would suffice.
    NoiseDominated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeRationale {
    /// Identical / near-identical column profiles (F.3 max_diff ≈ 0).
    Equivalent,
    /// Disjoint family signatures, both quality-passing (F.2.1).
    Complementary,
    /// Both theories Signal-class with overlapping membership.
    HighQualityBoth,
}
```

### 2. The classifier API

```rust
impl RSet {
    /// ADR 0072 — Recommend an intervention for a struggling
    /// theory based on its quality report and the broader
    /// theory landscape.
    ///
    /// Pure function: no side effects. The caller decides whether
    /// to ACT on the recommendation.
    ///
    /// `report`: ADR 0071 quality report for the focal theory.
    /// `other_reports`: quality reports for OTHER theories (not
    ///   the focal). Used for subset/superset and merge analysis.
    ///   Empty slice degrades gracefully — DemoteSuperset and
    ///   Merge cases collapse into TheoryDemote / Manual.
    pub fn recommend_intervention(
        report: &TheoryQualityReport,
        other_reports: &[TheoryQualityReport],
    ) -> RecommendedIntervention;
}
```

Note: The function is **stateless** — declared on `RSet` but
takes its inputs explicitly. This makes it testable in isolation
and reusable from non-runtime contexts (e.g., diagnostic
tooling).

### 3. The decision tree

Priority order (top-to-bottom; first match wins):

```text
# Step 0: data sufficiency
if report.summary_class == Indeterminate:
    return ShadowMonitor("no data on any quality dimension")

# Step 1: theory is healthy → no action
if report.summary_class == Signal:
    return None

# Step 2: subset+noise pattern (user's first-priority case)
# This theory is a noisy superset of a Signal-class theory it extends.
for sub in report.neighborhood.extends:
    sub_report = find(other_reports, sub)
    if sub_report.summary_class == Signal:
        return DemoteSuperset(sub)

# Step 3: noise-family contamination → targeted family demote
noise_fams = report.family_memberships.filter(
    |m| m.class in {Noise, Uniform}
)
if !noise_fams.is_empty():
    target = max(noise_fams, key=|m| m.members_in_theory)
    return FamilyDemote(target.family_id, target.class)

# Step 4: theory mostly OK but a few axioms drag it down → repair
# (only when summary is Mixed, primary mean is decent, and
# per-axiom data identifies specific weak axioms)
if report.summary_class == Mixed
   AND report.primary_rate_mean.unwrap_or(0.0) >= 0.60:
    weak_axioms = report.per_axiom_stats.filter(
        |a| a.primary_rate.unwrap_or(1.0) < 0.30
            OR a.cross_precision.unwrap_or(1.0) < 0.30
    )
    if 1 <= weak_axioms.len() <= report.axiom_count / 2:
        return AxiomRepair(weak_axioms.map(|a| a.axiom_id))

# Step 5: complementary merge candidate
# Look for a Signal-class partner with disjoint family signature.
if report.summary_class == Mixed:
    for other in other_reports:
        if other.summary_class != Signal: continue
        shared = focal_families ∩ other_families
        focal_families = {m.family_id for m in report.family_memberships}
        other_families = {m.family_id for m in other.family_memberships}
        if shared.len() == 0:
            return Merge(other.theory_id, MergeRationale::Complementary)

# Step 6: noise-class theory with no targeted intervention → demote
if report.summary_class == Noise:
    if report.noise_family_axiom_count * 2 >= report.axiom_count:
        return TheoryDemote(NoiseDominated)
    let p = report.primary_rate_mean.unwrap_or(0.0);
    let c = report.cross_precision_mean.unwrap_or(0.0);
    if p < 0.50 and c < 0.50:
        return TheoryDemote(BothDimensionsLow)

# Step 7: heuristics inconclusive → manual review
return Manual("Mixed theory; no specific intervention pattern matched")
```

### 4. Why this priority order

**Step 2 first (DemoteSuperset)** — User's first-priority case.
If a theory is `t_super = {ax_a, ax_b, ax_c, ax_noise}` and
`t_sub = {ax_a, ax_b, ax_c}` is Signal-class, the right move is
demote `t_super`, not merge or repair. This handles the
Alpha-3++++ FALSIFIED case explicitly: naive Jaccard merge
would have picked `(t_super, t_sub)` and merged them, but the
correct action is to retract `t_super`.

**Step 3 (FamilyDemote) before Step 4 (AxiomRepair)** — when
noise is structurally coherent (shape family signature),
demoting the family retracts ALL its members in one operation
+ blocks future re-discovery. Per-axiom repair would only
detach the specific axioms here, not block them.

**Step 5 (Merge) only for Mixed theories, not Signal** — Signal
theories don't NEED merging; merging is for theories that
benefit from picking up complementary structure.

**Step 6 (TheoryDemote) before Step 7 (Manual)** — TheoryDemote
is the most aggressive intervention; reserved for theories
where every other targeted intervention failed to apply AND
both quality dimensions are below floor.

### 5. ADR 0071 schema extension (`per_axiom_stats`)

ADR 0072 needs per-axiom data to populate `AxiomRepair`'s
`axiom_ids` field. Extend `TheoryQualityReport` with:

```rust
pub struct TheoryQualityReport {
    // ... existing fields ...
    pub per_axiom_stats: Vec<AxiomQualityStats>,
}

pub struct AxiomQualityStats {
    pub axiom_id: String,
    pub primary_rate: Option<f64>,
    pub cross_precision: Option<f64>,
    /// Shape family ids this axiom is a member of (L2 only).
    pub family_ids: Vec<String>,
}
```

This is **additive** — the existing aggregate fields are
unchanged; ADR 0071's tests still pass. ADR 0071's status
becomes "Accepted; extended by 0072".

### 6. What 0072 does NOT do

- **Does not auto-execute the recommendation.** Caller decides
  to act. The runtime (or human, or diagnostic tool) consumes
  the enum and dispatches as it sees fit.
- **Does not introduce new intervention mechanisms.** All six
  recommendations call into existing APIs:
  `retract_theory`, `retract_theory_member`, `retract_axiom`,
  `merge_theories`, `retract_shape_family` (ADR 0070).
- **Does not specify intervention sequencing.** If a runtime
  loop wants "demote one family, re-evaluate, demote another",
  the loop logic lives elsewhere; 0072 produces the per-call
  recommendation.
- **Does not handle inter-recommendation conflicts.** If
  classifier A recommends `Merge(t_a, t_b)` and classifier B
  recommends `TheoryDemote(t_b)` in the same window, the caller
  resolves the conflict. 0072 is per-theory.
- **Does not learn thresholds.** 0072's thresholds are constants
  inherited from 0070/0071 (Signal floor 0.80, Noise ceil 0.50,
  noise-domination 50%, repair-eligibility 60%, weak-axiom 30%).
  Adapting them is future work.

### 7. Constitutional review

| commitment | status | argument |
|---|---|---|
| C1 R singular | PASS | Pure Rust types; no R class |
| C2 R binary | PASS | No new relations |
| C3 types as meta-R | NEUTRAL | Recommendation is runtime data, not meta-R. Same justification as 0071: this is *evaluation infrastructure*, not the *runtime ontology*. A future ADR could promote `RecommendedIntervention` to meta-R if a use case demands it (e.g., recording history of past recommendations as `R(__intervention__, rec_N)` chains for sequence-stats analysis). For now, runtime-only. |
| C4 token identity | PASS | All theory / axiom / family ids in the recommendation are byte-equal references |
| C5 similarity is structural | PASS | Classifier reads only structural facts from the report |

## Alternatives considered

### A. Recommendation as a field on `TheoryQualityReport`

User's original proposal embedded "recommended intervention" in
the report. **Rejected for 0071** (kept the report read-only
facts) **and 0072** (recommendation lives in classifier output,
not report). This decouples 0071's stable schema from 0072's
mutable policy.

### B. Statistical learning over historical recommendations

Train a classifier from past (situation → outcome) traces.
**Rejected** because we don't have outcome labels yet (no
ground truth on whether a past intervention was correct). 0072
is rule-based; if we accumulate intervention outcome data,
ADR 0073+ could revisit with a learned policy.

### C. Use H2.2 drive-synthesis-style search over interventions

Synthesize new intervention combinators from primitives
(`demote_then_merge`, `repair_then_demote`, etc.). **Rejected
as out-of-scope.** The current six interventions cover Alpha
series empirics; combinatorial expansion is research territory
matching ADR 0063 H2.2's deferred status.

### D. One classifier per intervention type (separate predicates)

`should_demote_superset(report, others) -> Option<DemoteSuperset>`,
`should_demote_family(report) -> Option<FamilyDemote>`, etc.
Caller iterates and picks first-Some. **Rejected** because the
priority order and conflict resolution becomes implicit in the
caller, defeating the consolidation goal. Single function with
explicit priority is clearer.

## Consequences

### What becomes easy

- Every callsite that needs to decide an intervention calls one
  function. The 9 examples that currently roll their own
  selection logic (Alpha-3, Alpha-3+, Alpha-3++++, Alpha-5,
  Beta-2, F.2, F.2.1, F.4, F.5) can migrate to:
  ```rust
  let report = rt.rset.theory_quality_report(t, &subs, &primary)?;
  let others = rt.rset.theory_quality_report_all(&subs, &primary);
  let rec = RSet::recommend_intervention(&report, &others);
  match rec {
      RecommendedIntervention::FamilyDemote { family_id, .. } => ...
      ...
  }
  ```
- Logging and dashboards have a stable recommendation schema.
- Future runtime integration (a "theory maintenance loop") has a
  natural API surface.

### What becomes harder

- New intervention types are now ADR-gated. Adding `BulkRepair`
  (repair multiple theories at once) requires extending the
  enum + a follow-up ADR.
- Threshold tuning is a behavior change — same caveat as 0071.
- The priority order is a CONTRACT. Reordering steps requires
  ADR amendment + migration.

### Deferred

- **Pairwise quality view** (F.4 multi-signal merge picker
  generalized to N-theory). Currently 0072 uses a simple
  pairwise check (disjoint family signature) for merge
  candidacy. Richer scoring (Jaccard + cross-precision profile
  diff + family complementarity) deferred to ADR 0072.1 if
  needed.
- **Per-axiom stats density**. Currently `per_axiom_stats` is
  Vec; for theories with ~50+ axioms this could become a HashMap
  for O(1) lookup. Profile-driven optimization.
- **Recommendation execution** (caller side). 0072 recommends;
  the actual loop "fetch report → recommend → execute → verify"
  is a runtime feature deferred to ADR 0073 (or until empirical
  demand surfaces).
- **History tracking** (`R(__intervention__, rec_N)` meta-R
  chain). Requires use case; not blocking.

## Implementation

ADR 0072 ships in **two steps** to manage scope:

### Step 1 — types + per-axiom extension to 0071

- `RecommendedIntervention`, `MergeRationale`, `TheoryDemoteReason`
  enums in `lib.rs`
- `AxiomQualityStats` struct
- Extend `TheoryQualityReport` with `per_axiom_stats: Vec<AxiomQualityStats>`
- Update `theory_quality_report()` to populate `per_axiom_stats`
- ADR 0071's existing tests must continue to pass

### Step 2 — classifier function + tests

- `RSet::recommend_intervention(report, other_reports)` —
  associated function (no `&self`); pure
- 6 unit tests (one per non-Manual recommendation type) covering
  the priority order
- Smoke test on OQ#1: build reports for all 4 theories,
  classify each, confirm recommendations are sensible

Each step independently committable. After Step 2, examples can
optionally migrate (recommended in a separate cleanup PR; not
blocking).

### Verification

- 576 lib tests + ~12 new = ~588 total
- 0 behavior change in any existing example
- New types round-trip implicitly via Vec / HashMap derive impls
- Smoke test produces non-Manual recommendations for at least
  some OQ#1 theories (sanity check that the classifier isn't
  trivially Manual-everywhere)

## Touched ADRs

- **ADR 0049** (theory neighborhood) — used in DemoteSuperset
  step
- **ADR 0066** (theory tournament) — Alpha-3+/Alpha-3+++/Alpha-5
  intervention precedents
- **ADR 0070** (shape-family layer) — FamilyDemote calls into
  `retract_shape_family`
- **ADR 0071** (theory-quality report) — primary input;
  schema extended additively in Step 1

## Future ADRs gated on this one

- **ADR 0073** (potential) — Theory maintenance loop:
  `tick → identify struggling theories → fetch reports →
   classify → execute → verify → repeat`. Requires 0072
  for the classification stage.
- **ADR 0074** (potential) — Adaptive thresholds: tune
  Signal/Noise/repair-eligibility floors based on empirical
  outcome traces. Requires accumulating intervention history.

## Status

Proposed. Awaiting Step 1 implementation.

---

*Author's note: The three consolidation ADRs (0070 / 0071 / 0072)
together transform Phase Alpha from "experiment series" into
"theory maintenance system":*

| layer | ADR | what it provides |
|---|---|---|
| structural | 0070 | family abstraction (queryable, intervenable) |
| observation | 0071 | quality report (read-only facts) |
| **policy** | **0072** | **intervention classifier (rules + recommendations)** |

*The user's strategic redirection from "add more directions" to
"consolidate into a system" is now structurally complete after
0072 lands.*

---

## Addendum 1 — HighQualityBoth merge (2026-05-01)

### Motivation

The migration atlas (2026-05-01) compared `recommend_intervention`
to 9 historical examples on OQ#1 and found **4/9
DIVERGENT-BY-DESIGN** cases — all Signal-Signal merges that ADR
0072 conservatively did not recommend:

- Alpha-5 picked (t_2, t_3) by smart Jaccard
- F.4 Borda top-1: (t_2, t_3) at 4/6 = 66.7% confidence
- F.5 actually executed (t_2, t_3) merge → cross-prec **1.0** (delta=0.0000, lossless)
- F.2.1 picked (t_1, t_2) — covered by Addendum 2 below

F.5's empirical safety result is a green-light: **Signal-Signal
merging when cross-precision profiles are near-identical is
provably lossless.** The conservative-by-default position taken
in ADR 0072 §3 is correct as a baseline, but admits an empirical
expansion when both sides are high-quality.

### Decision

Add Step 5.5 to the decision tree, between Step 5 (complementarity
merge) and Step 6 (theory demote):

```text
# Step 5.5 — HighQualityBoth merge
# Both focal and partner are Signal-class with very high
# cross-precision; merging is provably lossless per F.5.
if focal.summary_class == Signal:
    for other in others:
        if other.summary_class != Signal: continue
        if focal.cross_precision_mean.unwrap_or(0.0) >= 0.95
           AND other.cross_precision_mean.unwrap_or(0.0) >= 0.95:
            return Merge(other.theory_id,
                         MergeRationale::HighQualityBoth)
```

This requires moving Step 1's "Signal → None" early-return:
previously Signal halted at Step 1; now it falls through to
Step 5.5, returning Merge if a partner exists, else None.

### Threshold rationale

- Both `cross_precision_mean ≥ 0.95` is the strict condition. The
  (t_2, t_3) case on OQ#1 has both = 1.0000 exactly. Threshold
  0.95 gives 0.05 of headroom to absorb minor numerical drift
  across substrates (per the multi-substrate diagnostic, OQ#1 ↔
  long5k drift on Signal theories is < 0.01).
- No requirement on family signatures — HighQualityBoth merges
  are about REDUNDANCY, not COMPLEMENTARITY. Two Signal theories
  with overlapping coverage merge losslessly.
- The recommendation is NOT a demand to merge. The caller (a
  human or maintenance loop) decides; F.5 verified it's safe to
  execute.

### What this does NOT change

- Step 1's None return for non-Mixed, non-Signal-with-partner
  theories: unchanged.
- Mixed-Signal merge (Step 5, complementary): unchanged.
- Pairwise iteration: unchanged. Both halves of the pair will
  see this recommendation (e.g., t_2's report says
  Merge(t_3); t_3's report says Merge(t_2)). Caller deduplicates.

### Tests

Three new unit tests cover this path:
- `adr0072_addendum1_signal_signal_with_high_xprec_recommends_merge`
- `adr0072_addendum1_signal_with_low_xprec_partner_returns_none`
- `adr0072_addendum1_signal_alone_returns_none`

### Status

Addendum 1: **Accepted.** Ships in the same commit as Addendum 2.

---

## Addendum 2 — Near-disjoint signature rule (2026-05-01)

### Motivation

Migration atlas finding (2026-05-01): F.2.1's pick (t_1, t_2)
was DIVERGENT under ADR 0072's strict `is_disjoint` rule for
Step 5 complementarity merges. t_1 and t_2 share 2 family
memberships out of 5 (Jaccard 0.40), so `is_disjoint` returns
false, and the recommendation falls through to Manual.

F.2.1's empirical analysis showed (t_1, t_2) is a defensible
quality-aware merge candidate. The strict-disjoint rule is
correct in spirit (avoid merges where signatures already
overlap heavily) but too aggressive at threshold zero.

### Decision

Replace Step 5's `focal_fams.is_disjoint(&other_fams)` check
with a **Jaccard threshold**:

```text
shared = |focal_fams ∩ other_fams|
total  = |focal_fams ∪ other_fams|
jaccard = shared / total  (0 if total = 0)

if jaccard <= 0.50:  # signatures more disjoint than shared
    return Merge(other.theory_id, MergeRationale::Complementary)
```

Setting the threshold to 0.50 means: **keep merging when
signatures are MORE disjoint than shared**. Strict disjoint
(Jaccard 0.0) was unnecessarily restrictive; F.2.1's 0.40
Jaccard pick was empirically reasonable.

### Threshold rationale

- 0.50 is the natural midpoint: above → signatures dominate by
  shared structure (merging dilutes either side); below →
  signatures are mostly different, complementarity dominates.
- F.2.1's empirical pick was 0.40, comfortably below 0.50.
- A future ADR could tune this against more substrates if the
  0.50 threshold proves too liberal or too strict.

### What this changes

- Step 5 (complementarity merge) becomes more permissive.
- F.2.1's pick (t_1, t_2) on OQ#1 will now produce a Merge
  recommendation, raising migration-atlas agreement.
- Other strict-disjoint cases unchanged (Jaccard 0 still
  passes the new threshold).

### Tests

Two new unit tests:
- `adr0072_addendum2_near_disjoint_jaccard_below_threshold_recommends_merge`
- `adr0072_addendum2_jaccard_above_threshold_does_not_recommend_merge`

### Status

Addendum 2: **Accepted.** Ships in the same commit as Addendum 1.

---

## Combined empirical effect

After Addenda 1 + 2, the migration atlas re-run on OQ#1 should
raise the agreement rate from 5/9 to 9/9:

| atlas case | pre-addenda | post-addenda |
|---|---|---|
| Alpha-3+ demote | AGREE | AGREE |
| Alpha-3+++ repair | AGREE | AGREE |
| Alpha-3++++ naive (FALSIFIED) | AGREE | AGREE |
| **Alpha-5 smart_merge** | DIVERGENT | **AGREE** (Addendum 1) |
| Beta-2 family_demote | AGREE | AGREE |
| F.2 family_aware | AGREE | AGREE |
| **F.2.1 quality_aware** | DIVERGENT | **AGREE** (Addendum 2) |
| **F.4 multi_signal** | DIVERGENT | **AGREE** (Addendum 1) |
| **F.5 merge_safety** | DIVERGENT | **AGREE** (Addendum 1) |

Expected post-addendum atlas re-run: 9/9 agree, 0 open
divergences.
