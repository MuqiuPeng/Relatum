# 0064: Drives as meta-R objects (Phase H2.1)

Status: Proposed
Date: 2026-04-27

## Context

ADR 0063 specified Phase H2 in three slices:
- **H2.0** — multi-drive blend with feedback-tuned weights.
- **H2.1** — drives registered as meta-R objects under
  `DRIVE_MARKER`, with the existing ESTABLISHED-promotion
  lifecycle applied to drives.
- **H2.2** — drive synthesis from primitive metrics.

H2.0 is now complete: trait + 3 baseline impls (step 1),
DriveMix A/B + checkpoint (step 2), combined-signal API
(step 3a), OQ #4 penalty handling, and the load-bearing
shape (α) on the EP gate. The self-tuning evaluation
loop is closed end-to-end.

Long-run empirics under α (HORIZON=5000) confirm:
- Hand-tuned mix: stable, baseline-preserving behaviour.
- Equal-weighted mix: 39% more EP attempts via α; mutation
  trajectories diverge from hand-tuned.
- DriveMix mutation observable in mutation-trajectory
  space, not just signal-magnitude space.

The natural next direction is H2.1: lift drives into the
meta-R class hierarchy alongside PATTERN_MARKER,
THEORY_MARKER, ESTABLISHED_MARKER, SHARED_AXIOM_MARKER,
ACTION_SEQ_MARKER. This ADR scopes that move.

This is the slice that opens commitment 3 — types are
meta-R instances. Pre-H2.1, drive identities are
compile-time strings (`"compression"`, etc.); their
existence is a Rust impl, not an R fact. H2.1 makes the
existence of a drive a fact about the runtime's *current*
catalogue, observable to the runtime itself.

## Decision

### Three sub-slices

**H2.1.0 — `DRIVE_MARKER` registration only.**
Each compile-time drive registers as
`R(DRIVE_MARKER, drive_<id>)`. The drive's weight in the
active mix is observable as a separate edge:
`R(drive_<id>, drive_<id>_weight)` where the weight
identifier encodes the value (or alternatively, weight
lives outside meta-R as a HashMap<drive_id, f64>
referenced by the registered drive_id token).

This slice is registration-only — no behaviour change.
The runtime can observe its drive catalogue via
`rset.right_of(DRIVE_MARKER)`. Constitutionally:
commitment 3 explicitly says "types are meta-R instances";
this slice satisfies it for drives.

**H2.1.1 — ESTABLISHED-promotion lifecycle for drives.**
Apply ADR 0053 ESTABLISHED machinery to drives:
- A drive registers under DRIVE_MARKER on construction.
- Per-window EP-delta contribution accumulates per drive
  (a new accounting layer).
- When a drive's accumulated contribution crosses an
  ESTABLISHED threshold, promote:
  `R(drive_<id>, ESTABLISHED_MARKER)`.
