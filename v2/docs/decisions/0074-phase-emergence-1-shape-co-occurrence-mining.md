# 0074: Phase Emergence-1 — shape co-occurrence mining (concept lifting)

Status: Proposed
Date: 2026-05-05

Parent: [0073 — phase pivot to concept emergence](0073-phase-pivot-concept-emergence.md)

## Context

ADR 0073 pivoted v2 from concept curation (Phase 0070-0072) to
concept emergence. It identified three entry points:
**E1 shape mining**, **E2 object lifting**, **E3 intrinsic
drive** — and named E1+E3 as paired highest priority.

This ADR specifies E1's minimum viable mechanism.

### What "new concept" means precisely at this layer

The system has two existing layers of abstraction over R:

1. **First-order shape pattern** (axiom-level). An axiom is an
   instance of an `AxiomTemplate` — a `(premise edges, conclusion
   edge)` rule with integer-indexed variables (ADR 0027). The
   system can already discover axioms by enumerating templates
   and measuring their rate.

2. **First-order shape family** (collection-of-axioms level). ADR
   0070 / 0068's `discover_axiom_shape_families` clusters axioms
   by canonicalized premise key or conclusion key — e.g.
   `shape_premise_p0-0_p1-2` collects every axiom whose canonical
   premise has those two edges. This is one shape per family.

What's missing is the **second-order layer**: patterns of *which
shape-families co-occur* within Signal-class theories. Empirical
example from OQ#1 / long5k:

- `t_2` and `t_3` are both Signal-class
- Both contain axioms drawn from `shape_premise_p0-1_p1-0` (the
  antisymmetry premise) AND from a totality-style conclusion
  family
- The pair (antisymmetry-shape, totality-shape) is what
  "Signal theory" looks like on these substrates

The system never names this pair. There is no `concept_linear_order`
that captures "this is a recurring shape-shape pattern". E1 fixes
that.

### Why this is "creation", not curation

Curation operates inside the existing vocabulary: discover
axioms within fixed templates, cluster axioms within fixed
canonicalization, recommend interventions within fixed enum
variants. None of these enlarges the *vocabulary* the system
reasons in.

A registered concept is a new noun. Once `concept_linear_order`
exists as a meta-R object, downstream machinery can query it,
attribute it, retract it, *predict* it — none of which was
previously expressible.

This is the smallest step that crosses the curation/creation
boundary identified in ADR 0073.

## Decision

Ship a **shape co-occurrence mining** layer with three
operations: `propose_concepts`, `validate_concepts`,
`register_concept`. Concepts are persisted as meta-R objects.

### 1. propose_concepts

Iterate `theories()` × `theory_axioms()`. For each axiom in each
theory, look up its shape-family memberships
(`shape_family_members` reverse-indexed). Build a per-theory
shape-family multiset.

Enumerate candidate concepts as (shape_family_subset) tuples
satisfying:

- The subset has size ≥ 2 (a concept is a *pair* or larger; size 1
  is just a shape family)
- The subset's shape-families are jointly present in ≥ `min_theories`
  theories (default: 2 — the smallest non-degenerate co-occurrence)
- Optionally: the theories where they co-occur are
  predominantly Signal-class (filter via
  `theory_quality_report` from ADR 0071) — controlled by the
  `require_signal_only` flag, default true

Candidate id format: `concept_<sorted_shape_id_hash>` for
collision safety, with optional human-readable alias
(`concept_premise-p0-1-p1-0__conclusion-c0-1` etc.).

### 2. validate_concepts

For each candidate, compute aggregate cross-precision:

- For each theory in `theories_attested`, generate a substrate
  via `generate_substrate_from_theory` (existing API from ADR
  0071)
- For each axiom in the theory that belongs to one of the
  concept's constituent shape-families, compute its
  `axiom_cross_precision` against all generated substrates
- Aggregate: take the mean over (axiom, substrate) pairs

A candidate passes validation iff aggregate ≥
`CONCEPT_VALIDATION_FLOOR`. Initial threshold: **0.80**, since
we mine from Signal-class theories which already exhibit
cross-precision ≥ 0.85 in observed cases. The threshold may be
empirically tuned (analog of Phase 0072-B's threshold scan, once
enough concept candidates exist to scan over).

Failure modes:
- Empty constituent intersection on some theory → skip that
  theory's contribution
- All theories empty → candidate fails (cannot validate)

### 3. register_concept

A passing candidate becomes meta-R:

```
R(CONCEPT_MARKER, concept_id)              — concept existence
R(concept_id, HAS_CONSTITUENT_SHAPE, sf_id) — one per constituent
R(concept_id, ATTESTED_IN_THEORY, t_id)    — one per theory
R(concept_id, CROSS_PRECISION_AT_MINT, "0.94")  — recorded value
```

