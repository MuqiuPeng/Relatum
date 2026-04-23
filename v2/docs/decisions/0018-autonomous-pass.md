# 0018: Autonomous pass — close the abstraction loop

Status: Accepted
Date: 2026-04-23

## Context

Four mechanisms sit ready to be composed:

- **ADR 0016** `discover_motifs` — sample candidates from the data,
  score by canonical-form frequency.
- **ADR 0017** `refine_candidates` — polish representatives toward
  clean instances.
- **ADR 0015** `find_instances_of` — enumerate all clean instances of
  a given canonical.
- **ADR 0010** `name_pattern_instances` — record a pattern as meta-R.

Composed, they implement the design-notes goal: *a system that,
under intrinsic drive, proposes new relational types from data and
records them.* ADR 0018 is that composition — the first mechanism in
v2 where nothing external (not even a specific canonical form) is
required. The system samples, refines, and names.

## Decision

### API

```rust
pub struct AutonomousConfig {
    pub discovery: DiscoveryConfig,
    pub refinement: RefinementConfig,
    pub naming: NamingPolicy,
}

#[derive(Debug, Clone)]
pub enum AutonomousSkip {
    /// No clean instance of this canonical exists in the data.
    NoCleanInstance,
    /// Naming policy filtered the candidate out.
    PolicyFiltered(SkipReason),
}

#[derive(Debug, Clone)]
pub enum AutonomousOutcome {
    NewPattern {
        pattern_id: String,
        instance_count: usize,
        canonical: CanonicalForm,
    },
    Existing {
        pattern_id: String,
        canonical: CanonicalForm,
    },
    Skipped {
        canonical: CanonicalForm,
        reason: AutonomousSkip,
    },
}

impl RSet {
    pub fn autonomous_pass(
        &mut self,
        config: &AutonomousConfig,
    ) -> Vec<AutonomousOutcome>;
}
```

### Algorithm

```
for each candidate from discover_motifs → refine_candidates:
    if canonical matches an existing named pattern:
        record Existing(that pattern)
        (no action — use attach_only separately if you want more instances)
    else:
        instances = find_instances_of(canonical)    # all clean instances
        if instances is empty:
            record Skipped(NoCleanInstance)
        else:
            try consider_naming(instances, policy):
                Named(pid)         -> record NewPattern(pid, |instances|, canonical)
                Skipped(reason)    -> record Skipped(PolicyFiltered(reason))
                (no other error paths are reachable by construction)
```

The output is one `AutonomousOutcome` per candidate, in the order
the discovery pass produced them. Caller can summarize by filtering
on variant.

### Separation of concerns

- Attach-only (extending existing patterns with new instances) is
  *not* part of this pass. The caller runs `run_naming_pass` with
  `attach_only=true` separately if desired. Rationale: autonomous
  pass focuses on *creating new types*; attach is about *filling in
  existing types*. Mixing them hides intent.
- Tuning: any knob that belonged to the sub-configs (sample budget,
  re-sample tries, policy thresholds) remains on the sub-configs.
  `AutonomousConfig` just groups them.

## Alternatives considered

- **Include attach-only in the same pass.** Rejected — conflates two
  conceptually distinct operations. A caller who wants both simply
  runs attach-only before or after autonomous_pass.
- **Return a single summary struct (total new, total existing,
  total skipped) instead of a Vec.** Rejected — loses per-candidate
  resolution that logs and tests need.
- **Use `NamingDecision` directly instead of `AutonomousOutcome`.**
  Rejected. `AutonomousOutcome` carries the extra distinction
  (NewPattern vs Existing) that is specific to this pass; forcing
  it into `NamingDecision` would be awkward.
- **Auto-retry under different seeds on NoCleanInstance.** Deferred.
  If a pattern's clean instances are rare, find_instances_of is
  authoritative — the enumeration is exact for the current data.
  If the data genuinely lacks clean instances, sampling more won't
  create them.
- **Make the pass iterative — run until no more new patterns emerge.**
  Deferred. One pass at a time makes the semantic simpler; iterative
  operation would trigger the meta-R feedback loop (ADR 0011) and
  needs its own policy.

## Consequences

- **Autonomous abstraction loop closes.** The system can run
  `autonomous_pass(&config)` on a fresh RSet and produce named
  patterns without any user-supplied canonical forms or instance
  lists. This is the design-notes' stated goal at its lightest form.
- **Sample determinism matters more than before.** A caller that
  re-runs autonomous_pass wants reproducibility; the ADR 0017
  sorted-data-edge fix guarantees it.
- **`find_instances_of` still uses BFS enumeration.** The
  `v2_search_mode` memory notes this is philosophically at odds
  with the architecture; 0018 does not fix that. Future ADR may
  replace `find_instances_of`'s internals with
  propose-score-refine. Does not block this ADR.
- **One autonomous_pass call may name multiple patterns.** On the
  mixed graph at target_size=3, expect 3 or 4 NewPattern outcomes
  (chain, cycle, star, tree) — each novel canonical discovered by
  sampling gets named. Running again returns Existing for each.
- **Small surface area.** One new struct (`AutonomousConfig`), two
  new enums (`AutonomousSkip`, `AutonomousOutcome`), one new method
  (`autonomous_pass`). No changes to prior mechanisms.
- **Separation from `run_naming_pass`.** `run_naming_pass` (ADR
  0012/0015) operates on compound_class_subgraphs discovery (or
  attach-only matching). `autonomous_pass` operates on
  sample-based discovery. They are parallel entry points; neither
  replaces the other. Discovery via compound class remains cheaper
  for symmetric patterns when size is unknown; sample-based shines
  on asymmetric motifs at known target sizes.

## Implementation

- Source: `v2/src/lib.rs` — `AutonomousConfig`, `AutonomousSkip`,
  `AutonomousOutcome`, `RSet::autonomous_pass`.
- Tests: 4 new unit tests — empty RSet returns empty, mixed graph
  produces at least one NewPattern with size=3, re-running returns
  only Existing / Skipped (no new patterns), policy filter kicks in.
- Example: `v2/examples/autonomous_pass.rs` — one autonomous_pass
  on the canonical mixed graph, summary report, then a second pass
  to confirm idempotence.
- Experiment log: `v2/logs/2026-04-23_autonomous_pass.log`.
