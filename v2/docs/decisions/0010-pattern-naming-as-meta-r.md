# 0010: Pattern naming as meta-R instances

Status: Accepted
Date: 2026-04-23

## Context

ADR 0009's log showed that subgraph canonical form and compound
fingerprint are independent axes, and that the single RSet already
contains isomorphic subgraph instances waiting to be recognized as
one pattern. What is missing is a way to record the recognition
persistently. Commitment 3 (types are meta-R instances) and
commitment 1 (only R) constrain how that recording can happen: all
of the pattern data has to live in the same RSet, as plain R
instances.

This ADR is β step 3 of 4. It establishes the naming mechanism
itself — invoked explicitly by a caller. ADR 0011 (γ) will later
decide *which* candidate patterns are actually worth naming.

## Decision

### Encoding convention

- Reserved marker identifier: `PATTERN_MARKER = "__pattern__"`.
- Pattern identifiers: `p_0, p_1, ...` (numeric, sequential, monotone;
  derived on demand by scanning existing patterns).
- Instance identifiers: `p_N_i_0, p_N_i_1, ...`, namespaced under
  the owning pattern.
- Three R-instance shapes encode everything:
  - `R(__pattern__, p_N)` — registry entry: `p_N` is a pattern.
  - `R(p_N, p_N_i_M)` — pattern owns instance.
  - `R(p_N_i_M, id)` — instance participates identifier.

The pattern's canonical form is **not** stored. It is recoverable by
taking any instance, reading its participants, restricting the RSet's
edges to those participants, and canonicalizing the resulting
subgraph. Canonical recovery holds under the invariant that instance
participants do not acquire new edges among themselves after naming.
Enforcing or relaxing that invariant is ADR 0011's concern; this ADR
documents the invariant and lets its violation manifest as
re-classification at query time.

### API

```rust
pub const PATTERN_MARKER: &str = "__pattern__";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternError {
    EmptyInstanceList,
    EmptyInstance,
    NotIsomorphic,
}

impl RSet {
    pub fn name_pattern_instances(
        &mut self,
        instances: &[Subgraph],
    ) -> Result<String, PatternError>;

    pub fn patterns(&self) -> Vec<&str>;
    pub fn instances_of(&self, pattern: &str) -> Vec<&str>;
    pub fn participants_of(&self, instance: &str) -> HashSet<&str>;
    pub fn find_pattern_matching(
        &self,
        canon: &CanonicalForm,
    ) -> Option<&str>;
}
```

`name_pattern_instances` rejects empty lists and empty subgraphs,
verifies pairwise isomorphism against the first instance, looks up
or mints a pattern id (skipping collision with any existing
identifier), and writes the three-shape encoding above.

## Alternatives considered

- **Synthetic edge identifiers** (give each R instance a name,
  encode membership as `R(pattern, edge_id)`). Rejected: commitment 4
  says identity is token-based and an edge's identity is the pair
  `(x, y)`; no synthetic edge id is needed. Storing one would
  introduce a parallel identity regime.
- **Storing the canonical form in R** (materialize each canonical
  edge as its own `R(pattern, canon_k)` triple). Rejected: introduces
  a second "synthetic canonical edge" identifier layer and grows
  linearly with pattern complexity. Recovery by `Subgraph::from_edges
  + canonicalize` is cheaper and keeps R instances proportional to
  observations, not to definitions.
- **Position markers per edge** (encode edge direction via
  `R(left_of_edge, x)` / `R(right_of_edge, y)`). Rejected: three R
  instances per edge where the primary R already captures the
  information. Bloat with no compensating structure.
- **Two-layer RSet** (separate data and meta stores). Rejected —
  commitment 3 explicitly unifies them. A layered RSet would defeat
  β's purpose.
- **Canonical-form-as-pattern-name** (e.g., `p_canon_1_0_2_0`).
  Rejected as mixing structure into identity; commits too early to
  a stringification of canonical forms and erodes "meaning from
  usage."

## Consequences

- **Feedback loop on every observation layer.** After naming, each
  participant identifier has additional in-edges from its instance
  id; `IdentifierProfile`, `Signature`, `RSignature`,
  `LocalityProfile`, `EdgeFingerprint`, `compound_class_subgraphs`,
  and even `Subgraph::canonicalize` applied to subgraphs spanning
  the pattern tokens all return different values post-naming. This
  is intended under commitment 3; ADR 0011 will decide whether to
  re-run the pipeline against the enlarged RSet and discover
  patterns-of-patterns.
- **Recovery invariant.** Canonical-form recovery assumes instance
  participants acquire no new edges among themselves between naming
  and querying. A violation does not corrupt the RSet; it means
  `find_pattern_matching` may fail to match an "old" canonical form
  that no longer describes the current edges-among-participants. An
  acceptable cost for not storing the canonical explicitly; ADR 0011
  is the place to decide whether to enforce stability or to add
  a stored-canonical fallback.
- **Commitment 4 pragmatism.** Reserving `__pattern__`, `p_N`, and
  `p_N_i_M` is a naming discipline, not an ontological exception.
  Token identity is unchanged: a caller that happened to feed the
  string `p_0` as a user identifier would, under commitment 4, be
  the same object as a minted pattern id. The collision guard in
  `name_pattern_instances` reduces — does not eliminate — that
  hazard.
- **Cost.** Naming N instances adds
  `len(patterns_new) + len(instances_new) + Σ |participants|` R
  instances. For the ADR 0009 mixed graph this is a modest increase.

## Implementation

- Source: `v2/src/lib.rs` — `PATTERN_MARKER`, `PatternError`,
  `RSet::{name_pattern_instances, patterns, instances_of,
  participants_of, find_pattern_matching}`, 8 unit tests.
- Example: `v2/examples/pattern_naming.rs` — runs the full
  0007 → 0008 → 0009 pipeline on the ADR 0007 mixed graph, names
  every canonical class, and dumps the resulting RSet.
- Experiment log: `v2/logs/2026-04-23_pattern_naming.log` — before /
  after RSet sizes, full registry view, and observations on the
  feedback-loop implications for ADR 0011.
