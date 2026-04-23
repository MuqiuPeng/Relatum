# 0020: Pattern retraction

Status: Accepted
Date: 2026-04-23

## Context

Every ADR from 0010 through 0019 has been *additive* with respect to
the named-pattern registry: patterns get created, instances get
appended, but nothing is ever removed. For experimentation —
"try naming under this policy, see if it compresses, roll back if not"
— this is a one-way door.

ADR 0020 opens the door in the other direction: `retract_pattern`
removes a named pattern and all of its meta-R (registry entry,
ownership edges, participant edges), leaving data edges untouched.

This is pure housekeeping — no new semantic commitment. It simply
makes the registry edit-able in both directions.

## Decision

### API

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetractionError {
    UnknownPattern,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetractionSummary {
    pub pattern_id: String,
    pub instances_removed: usize,
    pub meta_edges_removed: usize,
}

impl RSet {
    /// Remove a single R instance from the RSet. Returns true if the
    /// edge was present. Primitive used by retraction and any future
    /// maintenance ops.
    pub fn remove(&mut self, r: &R) -> bool;

    /// Retract a named pattern. Removes:
    ///   - the registry edge R(PATTERN_MARKER, pattern_id)
    ///   - every ownership edge R(pattern_id, instance_id)
    ///   - every participant edge R(instance_id, participant_id)
    /// Data edges are NOT touched.
    pub fn retract_pattern(
        &mut self,
        pattern_id: &str,
    ) -> Result<RetractionSummary, RetractionError>;
}
```

### Algorithm

`retract_pattern(pattern_id)`:

1. If `pattern_id` is not in `self.patterns()`, return
   `Err(UnknownPattern)`.
2. Collect instance ids: `self.instances_of(pattern_id)` (owned).
3. For each instance id, remove every `R(instance_id, *)` edge.
4. Remove every `R(pattern_id, instance_id)` ownership edge.
5. Remove the registry edge `R(PATTERN_MARKER, pattern_id)`.
6. Return a summary with counts.

The removal order — participants → ownership → registry — is defensive:
even if interrupted partway, the RSet stays in a form that queries
(e.g., `instances_of`) still return consistent results.

### Scope restriction

This ADR does **not** cascade. If future patterns are named in terms
of other patterns (not yet the case — patterns currently reference
only data identifiers via participants), removing a pattern that
another pattern depends on would leave the dependent with dangling
references. A future ADR can add cascading semantics when the need
arises.

## Alternatives considered

- **Soft delete** (mark retracted but keep edges). Rejected — adds
  registry state for a use case (undo) that isn't yet demonstrated.
  Hard delete is simpler and reversible by re-naming.
- **Return just the edge count instead of a summary struct.**
  Rejected — a summary lets logs and tests assert on the individual
  components (instances removed, edges removed) rather than
  reconstructing them from deltas.
- **Remove participant identifiers from `identifiers()` if they have
  no other edges.** Not applicable — `identifiers()` is derived from
  R instances on the fly; once edges are removed, the identifier is
  gone from the query automatically.
- **Refuse to retract if any instance is referenced by another
  pattern.** Deferred; no such cross-reference exists in the current
  encoding. Will revisit if hierarchical patterns are introduced.
- **Rename `remove` to `retract_instance` or similar.** Rejected —
  `remove` is the natural dual of `add`, usable for any R instance
  (not just pattern-metadata). Keeping it minimal.

## Consequences

- **Registry becomes bidirectional.** Users can try a naming,
  inspect, and undo. Enables experimentation workflows.
- **`remove` is a generally useful primitive.** Future maintenance
  work (pattern cascading, data cleanup) can reuse it.
- **Retraction leaves id gaps.** `mint_pattern_id` scans upward from
  `self.patterns().len()` looking for an identifier not already in
  the RSet. After retracting `p_1` from a registry containing
  `{p_0, p_1, p_2, p_3}`, the next minting starts at `n=3` (three
  patterns remain), finds `p_3` taken, and mints `p_4`. The slot
  `p_1` is simply vacant. Ids are tokens; gaps are harmless. A
  future ADR could change mint to fill gaps if anyone cares, but
  nothing depends on contiguity today.
- **Autonomous pass behavior unchanged.** An autonomous pass after
  retraction can re-discover the same canonical and re-name it;
  idempotence holds per the existing semantics (dedup by
  participant set ensures no duplicate instances even across
  retract/re-name cycles).

## Implementation

- Source: `v2/src/lib.rs` — `RSet::remove`, `RetractionError`,
  `RetractionSummary`, `RSet::retract_pattern`.
- Tests: 5 new unit tests — remove one edge, retract existing
  pattern (summary correct, data edges intact, classify returns
  None), retract non-existent errors, retract doesn't affect
  other patterns, post-retraction autonomous_pass can re-discover.
- Example: `v2/examples/pattern_retraction.rs` — create patterns,
  retract one, verify registry state and data preserved.
- Experiment log: `v2/logs/2026-04-23_pattern_retraction.log`.