The concept is now queryable through new RSet methods (see
Implementation sketch). It participates in normal R operations:
listing, retraction, etc.

### Lifecycle and reversibility

- `register_concept` is reversible: `retract_concept(id)` removes
  all four meta-R edges associated with the concept
- If a constituent shape-family is later retracted (via existing
  `retract_shape_family`), the concept does NOT auto-cascade.
  The concept simply becomes "stale": one of its constituents
  no longer exists. Detection: `concept_status(id)` returns
  `Live | Stale | Validated | Falsified`. Cascade behavior is a
  future ADR's call.
- Re-validation: a registered concept can be re-tested at any
  later time via the same validate machinery; if it fails the
  current threshold, status flips to `Falsified` but the meta-R
  remains until explicit retraction. Mirrors ADR 0072's
  philosophy that demotion is structural, not silent.

### Out of scope for E1

- **Concept composition**: building concepts of concepts (third-
  order abstraction). Deferred until at least 3 first-order
  concepts have been minted and lived long enough to assess
  composability.
- **AxiomComplete intervention**: a new
  `RecommendedIntervention::AxiomComplete { missing_shape, target_concept }`
  that ADR 0072 could use when a Mixed theory has some-but-not-
  all constituents of a known concept. Implementation deferred
  to Phase Emergence-1.5.
- **Drive integration**: concept-aware attention. Deferred to
  Phase Emergence-2 (E3).