- Demotion when contribution drops below a retention
  floor (mirror of ADR 0053's hysteresis).

The mechanics mirror PATTERN-ESTABLISHED chains exactly.
Constitutionally clean: drive_<id> tokens follow the
same lifecycle as pattern ids, axiom ids, etc.

**H2.1.2 — DriveMix weight tied to drive ESTABLISHED status.**
Weight is no longer governed only by H2.0's A/B
mutation. ESTABLISHED drives have weight floored above
zero (e.g., 0.1); demoted drives have weight zeroed.
The A/B mutation continues but operates within the
constraints set by ESTABLISHED status.

This is the load-bearing slice — drives that the runtime
has empirically validated stay active; drives that
don't earn ESTABLISHED status get removed from the
active mix.

### Why H2.1.0 first

Three reasons:

1. **Constitutional**. H2.1.0 is the smallest slice that
   satisfies commitment 3 for drives. Without it, drives
   are stuck in compile-time-only land, contradicting
   the constitution. H2.1.0 fixes that.

2. **No behaviour risk**. Just registration; existing
   H2.0 logic untouched. F0 + long-run should be
   byte-identical pre/post H2.1.0.

3. **Verification surface**. After H2.1.0, the runtime
   can `for d in rset.right_of(DRIVE_MARKER) { ... }`
   uniformly with how it iterates pattern ids. That
   surface is needed for H2.1.1's accounting.

### H2.1.0 design (concrete enough to start when chosen)

#### Constants

```rust
pub const DRIVE_MARKER: &str = "__drive__";
```

Joins the existing meta-R class roster:
- `PATTERN_MARKER`
- `AXIOM_MARKER`
- `THEORY_MARKER`
- `ESTABLISHED_MARKER`
- `SHARED_AXIOM_MARKER`
- `ACTION_SEQ_MARKER`
- (new) `DRIVE_MARKER`

#### Drive registration

`AutonomousRuntime::new` (and `from_checkpoint_text`)
registers each drive on construction:

```rust
for drive in &self.drives {
    let drive_id = format!("drive_{}", drive.id());
    self.rset.add(R::new(DRIVE_MARKER, drive_id.as_str()));
}
```

Now `rset.right_of(DRIVE_MARKER)` returns
`["drive_compression", "drive_prediction_error",
"drive_mode_thrash"]`. Drive weights stay in DriveMix
HashMap (not in rset) for H2.1.0 — adding weight as
meta-R edges is H2.1.1 work.

#### Penalty marker

Penalty drives (currently just ModeThrashPenalty) get a
secondary registration:

```rust
pub const PENALTY_MARKER: &str = "__penalty__";
// ...
if drive.is_penalty() {
    self.rset.add(R::new(drive_id.as_str(), PENALTY_MARKER));
}
```

Now `rset.right_of_for_left("drive_mode_thrash")` includes
`PENALTY_MARKER`, making the penalty status observable
via meta-R. This generalizes the current compile-time
`is_penalty()` semantic to a meta-R fact.

Actually — simpler: penalty drives register *both* under
DRIVE_MARKER and PENALTY_MARKER:

```rust
self.rset.add(R::new(DRIVE_MARKER, drive_id));
if drive.is_penalty() {
    self.rset.add(R::new(PENALTY_MARKER, drive_id));
}
```

Now `is_penalty(drive_id)` is queryable via `rset` without
any compile-time impl needed.

#### Update existing code paths

`combined_drive_signal` and `normalized_drive_signal`
gain a meta-R-aware variant: instead of
`drive.is_penalty()` (compile-time method call), check
`rset.contains(R::new(PENALTY_MARKER, drive_id))`.

This is the constitutional shift: penalty status is now a
*fact about the drive*, not a *method on the impl*. The
runtime could in principle add or remove penalty status
at runtime by retracting/asserting the relevant meta-R
edge. (H2.1.0 doesn't yet exercise that capability, but
the door is open.)

#### Checkpoint round-trip

The `[rset]` section already serializes meta-R edges, so
DRIVE_MARKER registrations round-trip automatically. No
new `[drive_registry]` section needed. After restore,
`AutonomousRuntime::from_checkpoint_text` re-registers
drives but skips edges already present (idempotency).

#### What H2.1.0 does NOT do

- Does NOT track per-drive EP-delta contribution. That's
  H2.1.1.
- Does NOT promote/demote drives. That's H2.1.1 + H2.1.2.
- Does NOT couple drive weights to ESTABLISHED status.
  That's H2.1.2.
- Does NOT add new drives. The catalogue stays at 3 baseline.
- Does NOT support runtime-added drives (e.g., from H2.2
  synthesis). That requires more design.

### Alternatives considered

- **Skip H2.1; jump to H2.2**. H2.2 is "synthesize new
  drives from primitives". Without H2.1, synthesized
  drives have no constitutional home — they're either
  compile-time impls (which H2.2 conceptually rules out)
  or floating Rust constructs. H2.1 is the foundation
  H2.2 needs.
- **Use a separate `[drives]` checkpoint section instead
  of meta-R**. Constitutionally weaker — drives become a
  privileged side-channel state, not a class of meta-R
  objects. H2.1 explicitly chooses meta-R for
  constitutional reasons.
- **Tie weights to meta-R immediately (skip H2.1.0)**.
  Bigger surface, more coupling, more risk. Phasing
  H2.1.0 → H2.1.1 → H2.1.2 limits each slice's blast
  radius.

### Constitutional review

H2.1.0 against the five v2 commitments:

1. **R is singular.** New marker `DRIVE_MARKER` is just
   another R-edge identifier. Same as PATTERN_MARKER /
   THEORY_MARKER. No new R relation type. PASS.
2. **R is binary.** No new edge shapes. PASS.
3. **Types are meta-R instances.** PASS — this is the
   slice that opens this commitment for drives. Drive
   existence becomes a meta-R fact:
   `R(DRIVE_MARKER, drive_compression)` etc.
4. **Identity is token-based.** Drive ids
   (`drive_compression`, `drive_prediction_error`,
   `drive_mode_thrash`) are token strings. Same as
   pattern ids, axiom ids. PASS.
5. **Similarity is structural.** No similarity claim
   involving drives. PASS.

For H2.1.1, the ESTABLISHED lifecycle reuses ADR 0053
mechanics that already pass commitments 1-5.

For H2.1.2, weights become rset-derived; commitments
hold provided weights remain numeric values keyed by
the registered drive ids.

### Non-goals

- Cross-runtime drive sharing.
- Runtime-added drives (deferred to H2.2 synthesis).
- Drive composition (e.g., a drive built from other drives'
  outputs) — also H2.2 territory.

### Verification plan (H2.1.0 only)

- Existing 507 tests pass after introducing DRIVE_MARKER
  and PENALTY_MARKER constants + registration logic.
- New unit tests:
  - `h2_1_0_drive_marker_registers_three_baseline_drives`
  - `h2_1_0_penalty_marker_only_for_mode_thrash`
  - `h2_1_0_drive_registration_round_trips_through_checkpoint`
  - `h2_1_0_normalized_signal_uses_meta_r_penalty_query`
    (verifies the constitutional shift — penalty check
    via rset, not via compile-time method).
- F0 battery: stream_diamond CONVERGED. No regression.
- Long-run: 268/129/1/4/8 baseline preserved hand-tuned;
  α divergence preserved equal-weighted.

### Open questions

1. **Should penalty status be queryable via the trait
   `is_penalty()` method too, or only via meta-R**?
   Recommendation: keep `is_penalty()` method as fast-path
   (no rset lookup), but the canonical answer is the
   meta-R edge. Method becomes a memoization of the meta-R
   query. H2.1.1 may need to retract penalty status at
   runtime; the meta-R representation is the source of
   truth.

2. **What about drive_<id>_weight as a meta-R edge?**
   Deferred to H2.1.1 / H2.1.2. The naive encoding
   (`R(drive_id, weight_token)` with weight encoded as
   string) doesn't work cleanly — weights are continuous,
   identifiers are discrete. Likely solution: weight
   stays in DriveMix HashMap; meta-R holds only the
   structural facts (which drives exist, which are
   penalties, which are ESTABLISHED).

3. **Drive registration timing**: at construction, or
   lazy on first `combined_drive_signal` call? Construction
   is simpler; lazy avoids unnecessary edges if drives
   are never consulted. Likely answer: construction (the
   3-edge cost is trivial; lazy adds complexity).

4. **What if a drive is registered with a non-baseline
   id?** E.g., a future user adds a custom Drive impl with
   id `"my_custom"`. Registration generalizes to
   `drive_my_custom`. No special handling needed.

## Touched ADRs

- **ADR 0063** (Phase H2 / drive self-modification) is the
  parent. H2.1 is the next slice after H2.0's α completion.
- **ADR 0053** (selective declarativization, ESTABLISHED
  promotion) is the lifecycle template H2.1.1 will reuse.
- **ADR 0033** (defeasible axioms) is the precedent for
  rate-based promotion / demotion; H2.1.1 will mirror its
  hysteresis design.

## Summary

H2.1 is the constitutional slice — it lifts drives from
compile-time Rust constructs into meta-R instances,
satisfying commitment 3 for the drive catalogue. The
phasing (0/1/2) lets each sub-slice land independently:

- H2.1.0 — registration only. No behaviour change. Small,
  safe, constitutionally load-bearing.
- H2.1.1 — accounting + lifecycle. Drives can be
  ESTABLISHED / demoted by their EP-delta contribution.
- H2.1.2 — weights tied to ESTABLISHED status. The
  feedback loop becomes: drive contributes → drive earns
  ESTABLISHED → drive's weight stays positive → drive
  contributes more.

H2.1.0 is the recommended starting slice. It opens the
constitutional door without touching behaviour. H2.1.1 +
H2.1.2 follow when empirical motivation surfaces.

Status: H2.1.0 + H2.1.0+ **Accepted (implemented)**; H2.1.1 / H2.1.2 Proposed.

---

## Addendum 1 — H2.1.0 implemented (2026-04-27 late⁶)

User signaled readiness for H2.1.0. Implemented per the
ADR's "registration-only, no behaviour change" specification.

#### Changes

- `pub const DRIVE_MARKER: &str = "__drive__"` and
  `pub const PENALTY_MARKER: &str = "__penalty__"` added
  to `lib.rs` alongside the other meta-R class markers.
- `RSet::collect_meta_ids` extended to treat both markers
  AND the registered `drive_<id>` tokens as meta-R (not
  data) for the prediction-error drive's data-edge filter.
- `AutonomousRuntime::register_drives_in_rset()` private
  helper. Called from `new` and `from_checkpoint_text`.
  Iterates `self.drives`, adds `R(DRIVE_MARKER, drive_<id>)`
  for each, and `R(PENALTY_MARKER, drive_<id>)` if
  `drive.is_penalty()` is true.
- Idempotent by construction (RSet::add is set-semantics).

#### What H2.1.0 does NOT do

- Does NOT rewire `combined_drive_signal` /
  `normalized_drive_signal` to query meta-R for penalty
  status. The compile-time `Drive::is_penalty()` method
  remains the source of truth. The ADR's "Update existing
  code paths" section is deferred to a follow-up
  slice (H2.1.0+ or H2.1.1) — keeping this slice
  strictly registration-only minimizes blast radius.

#### Empirical verification

- 507 → 512 tests pass (+5 H2.1.0-specific):
  - `h2_1_0_drive_marker_registers_three_baseline_drives`
  - `h2_1_0_penalty_marker_only_for_mode_thrash`
  - `h2_1_0_drive_registration_round_trips_through_checkpoint`
  - `h2_1_0_drive_registration_is_idempotent`
  - `h2_1_0_drive_ids_treated_as_meta_not_data`
- F0 battery: stream_diamond CONVERGED. All other seeds
  CONVERGED. No regression vs post-EP-fix baseline.
- OQ #1 long-run (HORIZON=2000):
  - hand-tuned: 268/129/1/4/8 — byte-identical to
    post-α baseline.
  - equal-weighted: 203/179/0/1/3 — byte-identical to
    post-α baseline.

#### Constitutional verdict

H2.1.0 satisfies commitment 3 (types are meta-R instances)
for the drive catalogue. Drive existence is now a
queryable rset fact. The shape is identical to existing
class chains (PATTERN_MARKER, AXIOM_MARKER, etc.). All
five v2 commitments PASS.

#### Status

H2.1.0 implemented; H2.1.1 (ESTABLISHED-promotion
lifecycle for drives) and H2.1.2 (DriveMix weights tied to
ESTABLISHED status) remain Proposed pending future
iteration. The natural follow-up is to use the registered
DRIVE_MARKER / PENALTY_MARKER edges as the canonical source
of penalty status (rewire `combined_drive_signal` to
query rset). That's a small, targeted slice when the time
comes.

---

## Addendum 2 — H2.1.0+ rewires query path to meta-R (2026-04-28)

The "Update existing code paths" section of this ADR called
for `combined_drive_signal` / `normalized_drive_signal` to
query meta-R for penalty status (rather than calling the
compile-time `Drive::is_penalty()` method). H2.1.0 deferred
this to keep the registration slice strictly additive.
H2.1.0+ now lands the query rewire.

#### Changes

- `AutonomousRuntime::is_drive_penalty_via_meta_r(drive_id)`
  private helper: returns `rset.contains(R::new(PENALTY_MARKER, drive_<id>))`.
- `combined_drive_signal` consults this helper instead of
  `drive.is_penalty()` to decide add-vs-subtract.
- `normalized_drive_signal` consults this helper for the
  positive-only weight-sum denominator.
- `Drive::is_penalty()` method retained on the trait but no
  longer consulted by either method. Documentation updated
  to describe its new role: a fast-path fallback / convenience
  marker that the registration logic uses to seed meta-R, but
  not the canonical answer.

#### Empirical verification

- 512 → 515 tests pass (+3 H2.1.0+):
  - `h2_1_0_plus_retracting_penalty_marker_flips_drive_to_positive`
  - `h2_1_0_plus_asserting_penalty_marker_flips_drive_to_negative`
  - `h2_1_0_plus_normalized_signal_denominator_uses_meta_r`
- F0 battery: stream_diamond CONVERGED. Other seeds CONVERGED.
  No regression vs post-α / post-H2.1.0 baseline.
- OQ #1 long-run (HORIZON=2000):
  - hand-tuned: 268/129/1/4/8 — byte-identical to baseline.
    Signal trajectory identical to post-α: -0.654 → -1.235 → -0.988.
  - equal-weighted: 203/179/0/1/3 — byte-identical to baseline.

#### Why behaviour is byte-identical

The runtime's behaviour didn't change because `register_drives_in_rset`
faithfully encodes `Drive::is_penalty()` as the meta-R edge
set. The query-path rewire shifts the *source of truth* without
changing the *answer* under the current registration policy.

The load-bearing tests verify the source-of-truth shift directly:
- `_retracting_penalty_marker_flips_drive_to_positive`: removing
  the meta-R edge makes mode_thrash contribute *positively* even
  though `Drive::is_penalty()` still returns `true`.
- `_asserting_penalty_marker_flips_drive_to_negative`: adding a
  meta-R edge for compression makes it contribute *negatively*
  even though `Drive::is_penalty()` returns `false`.

Both tests demonstrate that meta-R is now the canonical source.

#### Constitutional implications

This is the slice that *operationalizes* commitment 3 for drives.
H2.1.0 satisfied commitment 3 by *registering* drives in meta-R.
H2.1.0+ takes the next step: the runtime now *consults* meta-R
when making drive-related decisions, rather than the compile-time
catalogue.

The runtime could in principle now:
- Retract `R(PENALTY_MARKER, drive_id)` to flip a drive's role
  on the fly.
- Add a brand-new drive registration in meta-R that the runtime's
  drive registry doesn't have (though the evaluate function
  still has to come from somewhere — H2.2 territory).
- Demote / re-establish drives via the same lifecycle that
  patterns and theories use (H2.1.1).

H2.1.0+ doesn't yet exercise these capabilities, but the API
shape now supports them.

#### Status

H2.1.0 + H2.1.0+ implemented. The ADR's "Update existing code
paths" requirement is fully satisfied. H2.1.1 + H2.1.2 remain
Proposed — they're separate slices about *lifecycle*
(promotion / demotion) and *coupling* (weights tied to
ESTABLISHED), not about query routing.
