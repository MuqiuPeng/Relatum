# 0012: γ — naming-pass driver and relevance filter

Status: Accepted
Date: 2026-04-23

## Context

ADR 0010 gave the system a way to record named patterns as meta-R
instances. ADR 0009 showed that naming every canonical-form group
would be led by the trivial single-edge pattern (P2, 6 instances
spanning 5 compound classes). ADR 0011's probe showed that the
feedback loop from meta-R, if iterated naively, mostly rediscovers
the encoding itself as k-spoke stars — the "artifacts" it produces.

γ closes the β layer by:
1. Filtering candidate patterns so trivial ones are not named
   automatically.
2. Driving the naming pipeline from the RSet state — the first
   non-deterministic step in v2 where the *system*, rather than the
   caller, chooses what to name.
3. Defaulting to no iteration, with an opt-out switch for
   experiments that want to explore patterns-of-patterns.

This ADR is scoped deliberately small. ADR 0011's findings argued
against a full iterative mechanism; the data-driven decisions γ
makes at this stage are limited to threshold-based candidate
admission, and the driver runs exactly one pass.

## Decision

### Policy

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamingPolicy {
    pub min_edges: usize,        // reject canonical forms with < min_edges edges
    pub min_instances: usize,    // reject groups with < min_instances subgraphs
    pub skip_meta_subgraphs: bool, // ignore subgraphs that touch meta-R tokens
}

impl Default for NamingPolicy {
    fn default() -> Self {
        NamingPolicy {
            min_edges: 2,
            min_instances: 1,
            skip_meta_subgraphs: true,
        }
    }
}
```

Default behavior: suppress single-edge patterns (ADR 0009 P2), allow
singleton instances, skip meta-R artifacts during a pass.

### Decisions

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    BelowMinEdges { edges: usize, min: usize },
    BelowMinInstances { instances: usize, min: usize },
    AlreadyKnown, // every candidate deduped against existing instances
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamingDecision {
    Named(String),
    Skipped(SkipReason),
}
```

### API

```rust
impl RSet {
    /// Apply the policy filter to a candidate group; if it passes,
    /// invoke `name_pattern_instances`. Returns a `NamingDecision` in
    /// the non-error case and propagates any `PatternError`.
    pub fn consider_naming(
        &mut self,
        instances: &[Subgraph],
        policy: &NamingPolicy,
    ) -> Result<NamingDecision, PatternError>;

    /// γ driver: run the pipeline once, group subgraphs by canonical
    /// form, apply the policy per group, return a record of what
    /// happened. Subgraphs touching meta-R are filtered when
    /// `policy.skip_meta_subgraphs` is true.
    pub fn run_naming_pass(
        &mut self,
        policy: &NamingPolicy,
    ) -> Vec<(CanonicalForm, NamingDecision)>;
}
```

`run_naming_pass` order:
1. Collect meta-R identifier set (PATTERN_MARKER + patterns + instances).
2. Run `compound_class_subgraphs`, flatten into a by-canonical-form map
   in a `BTreeMap` for deterministic iteration.
3. Skip subgraphs touching meta-R when `skip_meta_subgraphs` is set.
4. For each canonical-form group, call `consider_naming`.
5. Return the ordered `(canonical_form, decision)` list.

### Idempotence (via participant-set dedup)

Two consecutive `run_naming_pass` calls under the default policy
produce the same named patterns on the first call and no new
patterns *or instances* on the second. Two mechanisms together keep
this hold:

- `skip_meta_subgraphs` excludes subgraphs that touch meta-R tokens
  (the pattern-registry artifact filter from ADR 0011).
- Dedup: before calling `consider_naming` on a candidate group,
  `run_naming_pass` checks each subgraph's participant set against
  existing instances of any matching pattern (by canonical form). A
  candidate whose participant set already matches a recorded
  instance is dropped. If every candidate is dropped, the group
  returns `NamingDecision::Skipped(SkipReason::AlreadyKnown)`.

Dedup by participant set is semantically correct: two subgraphs
with the same participants and the same canonical form *are* the
same instance, given that the participants' induced subgraph in the
RSet is deterministic.

Turn `skip_meta_subgraphs` off to explore patterns-of-patterns
explicitly. Dedup remains on regardless — it is a correctness
property, not a tunable.

## Alternatives considered

- **MDL-based relevance filter.** Deferred. MDL needs a coding
  scheme for R instances and a counterfactual "without this pattern"
  comparison — both nontrivial. A later ADR can add `min_mdl_gain`
  as a policy field without changing the rest of the API.
- **Automatic trigger** (γ runs on every `add` / on a timer). Deferred.
  Explicit invocation keeps β experiments predictable; automation is
  a separate concern from what makes a pattern worth naming.
- **Strict iteration until fixed point.** Rejected per ADR 0011: the
  encoding artifacts dominate iterations. A fixed-point loop would
  spend most of its work rediscovering encoding shapes.
- **Unified `NamingOutcome = Result<Named | Skipped, Error>`.**
  Rejected in favor of `Result<NamingDecision, PatternError>`.
  Keeps error (non-policy-driven failure) distinct from skip (policy
  said no).
- **Remove `consider_naming`** and only expose `run_naming_pass`.
  Rejected. `consider_naming` is useful for callers who already have
  a candidate group (e.g., from a domain-specific picker) and want
  the policy applied without running the whole pipeline.

## Consequences

- **β closes.** Extraction (0008), canonicalization (0009), naming
  (0010), and γ (this ADR) together implement the autonomous-
  abstraction path promised in the design notes, at a minimum-
  viable level.
- **First non-deterministic step.** `run_naming_pass` is the first
  place in v2 where a policy parameter influences output — previous
  mechanisms were deterministic functions of the RSet alone. The
  policy is explicit, inspectable, and configurable; drive is
  honest.
- **Meta-R artifact filter is the default.** This matches ADR 0011's
  recommendation and keeps `run_naming_pass` idempotent by default.
  Callers who want iteration set `skip_meta_subgraphs = false`.
- **Relevance filter is thin.** min_edges ≥ 2 and min_instances ≥ 1
  capture only the cheapest triviality judgment. Richer filters
  (MDL, cross-graph repetition, stability under perturbation) can be
  added to `NamingPolicy` without breaking the API.
- **γ does not automate trigger.** Callers still invoke γ. This is
  the remaining "drive" hole; its absence is documented rather than
  filled because v2 has no use case yet that benefits from automatic
  triggers.

## Implementation

- Source: `v2/src/lib.rs` — `NamingPolicy`, `SkipReason`,
  `NamingDecision`, `RSet::{consider_naming, run_naming_pass}`, and
  private helper `RSet::collect_meta_ids`.
- Tests: 6 new unit tests (default suppresses single-edge, low
  min_edges allows, min_instances threshold, mixed-graph full pass,
  min_instances=2 empties the pass, idempotence under default).
- Example: `v2/examples/gamma_naming_pass.rs` — default pass on the
  mixed graph plus a second-pass idempotence demonstration.
- Experiment log: `v2/logs/2026-04-23_gamma_naming_pass.log` —
  decisions recorded, comparison with the 0010 unfiltered baseline,
  idempotence confirmation.
