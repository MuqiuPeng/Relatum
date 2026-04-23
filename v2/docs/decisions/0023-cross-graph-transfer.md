# 0023: Cross-graph pattern transfer

Status: Accepted
Date: 2026-04-23

## Context

Patterns named in one RSet can describe structure in another RSet.
A user who explored graph A and learned that 3-chains and 3-stars
recur there might want to ask: *does graph B contain those same
structures?* Today this requires manually extracting canonical
forms and re-running the pipeline against B — mechanically simple
but ceremony-heavy.

ADR 0023 makes the transfer a two-call flow:

```
let library = rs_a.canonical_library();
let outcomes = rs_b.attach_canonicals(&library, &policy);
```

The library is a `Vec<CanonicalForm>` — the portable abstraction,
independent of either RSet's identifiers. Applying it to a new RSet
finds clean instances of each canonical and names them per policy.

## Decision

```rust
impl RSet {
    /// All named canonical forms in this RSet, recovered via each
    /// pattern's first instance. ADR 0023. The vector is a
    /// portable pattern library: canonicals are identifier-free
    /// (they use WL integer labels), so applying this library to a
    /// different RSet is semantically meaningful.
    pub fn canonical_library(&self) -> Vec<CanonicalForm>;

    /// For each canonical in `library`, run the same find-instances
    /// + name pipeline as `autonomous_pass` uses internally.
    /// Returns the usual per-canonical outcome stream. ADR 0023.
    pub fn attach_canonicals(
        &mut self,
        library: &[CanonicalForm],
        policy: &NamingPolicy,
    ) -> Vec<AutonomousOutcome>;
}
```

`attach_canonicals` algorithm per canonical:

1. If the canonical already matches a named pattern in `self` →
   `AutonomousOutcome::Existing`.
2. Else find clean instances via `find_instances_of`.
3. If no clean instances → `AutonomousOutcome::Skipped(NoCleanInstance)`.
4. Else apply policy via `consider_naming`:
   - Named → `NewPattern`.
   - Skipped(reason) → `Skipped(PolicyFiltered(reason))`.

Same outcome enum as `autonomous_pass` (ADR 0018) so callers that
already handle `AutonomousOutcome` don't need new code paths.

## Alternatives considered

- **Serialize the whole pattern registry (ids, instance ids,
  participant ids) and rebuild it in RSet B.** Rejected. Pattern
  ids and instance ids are internal-to-source-RSet tokens; carrying
  them across would either collide with B's tokens or force a
  rewrite. Carrying only canonicals keeps the library portable.
- **Extract the registry as a richer structure (frequency,
  metadata).** Deferred. `CanonicalForm` alone suffices for the
  core transfer; frequency / history metadata can be layered on
  later without changing the API shape.
- **Apply the library via `run_naming_pass(attach_only=true)`
  combined with some injection.** Rejected. `attach_only` iterates
  *registered* patterns; a library is not yet registered. The
  `attach_canonicals` API is explicit about what it does.

## Consequences

- Patterns become **portable artifacts**. Export from one RSet,
  import to another. Enables transfer-learning-style workflows.
- Library serialization is trivial: `Vec<CanonicalForm>` is
  `Vec<Vec<(u32, u32)>>` — JSON-encodable or similar with no custom
  format. (This ADR does not add a serialization format; any format
  works.)
- Canonical recovery in `canonical_library` reuses the ADR 0010
  invariant: each pattern's canonical is reconstructible from its
  first instance. If a pattern's instance participants have been
  corrupted since naming, the recovered canonical is whatever the
  current edges yield — "garbage in, garbage out" by design.
- `attach_canonicals` on an empty library or an RSet with no
  matching data is a clean no-op.
- The API allows intentional structural hypothesis testing:
  "I believe graph B contains 3-chains; let's check" via
  `attach_canonicals(&[chain_canonical], &policy)`.

## Implementation

- Source: `v2/src/lib.rs` — `RSet::canonical_library`,
  `RSet::attach_canonicals`.
- Tests: 4 new — library extraction is round-trippable (import
  back into source gives all Existing), applied to empty RSet is
  all-NoCleanInstance, applied to matching RSet names appropriately,
  applied twice is idempotent.
- Example: `v2/examples/cross_graph_transfer.rs` — build two
  unrelated graphs that happen to share structural motifs, extract
  library from one, apply to the other.
- Experiment log: `v2/logs/2026-04-23_cross_graph_transfer.log`.
