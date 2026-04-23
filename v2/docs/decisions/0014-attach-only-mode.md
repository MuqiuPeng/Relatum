# 0014: Attach-only mode for naming pass

Status: Accepted
Date: 2026-04-23

## Context

ADR 0013 made named meta-R queryable. The next natural capability
is: extend the registry with *new data* without introducing *new
patterns*. Typical workflow:

1. Initial dataset + `run_naming_pass(default)` — names the patterns
   the system recognizes.
2. More data arrives over time.
3. The operator wants to see whether new data fits *existing*
   patterns, without the naming pass potentially minting fresh
   pattern ids for anything previously unseen.

Currently `run_naming_pass` always creates new patterns for
unmatched canonical forms. That is correct for discovery but
inappropriate for "attach-only" workflows, where the registry is
considered stable and only new instances should be added.

This ADR adds a single boolean knob on `NamingPolicy` to toggle the
behavior, plus a new `SkipReason` variant for the outcome.

## Decision

Add `attach_only: bool` to `NamingPolicy` (default `false`):

```rust
pub struct NamingPolicy {
    pub min_edges: usize,
    pub min_instances: usize,
    pub skip_meta_subgraphs: bool,
    pub attach_only: bool,
}
```

When `attach_only` is `true`, `run_naming_pass` rejects any
candidate group whose canonical form does not match an existing
named pattern. The rejection surfaces as a new skip reason:

```rust
pub enum SkipReason {
    BelowMinEdges { edges: usize, min: usize },
    BelowMinInstances { instances: usize, min: usize },
    AlreadyKnown,
    NoMatchingPattern,  // attach-only rejection
}
```

`consider_naming` is unchanged — the attach-only check is evaluated
in `run_naming_pass` before invoking `consider_naming`, since the
pass has the full RSet view and can do the lookup cheaply.

Default policy keeps `attach_only = false` to preserve the
discovery-by-default behavior from ADR 0012.

## Alternatives considered

- **Separate `run_attach_pass` method.** Rejected. A single `run_naming_pass`
  with a policy knob is more composable: callers that need both
  discovery and attach-only can flip one field instead of maintaining
  two separate entry points. Policy-as-data also reads better in
  logs (the policy is part of the outcome record).
- **Add a `retracts_allowed` or `freeze_registry` field.** Same
  effect under a different name; `attach_only` is active voice and
  matches the "attach new instances" terminology in the docs.
- **Return a new `AttachOutcome` enum instead of reusing
  `NamingDecision`.** Rejected. The outcomes line up with existing
  Named / Skipped cases; the new `NoMatchingPattern` is exactly the
  missing skip reason. No parallel taxonomy needed.
- **Detect "new pattern would be created" at `consider_naming` level.**
  Rejected. `consider_naming` doesn't currently look up canonical
  matches (that's inside `name_pattern_instances`). Keeping the
  attach-only logic in `run_naming_pass`, where canonical-form
  grouping already happens, avoids duplicating work.

## Consequences

- **Discovery and attach stay orthogonal.** A caller who wants
  attach-only just sets the flag; the rest of the pipeline is
  unchanged. Two successive passes — one discovery, one attach —
  form a natural workflow: explore, then freeze.
- **Re-running under `attach_only = true` is also idempotent.**
  The participant-set dedup from ADR 0012 still applies; an
  attach-only second pass on unchanged data is all AlreadyKnown
  and NoMatchingPattern.
- **The registry becomes a stable artifact.** Once named, patterns
  persist indefinitely in the RSet. Attach-only mode lets callers
  grow the RSet without worrying about accidental new-pattern
  creation — useful for long-running experiments where the
  investigator has decided on a fixed vocabulary.
- **Attach-only does not answer "is this data classifiable?"**
  It answers "which canonical forms have matching named patterns
  and should have instances added." Classification of a specific
  subgraph is still the job of ADR 0013's `classify_subgraph`.
- **Known limit: compound-class fragmentation on asymmetric
  structures.** The attach-only pass inherits
  `compound_class_subgraphs` as its enumeration strategy. Any
  connected structure whose edges do NOT all share the same
  compound fingerprint cannot be detected as a single subgraph —
  it fragments across compound classes. Cyclic and symmetric
  structures survive (every edge has the same endpoint-profile
  pair). Asymmetric structures — notably chains and trees — may
  fragment when the new data's identifiers don't share profiles
  with already-named participants. The experiment log for this
  ADR demonstrates a fresh 2-chain failing to attach to the
  existing 2-chain pattern for exactly this reason. Fixing this
  requires subgraph *matching* (enumerate connected subgraphs of
  the pattern's size, canonicalize, compare) rather than subgraph
  *discovery*. That is ADR 0015 territory.

## Implementation

- Source: `v2/src/lib.rs` — one new field on `NamingPolicy`, one new
  `SkipReason` variant, a short branch inside `run_naming_pass`.
- Tests: 3 new unit tests — attach-only rejects novel canonical
  forms, attach-only admits matching ones, attach-only is
  idempotent after the same pass.
- Example: `v2/examples/attach_only.rs` — run discovery pass, add
  new edges, run attach-only pass, show decisions.
- Experiment log: `v2/logs/2026-04-23_attach_only.log`.