- **Concept transfer / generalization probes**: testing whether
  a concept registered on OQ#1 holds on a structurally distinct
  substrate. The Phase 0072-A long5k finding (OQ#1 / long5k
  RSets are isomorphic) makes this trivially true on those
  pairs; truly distinct substrates (narrow_a, OQ#2 at maturity)
  are needed for the real test, which is itself blocked on the
  open item from Phase 0072-A.

## Alternatives considered

**Alt A: bottom-up axiom-instance co-occurrence (no shape
abstraction).**
Track `(ax_id_a, ax_id_b)` co-occurrence directly. Rejected:
specific axiom ids are episode-bound (`ax_antisymmetry_001` on
substrate X is not the same identifier as `ax_antisymmetry_001'`
on substrate Y, even if structurally identical). Co-occurrence
at instance level does not transfer across episodes.
Shape-family abstraction is the right granularity for
*type-level* concepts.

**Alt B: mint new `AxiomTemplate`s by composing existing
templates.** True axiom invention — e.g. derive a length-3
transitivity from length-2 transitivity + composition reasoning.
Rejected for E1: this requires constraint reasoning (is the
composed template logically valid? does its rate hold on the
substrate?) which is far larger in scope. Concept-as-meta-R is
strictly weaker but already crosses the curation/creation
boundary.

**Alt C: defer concepts until E3 ships first.** Drive without
concepts is directionless; concepts without drive are nameable
but unused. Pair priority is right (ADR 0073 names E1+E3 as
paired) but E1 produces concrete artifacts that E3 can
immediately consume; E3 alone produces a signal with no
consumer. Order: E1 first.

**Alt D: concept = theory.** A theory is also "a named axiom
collection." Why need concepts at all? Rejected: theories are
*instance-bound* — `t_2` is the specific axiom set fired on a
specific stream. Concepts are *type-bound* — they refer to
shape-families, which abstract over instances. The distinction
is exactly the curation/creation boundary: many theories per
concept, not the other way around.

## Consequences

**Now possible:**

- Cross-substrate concept claims: a registered concept
  predicts "this shape-shape pattern is what high-quality
  theories look like." The claim is testable on any future
  substrate.
- A path to AxiomComplete intervention (Phase Emergence-1.5)
- A target for E3 drive: "is the stream firing only one
  constituent of a known concept? attend to find the partners"
- A genuinely new noun in the system's vocabulary, queryable as
  meta-R

**Now harder:**

- Defining "co-occurrence" precisely. Initial spec uses (a)
  "both shape-families have ≥1 axiom in the same theory."
  Alternative (b) "the theory contains specific axiom instances
  from both shape-families that bind the same variables" is
  more semantically precise but harder to implement and may
  reduce candidate count below useful threshold. E1 ships (a);
  (b) is a refinement.
- Concept identifier policy. Hash-based for collision safety;
  human-readable alias for debugging. Aliases must be stable
  across runs (deterministic from constituent shape ids).
- Validation cost. For each candidate, generating substrates +
  computing cross-precision is O(theories × axioms × substrate-
  size). With 4 theories × ~13 axioms on OQ#1, this is small;
  scaling to dozens of theories needs caching strategy.
  Initial implementation: no cache, accept O(n²) cost.

**Newly easy:**

- Concept-aware diagnostic logging. The visualization tooling
  (lib's `format_decision_trace` etc.) gains a new dimension:
  "is this theory an instance of any registered concept?"
- Cross-substrate ablation refinement. Instead of comparing
  all theories' cross-precision, compare *concept instances*
  on different substrates — concept-level signal is more
  abstraction-stable.

## Implementation sketch

New file `src/types_concept.rs`:

```rust
use crate::R;

pub const CONCEPT_MARKER: &str = "__concept";
pub const HAS_CONSTITUENT_SHAPE: &str = "__concept_has_shape";
pub const ATTESTED_IN_THEORY: &str = "__concept_attested";
pub const CROSS_PRECISION_AT_MINT: &str = "__concept_xprec_mint";

#[derive(Debug, Clone)]
pub struct ConceptCandidate {
    pub id: String,
    pub alias: Option<String>,
    pub constituent_shapes: Vec<String>,
    pub theories_attested: Vec<String>,
    pub aggregate_cross_precision: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptStatus {
    Live,
    Stale,        // a constituent shape-family was retracted
    Validated,    // recently re-validated and passing
    Falsified,    // recently re-validated and failing
}

pub struct ConceptMiningConfig {
    pub min_theories: usize,           // default: 2
    pub require_signal_only: bool,     // default: true
    pub validation_floor: f64,         // default: 0.80
    pub max_candidate_size: usize,     // default: 4 (no concept of >4 shapes)
}
```

New methods on `RSet` (in `lib.rs`):

```rust
pub fn propose_concept_candidates(
    &self,
    config: &ConceptMiningConfig,
    quality_reports: &[TheoryQualityReport],
) -> Vec<ConceptCandidate>;

pub fn validate_concept(
    &self,
    candidate: &ConceptCandidate,
    substrates: &[RSet],
    floor: f64,
) -> Option<f64>;

pub fn register_concept(
    &mut self,
    candidate: &ConceptCandidate,
) -> Result<String, ConceptRegistrationError>;

pub fn retract_concept(&mut self, id: &str) -> bool;
pub fn concepts(&self) -> Vec<&str>;
pub fn concept_constituent_shapes(&self, id: &str) -> Vec<&str>;
pub fn concept_attested_theories(&self, id: &str) -> Vec<&str>;
pub fn concept_cross_precision_at_mint(&self, id: &str) -> Option<f64>;
pub fn concept_status(&self, id: &str) -> ConceptStatus;
```

Test plan:
- `concept_mining_proposes_pair_for_signal_cooccurrence`
- `concept_mining_skips_below_min_theories`
- `concept_mining_skips_noise_theories_when_signal_only`
- `concept_validation_passes_high_cross_precision`
- `concept_validation_fails_below_floor`
- `concept_register_creates_meta_r`
- `concept_retract_removes_meta_r`
- `concept_status_is_stale_after_constituent_retract`
- `concept_status_is_falsified_after_revalidate_fail`
- `concept_constituent_query_returns_registered_shapes`

Example program: `phase_emergence_1_concept_mining.rs`
- Run OQ#1 to Phase 0 maturity
- Compute reports
- propose → validate → register
- Print: which concepts minted, which constituents, cross-
  precision at mint
- Re-run on long5k; verify concept identity is portable across
  the two substrates (corollary of structural isomorphism from
  Phase 0072-A)

Shipping target:
- ~250 lines lib + ~150 lines tests + 1 example
- Lib tests: 600 → ~610
- ADRs: 73 → 74
- No regressions in existing 600 tests

## Open questions for implementation

- **Subsumption among candidates**: if `{A, B}` and `{A, B, C}`
  both pass validation, prefer one or both? Suggestion: prefer
  maximal — sub-concepts of validated maximal concepts are
  silently absorbed, mirroring ADR 0072's `DemoteSuperset`
  philosophy. Implementation: filter post-validation.
- **Concept-id stability across runs**: deterministic from
  sorted constituent shape ids. Hash function: SHA-256
  truncated to 16 hex chars. Alias: dash-joined sorted
  constituents.
- **Re-validation cadence**: Live concepts can become Falsified
  after sufficient stream evolution. When does the system
  re-validate? Suggestion: opt-in (`revalidate_concepts(&mut self)`
  is a separate call, callable from runtime cleanup phase).
  Lazy revalidation; do not wedge into the discover-loop.
- **Empty-stream edge case**: cross-precision computed against
  zero substrates is undefined. Validate must reject this
  cleanly with `None`, not panic.

These are decided in implementation, not in this ADR.

## Implementation

Pending. ADR records the design only. Initial implementation
target: separate commit referencing this ADR by number.

## Success criteria

- ≥ 1 concept successfully mined from OQ#1's Phase 0 state
- The same concept (by constituent shape ids) mined from
  long5k's Phase 0 state, demonstrating cross-substrate
  identity portability
- All shipped tests pass; existing 600 tests unaffected
- Example program produces a clean, readable concept-listing
  log demonstrating the propose-validate-register loop
